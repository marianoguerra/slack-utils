use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::types::{ChannelSelection, ConvExportField};
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
    let from_active = active_field == ConvExportField::FromDate;
    let from_style = if from_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let from_block = Block::default()
        .borders(Borders::ALL)
        .title("From Date (YYYY-MM-DD)")
        .border_style(from_style);
    let from_inner = from_block.inner(chunks[0]);
    f.render_widget(from_block, chunks[0]);
    if from_active {
        from_date.render(f, from_inner, from_style);
    } else {
        let from_para = Paragraph::new(from_date.text());
        f.render_widget(from_para, from_inner);
    }

    // To Date field
    let to_active = active_field == ConvExportField::ToDate;
    let to_style = if to_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let to_block = Block::default()
        .borders(Borders::ALL)
        .title("To Date (YYYY-MM-DD)")
        .border_style(to_style);
    let to_inner = to_block.inner(chunks[1]);
    f.render_widget(to_block, chunks[1]);
    if to_active {
        to_date.render(f, to_inner, to_style);
    } else {
        let to_para = Paragraph::new(to_date.text());
        f.render_widget(to_para, to_inner);
    }

    // Output Path field
    let output_active = active_field == ConvExportField::OutputPath;
    let output_style = if output_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let output_block = Block::default()
        .borders(Borders::ALL)
        .title("Output Path")
        .border_style(output_style);
    let output_inner = output_block.inner(chunks[2]);
    f.render_widget(output_block, chunks[2]);
    if output_active {
        output_path.render(f, output_inner, output_style);
    } else {
        let output_para = Paragraph::new(output_path.text());
        f.render_widget(output_para, output_inner);
    }

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
