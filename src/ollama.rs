use anyhow::{Context, Result, bail};
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
struct ChatResponse {
    message: Message,
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
    pub async fn chat(&self, messages: &[Message], tools: Option<&Value>) -> Result<Message> {
        let resp = self.send(messages, tools, false).await?;
        let parsed: ChatResponse = resp.json().await?;
        Ok(parsed.message)
    }
}
