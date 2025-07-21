use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget, Wrap},
};

use crate::{
    database::models::message::MessageContent,
    state::AppState,
    widgets::{header::Header, nav_tabs::NavTabs, status_line::StatusLine},
};

pub struct MainWidget<'a> {
    state: &'a AppState,
}

impl<'a> MainWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for MainWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Fill(1),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(area);

        let block = Block::bordered()
            .border_style(Style::new().yellow())
            .border_type(BorderType::Rounded);

        let mut lines = vec![];
        let messages = &self.state.messages;
        let messages_len = messages.len();

        for message in &messages[messages_len.saturating_sub(2)..] {
            match &message.content {
                MessageContent::User { text } => {
                    lines.push(Line::from(text.clone()));
                    lines.push(Line::from(""));
                }
                MessageContent::Assistant { text } => {
                    lines.push(Line::from(text.clone()).style(Style::new().yellow()));
                }
                MessageContent::Error { text } => {
                    lines.push(Line::from("[Error]:").style(Style::new().red()));
                    lines.push(Line::from(text.clone()));
                }
                MessageContent::ToolCall { .. } => {
                    lines.push(Line::from("[ToolCall]:").style(Style::new().magenta()));
                }
                MessageContent::ToolResult { .. } => {
                    lines.push(Line::from("[ToolResult]:").style(Style::new().magenta()));
                }
            }
        }

        let transcript = Paragraph::new(lines)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
            .block(block);

        NavTabs::new(self.state).render(chunks[0], buf);
        transcript.render(chunks[1], buf);
        StatusLine::new(self.state).render(chunks[2], buf);
    }
}
