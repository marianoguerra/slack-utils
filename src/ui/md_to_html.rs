use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};

use super::{render_checkbox_field, render_help_text, render_static_field, types::MdToHtmlField};

pub fn render(
    f: &mut Frame,
    input_path: &str,
    output_path: &str,
    gfm: bool,
    active_field: MdToHtmlField,
    area: Rect,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Input path
            Constraint::Length(3), // Output path
            Constraint::Length(3), // GFM toggle
            Constraint::Min(0),   // Help text
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Convert Markdown to HTML");
    f.render_widget(block, area);

    render_static_field(f, input_path, "Input Path (markdown file)", active_field == MdToHtmlField::InputPath, chunks[1]);

    let output_display = if output_path.is_empty() { "(auto: input.html)" } else { output_path };
    render_static_field(f, output_display, "Output Path (optional, defaults to input with .html)", active_field == MdToHtmlField::OutputPath, chunks[2]);

    render_checkbox_field(f, gfm, "GFM", "GitHub Flavored Markdown (tables, strikethrough, task lists)", active_field == MdToHtmlField::Gfm, chunks[3]);
    render_help_text(f, "Tab/Shift+Tab: Switch fields | Space: Toggle GFM | Enter: Convert | Esc: Back", chunks[4]);
}
