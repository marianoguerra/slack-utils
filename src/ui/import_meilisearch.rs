use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::types::ImportMeilisearchField;

pub struct ImportMeilisearchProps<'a> {
    pub input_path: &'a str,
    pub url: &'a str,
    pub api_key: &'a str,
    pub index_name: &'a str,
    pub clear: bool,
    pub active_field: ImportMeilisearchField,
}

pub fn render(f: &mut Frame, props: ImportMeilisearchProps, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Import Index to Meilisearch");

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
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    // Input path field
    let input_style = if props.active_field == ImportMeilisearchField::Input {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let input_widget = Paragraph::new(props.input_path)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Index File"),
        );
    f.render_widget(input_widget, chunks[0]);

    // URL field
    let url_style = if props.active_field == ImportMeilisearchField::Url {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let url_widget = Paragraph::new(props.url)
        .style(url_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Meilisearch URL"),
        );
    f.render_widget(url_widget, chunks[1]);

    // API Key field (masked)
    let api_key_style = if props.active_field == ImportMeilisearchField::ApiKey {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let masked_key = if props.api_key.is_empty() {
        "(none)".to_string()
    } else {
        "*".repeat(props.api_key.len().min(20))
    };
    let api_key_widget = Paragraph::new(masked_key)
        .style(api_key_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("API Key"),
        );
    f.render_widget(api_key_widget, chunks[2]);

    // Index name field
    let index_style = if props.active_field == ImportMeilisearchField::IndexName {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let index_widget = Paragraph::new(props.index_name)
        .style(index_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Index Name"),
        );
    f.render_widget(index_widget, chunks[3]);

    // Clear checkbox
    let clear_style = if props.active_field == ImportMeilisearchField::Clear {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let clear_text = if props.clear { "[x] Clear index" } else { "[ ] Clear index" };
    let clear_widget = Paragraph::new(clear_text)
        .style(clear_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Options (Space to toggle)"),
        );
    f.render_widget(clear_widget, chunks[4]);

    let help = Paragraph::new("Tab: Next Field | Space: Toggle Clear | Enter: Import | Esc: Back")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[5]);
}
