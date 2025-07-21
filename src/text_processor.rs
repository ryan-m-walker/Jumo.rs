use tokio::sync::mpsc;

use crate::events::{AppEvent, LLMDelta, TextProcessorChunkPayload};

pub struct TextProcessor {
    event_sender: mpsc::Sender<AppEvent>,
}

impl TextProcessor {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self { event_sender }
    }

    pub async fn process_delta(&mut self, delta: &str) -> Result<(), anyhow::Error> {
        let mut chars = delta.chars().peekable();
        let mut buffer = String::new();

        while let Some(c) = chars.next() {
            buffer.push(c);

            if matches!(c, '.' | '!' | '?' | ';' | '\n') {
                if let Some(next_c) = chars.peek() {
                    if next_c.is_whitespace() {
                        let payload = TextProcessorChunkPayload {
                            text: buffer.trim().to_string(),
                            flush: true,
                        };

                        self.event_sender
                            .send(AppEvent::TextProcessorTextChunk(payload))
                            .await?;
                        buffer.clear();
                    }
                }
            }
        }

        if !buffer.trim().is_empty() {
            let payload = TextProcessorChunkPayload {
                text: buffer.trim().to_string(),
                flush: false,
            };

            self.event_sender
                .send(AppEvent::TextProcessorTextChunk(payload))
                .await?;
        }

        Ok(())
    }
}
