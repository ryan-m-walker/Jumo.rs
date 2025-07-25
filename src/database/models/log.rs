use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::models::Model;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub id: String,
    pub text: String,
    pub level: LogLevel,
    pub timestamp: String,
}

impl Log {
    pub fn new(text: &str, level: LogLevel) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            text: text.to_string(),
            level,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl Model for Log {
    fn init_table(connection: &Connection) -> Result<(), anyhow::Error> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS logs (
                id TEXT PRIMARY KEY NOT NULL,
                text TEXT NOT NULL,
                level TEXT NOT NULL CHECK (level IN ('info', 'warn', 'error')),
                timestamp TEXT NOT NULL
            )",
            (),
        )?;

        Ok(())
    }
}
