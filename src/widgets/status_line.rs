use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Padding, Paragraph, Widget},
};

use crate::state::AppState;

pub struct StatusLine<'a> {
    state: &'a AppState,
}

struct Status {
    code: String,
    active: bool,
}

impl<'a> StatusLine<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn get_statuses(&self) -> Vec<Status> {
        vec![
            Status {
                code: String::from("REC"),
                active: self.state.is_audio_recording_running,
            },
            Status {
                code: String::from("STT"),
                active: self.state.is_audio_transcription_running,
            },
            Status {
                code: String::from("LLM"),
                active: self.state.is_llm_message_running,
            },
            Status {
                code: String::from("TTS"),
                active: self.state.is_tts_running,
            },
        ]
    }
}

impl Widget for StatusLine<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Fill(1),
            ])
            .split(area);

        for (i, status) in self.get_statuses().iter().enumerate() {
            let block = Block::default().padding(Padding::horizontal(1));

            let bg = if status.active {
                Color::Yellow
            } else {
                Color::Reset
            };

            let fg = if status.active {
                Color::Black
            } else {
                Color::DarkGray
            };

            let code = Paragraph::new(status.code.clone())
                .style(Style::new().fg(fg).bg(bg))
                .block(block);

            code.render(chunks[i], buf);
        }

        let devices = Paragraph::new(format!(
            " [Mic]: {} | [Speaker]: {}",
            self.state.audio_input_device, self.state.audio_output_device
        ));

        devices.render(chunks[4], buf);
    }
}
