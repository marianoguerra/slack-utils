use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::types::MarkdownExportField;

pub fn render(
    f: &mut Frame,
    conversations_path: &str,
    users_path: &str,
    channels_path: &str,
    output_path: &str,
    active_field: MarkdownExportField,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Selected Conversations to Markdown");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    let conv_style = if active_field == MarkdownExportField::Conversations {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let conv_input = Paragraph::new(conversations_path)
        .style(conv_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Selected Conversations File"),
        );
    f.render_widget(conv_input, chunks[0]);

    let users_style = if active_field == MarkdownExportField::Users {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let users_input = Paragraph::new(users_path)
        .style(users_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Users File"),
        );
    f.render_widget(users_input, chunks[1]);

    let channels_style = if active_field == MarkdownExportField::Channels {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let channels_input = Paragraph::new(channels_path)
        .style(channels_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Channels File"),
        );
    f.render_widget(channels_input, chunks[2]);

    let output_style = if active_field == MarkdownExportField::Output {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let output_input = Paragraph::new(output_path)
        .style(output_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Output File"),
        );
    f.render_widget(output_input, chunks[3]);

    let help = Paragraph::new("Tab: Next Field | Enter: Export | Esc: Back")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[4]);
}
