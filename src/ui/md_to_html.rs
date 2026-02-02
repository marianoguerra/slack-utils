use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::types::MdToHtmlField;

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
            Constraint::Min(0),    // Help text
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Convert Markdown to HTML");
    f.render_widget(block, area);

    // Input path field
    let input_style = if active_field == MdToHtmlField::InputPath {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let input_widget = Paragraph::new(input_path)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input Path (markdown file)"),
        );
    f.render_widget(input_widget, chunks[1]);

    // Output path field
    let output_style = if active_field == MdToHtmlField::OutputPath {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let output_widget = Paragraph::new(if output_path.is_empty() {
        "(auto: input.html)"
    } else {
        output_path
    })
    .style(output_style)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Output Path (optional, defaults to input with .html)"),
    );
    f.render_widget(output_widget, chunks[2]);

    // GFM toggle
    let gfm_style = if active_field == MdToHtmlField::Gfm {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let gfm_text = if gfm { "[x] GFM" } else { "[ ] GFM" };
    let gfm_widget = Paragraph::new(gfm_text).style(gfm_style).block(
        Block::default()
            .borders(Borders::ALL)
            .title("GitHub Flavored Markdown (tables, strikethrough, task lists)"),
    );
    f.render_widget(gfm_widget, chunks[3]);

    // Help text
    let help = Paragraph::new(
        "Tab/Shift+Tab: Switch fields | Space: Toggle GFM | Enter: Convert | Esc: Back",
    )
    .style(
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    );
    f.render_widget(help, chunks[4]);
}
