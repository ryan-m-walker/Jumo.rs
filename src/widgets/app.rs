use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget, Wrap},
};

use crate::state::{AppState, Speaker, TranscriptLine};

pub struct AppWidget<'a> {
    state: &'a AppState,
}

impl<'a> AppWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for AppWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Fill(1)].as_ref())
            .split(area);

        let title = Line::from(" Transcript ".bold());
        let block = Block::bordered()
            .title(title.centered())
            .border_style(Style::new().yellow())
            .border_type(BorderType::Rounded);

        let header_text = if self.state.is_audio_recording_running {
            "Recording audio..."
        } else if self.state.is_audio_transcription_running {
            "Transcribing audio..."
        } else if self.state.is_llm_message_running {
            "Sending message to LLM..."
        } else {
            "Press space to start recording audio..."
        };

        let header = Paragraph::new(header_text).style(Style::new().dim()).block(
            Block::bordered()
                .border_style(Style::new().yellow())
                .border_type(BorderType::Rounded),
        );

        let mut lines = vec![];

        for line in self.state.transcript.iter() {
            match line {
                TranscriptLine::TranscriptMessage(line) => {
                    match line.speaker {
                        Speaker::User => {
                            lines.push(Line::from("[User]:").style(Style::new().yellow()));
                        }
                        Speaker::Assistant => {
                            lines.push(Line::from("[Claude]:").style(Style::new().red()));
                        }
                    }

                    lines.push(Line::from(line.text.clone()));
                }
                TranscriptLine::TranscriptError(line) => {
                    lines.push(Line::from(line.text.clone()));
                }
            }
        }

        let transcript = Paragraph::new(lines)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
            .block(block);

        header.render(chunks[0], buf);
        transcript.render(chunks[1], buf);
    }
}
