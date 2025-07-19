use rusqlite::Connection;

use crate::database::models::{Message, MessageContent, MessageType};

pub mod models;

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn new() -> Self {
        let connection = Connection::open("./data/v1.db").expect("Failed to open database");
        Self { connection }
    }

    pub fn init(&self) -> Result<(), anyhow::Error> {
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                message_type TEXT NOT NULL CHECK (message_type IN ('user', 'assistant', 'error', 'tool_call', 'tool_result')),
                content TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            (),
        )?;

        Ok(())
    }

    pub fn get_messages(&self) -> Result<Vec<Message>, anyhow::Error> {
        let mut stmt = self
            .connection
            .prepare("SELECT * FROM messages ORDER BY created_at DESC LIMIT 50")?;

        let messages_iter = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let message_type_str: String = row.get(1)?;
            let content_str: String = row.get(2)?;
            let created_at: Option<String> = row.get(3)?;

            let message_type = match message_type_str.as_str() {
                "user" => MessageType::User,
                "assistant" => MessageType::Assistant,
                "error" => MessageType::Error,
                "tool_call" => MessageType::ToolCall,
                "tool_result" => MessageType::ToolResult,
                _ => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        1,
                        "message_type".to_string(),
                        rusqlite::types::Type::Text,
                    ));
                }
            };

            let content: MessageContent = serde_json::from_str(&content_str).map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    2,
                    "content".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?;

            Ok(Message {
                id,
                message_type,
                content,
                created_at,
            })
        })?;

        let mut messages: Vec<Message> = Vec::new();

        for message in messages_iter {
            if let Ok(message) = message {
                messages.push(message);
            }
        }

        Ok(messages)
    }

    pub fn insert_message(&self, message: &Message) -> Result<(), anyhow::Error> {
        let message_type_str = match message.message_type {
            MessageType::User => "user",
            MessageType::Assistant => "assistant",
            MessageType::Error => "error",
            MessageType::ToolCall => "tool_call",
            MessageType::ToolResult => "tool_result",
        };

        let content_json = serde_json::to_string(&message.content)?;

        self.connection.execute(
            "INSERT INTO messages (id, message_type, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            (
                &message.id,
                message_type_str,
                content_json,
                &message.created_at,
            ),
        )?;

        Ok(())
    }
}
