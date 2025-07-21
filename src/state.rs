use crate::{
    database::models::{
        log::Log,
        message::{Message, MessageContent},
    },
    events::LLMMessageDeltaEventPayload,
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

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Home,
    Logs,
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
    pub view: View,
    pub logs: Vec<Log>,
}

impl AppState {
    pub fn on_llm_text_delta(&mut self, delta: &LLMMessageDeltaEventPayload) {
        for message in self.messages.iter_mut() {
            if let MessageContent::Assistant { text } = &mut message.content {
                if message.id == delta.message_id {
                    text.push_str(delta.text.as_str());
                }
            }
        }
    }

    pub fn get_message(&self, id: &str) -> Option<&Message> {
        for message in self.messages.iter() {
            if message.id == id {
                return Some(message);
            }
        }

        None
    }

    pub fn log(&mut self, log: Log) {
        self.logs.push(log);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            error: None,
            is_app_running: true,
            is_audio_transcription_running: false,
            is_llm_message_running: false,
            is_tts_running: false,
            is_audio_recording_running: false,
            is_audio_playback_running: false,
            logs: Vec::new(),
            view: View::Home,
        }
    }
}
