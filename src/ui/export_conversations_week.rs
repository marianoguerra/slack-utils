use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::{channel_list, render_text_field, types::{ChannelSelection, ConvExportWeekField}};
use crate::widgets::TextInput;

#[allow(clippy::too_many_arguments)]
pub fn render(
    f: &mut Frame,
    year: &TextInput,
    week: &TextInput,
    output_path: &TextInput,
    active_field: ConvExportWeekField,
    channel_selection: Option<&mut ChannelSelection>,
    loading_channels: bool,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Export Conversations for Work Week");

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

    // Year field
    render_text_field(
        f,
        year,
        "Year",
        active_field == ConvExportWeekField::Year,
        chunks[0],
    );

    // Week field
    render_text_field(
        f,
        week,
        "Week (1-53)",
        active_field == ConvExportWeekField::Week,
        chunks[1],
    );

    // Output Path field
    render_text_field(
        f,
        output_path,
        "Output Path",
        active_field == ConvExportWeekField::OutputPath,
        chunks[2],
    );

    // Channel list
    channel_list::render(
        f,
        channel_selection,
        loading_channels,
        active_field == ConvExportWeekField::Channels,
        chunks[3],
    );

    let help_text = if active_field == ConvExportWeekField::Channels {
        "^/v: Navigate | Space: Toggle | a: All | n: None | r: Refresh | Tab: Next | Enter: Export | Esc: Back"
    } else {
        "Tab: Next Field | Enter: Export | Esc: Back"
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[4]);
}
