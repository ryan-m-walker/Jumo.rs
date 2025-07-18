#[derive(Debug, Clone)]
pub enum Speaker {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct TranscriptMessage {
    pub speaker: Speaker,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct TranscriptError {
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum TranscriptLine {
    TranscriptMessage(TranscriptMessage),
    TranscriptError(TranscriptError),
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub transcript: Vec<TranscriptLine>,
    pub error: Option<String>,
    pub is_app_running: bool,
    pub is_audio_transcription_running: bool,
    pub is_llm_message_running: bool,
    pub is_tts_running: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            transcript: vec![],
            error: None,
            is_app_running: true,
            is_audio_transcription_running: false,
            is_llm_message_running: false,
            is_tts_running: false,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
