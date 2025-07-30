use eventsource_stream::Eventsource;
use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    database::models::{
        log::LogLevel,
        message::{ContentBlock, Message},
    },
    events::{
        AppEvent, LLMGenerationCompletedEventPayload, LLMGenerationStartedEventPayload,
        LLMStreamEventPayload, LogEventPayload,
    },
    prompts::get_system_prompt,
    services::anthropic::types::{AnthropicInput, AnthropicMessage, AnthropicMessageStreamEvent},
    state::AppState,
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

    pub async fn prompt(
        &mut self,
        messages: &[Message],
        state: &AppState,
    ) -> Result<(), anyhow::Error> {
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
                content: message
                    .content
                    .iter()
                    .map(|content| match content {
                        ContentBlock::Text { text } => {
                            let text = if text.is_empty() { "<empty>" } else { text };

                            ContentBlock::Text {
                                text: text.to_string(),
                            }
                        }
                        _ => content.clone(),
                    })
                    .collect(),
            });
        }

        let system = get_system_prompt(state);

        let body = AnthropicInput {
            model: String::from("claude-sonnet-4-20250514"),
            max_tokens: 5000,
            messages: claude_messages,
            stream: true,
            system: Some(system),
            tools: ToolType::all_tools(state),
        };

        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            let send_error = async |message: &str| {
                let _ = event_sender
                    .send(AppEvent::LLMGenerationError(message.to_string()))
                    .await;
            };

            let client = reqwest::Client::new();

            let resp = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await;

            let resp = match resp {
                Ok(r) => r,
                Err(e) => {
                    send_error(&format!("Failed to send message to anthropic: {e}")).await;
                    return;
                }
            };

            if !resp.status().is_success() {
                match resp.text().await {
                    Ok(text) => {
                        send_error(&format!("Failed to send message to Anthropic: {text}")).await;
                    }
                    Err(e) => {
                        send_error(&format!("Failed to send message to Anthropic: {e}")).await;
                    }
                }
                return;
            }

            let mut stream = resp.bytes_stream().eventsource();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        if event.data.is_empty() {
                            let _ = event_sender
                                .send(AppEvent::Log(LogEventPayload {
                                    level: LogLevel::Info,
                                    message: String::from("Anthropic stream ended"),
                                }))
                                .await;

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

                                let _ = event_sender.send(AppEvent::LLMStreamEvent(payload)).await;
                            }
                            Err(err) => {
                                let data = &event.data;
                                let message = format!("LLM error: {err} -> {data}");
                                send_error(&message).await;
                            }
                        }
                    }
                    Err(err) => {
                        send_error(&format!("Failed to send message to Anthropic: {err}")).await;
                    }
                }
            }

            let completed_payload = LLMGenerationCompletedEventPayload {
                message_id: message_id.clone(),
            };

            let _ = event_sender
                .send(AppEvent::LLMGenerationCompleted(completed_payload))
                .await;
        });

        Ok(())
    }

    pub fn cancel(&mut self) {
        // TODO: cancel requests
    }
}
