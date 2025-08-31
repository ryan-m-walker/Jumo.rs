use chrono::DateTime;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget, Wrap},
};

use crate::{
    state::AppState,
    types::message::{ContentBlock, Role},
};

mod audio_bars;

#[derive(Default, Debug, Clone)]
pub struct HomeViewState {
    pub message_index: usize,
}

pub struct HomeViewWidget<'a> {
    state: &'a AppState,
}

impl<'a> HomeViewWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for HomeViewWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // let layout = Layout::default()
        //     .direction(Direction::Vertical)
        //     .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        //     .split(area);

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(self.state.color));

        if let Some(err) = &self.state.error {
            let error_message = format!("Error: {err}");
            Paragraph::new(error_message.lines().map(Line::from).collect::<Vec<_>>())
                .style(Style::default().fg(Color::Red).bold())
                .wrap(Wrap { trim: true })
                .block(block)
                .render(area, buf);
            return;
        }

        let selected_message_index = self.state.home_view.message_index;

        let assistant_messages = self
            .state
            .messages
            .iter()
            .filter(|message| message.role == Role::Assistant)
            .collect::<Vec<_>>();

        let selected_message = assistant_messages.get(selected_message_index);
        let message_count = assistant_messages.len();

        let timestamp = match selected_message {
            Some(message) => {
                if let Ok(created_at) = &message.created_at.try_to_rfc3339_string() {
                    match DateTime::parse_from_rfc3339(created_at) {
                        Ok(ts) => ts.format("%b %d, %I:%M:%S%P").to_string(),
                        Err(_) => String::from("Invalid Timestamp"),
                    }
                } else {
                    String::new()
                }
            }
            None => String::new(),
        };

        let nav_line = format!(
            "[{}/{message_count}] {timestamp}",
            selected_message_index + 1
        );

        let mut all_lines = vec![
            Line::from(nav_line).style(Style::default().fg(Color::Reset).bold()),
            Line::from(""),
        ];

        let mut lines = vec![];

        if let Some(selected_message) = selected_message {
            for block in selected_message.content.iter() {
                match block {
                    ContentBlock::Text { text } => {
                        for line in text.lines() {
                            lines.push(Line::from(line));
                        }
                    }
                    ContentBlock::ToolUse { name, input, .. } => {
                        match serde_json::to_string(input) {
                            Ok(input) => {
                                lines.push(Line::from(""));
                                lines.push(Line::from(format!("[Tool Call] {name}:")));
                                lines.push(Line::from(input));
                            }
                            Err(e) => {
                                lines.push(Line::from(""));
                                lines.push(Line::from(format!("[Tool Call] {name}:")));
                                lines.push(
                                    Line::from(format!("Error: {e}"))
                                        .style(Style::default().fg(Color::Red)),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        all_lines.extend(lines);

        // Render audio bars in the top section
        // AudioBarsWidget::new(self.state).render(layout[0], buf);

        Paragraph::new(all_lines)
            .style(Style::default().fg(Color::Reset))
            .block(block)
            .wrap(Wrap { trim: true })
            // .render(layout[1], buf);
            .render(area, buf);
    }
}
