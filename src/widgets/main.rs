use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget, Wrap},
};

use crate::{
    database::models::message::{ContentBlock, Role},
    state::AppState,
    widgets::{nav_tabs::NavTabs, status_line::StatusLine},
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

        let messages = &self.state.messages;
        let messages_len = messages.len();

        let most_recent_assistant_message = messages
            .iter()
            .rev()
            .find(|message| message.role == Role::Assistant);

        let mut lines = vec![];

        if let Some(most_recent_assistant_message) = most_recent_assistant_message {
            for block in most_recent_assistant_message.content.iter() {
                match block {
                    ContentBlock::Text { text } => {
                        lines.push(Line::from(text.clone()));
                        lines.push(Line::from(""));
                    }
                    _ => {}
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
