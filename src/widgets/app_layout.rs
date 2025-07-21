use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::state::AppState;

pub struct AppLayout<'a> {
    state: &'a AppState,
}

impl<'a> AppLayout<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for AppLayout<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.state.view {
            crate::state::View::Main => {
                crate::widgets::main::MainWidget::new(self.state).render(area, buf)
            }
            crate::state::View::Logs => {
                crate::widgets::logs::LogsWidget::new(self.state).render(area, buf)
            }
        }
    }
}
