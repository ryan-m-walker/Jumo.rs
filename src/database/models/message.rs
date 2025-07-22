use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::database::models::Model;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MessageType {
    User,
    Assistant,
    Error,
    ToolCall,
    ToolResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl Model for Message {
    fn init_table(connection: &rusqlite::Connection) -> Result<(), anyhow::Error> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                message_type TEXT NOT NULL CHECK (message_type IN ('user', 'assistant', 'error', 'tool_call', 'tool_result')),
                content TEXT NOT NULL,
                created_at TEXT
            )",
            (),
        );

        Ok(())
    }
}
