use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use ratatui::style::Color;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    database::models::{
        log::Log,
        message::{Message, Role},
    },
    emote::Emote,
    widgets::views::{home::HomeViewState, logs::LogsViewState},
};

#[derive(Debug, Clone)]
pub enum Speaker {
    User,
    Assistant,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum View {
    #[default]
    /// Main home view that shows basic input and status information.
    Home,

    /// View for displaying application logs.
    Logs,
}

impl Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            View::Home => write!(f, "Home"),
            View::Logs => write!(f, "Logs"),
        }
    }
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
    pub tool_input_buffers: HashMap<(String, usize), String>,
    pub emote: Emote,
    pub color: Color,

    pub view: View,
    pub home_view: HomeViewState,
    pub logs_view: LogsViewState,
}

impl AppState {
    pub fn get_message(&self, id: &str) -> Option<&Message> {
        for message in self.messages.iter() {
            if message.id == id {
                return Some(message);
            }
        }

        None
    }

    pub fn get_message_mut(&mut self, id: &str) -> Option<&mut Message> {
        for message in self.messages.iter_mut() {
            if message.id == id {
                return Some(message);
            }
        }

        None
    }

    pub fn get_assistant_message_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|message| message.role == Role::Assistant)
            .count()
    }

    pub fn log(&mut self, log: Log) {
        self.logs_view.logs.push(log);
    }
}
