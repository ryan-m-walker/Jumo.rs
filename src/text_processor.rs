use std::mem::take;

use tokio::sync::mpsc;

use crate::{
    emote::{get_color, get_emote},
    events::{AppEvent, TextProcessorChunkEventPayload},
};

pub struct TextProcessor {
    event_sender: mpsc::Sender<AppEvent>,
    pending_chunk: String,
}

impl TextProcessor {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self {
            event_sender,
            pending_chunk: String::new(),
        }
    }

    pub async fn process_delta(&mut self, delta: &str) -> Result<(), anyhow::Error> {
        let mut chars = delta.chars().peekable();
        let mut buffer = String::new();

        while let Some(c) = chars.next() {
            buffer.push(c);

            if let Some(emote) = get_emote(c) {
                self.event_sender.send(AppEvent::SetEmote(emote)).await?;
            }

            if let Some(color) = get_color(c) {
                self.event_sender.send(AppEvent::SetColor(color)).await?;
            }

            if matches!(c, '.' | '!' | '?' | ';' | '\n') {
                if let Some(next_c) = chars.peek() {
                    if next_c.is_whitespace() {
                        let mut pending_chunk = take(&mut self.pending_chunk);
                        pending_chunk.push_str(&buffer);

                        let payload = TextProcessorChunkEventPayload {
                            text: pending_chunk,
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
            self.pending_chunk.push_str(&buffer);
        }

        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), anyhow::Error> {
        if !self.pending_chunk.is_empty() {
            let payload = TextProcessorChunkEventPayload {
                text: self.pending_chunk.clone(),
                flush: true,
            };

            self.event_sender
                .send(AppEvent::TextProcessorTextChunk(payload))
                .await?;
            self.event_sender
                .send(AppEvent::TextProcessorFlushed)
                .await?;

            self.pending_chunk.clear();
        }

        Ok(())
    }
}
