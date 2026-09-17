use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
const OLLAMA_URL: &str = "http://localhost:11434";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolCall {
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Value,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
            ..Default::default()
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            ..Default::default()
        }
    }
    pub fn tool(name: &str, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
            tool_name: Some(name.into()),
            ..Default::default()
        }
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a Value>,
    stream: bool,
    options: Value,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChatResponse {
    message: Message,
}
#[derive(Deserialize)]
struct StreamChunk {
    #[serde(default)]
    message: Message,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    error: Option<String>,
}

pub struct Ollama {
    http: reqwest::Client,
    model: String,
}

impl Ollama {
    pub fn new(model: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            model: model.to_string(),
        }
    }
    async fn send(
        &self,
        messages: &[Message],
        tools: Option<&Value>,
        stream: bool,
    ) -> Result<reqwest::Response> {
        let body = ChatRequest {
            model: &self.model,
            messages,
            tools,
            stream,
            options: json!({"num_ctx": 8192}),
        };

        let resp = self
            .http
            .post(format!("{OLLAMA_URL}/api/chat"))
            .json(&body)
            .send()
            .await
            .context("Failed to send request systemctl status ollama")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Ollama returned {status}: {text}");
        }
        Ok(resp)
    }
    #[allow(dead_code)]
    pub async fn chat(&self, messages: &[Message], tools: Option<&Value>) -> Result<Message> {
        let resp = self.send(messages, tools, false).await?;
        let parsed: ChatResponse = resp.json().await?;
        Ok(parsed.message)
    }

    pub async fn chat_stream(
        &self,
        messages: &[Message],
        tools: Option<&Value>,
        mut on_token: impl FnMut(&str),
    ) -> Result<Message> {
        let resp = self.send(messages, tools, true).await?;
        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        while let Some(chunk) = stream.next().await {
            buf.extend_from_slice(&chunk?);
            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=pos).collect();
                let line = line.trim_ascii();
                if line.is_empty() {
                    continue;
                }
                let part: StreamChunk = serde_json::from_slice(line)?;
                if let Some(err) = part.error {
                    bail!("Ollama error: {err}");
                }
                if !part.message.content.is_empty() {
                    on_token(&part.message.content);
                    content.push_str(&part.message.content);
                }
                tool_calls.extend(part.message.tool_calls);
                if part.done {
                    break;
                }
            }
        }

        Ok(Message {
            role: "assistant".into(),
            content,
            tool_calls,
            tool_name: None,
        })
    }
}
