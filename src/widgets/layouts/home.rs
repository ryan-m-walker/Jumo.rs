use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::state::AppState;

pub struct HomeLayoutWidget<'a> {
    state: &'a AppState,
}

impl<'a> HomeLayoutWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for HomeLayoutWidget<'_> {
    fn render(self, _area: Rect, _buf: &mut Buffer) {
        println!("Hello, world!");
    }
}
