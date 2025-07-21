use ratatui::{buffer::Buffer, layout::Rect, text::Line, widgets::Widget};

use crate::state::AppState;

pub struct LogsWidget<'a> {
    state: &'a AppState,
}

impl<'a> LogsWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for LogsWidget<'_> {
    fn render(self, _area: Rect, _buf: &mut Buffer) {
        let _log_lines = self
            .state
            .logs
            .iter()
            .map(|log| Line::from(log.text.clone()));
    }
}
