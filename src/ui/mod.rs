pub mod types;

mod download_attachments;
mod edit_conversations;
mod export_conversations;
mod export_simple;
mod loading;
mod main_menu;
mod markdown_export;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
pub use types::*;

use crate::app::App;

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.area());

    let banner = Paragraph::new("Slack Utils")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(banner, chunks[0]);

    match &mut app.screen {
        Screen::MainMenu => main_menu::render(f, &mut app.menu_state, chunks[1]),
        Screen::ExportConversations {
            from_date,
            to_date,
            output_path,
            active_field,
            channel_selection,
            loading_channels,
        } => export_conversations::render(
            f,
            from_date,
            to_date,
            output_path,
            *active_field,
            channel_selection.as_mut(),
            *loading_channels,
            chunks[1],
        ),
        Screen::ExportUsers { output_path } => {
            export_simple::render(f, "Export Users", output_path, chunks[1])
        }
        Screen::ExportChannels { output_path } => {
            export_simple::render(f, "Export Channels", output_path, chunks[1])
        }
        Screen::DownloadAttachments {
            conversations_path,
            output_path,
            active_field,
        } => download_attachments::render(
            f,
            conversations_path,
            output_path,
            *active_field,
            chunks[1],
        ),
        Screen::MarkdownExport {
            conversations_path,
            users_path,
            channels_path,
            output_path,
            active_field,
        } => markdown_export::render(
            f,
            conversations_path,
            users_path,
            channels_path,
            output_path,
            *active_field,
            chunks[1],
        ),
        Screen::EditConversationsPathInput {
            conversations_path,
            users_path,
            channels_path,
            active_field,
        } => edit_conversations::render_path_input(
            f,
            conversations_path,
            users_path,
            channels_path,
            *active_field,
            chunks[1],
        ),
        Screen::EditConversationsChannelList {
            channels,
            users: _,
            channel_data: _,
            editing_export_path,
        } => edit_conversations::render_channel_list(f, channels, *editing_export_path, chunks[1]),
        Screen::EditConversationsMessageList {
            channel_idx,
            channels,
            users,
            channel_data: _,
        } => edit_conversations::render_message_list(f, *channel_idx, channels, users, chunks[1]),
        Screen::EditConversationsMessageDetail {
            channel_idx,
            message_idx,
            channels,
            users,
            channel_data: _,
            attachment_list_state,
            editing_title,
        } => edit_conversations::render_message_detail(
            f,
            *channel_idx,
            *message_idx,
            channels,
            users,
            attachment_list_state,
            editing_title.as_ref(),
            chunks[1],
        ),
        Screen::Loading { message, progress } => {
            loading::render_loading(f, message, progress.as_ref(), chunks[1])
        }
        Screen::Success { message } => loading::render_success(f, message, chunks[1]),
        Screen::Error { message } => loading::render_error(f, message, chunks[1]),
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
