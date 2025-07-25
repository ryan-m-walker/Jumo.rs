use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    widgets::{Block, Padding, Paragraph, Widget},
};

use crate::state::AppState;

pub struct Header<'a> {
    state: &'a AppState,
}

impl<'a> Header<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for Header<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(1)].as_ref())
            .split(area);

        let title = Paragraph::new("JUMO 0.1.0").fg(self.state.color).bold();
        title.render(chunks[0], buf);

        let block = Block::default().padding(Padding::horizontal(1));
        let subtitle = Paragraph::new("[q]uit")
            .alignment(Alignment::Right)
            .block(block);
        subtitle.render(chunks[1], buf);
    }
}
