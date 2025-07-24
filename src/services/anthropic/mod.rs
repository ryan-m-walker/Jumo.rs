use eventsource_stream::Eventsource;
use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    database::models::{
        log::LogLevel,
        message::{ContentBlock, Message, Role},
    },
    events::{
        AppEvent, LLMGenerationCompletedEventPayload, LLMGenerationStartedEventPayload,
        LLMStreamEventPayload, LogEventPayload,
    },
    prompts::system::SystemPrompt,
    services::anthropic::types::{AnthropicInput, AnthropicMessage, AnthropicMessageStreamEvent},
    tools::tools::ToolType,
};

pub mod types;

pub struct AnthropicService {
    event_sender: mpsc::Sender<AppEvent>,
}

impl AnthropicService {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self { event_sender }
    }

    pub async fn prompt(&mut self, messages: &[Message]) -> Result<(), anyhow::Error> {
        let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") else {
            panic!("ANTHROPIC_API_KEY is not set");
        };

        let message_id = Uuid::new_v4().to_string();

        let start_payload = LLMGenerationStartedEventPayload {
            message_id: message_id.clone(),
        };

        self.event_sender
            .send(AppEvent::LLMGenerationStarted(start_payload))
            .await?;

        let mut claude_messages = vec![];

        for message in messages {
            claude_messages.push(AnthropicMessage {
                role: message.role,
                content: message.content.clone(),
            });
        }

        // self.event_sender
        //     .send(AppEvent::Log(LogEventPayload {
        //         level: LogLevel::Info,
        //         message: format!("Sending message to anthropic: {message}"),
        //     }))
        //     .await?;

        let system = SystemPrompt::get();

        let body = AnthropicInput {
            model: String::from("claude-sonnet-4-20250514"),
            max_tokens: 5000,
            messages: claude_messages,
            stream: true,
            system: Some(system),
            tools: ToolType::all_tools(),
        };

        let event_sender = self.event_sender.clone();

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
                    .send(AppEvent::LLMGenerationFailed(message))
                    .await
                    .unwrap();
                return;
            };

            if !resp.status().is_success() {
                let text = resp.text().await.unwrap();

                event_sender
                    .send(AppEvent::LLMGenerationFailed(text))
                    .await
                    .unwrap();

                return;
            }

            let mut stream = resp.bytes_stream().eventsource();

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

                        let stream_event: Result<AnthropicMessageStreamEvent, _> =
                            serde_json::from_str(&event.data);

                        match stream_event {
                            Ok(event) => {
                                let payload = LLMStreamEventPayload {
                                    message_id: message_id.clone(),
                                    event,
                                };

                                event_sender
                                    .send(AppEvent::LLMStreamEvent(payload))
                                    .await
                                    .unwrap()
                            }
                            Err(err) => {
                                let data = &event.data;
                                let message = format!("LLM error: {err} -> {data}");
                                event_sender
                                    .send(AppEvent::LLMGenerationError(message))
                                    .await
                                    .unwrap()
                            }
                        }
                    }
                    Err(err) => {
                        event_sender
                            .send(AppEvent::LLMGenerationFailed(err.to_string()))
                            .await
                            .unwrap();
                    }
                }
            }

            let completed_payload = LLMGenerationCompletedEventPayload {
                message_id: message_id.clone(),
            };

            event_sender
                .send(AppEvent::LLMGenerationCompleted(completed_payload))
                .await
                .unwrap();
        });

        Ok(())
    }

    pub fn cancel(&mut self) {
        // TODO: cancel requests
    }
}
