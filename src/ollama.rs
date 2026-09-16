use anyhow::{Context, Result,bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const  OLLAMA_URL :&str = "http://localhost:11434";

#[derive(Debug, Clone, Default, Serialize, Deserialize)];
pub struct Message {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if="Vec::is_empty)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if="Option::is_none)]
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
        Self { role: "System".into(),content: content.into(), ..Default::default() }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "User".into(),content: content.into(), ..Default::default() }
    }
    pub fn tool(name: &str, content: impl Into<String>) -> Self {
        Self { role: "tool".into(), content}
    }
}