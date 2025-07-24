use serde::{Deserialize, Serialize};

use crate::{
    database::models::message::{ContentBlock, Role},
    tools::ToolInput,
};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AnthropicMessage {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Serialize)]
pub struct AnthropicInput {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<AnthropicMessage>,
    pub stream: bool,
    pub system: Option<String>,
    pub tools: Vec<ToolInput>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AnthropicMessageStreamEvent {
    MessageStart {
        message: AnthropicMessage,
    },
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: AnthropicContentBlockDelta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: AnthropicMessageDelta,
    },
    MessageStop,
    Ping,
    Error {
        error: AnthropicErrorInfo,
    },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum AnthropicContentBlockDelta {
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

#[derive(Debug, Deserialize, Clone)]
pub struct AnthropicMessageDelta {
    pub stop_reason: Option<String>,
    pub usage: Option<AnthropicUsage>,
}

/// Billing and rate-limit usage.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnthropicUsage {
    /// The number of input tokens which were used.
    pub input_tokens: usize,

    /// The number of output tokens which were used.
    pub output_tokens: usize,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize)]
pub struct AnthropicErrorInfo {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}
