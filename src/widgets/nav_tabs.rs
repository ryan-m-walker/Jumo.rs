use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Widget,
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
        let tabs = [
            NavTab {
                title: String::from("Home"),
                is_active: self.state.view == View::Home,
            },
            NavTab {
                title: String::from("Logs"),
                is_active: self.state.view == View::Logs,
            },
            NavTab {
                title: String::from("Chat"),
                is_active: self.state.view == View::Chat,
            },
        ];

        Line::from(
            tabs.iter()
                .enumerate()
                .map(|(i, tab)| {
                    Span::styled(
                        format!("[{}] {} ", i + 1, tab.title),
                        if tab.is_active {
                            Style::default().fg(self.state.color).bold()
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    )
                })
                .collect::<Vec<_>>(),
        )
        .render(area, buf);
    }
}
