use std::path::PathBuf;

use tempfile::TempPath;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Bytes;

#[derive(Debug, Clone)]
pub struct TTSResult {
    pub audio_bytes: Bytes,
    pub duration_seconds: f64,
}

#[derive(Debug)]
pub enum AppEvent {
    // Audio events
    AudioRecordingStarted,
    AudioRecordingCompleted(TempPath),
    AudioRecordingFailed(String),

    AudioPlaybackStarted,
    AudioPlaybackCompleted,
    AudioPlaybackFailed(String),

    // Transcription events
    TranscriptionStarted,
    TranscriptionCompleted(String),
    TranscriptionFailed(String),

    // LLM events
    LLMMessageStarted,
    LLMTextDelta(String),
    LLMMessageCompleted(String),
    LLMRequestFailed(String),

    // Text to speech events
    TTSStarted,
    TTSCompleted(TTSResult),
    TTSFailed(String),

    Error(String),
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
