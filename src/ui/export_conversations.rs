use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::{channel_list, render_text_field, types::{ChannelSelection, ConvExportField}};
use crate::widgets::TextInput;

#[allow(clippy::too_many_arguments)]
pub fn render(
    f: &mut Frame,
    from_date: &TextInput,
    to_date: &TextInput,
    output_path: &TextInput,
    active_field: ConvExportField,
    channel_selection: Option<&mut ChannelSelection>,
    loading_channels: bool,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Export Conversations");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(inner);

    // From Date field
    render_text_field(
        f,
        from_date,
        "From Date (YYYY-MM-DD)",
        active_field == ConvExportField::FromDate,
        chunks[0],
    );

    // To Date field
    render_text_field(
        f,
        to_date,
        "To Date (YYYY-MM-DD)",
        active_field == ConvExportField::ToDate,
        chunks[1],
    );

    // Output Path field
    render_text_field(
        f,
        output_path,
        "Output Path",
        active_field == ConvExportField::OutputPath,
        chunks[2],
    );

    // Channel list
    channel_list::render(
        f,
        channel_selection,
        loading_channels,
        active_field == ConvExportField::Channels,
        chunks[3],
    );

    let help_text = if active_field == ConvExportField::Channels {
        "↑/↓: Navigate | Space: Toggle | a: All | n: None | r: Refresh | Tab: Next | Enter: Export | Esc: Back"
    } else {
        "Tab: Next Field | Enter: Export | Esc: Back"
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[4]);
}
