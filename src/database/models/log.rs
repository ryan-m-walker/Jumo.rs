use rusqlite::Connection;
use serde::{Deserialize, Serialize};

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

impl Model for Log {
    fn init_table(connection: &Connection) -> Result<(), anyhow::Error> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS logs (
                id TEXT PRIMARY KEY,
                text TEXT NOT NULL,
                level TEXT NOT NULL CHECK (level IN ('info', 'warn', 'error')),
                timestamp TEXT NOT NULL
            )",
            (),
        );

        Ok(())
    }
}
