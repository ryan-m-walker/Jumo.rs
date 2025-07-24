use serde::{Deserialize, Serialize};

use crate::tools::ToolInput;

#[derive(Debug, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClaudeMessageContentInput {
    Text(String),
    // TODO
    Json(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ClaudeInput {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ClaudeMessage>,
    pub stream: bool,
    pub system: Option<String>,
    pub tools: Vec<ToolInput>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: Message },

    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: Delta },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },

    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDelta },

    #[serde(rename = "message_stop")]
    MessageStop,

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "error")]
    Error { error: ErrorInfo },
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: String,
    },

    #[serde(rename = "thinking")]
    Thinking { content: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Delta {
    /// The deltas for normal text output.
    #[serde(rename = "text_delta")]
    Text { text: String },

    /// The deltas for tool_use content blocks correspond to updates for the input field of the block.
    #[serde(rename = "input_json_delta")]
    InputJson { partial_json: String },

    /// When using extended thinking with streaming enabled, you’ll receive thinking content via thinking_delta events.
    #[serde(rename = "thinking_delta")]
    Thinking { text: String },

    ///For thinking content, a special signature_delta event is sent just before the content_block_stop event. This signature is used to verify the integrity of the thinking block.
    #[serde(rename = "signature_delta")]
    Signature { signature: String },
}

#[derive(Debug, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub struct ErrorInfo {
    pub error_type: String,
    pub message: String,
}
