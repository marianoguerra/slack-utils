use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};

use super::{render_help_text, render_static_field, types::ExportIndexField};

pub fn render(
    f: &mut Frame,
    conversations_path: &str,
    users_path: &str,
    channels_path: &str,
    output_path: &str,
    active_field: ExportIndexField,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Export Index");

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

    render_static_field(f, conversations_path, "Conversations File", active_field == ExportIndexField::Conversations, chunks[0]);
    render_static_field(f, users_path, "Users File", active_field == ExportIndexField::Users, chunks[1]);
    render_static_field(f, channels_path, "Channels File", active_field == ExportIndexField::Channels, chunks[2]);
    render_static_field(f, output_path, "Output File", active_field == ExportIndexField::Output, chunks[3]);
    render_help_text(f, "Tab: Next Field | Enter: Export | Esc: Back", chunks[4]);
}
