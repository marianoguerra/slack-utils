use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::types::MarkdownExportField;

pub struct MarkdownExportProps<'a> {
    pub conversations_path: &'a str,
    pub users_path: &'a str,
    pub channels_path: &'a str,
    pub output_path: &'a str,
    pub formatter_script: &'a str,
    pub backslash_line_breaks: bool,
    pub active_field: MarkdownExportField,
}

pub fn render(f: &mut Frame, props: MarkdownExportProps, area: Rect) {
    let MarkdownExportProps {
        conversations_path,
        users_path,
        channels_path,
        output_path,
        formatter_script,
        backslash_line_breaks,
        active_field,
    } = props;
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Selected Conversations to Markdown");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    let conv_style = if active_field == MarkdownExportField::Conversations {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let conv_input = Paragraph::new(conversations_path)
        .style(conv_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Selected Conversations File"),
        );
    f.render_widget(conv_input, chunks[0]);

    let users_style = if active_field == MarkdownExportField::Users {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let users_input = Paragraph::new(users_path)
        .style(users_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Users File"),
        );
    f.render_widget(users_input, chunks[1]);

    let channels_style = if active_field == MarkdownExportField::Channels {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let channels_input = Paragraph::new(channels_path)
        .style(channels_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Channels File"),
        );
    f.render_widget(channels_input, chunks[2]);

    let output_style = if active_field == MarkdownExportField::Output {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let output_input = Paragraph::new(output_path)
        .style(output_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Output File"),
        );
    f.render_widget(output_input, chunks[3]);

    let formatter_style = if active_field == MarkdownExportField::FormatterScript {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let formatter_input = Paragraph::new(formatter_script)
        .style(formatter_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Formatter Script (optional)"),
        );
    f.render_widget(formatter_input, chunks[4]);

    let backslash_style = if active_field == MarkdownExportField::BackslashLineBreaks {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let checkbox = if backslash_line_breaks { "[x]" } else { "[ ]" };
    let backslash_text = format!("{} Backslash Line Breaks (adds \\ before newlines)", checkbox);
    let backslash_input = Paragraph::new(backslash_text)
        .style(backslash_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Options"),
        );
    f.render_widget(backslash_input, chunks[5]);

    let help = Paragraph::new("Tab: Next Field | Space: Toggle Checkbox | Enter: Export | Esc: Back")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[6]);
}
