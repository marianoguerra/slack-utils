use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};

use super::{render_checkbox_field, render_help_text, render_static_field, types::ImportMeilisearchField};

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

    render_static_field(f, props.input_path, "Index File", props.active_field == ImportMeilisearchField::Input, chunks[0]);
    render_static_field(f, props.url, "Meilisearch URL", props.active_field == ImportMeilisearchField::Url, chunks[1]);

    // API Key field (masked)
    let masked_key = if props.api_key.is_empty() {
        "(none)".to_string()
    } else {
        "*".repeat(props.api_key.len().min(20))
    };
    render_static_field(f, &masked_key, "API Key", props.active_field == ImportMeilisearchField::ApiKey, chunks[2]);

    render_static_field(f, props.index_name, "Index Name", props.active_field == ImportMeilisearchField::IndexName, chunks[3]);
    render_checkbox_field(f, props.clear, "Clear index", "Options (Space to toggle)", props.active_field == ImportMeilisearchField::Clear, chunks[4]);
    render_help_text(f, "Tab: Next Field | Space: Toggle Clear | Enter: Import | Esc: Back", chunks[5]);
}
