use std::mem::replace;

use tokio::{fs, io::AsyncWriteExt, sync::mpsc};

use crate::events::{AppEvent, TextProcessorChunkPayload};

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
        self.pending_chunk.push_str(delta);
        self.emit_complete_sentences().await?;
        Ok(())
    }

    pub async fn finalize(&mut self) -> Result<(), anyhow::Error> {
        if !self.pending_chunk.trim().is_empty() {
            self.emit_chunk(&self.pending_chunk).await?;
            self.pending_chunk.clear();
        }
        Ok(())
    }

    fn find_sentence_boundary(&self, text: &str) -> Option<usize> {
        let mut chars = text.char_indices().peekable();

        while let Some((i, c)) = chars.next() {
            if matches!(c, '.' | '!' | '?') {
                if let Some(&(next_i, next_c)) = chars.peek() {
                    if next_c.is_whitespace() {
                        let after_space = text[next_i..].trim_start();
                        if !after_space.is_empty() {
                            let boundary = next_i + (text[next_i..].len() - after_space.len());
                            return Some(boundary);
                        }
                    }
                } else {
                    return Some(i + c.len_utf8());
                }
            }
        }
        None
    }

    async fn emit_complete_sentences(&mut self) -> Result<(), anyhow::Error> {
        while let Some(boundary) = self.find_sentence_boundary(&self.pending_chunk) {
            let sentence = self.pending_chunk[..boundary].to_string();
            self.pending_chunk = self.pending_chunk[boundary..].to_string();

            if !sentence.trim().is_empty() {
                self.emit_chunk(&sentence).await?;
            }
        }
        Ok(())
    }

    async fn emit_chunk(&self, text: &str) -> Result<(), anyhow::Error> {
        let chunk = TextProcessorChunkPayload {
            text: text.trim().to_string(),
        };

        self.event_sender
            .send(AppEvent::TextProcessorChunk(chunk))
            .await?;

        Ok(())
    }
}
