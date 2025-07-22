use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Padding, Paragraph, Widget, Wrap},
};

use crate::{database::models::message::MessageContent, state::AppState};

pub struct HomeLayoutWidget<'a> {
    state: &'a AppState,
}

impl<'a> HomeLayoutWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for HomeLayoutWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::Yellow));

        if let Some(err) = &self.state.error {
            let error_message = format!("Error: {err}");
            Paragraph::new(error_message.lines().map(Line::from).collect::<Vec<_>>())
                .style(Style::default().fg(Color::Red).bg(Color::Black).bold())
                .render(area, buf);
            return;
        }

        let mut assistant_message: Option<String> = None;

        for message in self.state.messages.iter().rev() {
            if let MessageContent::Assistant { text } = &message.content {
                assistant_message = Some(text.clone());
                break;
            }
        }

        let assistant_message = assistant_message.unwrap_or_default();
        let lines = assistant_message
            .split('\n')
            .map(Line::from)
            .collect::<Vec<_>>();

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
