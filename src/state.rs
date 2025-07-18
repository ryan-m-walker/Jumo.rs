use crate::events::LLMDelta;

#[derive(Debug, Clone)]
pub enum Speaker {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct TranscriptMessage {
    pub id: String,
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
pub enum MessageState {
    Idle,
    RecordingAudio,
    TranscribingAudio,
    SendingMessageToLLM,
    PlayingAudio,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub transcript: Vec<TranscriptLine>,
    pub error: Option<String>,
    pub is_app_running: bool,
    pub is_audio_transcription_running: bool,
    pub is_llm_message_running: bool,
    pub is_tts_running: bool,
    pub is_audio_recording_running: bool,
    pub is_audio_playback_running: bool,

    pub message_state: MessageState,
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
            is_audio_recording_running: false,
            is_audio_playback_running: false,

            message_state: MessageState::Idle,
        }
    }

    pub fn on_llm_text_delta(&mut self, delta: &LLMDelta) {
        let message = self.transcript.iter_mut().find_map(|line| {
            if let TranscriptLine::TranscriptMessage(message) = line {
                if message.id == delta.id {
                    Some(message)
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(message) = message {
            message.text.push_str(&delta.text);
        }
    }

    pub fn get_message(&self, id: &str) -> Option<&TranscriptMessage> {
        self.transcript.iter().find_map(|line| {
            if let TranscriptLine::TranscriptMessage(message) = line {
                if &message.id == id {
                    Some(message)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
