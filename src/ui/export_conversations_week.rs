use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::types::{ChannelSelection, ConvExportWeekField};
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
    let year_active = active_field == ConvExportWeekField::Year;
    let year_style = if year_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let year_block = Block::default()
        .borders(Borders::ALL)
        .title("Year")
        .border_style(year_style);
    let year_inner = year_block.inner(chunks[0]);
    f.render_widget(year_block, chunks[0]);
    if year_active {
        year.render(f, year_inner, year_style);
    } else {
        let year_para = Paragraph::new(year.text());
        f.render_widget(year_para, year_inner);
    }

    // Week field
    let week_active = active_field == ConvExportWeekField::Week;
    let week_style = if week_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let week_block = Block::default()
        .borders(Borders::ALL)
        .title("Week (1-53)")
        .border_style(week_style);
    let week_inner = week_block.inner(chunks[1]);
    f.render_widget(week_block, chunks[1]);
    if week_active {
        week.render(f, week_inner, week_style);
    } else {
        let week_para = Paragraph::new(week.text());
        f.render_widget(week_para, week_inner);
    }

    // Output Path field
    let output_active = active_field == ConvExportWeekField::OutputPath;
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
