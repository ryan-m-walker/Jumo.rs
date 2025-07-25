use rusqlite::Connection;
use uuid::Uuid;

use crate::database::models::{
    Model,
    log::{Log, LogLevel},
    message::{Message, Role},
};

pub mod models;

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn new() -> Self {
        let connection = match Connection::open("./data/v1.db") {
            Ok(connection) => connection,
            Err(err) => {
                eprintln!("Failed to open database: {err}");
                ratatui::restore();
                std::process::exit(1);
            }
        };

        Self { connection }
    }

    pub fn init(&self) -> Result<(), anyhow::Error> {
        Log::init_table(&self.connection)?;
        Message::init_table(&self.connection)?;
        Ok(())
    }

    pub fn get_messages(&self) -> Result<Vec<Message>, anyhow::Error> {
        let mut stmt = self.connection.prepare(
            "SELECT id, role, content, created_at FROM messages ORDER BY created_at ASC LIMIT 50",
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
            match message {
                Ok(message) => messages.push(message),
                Err(err) => panic!("Failed to get message: {err}"),
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

    pub fn insert_log(&self, text: &str, level: LogLevel) -> Result<(), anyhow::Error> {
        let level_str = match level {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };

        self.connection.execute(
            "INSERT INTO logs (id, text, level, timestamp) VALUES (?1, ?2, ?3, ?4)",
            (
                Uuid::new_v4().to_string(),
                &text,
                level_str,
                &chrono::Utc::now().to_rfc3339(),
            ),
        )?;

        Ok(())
    }

    pub fn get_logs(&self) -> Result<Vec<Log>, anyhow::Error> {
        let mut stmt = self.connection.prepare(
            "SELECT id, text, level, timestamp FROM logs ORDER BY timestamp DESC LIMIT 50",
        )?;

        let logs_iter = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let text: String = row.get(1)?;
            let level_str: String = row.get(2)?;
            let timestamp: String = row.get(3)?;

            let level = match level_str.as_str() {
                "info" => LogLevel::Info,
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                _ => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        2,
                        "level".to_string(),
                        rusqlite::types::Type::Text,
                    ));
                }
            };

            Ok(Log {
                id,
                text,
                level,
                timestamp,
            })
        })?;

        let mut logs: Vec<Log> = Vec::new();

        for log in logs_iter {
            match log {
                Ok(log) => logs.push(log),
                Err(err) => panic!("Failed to get log: {err}"),
            }
        }

        Ok(logs)
    }
}
