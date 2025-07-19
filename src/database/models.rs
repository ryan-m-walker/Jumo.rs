use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MessageType {
    User,
    Assistant,
    Error,
    ToolCall,
    ToolResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    User {
        text: String,
    },
    Assistant {
        text: String,
    },
    Error {
        text: String,
    },
    ToolCall {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub message_type: MessageType,
    pub content: MessageContent,
    pub created_at: Option<String>,
}

impl Message {
    pub fn update() {
        let mut messages: Vec<Message> = vec![];

        let delta_id = "delta id";
        let new_text = "new text";

        for message in messages.iter_mut() {
            if let MessageContent::Assistant { text } = &mut message.content {
                if message.id == delta_id {
                    text.push_str(new_text);
                }
            }
        }
    }
}
