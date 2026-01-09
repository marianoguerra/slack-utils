use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};

use super::centered_rect;

pub fn render_loading(
    f: &mut Frame,
    message: &str,
    progress: Option<&(usize, usize, String)>,
    area: Rect,
) {
    let block = Block::default().borders(Borders::ALL).title("Processing");

    let popup_area = centered_rect(60, 30, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(40),
        ])
        .split(inner);

    let loading = Paragraph::new(message)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(loading, chunks[0]);

    if let Some((current, total, item_name)) = progress {
        let percentage = if *total > 0 {
            (*current as f64 / *total as f64 * 100.0) as u16
        } else {
            0
        };

        let progress_text = format!("{}/{} - {}", current, total, item_name);
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(percentage)
            .label(progress_text);
        f.render_widget(gauge, chunks[1]);
    }
}

pub fn render_success(f: &mut Frame, message: &str, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Success")
        .border_style(Style::default().fg(Color::Green));

    let popup_area = centered_rect(70, 40, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(inner);

    let icon = Paragraph::new("✓")
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(icon, chunks[0]);

    let msg = Paragraph::new(message)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    f.render_widget(msg, chunks[1]);

    let help = Paragraph::new("Press Enter to continue")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}

pub fn render_error(f: &mut Frame, message: &str, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Error")
        .border_style(Style::default().fg(Color::Red));

    let popup_area = centered_rect(70, 40, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(inner);

    let icon = Paragraph::new("✗")
        .style(
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(icon, chunks[0]);

    let msg = Paragraph::new(message)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    f.render_widget(msg, chunks[1]);

    let help = Paragraph::new("Press Enter to continue")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}
