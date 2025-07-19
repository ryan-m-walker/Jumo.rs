use eventsource_stream::Eventsource;
use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    database::models::{Message, MessageContent},
    events::{
        AppEvent, LLMDelta, LLMMessageCompletedPayload, LLMMessageDeltaPayload,
        LLMMessageStartedPayload,
    },
    prompts::system::SystemPrompt,
    services::anthropic::types::{ClaudeInput, ClaudeMessage, Delta, StreamEvent},
    state::Speaker,
};

pub mod types;

pub struct AnthropicService {
    event_sender: mpsc::Sender<AppEvent>,
    client: reqwest::Client,
    api_key: String,
    is_running: bool,
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
            is_running: false,
        }
    }

    pub async fn send_message(
        &mut self,
        message: &str,
        messages: &[Message],
    ) -> Result<(), anyhow::Error> {
        self.is_running = true;

        let message_id = Uuid::new_v4().to_string();
        let payload = LLMMessageStartedPayload {
            message_id: message_id.clone(),
        };

        self.event_sender
            .send(AppEvent::LLMMessageStarted(payload))
            .await?;

        let mut claude_messages = vec![];

        for message in messages {
            if let MessageContent::Assistant { text } = &message.content {
                claude_messages.push(ClaudeMessage {
                    role: "assistant".to_string(),
                    content: text.clone(),
                });
            }

            if let MessageContent::User { text } = &message.content {
                claude_messages.push(ClaudeMessage {
                    role: "user".to_string(),
                    content: text.clone(),
                });
            }
        }

        claude_messages.push(ClaudeMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        let system = SystemPrompt::new().get_prompt();

        let body = ClaudeInput {
            model: String::from("claude-sonnet-4-20250514"),
            max_tokens: 5000,
            messages: claude_messages,
            stream: true,
            system: Some(system),
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
            if !self.is_running {
                break;
            }

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
                            let payload = LLMMessageDeltaPayload {
                                message_id: message_id.clone(),
                                text: text.to_string(),
                            };

                            buffer.push_str(&text);

                            self.event_sender
                                .send(AppEvent::LLMMessageDelta(payload))
                                .await?;
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

        self.is_running = false;

        let payload = LLMMessageCompletedPayload {
            message_id: message_id.clone(),
            full_text: buffer,
        };

        self.event_sender
            .send(AppEvent::LLMMessageCompleted(payload))
            .await?;

        Ok(())
    }

    pub fn cancel(&mut self) {
        self.is_running = false;
    }
}
