use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    widgets::{Block, Padding, Paragraph, Widget},
};

pub struct Header;

impl Widget for Header {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(1)].as_ref())
            .split(area);

        let title = Paragraph::new("Fynn 0.1.0").yellow().bold();
        title.render(chunks[0], buf);

        let block = Block::default().padding(Padding::horizontal(1));
        let subtitle = Paragraph::new("[q]uit")
            .alignment(Alignment::Right)
            .block(block);
        subtitle.render(chunks[1], buf);
    }
}
