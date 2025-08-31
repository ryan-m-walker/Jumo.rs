use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
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
use tui_input::Input;

use crate::state::AppState;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatViewMode {
    #[default]
    Normal,
    Insert,
}

#[derive(Default, Debug, Clone)]
pub struct ChatViewState {
    pub input: Input,
    pub mode: ChatViewMode,
}

pub struct ChatViewWidget<'a> {
    state: &'a AppState,
}

impl<'a> ChatViewWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for ChatViewWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let _block = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(self.state.color));

        let title = if self.state.chat_view.mode == ChatViewMode::Insert {
            "[insert]"
        } else {
            "[normal]"
        };

        let input = Paragraph::new(self.state.chat_view.input.value())
            .block(Block::bordered().title(title));
        input.render(area, buf);
    }
}
