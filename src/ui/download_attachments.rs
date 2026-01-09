use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::types::DownloadAttachmentsField;

pub fn render(
    f: &mut Frame,
    conversations_path: &str,
    output_path: &str,
    active_field: DownloadAttachmentsField,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Download Attachments");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    let conv_style = if active_field == DownloadAttachmentsField::ConversationsPath {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let conv_input = Paragraph::new(conversations_path)
        .style(conv_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Conversations File"),
        );
    f.render_widget(conv_input, chunks[0]);

    let output_style = if active_field == DownloadAttachmentsField::OutputPath {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let output_input = Paragraph::new(output_path)
        .style(output_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Output Directory"),
        );
    f.render_widget(output_input, chunks[1]);

    let help = Paragraph::new("Tab: Next Field | Enter: Download | Esc: Back")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}
