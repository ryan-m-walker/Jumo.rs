use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    events::AppEvent,
    prompts::system::SYSTEM_PROMPT,
    state::{Speaker, TranscriptLine},
};

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeResponseMessage {
    #[serde(rename = "type")]
    message_type: String,
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeInput {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
    stream: bool,
    system: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: Message },

    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: Delta },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },

    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDelta },

    #[serde(rename = "message_stop")]
    MessageStop,

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "error")]
    Error { error: ErrorInfo },
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    #[serde(rename = "thinking")]
    Thinking { content: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Delta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },

    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },

    #[serde(rename = "thinking_delta")]
    ThinkingDelta { text: String },

    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },
}

#[derive(Debug, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub struct ErrorInfo {
    pub error_type: String,
    pub message: String,
}

pub struct AnthropicService {
    event_sender: mpsc::Sender<AppEvent>,
    client: reqwest::Client,
    api_key: String,
}

impl AnthropicService {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") else {
            panic!("ANTHROPIC_API_KEY is not set");
        };

        Self {
            event_sender,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn send_message(
        &self,
        message: &str,
        transcript: &[TranscriptLine],
    ) -> Result<(), anyhow::Error> {
        self.event_sender.send(AppEvent::LLMMessageStarted).await?;

        let mut messages = vec![];

        for line in transcript {
            if let TranscriptLine::TranscriptMessage(line) = line {
                messages.push(ClaudeMessage {
                    role: match line.speaker {
                        Speaker::User => "user".to_string(),
                        Speaker::Assistant => "assistant".to_string(),
                    },
                    content: line.text.clone(),
                });
            }
        }

        messages.push(ClaudeMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        let body = ClaudeInput {
            model: String::from("claude-sonnet-4-20250514"),
            max_tokens: 1024,
            messages,
            stream: true,
            system: Some(SYSTEM_PROMPT.to_string()),
        };

        let mut stream = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?
            .bytes_stream()
            .eventsource();

        let mut buffer = String::new();

        while let Some(event) = stream.next().await {
            match event {
                Ok(event) => {
                    if event.data.is_empty() {
                        continue;
                    }

                    let stream_event: Result<StreamEvent, _> = serde_json::from_str(&event.data);

                    match stream_event {
                        Ok(StreamEvent::ContentBlockDelta {
                            delta: Delta::TextDelta { text },
                            ..
                        }) => {
                            buffer.push_str(&text);
                        }
                        Ok(StreamEvent::MessageStop) => {
                            break;
                        }
                        Ok(StreamEvent::Error { error }) => {
                            self.event_sender
                                .send(AppEvent::LLMRequestFailed(error.message))
                                .await?;
                            break;
                        }
                        _ => {}
                    }
                }
                Err(err) => {
                    self.event_sender
                        .send(AppEvent::LLMRequestFailed(err.to_string()))
                        .await?;
                }
            }
        }

        self.event_sender
            .send(AppEvent::LLMMessageCompleted(buffer))
            .await?;

        Ok(())
    }
}
