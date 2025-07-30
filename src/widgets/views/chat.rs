use ratatui::prelude::*;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{
        Block,
        BorderType,
        // Widget
    },
};
use tui_textarea::TextArea;

use crate::state::AppState;

#[derive(Default, Debug, Clone)]
pub struct ChatViewState<'a> {
    pub textarea: TextArea<'
}

pub struct ChatViewWidget<'a> {
    state: &'a AppState<'a>,
}

impl<'a> ChatViewWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl ratatui::widgets::Widget for ChatViewWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(self.state.color));

        let mut textarea = TextArea::default();

        textarea.render(area, buf);
    }
}
