use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};

use super::{render_help_text, render_static_field, types::DownloadAttachmentsField};

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

    render_static_field(f, conversations_path, "Conversations File", active_field == DownloadAttachmentsField::ConversationsPath, chunks[0]);
    render_static_field(f, output_path, "Output Directory", active_field == DownloadAttachmentsField::OutputPath, chunks[1]);
    render_help_text(f, "Tab: Next Field | Enter: Download | Esc: Back", chunks[2]);
}
