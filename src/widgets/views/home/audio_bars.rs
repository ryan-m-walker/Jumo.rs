use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    symbols,
    widgets::{Block, BorderType, Widget},
};

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct AudioBarsWidget<'a> {
    state: &'a AppState,
}

impl<'a> AudioBarsWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for AudioBarsWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(self.state.color));

        let inner = block.inner(area);
        block.render(area, buf);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1); 20])
            .split(inner);

        // Render 20 bars in a hill pattern
        for (i, bar_area) in layout.iter().enumerate() {
            let bar_width = bar_area.width;
            let total_height = bar_area.height;

            // Calculate hill height - lowest at edges, highest in middle
            let distance_from_center = ((i as f32) - 9.5).abs(); // Center is at 9.5 for 20 bars
            let height_ratio = 1.0 - (distance_from_center / 10.0); // 0.0 at edges, 1.0 at center
            let bar_height = ((height_ratio * total_height as f32) as u16).max(1);

            // Start from bottom and fill upward
            let start_y = total_height - bar_height;

            for y in start_y..total_height {
                for x in 0..bar_width {
                    if bar_width > 0 && total_height > 0 {
                        buf.get_mut(bar_area.x + x, bar_area.y + y)
                            .set_symbol(symbols::block::FULL)
                            .set_fg(self.state.color);
                    }
                }
            }
        }
    }
}
