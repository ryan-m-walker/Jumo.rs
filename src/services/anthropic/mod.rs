use eventsource_stream::Eventsource;
use futures::StreamExt;
use indexmap::IndexMap;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    database::models::{
        log::LogLevel,
        message::{Message, MessageContent, MessageType},
    },
    events::{
        AppEvent, LLMMessageCompletedEventPayload, LLMMessageDeltaEventPayload,
        LLMMessageStartedEventPayload, LogEventPayload,
    },
    prompts::system::SystemPrompt,
    services::anthropic::types::{
        ClaudeInput, ClaudeMessage, ContentBlock, Delta, Role, StreamEvent,
    },
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

    pub async fn send_message(
        &mut self,
        message: &str,
        messages: &[Message],
    ) -> Result<(), anyhow::Error> {
        let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") else {
            panic!("ANTHROPIC_API_KEY is not set");
        };

        let message_id = Uuid::new_v4().to_string();
        let payload = LLMMessageStartedEventPayload {
            message_id: message_id.clone(),
        };

        self.event_sender
            .send(AppEvent::LLMMessageStarted(payload))
            .await?;

        let mut claude_messages = vec![];
        // let mut assistant_messages_grouped = vec![];

        for message in messages {
            if let MessageContent::Assistant { text } = &message.content {
                claude_messages.push(ClaudeMessage {
                    role: Role::Assistant,
                    content: if text.is_empty() {
                        "<empty>".to_string()
                    } else {
                        text.clone()
                    },
                });
            }

            if let MessageContent::User { text } = &message.content {
                // if !assistant_messages_grouped.is_empty() {
                //     assistant_messages_grouped = vec![];
                // }

                claude_messages.push(ClaudeMessage {
                    role: Role::User,
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
            role: Role::User,
            content: message.to_string(),
        });

        let system = SystemPrompt::get();

        let body = ClaudeInput {
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
                    .send(AppEvent::LLMRequestFailed(message))
                    .await
                    .unwrap();
                return;
            };

            if !resp.status().is_success() {
                let text = resp.text().await.unwrap();

                event_sender
                    .send(AppEvent::LLMRequestFailed(text))
                    .await
                    .unwrap();

                return;
            }

            let mut stream = resp.bytes_stream().eventsource();

            let mut outputs: IndexMap<usize, ContentBlock> = IndexMap::new();

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
                            Ok(StreamEvent::ContentBlockDelta { delta, index }) => match delta {
                                Delta::Text { text } => {
                                    let output = outputs.get_mut(&index);

                                    if let Some(ContentBlock::Text { text: t }) = output {
                                        t.push_str(&text);
                                    }

                                    let payload = LLMMessageDeltaEventPayload {
                                        message_id: message_id.clone(),
                                        text: text.to_string(),
                                    };

                                    event_sender
                                        .send(AppEvent::LLMMessageDelta(payload))
                                        .await
                                        .unwrap();
                                }
                                Delta::Thinking { text } => {
                                    let output = outputs.get_mut(&index);

                                    if let Some(ContentBlock::Thinking { content: t }) = output {
                                        t.push_str(&text);
                                    }
                                }
                                Delta::InputJson { partial_json } => {
                                    let output = outputs.get_mut(&index);

                                    if let Some(ContentBlock::ToolUse { input, .. }) = output {
                                        input.push_str(&partial_json);
                                    }
                                }
                                _ => {}
                            },
                            Ok(StreamEvent::ContentBlockStart {
                                index,
                                content_block,
                            }) => match content_block {
                                ContentBlock::Text { text } => {
                                    outputs.insert(index, ContentBlock::Text { text });
                                }
                                ContentBlock::ToolUse { id, name, input } => {
                                    outputs
                                        .insert(index, ContentBlock::ToolUse { id, name, input });
                                }
                                ContentBlock::Thinking { content } => {
                                    outputs.insert(index, ContentBlock::Thinking { content });
                                }
                            },
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

            let mut text_buffer = String::new();
            let mut output_messages: Vec<Message> = vec![];

            for output in outputs.values() {
                match output {
                    ContentBlock::Text { text } => {
                        output_messages.push(Message {
                            id: Uuid::new_v4().to_string(),
                            message_type: MessageType::Assistant,
                            content: MessageContent::Assistant { text: text.clone() },
                            created_at: None,
                        });

                        text_buffer.push_str(text);
                    }
                    ContentBlock::ToolUse { input, name, .. } => {
                        let result = ToolType::execute_tool(name, input, event_sender.clone())
                            .await
                            .unwrap();

                        output_messages.push(Message {
                            id: Uuid::new_v4().to_string(),
                            message_type: MessageType::ToolCall,
                            content: MessageContent::ToolCall {
                                id: Uuid::new_v4().to_string(),
                                name: name.clone(),
                                input: input.clone(),
                            },
                            created_at: None,
                        });

                        output_messages.push(Message {
                            id: Uuid::new_v4().to_string(),
                            message_type: MessageType::ToolResult,
                            content: MessageContent::ToolResult {
                                tool_use_id: Uuid::new_v4().to_string(),
                                content: result,
                            },
                            created_at: None,
                        })
                    }
                    _ => {}
                }
            }

            let payload = LLMMessageCompletedEventPayload {
                message_id: message_id.clone(),
                full_text: text_buffer,
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
