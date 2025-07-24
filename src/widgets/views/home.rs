use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget, Wrap},
};

use crate::{database::models::message::MessageContent, state::AppState};

#[derive(Default, Debug, Clone)]
pub struct HomeViewState {
    pub message_index: usize,
}

pub struct HomeViewWidget<'a> {
    state: &'a AppState,
}

impl<'a> HomeViewWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for HomeViewWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::Yellow));

        if let Some(err) = &self.state.error {
            let error_message = format!("Error: {err}");
            Paragraph::new(error_message.lines().map(Line::from).collect::<Vec<_>>())
                .style(Style::default().fg(Color::Red).bold())
                .wrap(Wrap { trim: true })
                .block(block)
                .render(area, buf);
            return;
        }

        let mut assistant_message = None;

        for message in self.state.messages.iter().rev() {
            if let MessageContent::Assistant { text } = &message.content {
                assistant_message = Some(text.clone());
                break;
            }
        }

        let message_count = self.state.messages.len();

        let mut all_lines = vec![
            Line::from(format!("[1/{message_count}]"))
                .style(Style::default().fg(Color::Reset).bold()),
            Line::from(""),
        ];

        let assistant_message = assistant_message.unwrap_or_default();
        let lines = assistant_message
            .split('\n')
            .map(Line::from)
            .collect::<Vec<_>>();

        all_lines.extend(lines);

        Paragraph::new(all_lines)
            .style(Style::default().fg(Color::Reset))
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
