use tempfile::TempPath;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Bytes;

#[derive(Debug, Clone)]
pub struct TTSResult {
    pub audio_bytes: Bytes,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone)]
pub struct LLMDelta {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct LLMMessageStartedPayload {
    pub message_id: String,
}

#[derive(Debug, Clone)]
pub struct LLMMessageDeltaPayload {
    pub message_id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct LLMMessageCompletedPayload {
    pub message_id: String,
    pub full_text: String,
}

#[derive(Debug, Clone)]
pub struct TextProcessorChunkPayload {
    pub text: String,
    pub flush: bool,
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
    LLMMessageStarted(LLMMessageStartedPayload),
    LLMMessageDelta(LLMMessageDeltaPayload),
    LLMMessageCompleted(LLMMessageCompletedPayload),
    LLMRequestFailed(String),

    TextProcessorTextChunk(TextProcessorChunkPayload),

    // Text to speech events
    TTSStarted,
    TTSCompleted(TTSResult),
    TTSChunk(Bytes),
    TTSFailed(String),
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
