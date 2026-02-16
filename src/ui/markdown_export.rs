use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};

use super::{render_checkbox_field, render_help_text, render_static_field, types::MarkdownExportField};

pub struct MarkdownExportProps<'a> {
    pub conversations_path: &'a str,
    pub users_path: &'a str,
    pub channels_path: &'a str,
    pub output_path: &'a str,
    pub formatter_script: &'a str,
    pub backslash_line_breaks: bool,
    pub active_field: MarkdownExportField,
}

pub fn render(f: &mut Frame, props: MarkdownExportProps, area: Rect) {
    let MarkdownExportProps {
        conversations_path,
        users_path,
        channels_path,
        output_path,
        formatter_script,
        backslash_line_breaks,
        active_field,
    } = props;
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
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    render_static_field(f, conversations_path, "Selected Conversations File", active_field == MarkdownExportField::Conversations, chunks[0]);
    render_static_field(f, users_path, "Users File", active_field == MarkdownExportField::Users, chunks[1]);
    render_static_field(f, channels_path, "Channels File", active_field == MarkdownExportField::Channels, chunks[2]);
    render_static_field(f, output_path, "Output File", active_field == MarkdownExportField::Output, chunks[3]);
    render_static_field(f, formatter_script, "Formatter Script (optional)", active_field == MarkdownExportField::FormatterScript, chunks[4]);
    render_checkbox_field(
        f, backslash_line_breaks,
        "Backslash Line Breaks (adds \\ before newlines)",
        "Options",
        active_field == MarkdownExportField::BackslashLineBreaks,
        chunks[5],
    );
    render_help_text(f, "Tab: Next Field | Space: Toggle Checkbox | Enter: Export | Esc: Back", chunks[6]);
}
