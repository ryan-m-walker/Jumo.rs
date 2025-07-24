use tempfile::TempPath;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Bytes;

use crate::{
    database::models::{log::LogLevel, message::Message},
    services::anthropic::types::{AnthropicContentBlockDelta, AnthropicMessageStreamEvent},
    state::{AppState, View},
};

#[derive(Debug, Clone)]
pub struct TTSResult {
    pub audio_bytes: Bytes,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone)]
pub struct LLMMessageStartedEventPayload {
    pub message_id: String,
}

#[derive(Debug, Clone)]
pub struct LLMMessageDeltaEventPayload {
    pub message_id: String,
    pub text: String,
    pub content_block_index: usize,
}

#[derive(Debug, Clone)]
pub struct LLMMessageCompletedEventPayload {
    pub message_id: String,
    pub full_text: String,
}

#[derive(Debug, Clone)]
pub struct TextProcessorChunkEventPayload {
    pub text: String,
    pub flush: bool,
}

#[derive(Debug, Clone)]
pub struct LogEventPayload {
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct LLMGenerationStartedEventPayload {
    pub message_id: String,
}

#[derive(Debug, Clone)]
pub struct LLMStreamEventPayload {
    pub message_id: String,
    pub event: AnthropicMessageStreamEvent,
}

#[derive(Debug, Clone)]
pub struct LLMGenerationCompletedEventPayload {
    pub message_id: String,
}

#[derive(Debug)]
pub enum AppEvent {
    // Audio events
    AudioRecordingStarted,
    AudioRecordingCompleted(TempPath),
    AudioRecordingFailed(String),
    AudioPlaybackError(String),

    AudioSetInputDevice(String),
    AudioSetOutputDevice(String),

    // Transcription events
    TranscriptionStarted,
    TranscriptionCompleted(String),
    TranscriptionFailed(String),

    // LLM events
    LLMGenerationStarted(LLMGenerationStartedEventPayload),
    LLMGenerationCompleted(LLMGenerationCompletedEventPayload),
    LLMStreamEvent(LLMStreamEventPayload),
    LLMGenerationFailed(String),
    LLMGenerationError(String),

    TextProcessorTextChunk(TextProcessorChunkEventPayload),
    TextProcessorFlushed,

    // Text to speech events
    TTSChunk(Bytes),
    TTSError(String),
    TTSFailed(String),

    Log(LogEventPayload),

    SetView(View),
}

pub struct EventBus {
    sender: mpsc::Sender<AppEvent>,
    receiver: mpsc::Receiver<AppEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self { sender, receiver }
    }

    pub fn sender(&self) -> mpsc::Sender<AppEvent> {
        self.sender.clone()
    }

    pub async fn recv(&mut self) -> Option<AppEvent> {
        self.receiver.recv().await
    }
}
