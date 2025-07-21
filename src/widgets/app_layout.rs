use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::{
    state::{AppState, View},
    widgets::layouts::{home::HomeLayoutWidget, logs::LogsLayoutWidget},
};

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
            View::Home => HomeLayoutWidget::new(self.state).render(area, buf),
            View::Logs => LogsLayoutWidget::new(self.state).render(area, buf),
        }
    }
}
