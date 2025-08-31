use mongodb::bson::{DateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub _id: ObjectId,
    pub text: String,
    pub level: LogLevel,
    pub created_at: DateTime,
}

impl Log {
    pub fn new(text: &str, level: LogLevel) -> Self {
        Self {
            _id: ObjectId::new(),
            text: text.to_string(),
            level,
            created_at: DateTime::now(),
        }
    }
}
