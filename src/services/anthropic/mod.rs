use eventsource_stream::Eventsource;
use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    database::models::{
        log::LogLevel,
        message::{Message, MessageContent},
    },
    events::{
        AppEvent, LLMMessageCompletedEventPayload, LLMMessageDeltaEventPayload,
        LLMMessageStartedEventPayload, LogEventPayload,
    },
    prompts::system::SystemPrompt,
    services::anthropic::types::{ClaudeInput, ClaudeMessage, Delta, StreamEvent},
};

pub mod types;

pub struct AnthropicService {
    event_sender: mpsc::Sender<AppEvent>,
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
        }
    }

    pub async fn send_message(
        &mut self,
        message: &str,
        messages: &[Message],
    ) -> Result<(), anyhow::Error> {
        let message_id = Uuid::new_v4().to_string();
        let payload = LLMMessageStartedEventPayload {
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

        self.event_sender
            .send(AppEvent::Log(LogEventPayload {
                level: LogLevel::Info,
                message: format!("Sending message to anthropic: {message}"),
            }))
            .await?;

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

        let event_sender = self.event_sender.clone();
        let api_key = self.api_key.clone();

        tokio::spawn(async move {
            let client = reqwest::Client::new();

            let resp = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await;

            let Ok(resp) = resp else {
                let message = String::from("Failed to send message to anthropic");
                event_sender
                    .send(AppEvent::LLMRequestFailed(message))
                    .await
                    .unwrap();
                return;
            };

            let mut stream = resp.bytes_stream().eventsource();

            let mut buffer = String::new();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        if event.data.is_empty() {
                            event_sender
                                .send(AppEvent::Log(LogEventPayload {
                                    level: LogLevel::Info,
                                    message: String::from("Anthropic stream ended"),
                                }))
                                .await
                                .unwrap();
                            continue;
                        }

                        let stream_event: Result<StreamEvent, _> =
                            serde_json::from_str(&event.data);

                        match stream_event {
                            Ok(StreamEvent::ContentBlockDelta {
                                delta: Delta::TextDelta { text },
                                ..
                            }) => {
                                let payload = LLMMessageDeltaEventPayload {
                                    message_id: message_id.clone(),
                                    text: text.to_string(),
                                };

                                buffer.push_str(&text);

                                event_sender
                                    .send(AppEvent::LLMMessageDelta(payload))
                                    .await
                                    .unwrap();
                            }
                            Ok(StreamEvent::MessageStop) => {
                                break;
                            }
                            Ok(StreamEvent::Error { error }) => {
                                event_sender
                                    .send(AppEvent::LLMRequestFailed(error.message))
                                    .await
                                    .unwrap();
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(err) => {
                        event_sender
                            .send(AppEvent::LLMRequestFailed(err.to_string()))
                            .await
                            .unwrap();
                    }
                }
            }

            let payload = LLMMessageCompletedEventPayload {
                message_id: message_id.clone(),
                full_text: buffer,
            };

            event_sender
                .send(AppEvent::LLMMessageCompleted(payload))
                .await
                .unwrap();
        });

        Ok(())
    }

    pub fn cancel(&mut self) {
        // TODO: cancel requests
    }
}
