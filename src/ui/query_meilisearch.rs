use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::{render_help_text, render_static_field, types::QueryMeilisearchField};
use crate::index::IndexEntry;

pub struct QueryMeilisearchProps<'a> {
    pub query: &'a str,
    pub url: &'a str,
    pub api_key: &'a str,
    pub index_name: &'a str,
    pub active_field: QueryMeilisearchField,
    pub results: Option<&'a Vec<IndexEntry>>,
    pub result_state: &'a mut ListState,
    pub error: Option<&'a str>,
}

pub fn render(f: &mut Frame, props: QueryMeilisearchProps, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Search Meilisearch");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Query
            Constraint::Length(3), // URL
            Constraint::Length(3), // API Key
            Constraint::Length(3), // Index name
            Constraint::Length(2), // Help text
            Constraint::Min(1),   // Results
        ])
        .split(inner);

    render_static_field(f, props.query, "Search Query", props.active_field == QueryMeilisearchField::Query, chunks[0]);
    render_static_field(f, props.url, "Meilisearch URL", props.active_field == QueryMeilisearchField::Url, chunks[1]);

    // API Key field (masked)
    let masked_key = if props.api_key.is_empty() {
        "(none)".to_string()
    } else {
        "*".repeat(props.api_key.len().min(20))
    };
    render_static_field(f, &masked_key, "API Key", props.active_field == QueryMeilisearchField::ApiKey, chunks[2]);

    render_static_field(f, props.index_name, "Index Name", props.active_field == QueryMeilisearchField::IndexName, chunks[3]);
    render_help_text(f, "Tab: Next Field | Enter: Search | ↑↓: Navigate Results | Esc: Back", chunks[4]);

    // Results area
    let results_block = Block::default()
        .borders(Borders::ALL)
        .title("Results");

    if let Some(err) = props.error {
        let error_widget = Paragraph::new(err)
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: true })
            .block(results_block);
        f.render_widget(error_widget, chunks[5]);
    } else if let Some(results) = props.results {
        if results.is_empty() {
            let no_results = Paragraph::new("No results found.")
                .style(Style::default().fg(Color::DarkGray))
                .block(results_block);
            f.render_widget(no_results, chunks[5]);
        } else {
            let items: Vec<ListItem> = results
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let users: String = entry
                        .users
                        .iter()
                        .map(|u| u.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");

                    // Truncate text preview
                    let preview: String = entry
                        .text
                        .chars()
                        .take(100)
                        .collect::<String>()
                        .replace('\n', " ");

                    let lines = vec![
                        Line::from(vec![
                            Span::styled(
                                format!("{}. ", i + 1),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                format!("[{}] ", entry.date.chars().take(10).collect::<String>()),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::styled(
                                format!("#{}", entry.channel.name),
                                Style::default().fg(Color::Green),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("   Users: ", Style::default().fg(Color::DarkGray)),
                            Span::raw(users),
                        ]),
                        Line::from(vec![
                            Span::styled("   ", Style::default()),
                            Span::raw(if preview.len() >= 100 {
                                format!("{}...", preview)
                            } else {
                                preview
                            }),
                        ]),
                    ];

                    ListItem::new(lines)
                })
                .collect();

            let list = List::new(items)
                .block(results_block)
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(list, chunks[5], props.result_state);
        }
    } else {
        let placeholder = Paragraph::new("Enter a search query and press Enter to search.")
            .style(Style::default().fg(Color::DarkGray))
            .block(results_block);
        f.render_widget(placeholder, chunks[5]);
    }
}
