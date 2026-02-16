use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};

use super::{render_help_text, render_static_field, types::ExportEmojisField};

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

    render_static_field(f, output_path, "Emoji Data Output (JSON)", active_field == ExportEmojisField::OutputPath, chunks[0]);
    render_static_field(f, emojis_folder, "Emoji Images Folder", active_field == ExportEmojisField::EmojisFolder, chunks[1]);
    render_help_text(f, "Tab: Next Field | Enter: Export | Esc: Back", chunks[2]);
}
