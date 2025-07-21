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
    color: Color,
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
                color: Color::Red,
                active: self.state.is_audio_recording_running,
            },
            Status {
                code: String::from("TRN"),
                color: Color::Yellow,
                active: self.state.is_audio_transcription_running,
            },
            Status {
                code: String::from("GEN"),
                color: Color::Yellow,
                active: self.state.is_llm_message_running,
            },
            Status {
                code: String::from("TTS"),
                color: Color::Green,
                active: self.state.is_tts_running,
            },
            Status {
                code: String::from("PLY"),
                color: Color::Green,
                active: self.state.is_audio_playback_running,
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

            // if status.active {
            //     code.style(Style::new().bg(self.get_bg_color()))
            // } else {
            //     code
            // }
            code.render(chunks[i], buf);
        }

        // let blocks = self.get_statuses()
        //     .iter()
        //     .map(|status| {
        //         let block = Block::default()
        //             .borders(BorderType::Rounded)
        //             .border_style(Style::new().fg(status.color))
        //             .title(status.code.clone())
        //             .padding(Padding::uniform(1));
        //
        //         if status.active {
        //             block.style(Style::new().bg(self.get_bg_color()))
        //         } else {
        //             block
        //         }
        //     })
    }
}
