use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::{render_text_field, types::{ChannelSelection, ConvExportWeekField}};
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

    let channels_block_style = if active_field == ConvExportWeekField::Channels {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    if loading_channels {
        let loading = Paragraph::new("Loading channels...")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Channels")
                    .border_style(channels_block_style),
            );
        f.render_widget(loading, chunks[3]);
    } else if let Some(sel) = channel_selection {
        let selected_count = sel.selected.len();
        let total_count = sel.channels.len();

        let items: Vec<ListItem> = sel
            .channels
            .iter()
            .map(|ch| {
                let checkbox = if sel.selected.contains(&ch.id) {
                    "[x]"
                } else {
                    "[ ]"
                };
                ListItem::new(Line::from(format!("{} #{}", checkbox, ch.name)))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Channels ({}/{})", selected_count, total_count))
                    .border_style(channels_block_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, chunks[3], &mut sel.list_state);
    } else {
        let no_channels = Paragraph::new("No channels loaded. Press 'r' to fetch from Slack.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Channels")
                    .border_style(channels_block_style),
            );
        f.render_widget(no_channels, chunks[3]);
    }

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
