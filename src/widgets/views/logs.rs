use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget, Wrap},
};

use crate::{
    database::models::log::{Log, LogLevel},
    state::AppState,
};

#[derive(Default, Debug, Clone)]
pub struct LogsViewState {
    pub logs: Vec<Log>,
}

pub struct LogsViewWidget<'a> {
    state: &'a AppState,
}

impl<'a> LogsViewWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for LogsViewWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::Yellow));

        let log_lines = self
            .state
            .logs_view
            .logs
            .iter()
            .map(|log| {
                let label = match log.level {
                    LogLevel::Info => "INFO",
                    LogLevel::Warn => "WARN",
                    LogLevel::Error => "ERROR",
                };

                let line = format!("[{label}] {}", log.text);

                let style = match log.level {
                    LogLevel::Info => Style::default().fg(Color::Reset),
                    LogLevel::Warn => Style::default().fg(Color::Yellow),
                    LogLevel::Error => Style::default().fg(Color::Red),
                };

                Line::from(line.to_string()).style(style)
            })
            .collect::<Vec<_>>();

        Paragraph::new(log_lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
