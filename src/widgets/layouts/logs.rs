use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::state::AppState;

pub struct LogsLayoutWidget<'a> {
    state: &'a AppState,
}

impl<'a> LogsLayoutWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for LogsLayoutWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::Yellow));

        let log_lines = self
            .state
            .logs
            .iter()
            .map(|log| Line::from(log.text.clone()))
            .collect::<Vec<_>>();

        Paragraph::new(log_lines).block(block).render(area, buf);
    }
}
