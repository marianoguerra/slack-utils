use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::types::QueryMeilisearchField;
use crate::index::IndexEntry;

pub fn render(
    f: &mut Frame,
    query: &str,
    url: &str,
    api_key: &str,
    index_name: &str,
    active_field: QueryMeilisearchField,
    results: Option<&Vec<IndexEntry>>,
    result_state: &mut ListState,
    error: Option<&str>,
    area: Rect,
) {
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
            Constraint::Min(1),    // Results
        ])
        .split(inner);

    // Query field
    let query_style = if active_field == QueryMeilisearchField::Query {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let query_widget = Paragraph::new(query)
        .style(query_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search Query"),
        );
    f.render_widget(query_widget, chunks[0]);

    // URL field
    let url_style = if active_field == QueryMeilisearchField::Url {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let url_widget = Paragraph::new(url)
        .style(url_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Meilisearch URL"),
        );
    f.render_widget(url_widget, chunks[1]);

    // API Key field (masked)
    let api_key_style = if active_field == QueryMeilisearchField::ApiKey {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let masked_key = if api_key.is_empty() {
        "(none)".to_string()
    } else {
        "*".repeat(api_key.len().min(20))
    };
    let api_key_widget = Paragraph::new(masked_key)
        .style(api_key_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("API Key"),
        );
    f.render_widget(api_key_widget, chunks[2]);

    // Index name field
    let index_style = if active_field == QueryMeilisearchField::IndexName {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let index_widget = Paragraph::new(index_name)
        .style(index_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Index Name"),
        );
    f.render_widget(index_widget, chunks[3]);

    // Help text
    let help = Paragraph::new("Tab: Next Field | Enter: Search | ↑↓: Navigate Results | Esc: Back")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[4]);

    // Results area
    let results_block = Block::default()
        .borders(Borders::ALL)
        .title("Results");

    if let Some(err) = error {
        let error_widget = Paragraph::new(err)
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: true })
            .block(results_block);
        f.render_widget(error_widget, chunks[5]);
    } else if let Some(results) = results {
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

            f.render_stateful_widget(list, chunks[5], result_state);
        }
    } else {
        let placeholder = Paragraph::new("Enter a search query and press Enter to search.")
            .style(Style::default().fg(Color::DarkGray))
            .block(results_block);
        f.render_widget(placeholder, chunks[5]);
    }
}
