use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::types::ExportEmojisField;

pub fn render(
    f: &mut Frame,
    output_path: &str,
    emojis_folder: &str,
    active_field: ExportEmojisField,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Export Custom Emojis");

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

    let output_style = if active_field == ExportEmojisField::OutputPath {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let output_input = Paragraph::new(output_path)
        .style(output_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Emoji Data Output (JSON)"),
        );
    f.render_widget(output_input, chunks[0]);

    let folder_style = if active_field == ExportEmojisField::EmojisFolder {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let folder_input = Paragraph::new(emojis_folder)
        .style(folder_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Emoji Images Folder"),
        );
    f.render_widget(folder_input, chunks[1]);

    let help = Paragraph::new("Tab: Next Field | Enter: Export | Esc: Back")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}
