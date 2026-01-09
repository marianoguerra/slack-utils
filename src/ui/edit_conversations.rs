use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::centered_rect;
use super::types::{EditConvPathField, EditableChannelList};
use crate::widgets::TextInput;

pub fn render_path_input(
    f: &mut Frame,
    conversations_path: &str,
    users_path: &str,
    channels_path: &str,
    active_field: EditConvPathField,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Edit Conversations - Select Files");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    let conv_style = if active_field == EditConvPathField::Conversations {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let conv_input = Paragraph::new(conversations_path).style(conv_style).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Conversations File"),
    );
    f.render_widget(conv_input, chunks[0]);

    let users_style = if active_field == EditConvPathField::Users {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let users_input = Paragraph::new(users_path)
        .style(users_style)
        .block(Block::default().borders(Borders::ALL).title("Users File"));
    f.render_widget(users_input, chunks[1]);

    let channels_style = if active_field == EditConvPathField::Channels {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let channels_input = Paragraph::new(channels_path).style(channels_style).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Channels File"),
    );
    f.render_widget(channels_input, chunks[2]);

    let help = Paragraph::new("Tab: Next Field | Enter: Load | Esc: Back")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[3]);
}

pub fn render_channel_list(
    f: &mut Frame,
    channels: &mut EditableChannelList,
    editing_export_path: bool,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Edit Conversations - Channels");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(6),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);

    let items: Vec<ListItem> = channels
        .channels
        .iter()
        .map(|ch| {
            let enabled = ch.enabled_count();
            let total = ch.messages.len();
            ListItem::new(Line::from(format!(
                "#{} ({}/{} messages enabled)",
                ch.name, enabled, total
            )))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Channels ({})", channels.channels.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[0], &mut channels.list_state);

    let export_style = if editing_export_path {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let export_input = Paragraph::new(channels.export_path.as_str())
        .style(export_style)
        .block(Block::default().borders(Borders::ALL).title("Export Path"));
    f.render_widget(export_input, chunks[1]);

    let help_text = if editing_export_path {
        "Enter: Confirm Path | Esc: Cancel"
    } else {
        "↑/↓: Navigate | Alt+↑/↓: Reorder | Enter: Edit Messages | p: Edit Path | e: Export | Esc: Back"
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}

pub fn render_message_list(
    f: &mut Frame,
    channel_idx: usize,
    channels: &mut EditableChannelList,
    users: &serde_json::Value,
    area: Rect,
) {
    let channel = match channels.channels.get_mut(channel_idx) {
        Some(ch) => ch,
        None => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Edit Messages - #{}", channel.name));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(6), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = channel
        .messages
        .iter()
        .map(|msg| {
            let checkbox = if msg.enabled { "[x]" } else { "[ ]" };
            let collapse_icon = if msg.collapsed { "▶" } else { "▼" };
            let user_name = msg
                .user_id()
                .and_then(|uid| {
                    users.as_array().and_then(|arr| {
                        arr.iter().find_map(|u| {
                            if u.get("id").and_then(|i| i.as_str()) == Some(uid) {
                                u.get("name")
                                    .and_then(|n| n.as_str())
                                    .or_else(|| u.get("real_name").and_then(|n| n.as_str()))
                            } else {
                                None
                            }
                        })
                    })
                })
                .unwrap_or("unknown");

            let text_preview: String = msg
                .text()
                .chars()
                .take(50)
                .map(|c| if c == '\n' { ' ' } else { c })
                .collect();
            let text_preview = if msg.text().len() > 50 {
                format!("{}...", text_preview)
            } else {
                text_preview
            };

            let files_count = msg.files().len();
            let links_count = msg.links().len();
            let extras = if files_count > 0 || links_count > 0 {
                format!(" [{}f/{}l]", files_count, links_count)
            } else {
                String::new()
            };

            let style = if msg.enabled {
                Style::default()
            } else {
                Style::default().fg(Color::DarkGray)
            };

            if msg.collapsed {
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{} {} ", checkbox, collapse_icon)),
                    Span::styled(format!("@{}: {}{}", user_name, text_preview, extras), style),
                ]))
            } else {
                let full_text = msg.text().replace('\n', "\n    ");
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{} {} ", checkbox, collapse_icon)),
                    Span::styled(format!("@{}: {}{}", user_name, full_text, extras), style),
                ]))
            }
        })
        .collect();

    let enabled_count = channel.enabled_count();
    let total_count = channel.messages.len();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Messages ({}/{})", enabled_count, total_count)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[0], &mut channel.list_state);

    let help = Paragraph::new(
        "↑/↓: Navigate | Space: Toggle | Tab: Expand/Collapse | Alt+↑/↓: Reorder | a/n: All/None | Enter: Details | Esc: Back",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center);
    f.render_widget(help, chunks[1]);
}

#[allow(clippy::too_many_arguments)]
pub fn render_message_detail(
    f: &mut Frame,
    channel_idx: usize,
    message_idx: usize,
    channels: &EditableChannelList,
    users: &serde_json::Value,
    attachment_list_state: &mut ListState,
    editing_title: Option<&(usize, TextInput)>,
    area: Rect,
) {
    let channel = match channels.channels.get(channel_idx) {
        Some(ch) => ch,
        None => return,
    };
    let msg = match channel.messages.get(message_idx) {
        Some(m) => m,
        None => return,
    };

    let user_name = msg
        .user_id()
        .and_then(|uid| {
            users.as_array().and_then(|arr| {
                arr.iter().find_map(|u| {
                    if u.get("id").and_then(|i| i.as_str()) == Some(uid) {
                        u.get("name")
                            .and_then(|n| n.as_str())
                            .or_else(|| u.get("real_name").and_then(|n| n.as_str()))
                    } else {
                        None
                    }
                })
            })
        })
        .unwrap_or("unknown");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Message Detail - @{}", user_name));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(inner);

    let message_text = Paragraph::new(msg.text())
        .block(Block::default().borders(Borders::ALL).title("Message"))
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(message_text, chunks[0]);

    let files = msg.files();
    let links = msg.links();

    let mut items: Vec<ListItem> = Vec::new();

    for (idx, _file) in files.iter().enumerate() {
        let name = msg
            .get_file_title(idx)
            .unwrap_or_else(|| "unnamed file".to_string());
        let checkbox = if msg.selected_files.contains(&idx) {
            "[x]"
        } else {
            "[ ]"
        };
        items.push(ListItem::new(Line::from(format!(
            "{} File: {}",
            checkbox, name
        ))));
    }

    for (idx, link) in links.iter().enumerate() {
        let checkbox = if msg.selected_link_previews.contains(&idx) {
            "[x]"
        } else {
            "[ ]"
        };
        let main_indicator = if msg.main_link == Some(idx) {
            " ★ MAIN"
        } else {
            ""
        };
        let display_url = if link.url.len() > 50 {
            format!("{}...", &link.url[..47])
        } else {
            link.url.clone()
        };
        let title = msg
            .get_link_title(idx)
            .unwrap_or_else(|| link.title.clone());
        items.push(ListItem::new(Line::from(format!(
            "{} Link{}: {} ({})",
            checkbox, main_indicator, title, display_url
        ))));
    }

    let selected_files = msg.selected_files.len();
    let selected_links = msg.selected_link_previews.len();
    let total_files = files.len();
    let total_links = links.len();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "Files & Links ({}/{} files, {}/{} links)",
                    selected_files, total_files, selected_links, total_links
                )),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[1], attachment_list_state);

    // Render title editing input if active
    if let Some((link_idx, text_input)) = editing_title {
        let files_count = files.len();
        let editing_area = centered_rect(60, 20, area);
        f.render_widget(Clear, editing_area);

        let original_title = if *link_idx < files_count {
            files
                .get(*link_idx)
                .and_then(|f| f.get("name").and_then(|n| n.as_str()))
                .unwrap_or("Unknown")
                .to_string()
        } else {
            let lp_idx = *link_idx - files_count;
            links
                .get(lp_idx)
                .map(|l| l.title.as_str())
                .unwrap_or("Unknown")
                .to_string()
        };
        let item_type = if *link_idx < files_count {
            "File"
        } else {
            "Link"
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Edit {} Title (was: {})", item_type, original_title))
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(editing_area);
        f.render_widget(block, editing_area);

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        text_input.render(f, input_chunks[0], Style::default().fg(Color::Yellow));

        let edit_help = Paragraph::new("Enter: Save | Esc: Cancel | Ctrl+A/E: Home/End")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(edit_help, input_chunks[1]);

        attachment_list_state.select(Some(files_count + *link_idx));
    }

    let help = Paragraph::new(
        "↑/↓: Navigate | Space: Toggle | m: Main | e: Edit Title | f: Fetch Title | a/n: All/None | Esc: Back",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}
