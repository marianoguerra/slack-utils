use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;

use crate::{AppError, Result};

/// Type alias for loaded conversation data: (channel_id, channel_name, messages)
pub type LoadedConversations = (
    Vec<(String, String, Vec<serde_json::Value>)>,
    serde_json::Value,
    serde_json::Value,
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
}

pub fn load_channels_from_file(path: &Path) -> Result<Vec<ChannelInfo>> {
    let file = File::open(path).map_err(|e| AppError::ReadFile {
        path: path.display().to_string(),
        source: e,
    })?;
    let reader = BufReader::new(file);

    let channels: Vec<serde_json::Value> =
        serde_json::from_reader(reader).map_err(|e| AppError::JsonParse(e.to_string()))?;

    let channel_infos = channels
        .into_iter()
        .filter_map(|c| {
            let id = c.get("id")?.as_str()?.to_string();
            let name = c
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string();
            Some(ChannelInfo { id, name })
        })
        .collect();

    Ok(channel_infos)
}

pub async fn fetch_channels(token: &str) -> Result<Vec<ChannelInfo>> {
    let client = SlackClient::new(
        SlackClientHyperConnector::new().expect("Failed to create Slack client connector"),
    );
    let token = SlackApiToken::new(SlackApiTokenValue(token.to_string()));
    let session = client.open_session(&token);

    let mut all_channels = Vec::new();
    let mut cursor: Option<SlackCursorId> = None;

    loop {
        let request = SlackApiConversationsListRequest::new()
            .with_limit(200)
            .with_types(vec![SlackConversationType::Public])
            .opt_cursor(cursor);

        let response = session
            .conversations_list(&request)
            .await
            .map_err(|e| AppError::SlackApi(e.to_string()))?;

        for channel in response.channels {
            all_channels.push(ChannelInfo {
                id: channel.id.0.clone(),
                name: channel.name.clone().unwrap_or_else(|| "unknown".to_string()),
            });
        }

        match response.response_metadata {
            Some(meta)
                if meta.next_cursor.is_some()
                    && !meta.next_cursor.as_ref().unwrap().0.is_empty() =>
            {
                cursor = meta.next_cursor;
            }
            _ => break,
        }
    }

    Ok(all_channels)
}

pub async fn export_users(token: &str, output_path: &Path) -> Result<usize> {
    let client = SlackClient::new(
        SlackClientHyperConnector::new().expect("Failed to create Slack client connector"),
    );
    let token = SlackApiToken::new(SlackApiTokenValue(token.to_string()));
    let session = client.open_session(&token);

    let mut all_users = Vec::new();
    let mut cursor: Option<SlackCursorId> = None;

    loop {
        let request = SlackApiUsersListRequest::new()
            .with_limit(200)
            .opt_cursor(cursor);

        let response = session
            .users_list(&request)
            .await
            .map_err(|e| AppError::SlackApi(e.to_string()))?;

        all_users.extend(response.members);

        match response.response_metadata {
            Some(meta)
                if meta.next_cursor.is_some()
                    && !meta.next_cursor.as_ref().unwrap().0.is_empty() =>
            {
                cursor = meta.next_cursor;
            }
            _ => break,
        }
    }

    let count = all_users.len();
    write_json(output_path, &all_users)?;

    Ok(count)
}

pub async fn export_channels(token: &str, output_path: &Path) -> Result<usize> {
    let client = SlackClient::new(
        SlackClientHyperConnector::new().expect("Failed to create Slack client connector"),
    );
    let token = SlackApiToken::new(SlackApiTokenValue(token.to_string()));
    let session = client.open_session(&token);

    let mut all_channels = Vec::new();
    let mut cursor: Option<SlackCursorId> = None;

    loop {
        let request = SlackApiConversationsListRequest::new()
            .with_limit(200)
            .with_types(vec![SlackConversationType::Public])
            .opt_cursor(cursor);

        let response = session
            .conversations_list(&request)
            .await
            .map_err(|e| AppError::SlackApi(e.to_string()))?;

        all_channels.extend(response.channels);

        match response.response_metadata {
            Some(meta)
                if meta.next_cursor.is_some()
                    && !meta.next_cursor.as_ref().unwrap().0.is_empty() =>
            {
                cursor = meta.next_cursor;
            }
            _ => break,
        }
    }

    let count = all_channels.len();
    write_json(output_path, &all_channels)?;

    Ok(count)
}

pub async fn export_conversations(
    token: &str,
    from_date: NaiveDate,
    to_date: NaiveDate,
    output_path: &Path,
    selected_channel_ids: Option<&HashSet<String>>,
) -> Result<usize> {
    let client = SlackClient::new(
        SlackClientHyperConnector::new().expect("Failed to create Slack client connector"),
    );
    let token_obj = SlackApiToken::new(SlackApiTokenValue(token.to_string()));
    let session = client.open_session(&token_obj);

    let oldest_ts = date_to_slack_ts(from_date);
    let latest_ts = date_to_slack_ts(to_date.succ_opt().unwrap_or(to_date));

    let mut all_channels = Vec::new();
    let mut cursor: Option<SlackCursorId> = None;

    loop {
        let request = SlackApiConversationsListRequest::new()
            .with_limit(200)
            .with_types(vec![SlackConversationType::Public])
            .opt_cursor(cursor);

        let response = session
            .conversations_list(&request)
            .await
            .map_err(|e| AppError::SlackApi(e.to_string()))?;

        all_channels.extend(response.channels);

        match response.response_metadata {
            Some(meta)
                if meta.next_cursor.is_some()
                    && !meta.next_cursor.as_ref().unwrap().0.is_empty() =>
            {
                cursor = meta.next_cursor;
            }
            _ => break,
        }
    }

    let mut all_conversations = Vec::new();

    for channel in &all_channels {
        let channel_id = &channel.id;

        if let Some(selected) = selected_channel_ids
            && !selected.contains(&channel_id.0)
        {
            continue;
        }

        let channel_name = channel.name.clone().unwrap_or_else(|| "unknown".to_string());

        let mut messages = Vec::new();
        let mut msg_cursor: Option<SlackCursorId> = None;

        loop {
            let request = SlackApiConversationsHistoryRequest::new()
                .with_channel(channel_id.clone())
                .with_oldest(oldest_ts.clone())
                .with_latest(latest_ts.clone())
                .with_limit(200)
                .opt_cursor(msg_cursor);

            let response = session
                .conversations_history(&request)
                .await
                .map_err(|e| AppError::SlackApi(e.to_string()))?;

            messages.extend(response.messages);

            match response.response_metadata {
                Some(meta)
                    if meta.next_cursor.is_some()
                        && !meta.next_cursor.as_ref().unwrap().0.is_empty() =>
                {
                    msg_cursor = meta.next_cursor;
                }
                _ => break,
            }
        }

        if !messages.is_empty() {
            all_conversations.push(ConversationExport {
                channel_id: channel_id.0.clone(),
                channel_name,
                messages,
            });
        }
    }

    let total_messages: usize = all_conversations.iter().map(|c| c.messages.len()).sum();
    write_json(output_path, &all_conversations)?;

    Ok(total_messages)
}

#[derive(Serialize)]
struct ConversationExport {
    channel_id: String,
    channel_name: String,
    messages: Vec<SlackHistoryMessage>,
}

fn date_to_slack_ts(date: NaiveDate) -> SlackTs {
    let timestamp = date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    SlackTs(format!("{}.000000", timestamp))
}

fn write_json<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let file = File::create(path).map_err(|e| AppError::WriteFile {
        path: path.display().to_string(),
        source: e,
    })?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, data).map_err(|e| AppError::JsonSerialize(e.to_string()))?;
    Ok(())
}

/// Load conversations from local JSON files for editing.
/// Returns (channels with messages, users data, channels metadata).
pub fn load_conversations_for_editing(
    conversations_path: &str,
    users_path: &str,
    channels_path: &str,
) -> Result<LoadedConversations> {
    // Load conversations
    let conv_file = File::open(conversations_path).map_err(|e| AppError::ReadFile {
        path: conversations_path.to_string(),
        source: e,
    })?;
    let conv_reader = BufReader::new(conv_file);
    let conversations: Vec<serde_json::Value> =
        serde_json::from_reader(conv_reader).map_err(|e| AppError::JsonParse(e.to_string()))?;

    // Load users
    let users_file = File::open(users_path).map_err(|e| AppError::ReadFile {
        path: users_path.to_string(),
        source: e,
    })?;
    let users_reader = BufReader::new(users_file);
    let users: serde_json::Value =
        serde_json::from_reader(users_reader).map_err(|e| AppError::JsonParse(e.to_string()))?;

    // Load channels metadata
    let channels_file = File::open(channels_path).map_err(|e| AppError::ReadFile {
        path: channels_path.to_string(),
        source: e,
    })?;
    let channels_reader = BufReader::new(channels_file);
    let channel_data: serde_json::Value =
        serde_json::from_reader(channels_reader).map_err(|e| AppError::JsonParse(e.to_string()))?;

    // Parse conversations into (channel_id, channel_name, messages) tuples
    let channels: Vec<(String, String, Vec<serde_json::Value>)> = conversations
        .into_iter()
        .filter_map(|conv| {
            let channel_id = conv.get("channel_id")?.as_str()?.to_string();
            let channel_name = conv
                .get("channel_name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string();
            let messages = conv.get("messages")?.as_array()?.to_vec();
            Some((channel_id, channel_name, messages))
        })
        .collect();

    Ok((channels, users, channel_data))
}

/// Export edited conversations to a JSON file.
/// Takes a list of channels with their messages (filtered by selection).
pub fn export_edited_conversations_to_file(
    channels: &[(String, String, Vec<serde_json::Value>)],
    output_path: &str,
) -> Result<usize> {
    #[derive(Serialize)]
    struct ExportedConversation {
        channel_id: String,
        channel_name: String,
        messages: Vec<serde_json::Value>,
    }

    let exported: Vec<ExportedConversation> = channels
        .iter()
        .filter(|(_, _, messages)| !messages.is_empty())
        .map(|(id, name, messages)| ExportedConversation {
            channel_id: id.clone(),
            channel_name: name.clone(),
            messages: messages.clone(),
        })
        .collect();

    let total_messages: usize = exported.iter().map(|c| c.messages.len()).sum();
    write_json(Path::new(output_path), &exported)?;

    Ok(total_messages)
}

/// Information about a file to download
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub filetype: Option<String>,
    pub url: Option<String>,
}

/// Result of downloading attachments
#[derive(Debug)]
pub struct DownloadResult {
    pub downloaded: usize,
    pub failed: usize,
    pub skipped: usize,
}

/// Extract file information from a conversations.json file
pub fn extract_files_from_conversations(conversations_path: &str) -> Result<Vec<FileInfo>> {
    let file = File::open(conversations_path).map_err(|e| AppError::ReadFile {
        path: conversations_path.to_string(),
        source: e,
    })?;
    let reader = BufReader::new(file);
    let conversations: Vec<serde_json::Value> =
        serde_json::from_reader(reader).map_err(|e| AppError::JsonParse(e.to_string()))?;

    let mut files = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for conv in conversations {
        if let Some(messages) = conv.get("messages").and_then(|m| m.as_array()) {
            for message in messages {
                if let Some(msg_files) = message.get("files").and_then(|f| f.as_array()) {
                    for file_obj in msg_files {
                        let id = file_obj
                            .get("id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("")
                            .to_string();

                        // Skip duplicates
                        if id.is_empty() || seen_ids.contains(&id) {
                            continue;
                        }
                        seen_ids.insert(id.clone());

                        let name = file_obj
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let filetype = file_obj
                            .get("filetype")
                            .and_then(|f| f.as_str())
                            .map(|s| s.to_string());

                        // Try url_private_download first, then url_private
                        let url = file_obj
                            .get("url_private_download")
                            .or_else(|| file_obj.get("url_private"))
                            .and_then(|u| u.as_str())
                            .map(|s| s.to_string());

                        files.push(FileInfo {
                            id,
                            name,
                            filetype,
                            url,
                        });
                    }
                }
            }
        }
    }

    Ok(files)
}

/// Download attachments from a conversations.json file
pub fn download_attachments(
    token: &str,
    conversations_path: &str,
    output_dir: &Path,
    progress_callback: Option<&dyn Fn(usize, usize, &str)>,
) -> Result<DownloadResult> {
    let files = extract_files_from_conversations(conversations_path)?;
    let total = files.len();

    if total == 0 {
        return Ok(DownloadResult {
            downloaded: 0,
            failed: 0,
            skipped: 0,
        });
    }

    // Create output directory
    std::fs::create_dir_all(output_dir).map_err(|e| AppError::WriteFile {
        path: output_dir.display().to_string(),
        source: e,
    })?;

    let client = reqwest::blocking::Client::new();
    let mut downloaded = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for (idx, file_info) in files.iter().enumerate() {
        if let Some(cb) = progress_callback {
            cb(idx + 1, total, &file_info.name);
        }

        let Some(url) = &file_info.url else {
            skipped += 1;
            continue;
        };

        // Create folder based on first 3 characters of file ID
        let folder_name = if file_info.id.len() >= 3 {
            &file_info.id[..3]
        } else {
            "unk"
        };
        let id_dir = output_dir.join(folder_name);
        if let Err(e) = std::fs::create_dir_all(&id_dir) {
            eprintln!("Failed to create directory {}: {}", id_dir.display(), e);
            failed += 1;
            continue;
        }

        // Create filename using ID and filetype extension
        let filename = match &file_info.filetype {
            Some(ft) if !ft.is_empty() => format!("{}.{}", file_info.id, ft),
            _ => file_info.id.clone(),
        };
        let file_path = id_dir.join(&filename);

        // Skip if already exists
        if file_path.exists() {
            skipped += 1;
            continue;
        }

        // Download file
        match client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes() {
                        Ok(bytes) => {
                            if let Err(e) = std::fs::write(&file_path, &bytes) {
                                eprintln!("Failed to write {}: {}", file_path.display(), e);
                                failed += 1;
                            } else {
                                downloaded += 1;
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to read response for {}: {}", file_info.name, e);
                            failed += 1;
                        }
                    }
                } else {
                    eprintln!(
                        "HTTP {} for {}: {}",
                        response.status(),
                        file_info.name,
                        url
                    );
                    failed += 1;
                }
            }
            Err(e) => {
                eprintln!("Failed to download {}: {}", file_info.name, e);
                failed += 1;
            }
        }
    }

    Ok(DownloadResult {
        downloaded,
        failed,
        skipped,
    })
}
