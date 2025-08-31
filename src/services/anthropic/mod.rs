use eventsource_stream::Eventsource;
use futures::StreamExt;
use mongodb::bson::oid::ObjectId;
use tokio::sync::mpsc;

use crate::{
    config::CONFIG,
    events::{
        AppEvent, LLMGenerationCompletedEventPayload, LLMGenerationStartedEventPayload,
        LLMStreamEventPayload, LogEventPayload,
    },
    features::Features,
    prompts::get_system_prompt,
    services::anthropic::types::{AnthropicInput, AnthropicMessage, AnthropicMessageStreamEvent},
    state::AppState,
    tools::tools::ToolType,
    types::{
        logs::LogLevel,
        message::{ContentBlock, Message},
    },
};

pub mod types;

pub struct AnthropicService {
    event_sender: mpsc::Sender<AppEvent>,
}

impl AnthropicService {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self { event_sender }
    }

    pub fn prompt(&mut self, input: &Message, messages: &[Message], state: &AppState) {
        let message_id = ObjectId::new();

        let start_payload = LLMGenerationStartedEventPayload { message_id };

        let mut claude_messages = vec![];

        let process_message = |message: &Message| {
            let role = message.role;
            let content = message
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
                .filter(|content| {
                    if let ContentBlock::Image { .. } = content {
                        if !Features::video_capture_enabled() {
                            return false;
                        }
                    }

                    true
                })
                .collect();

            AnthropicMessage { role, content }
        };

        for message in messages {
            claude_messages.push(process_message(message));
        }

        claude_messages.push(process_message(input));

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
            let _ = event_sender
                .send(AppEvent::LLMGenerationStarted(start_payload))
                .await;

            let send_error = async |message: &str| {
                let _ = event_sender
                    .send(AppEvent::LLMGenerationError(message.to_string()))
                    .await;
            };

            let client = reqwest::Client::new();

            let resp = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &CONFIG.anthropic_api_key)
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
    }

    pub fn cancel(&mut self) {
        // TODO: cancel requests
    }
}
