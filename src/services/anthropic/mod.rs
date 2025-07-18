use eventsource_stream::Eventsource;
use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    events::{AppEvent, LLMDelta},
    prompts::system::SystemPrompt,
    services::anthropic::types::{ClaudeInput, ClaudeMessage, Delta, StreamEvent},
    state::{Speaker, TranscriptLine},
};

pub mod types;

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
        let message_id = Uuid::new_v4().to_string();
        self.event_sender
            .send(AppEvent::LLMMessageStarted(message_id.clone()))
            .await?;

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

        let system = SystemPrompt::new().get_prompt();

        let body = ClaudeInput {
            model: String::from("claude-sonnet-4-20250514"),
            max_tokens: 1024,
            messages,
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
                            let delta = LLMDelta {
                                id: message_id.clone(),
                                text: text.to_string(),
                            };

                            self.event_sender
                                .send(AppEvent::LLMTextDelta(delta))
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

        self.event_sender
            .send(AppEvent::LLMMessageCompleted(message_id))
            .await?;

        Ok(())
    }
}
