use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    database::models::{
        log::Log,
        message::{Message, MessageContent},
    },
    events::LLMMessageDeltaEventPayload,
    widgets::views::{home::HomeViewState, logs::LogsViewState},
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

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum View {
    #[default]
    /// Main home view that shows basic input and status information.
    Home,

    /// View for displaying application logs.
    Logs,
}

#[derive(Debug, Default, Clone)]
pub struct AppState {
    pub messages: Vec<Message>,
    pub error: Option<String>,
    pub is_app_running: bool,
    pub is_audio_transcription_running: bool,
    pub is_llm_message_running: bool,
    pub is_tts_running: bool,
    pub is_audio_recording_running: bool,
    pub is_audio_playback_running: bool,

    pub audio_input_device: String,
    pub audio_output_device: String,

    pub view: View,
    pub home_view: HomeViewState,
    pub logs_view: LogsViewState,
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
        self.logs_view.logs.push(log);
    }
}
