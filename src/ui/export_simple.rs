use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::render_help_text;

pub fn render(f: &mut Frame, title: &str, output_path: &str, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(title);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(inner);

    let output_input = Paragraph::new(output_path)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Output Path"));
    f.render_widget(output_input, chunks[0]);

    render_help_text(f, "Enter: Confirm | Esc: Back", chunks[1]);
}
