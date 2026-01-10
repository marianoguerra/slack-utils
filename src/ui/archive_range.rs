use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::types::ArchiveRangeField;
use crate::widgets::TextInput;

#[allow(clippy::too_many_arguments)]
pub fn render(
    f: &mut Frame,
    from_year: &TextInput,
    from_week: &TextInput,
    to_year: &TextInput,
    to_week: &TextInput,
    output_path: &TextInput,
    active_field: ArchiveRangeField,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Archive Conversations (Week Range)");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // From Year
            Constraint::Length(3), // From Week
            Constraint::Length(3), // To Year
            Constraint::Length(3), // To Week
            Constraint::Length(3), // Output Path
            Constraint::Min(2),    // Info/Help
            Constraint::Length(1), // Help text
        ])
        .split(inner);

    // From Year field
    render_text_field(
        f,
        from_year,
        "From Year",
        active_field == ArchiveRangeField::FromYear,
        chunks[0],
    );

    // From Week field
    render_text_field(
        f,
        from_week,
        "From Week (1-53)",
        active_field == ArchiveRangeField::FromWeek,
        chunks[1],
    );

    // To Year field
    render_text_field(
        f,
        to_year,
        "To Year",
        active_field == ArchiveRangeField::ToYear,
        chunks[2],
    );

    // To Week field
    render_text_field(
        f,
        to_week,
        "To Week (1-53)",
        active_field == ArchiveRangeField::ToWeek,
        chunks[3],
    );

    // Output Path field
    render_text_field(
        f,
        output_path,
        "Output Directory",
        active_field == ArchiveRangeField::OutputPath,
        chunks[4],
    );

    // Info text
    let info_text = Paragraph::new(
        "Exports conversations for each week to parquet format.\n\
         Existing weeks are skipped. Rate limits are handled automatically.",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default());
    f.render_widget(info_text, chunks[5]);

    // Help text
    let help = Paragraph::new("Tab: Next Field | Enter: Start Archive | Esc: Back")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[6]);
}

fn render_text_field(
    f: &mut Frame,
    input: &TextInput,
    title: &str,
    active: bool,
    area: Rect,
) {
    let style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if active {
        input.render(f, inner, style);
    } else {
        let para = Paragraph::new(input.text());
        f.render_widget(para, inner);
    }
}
