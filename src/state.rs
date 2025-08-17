use std::{
    collections::HashMap,
    fmt::{self, Display},
};

use ratatui::style::Color;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    camera::Img,
    database::models::{
        log::Log,
        message::{Message, Role},
    },
    emote::{Emote, color_to_char, get_color},
    widgets::views::{chat::ChatViewState, home::HomeViewState, logs::LogsViewState},
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PersistedState {
    color: Option<char>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum View {
    #[default]
    /// Main home view that shows basic input and status information.
    Home,

    /// View for displaying application logs.
    Logs,

    Chat,
}

impl Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            View::Home => write!(f, "Home"),
            View::Logs => write!(f, "Logs"),
            View::Chat => write!(f, "Chat"),
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
    pub chat_view: ChatViewState,

    pub audio_detected: bool,
    pub input_volume: f32,

    pub img_base64: Option<String>,
}

impl AppState {
    pub fn get_message(&self, id: &str) -> Option<&Message> {
        self.messages.iter().find(|&message| message.id == id)
    }

    pub fn get_message_mut(&mut self, id: &str) -> Option<&mut Message> {
        self.messages.iter_mut().find(|message| message.id == id)
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

    pub async fn persist_state(&self) -> Result<(), anyhow::Error> {
        let state = PersistedState {
            color: color_to_char(self.color),
        };

        let serialized_state = serde_json::to_string(&state)?;
        tokio::fs::write("./data/state.json", serialized_state).await?;

        Ok(())
    }

    pub async fn load_state(&mut self) -> Result<(), anyhow::Error> {
        let serialized_state = tokio::fs::read_to_string("./data/state.json").await?;
        let state: PersistedState = serde_json::from_str(&serialized_state)?;

        if let Some(color) = state.color {
            self.color = get_color(color).unwrap_or(Color::Yellow);
        }

        Ok(())
    }
}
