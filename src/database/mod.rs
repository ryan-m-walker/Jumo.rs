use rusqlite::Connection;

use crate::database::models::{
    Model,
    log::Log,
    message::{Message, Role},
};

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
        Log::init_table(&self.connection)?;
        Message::init_table(&self.connection)?;
        Ok(())
    }

    pub fn get_messages(&self) -> Result<Vec<Message>, anyhow::Error> {
        let mut stmt = self.connection.prepare(
            "SELECT id, role, content, created_at FROM messages ORDER BY created_at DESC LIMIT 50",
        )?;

        let messages_iter = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let role_str: String = row.get(1)?;
            let content_str: String = row.get(2)?;
            let created_at: Option<String> = row.get(3)?;

            let role = match role_str.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        1,
                        "role".to_string(),
                        rusqlite::types::Type::Text,
                    ));
                }
            };

            let content = serde_json::from_str(&content_str).map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    2,
                    "content".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?;

            Ok(Message {
                id,
                role,
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
        let role_str = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let content_json = serde_json::to_string(&message.content)?;

        self.connection.execute(
            "INSERT INTO messages (id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            (&message.id, role_str, content_json, &message.created_at),
        )?;

        Ok(())
    }
}
