use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::{render_text_field, types::{ChannelSelection, ConvExportField}};
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

    let channels_block_style = if active_field == ConvExportField::Channels {
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
