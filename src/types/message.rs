use mongodb::bson::{DateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    #[serde(rename = "image/jpeg")]
    JPEG,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub image_type: String,
    pub media_type: MediaType,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
    Thinking {
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub _id: ObjectId,
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub created_at: DateTime,
}
