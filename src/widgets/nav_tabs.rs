use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::state::{AppState, View};

pub struct NavTabs<'a> {
    state: &'a AppState,
}

struct NavTab {
    title: String,
    is_active: bool,
}

impl<'a> NavTabs<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for NavTabs<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(9),
                Constraint::Min(9),
                Constraint::Fill(1),
            ])
            .split(area);

        let tabs = [
            NavTab {
                title: String::from("Home"),
                is_active: self.state.view == View::Home,
            },
            NavTab {
                title: String::from("Logs"),
                is_active: self.state.view == View::Logs,
            },
        ];

        for (i, tab) in tabs.iter().enumerate() {
            let block = Block::default();

            let bg = if tab.is_active {
                Color::Yellow
            } else {
                Color::Reset
            };

            let fg = if tab.is_active {
                Color::Black
            } else {
                Color::DarkGray
            };

            let key_code = i + 1;

            let title = Paragraph::new(format!("[{key_code}] {}", tab.title))
                .style(Style::new().fg(fg).bg(bg))
                .block(block);

            title.render(layout[i], buf);
        }
    }
}
