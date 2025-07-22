use tempfile::TempPath;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Bytes;

use crate::database::models::log::LogLevel;

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

#[derive(Debug)]
pub enum AppEvent {
    // Audio events
    AudioRecordingStarted,
    AudioRecordingCompleted(TempPath),
    AudioRecordingFailed(String),

    AudioPlaybackError(String),

    // Transcription events
    TranscriptionStarted,
    TranscriptionCompleted(String),
    TranscriptionFailed(String),

    // LLM events
    LLMMessageStarted(LLMMessageStartedEventPayload),
    LLMMessageDelta(LLMMessageDeltaEventPayload),
    LLMMessageCompleted(LLMMessageCompletedEventPayload),
    LLMRequestFailed(String),

    TextProcessorTextChunk(TextProcessorChunkEventPayload),
    TextProcessorFlushed,

    // Text to speech events
    TTSChunk(Bytes),
    TTSError(String),
    TTSFailed(String),

    Log(LogEventPayload),
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
