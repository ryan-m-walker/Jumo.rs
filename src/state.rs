use crate::{
    database::models::{Message, MessageContent},
    events::{LLMDelta, LLMMessageDeltaPayload},
};

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
pub struct AppState {
    pub messages: Vec<Message>,
    pub error: Option<String>,
    pub is_app_running: bool,
    pub is_audio_transcription_running: bool,
    pub is_llm_message_running: bool,
    pub is_tts_running: bool,
    pub is_audio_recording_running: bool,
    pub is_audio_playback_running: bool,
}

impl AppState {
    pub fn on_llm_text_delta(&mut self, delta: &LLMMessageDeltaPayload) {
        for message in self.messages.iter_mut() {
            if let MessageContent::Assistant { text } = &mut message.content {
                if message.id == delta.message_id {
                    text.push_str(delta.text.as_str());
                }
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: vec![],
            error: None,
            is_app_running: true,
            is_audio_transcription_running: false,
            is_llm_message_running: false,
            is_tts_running: false,
            is_audio_recording_running: false,
            is_audio_playback_running: false,
        }
    }
}
