use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::state::AppState;

pub struct StatusLine<'a> {
    state: &'a AppState,
}

struct Status {
    code: &'static str,
    active: bool,
}

impl<'a> StatusLine<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn get_statuses(&self) -> Vec<Status> {
        vec![
            Status {
                code: "REC",
                active: self.state.is_audio_recording_running,
            },
            Status {
                code: "STT",
                active: self.state.is_audio_transcription_running,
            },
            Status {
                code: "LLM",
                active: self.state.is_llm_message_running,
            },
            Status {
                code: "TTS",
                active: self.state.is_tts_running,
            },
        ]
    }
}

impl Widget for StatusLine<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut spans = Vec::with_capacity(16);

        for status in self.get_statuses() {
            let bg = if status.active {
                self.state.color
            } else {
                Color::Reset
            };

            let fg = if status.active {
                Color::Black
            } else {
                Color::DarkGray
            };

            spans.push(Span::styled(
                format!(" {} ", status.code),
                Style::new().fg(fg).bg(bg),
            ));
        }

        let volume = self.state.input_volume;
        let volume_db = 20.0 * volume.max(0.001).log10(); // Convert to dB
        let volume_percent = ((volume_db + 60.0) / 60.0 * 100.0).max(0.0); // -60dB to 0dB range

        spans.push(Span::styled(" IN ", Style::new().fg(self.state.color)));
        spans.push(Span::styled(
            format!("({}) ", &self.state.audio_input_device),
            Style::new().fg(self.state.color),
        ));

        for i in 0..10 {
            let color = if volume_percent >= ((i as f32 + 1.0) * 10.0) {
                self.state.color
            } else {
                Color::DarkGray
            };

            spans.push(Span::styled("■", Style::new().fg(color)));
        }

        let bars = Line::from(spans);
        bars.render(area, buf);
    }
}
