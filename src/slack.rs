use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::time::Duration;

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use slack_morphism::errors::SlackClientError;
use slack_morphism::prelude::*;

use crate::{
    parquet, week_to_date_range, AppError, OutputFormat, ProgressCallback, RateLimitCallback,
    SlackApiCallbacks, Result,
};

/// Maximum retries for rate-limited API calls
const MAX_RATE_LIMIT_RETRIES: u32 = 5;

/// Creates a Slack client and token for API calls.
/// Returns a tuple that can be used to open a session: `client.open_session(&token)`
fn create_slack_client(
    token: &str,
) -> (
    SlackClient<SlackClientHyperHttpsConnector>,
    SlackApiToken,
) {
    let client = SlackClient::new(
        SlackClientHyperConnector::new().expect("Failed to create Slack client connector"),
    );
    let token_obj = SlackApiToken::new(SlackApiTokenValue(token.to_string()));
    (client, token_obj)
}

/// Handle a Slack API result, retrying on rate limit errors.
/// Returns Ok(response) on success, or Err on non-rate-limit errors or max retries exceeded.
/// Optionally accepts a RateLimitCallback to report rate limit waits.
macro_rules! with_rate_limit_retry {
    ($api_call:expr) => {
        with_rate_limit_retry!($api_call, None::<&dyn Fn(u64, u32, u32)>)
    };
    ($api_call:expr, $on_rate_limit:expr) => {{
        let mut retries = 0u32;
        let callback: RateLimitCallback = $on_rate_limit;
        loop {
            match $api_call.await {
                Ok(response) => break Ok(response),
                Err(e) => {
                    let app_err = slack_error_to_app_error(e);
                    if let AppError::SlackRateLimit { retry_after_secs } = app_err {
                        retries += 1;
                        if retries > MAX_RATE_LIMIT_RETRIES {
                            break Err(AppError::SlackApi(format!(
                                "Rate limited {} times, giving up",
                                retries
                            )));
                        }
                        if let Some(cb) = callback {
                            cb(retry_after_secs, retries, MAX_RATE_LIMIT_RETRIES);
                        }
                        tokio::time::sleep(Duration::from_secs(retry_after_secs)).await;
                        continue;
                    }
                    break Err(app_err);
                }
            }
        }
    }};
}

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
    let channels: Vec<serde_json::Value> = crate::load_json_file(&path.display().to_string())?;

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
    let (client, token_obj) = create_slack_client(token);
    let session = client.open_session(&token_obj);

    let mut all_channels = Vec::new();
    let mut cursor: Option<SlackCursorId> = None;

    loop {
        let request = SlackApiConversationsListRequest::new()
            .with_limit(200)
            .with_types(vec![SlackConversationType::Public])
            .opt_cursor(cursor);

        let response = with_rate_limit_retry!(session.conversations_list(&request))?;

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

pub async fn export_users(token: &str, output_path: &Path, format: OutputFormat) -> Result<usize> {
    let (client, token_obj) = create_slack_client(token);
    let session = client.open_session(&token_obj);

    let mut all_users = Vec::new();
    let mut cursor: Option<SlackCursorId> = None;

    loop {
        let request = SlackApiUsersListRequest::new()
            .with_limit(200)
            .opt_cursor(cursor);

        let response = with_rate_limit_retry!(session.users_list(&request))?;

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

    match format {
        OutputFormat::Json => {
            write_json(output_path, &all_users)?;
        }
        OutputFormat::Parquet => {
            let users_json: Vec<serde_json::Value> = all_users
                .iter()
                .map(serde_json::to_value)
                .collect::<std::result::Result<_, _>>()
                .map_err(|e| AppError::JsonSerialize(e.to_string()))?;
            parquet::write_users_parquet(output_path, &users_json)?;
        }
    }

    Ok(count)
}

pub async fn export_channels(token: &str, output_path: &Path, format: OutputFormat) -> Result<usize> {
    let (client, token_obj) = create_slack_client(token);
    let session = client.open_session(&token_obj);

    let mut all_channels = Vec::new();
    let mut cursor: Option<SlackCursorId> = None;

    loop {
        let request = SlackApiConversationsListRequest::new()
            .with_limit(200)
            .with_types(vec![SlackConversationType::Public])
            .opt_cursor(cursor);

        let response = with_rate_limit_retry!(session.conversations_list(&request))?;

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

    match format {
        OutputFormat::Json => {
            write_json(output_path, &all_channels)?;
        }
        OutputFormat::Parquet => {
            let channels_json: Vec<serde_json::Value> = all_channels
                .iter()
                .map(serde_json::to_value)
                .collect::<std::result::Result<_, _>>()
                .map_err(|e| AppError::JsonSerialize(e.to_string()))?;
            parquet::write_channels_parquet(output_path, &channels_json)?;
        }
    }

    Ok(count)
}

pub async fn export_conversations(
    token: &str,
    from_date: NaiveDate,
    to_date: NaiveDate,
    output_path: &Path,
    selected_channel_ids: Option<&HashSet<String>>,
    callbacks: SlackApiCallbacks<'_>,
    format: OutputFormat,
) -> Result<usize> {
    let rate_limit_cb = callbacks.on_rate_limit;
    let (client, token_obj) = create_slack_client(token);
    let session = client.open_session(&token_obj);

    let oldest_ts = date_to_slack_ts(from_date);
    let latest_ts = date_to_slack_ts(to_date.succ_opt().unwrap_or(to_date));

    callbacks.report_progress(0, 0, "Fetching channel list...");

    let mut all_channels = Vec::new();
    let mut cursor: Option<SlackCursorId> = None;

    loop {
        let request = SlackApiConversationsListRequest::new()
            .with_limit(200)
            .with_types(vec![SlackConversationType::Public])
            .opt_cursor(cursor);

        let response =
            with_rate_limit_retry!(session.conversations_list(&request), rate_limit_cb)?;

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

    // Filter to selected channels
    let channels_to_fetch: Vec<_> = all_channels
        .iter()
        .filter(|ch| {
            selected_channel_ids
                .map(|selected| selected.contains(&ch.id.0))
                .unwrap_or(true)
        })
        .collect();

    let total_channels = channels_to_fetch.len();
    let mut all_conversations = Vec::new();

    for (channel_idx, channel) in channels_to_fetch.iter().enumerate() {
        let channel_id = &channel.id;
        let channel_name = channel.name.clone().unwrap_or_else(|| "unknown".to_string());

        callbacks.report_progress(
            channel_idx + 1,
            total_channels,
            &format!("Fetching #{}", channel_name),
        );

        let mut messages: Vec<SlackHistoryMessage> = Vec::new();
        let mut msg_cursor: Option<SlackCursorId> = None;

        loop {
            let request = SlackApiConversationsHistoryRequest::new()
                .with_channel(channel_id.clone())
                .with_oldest(oldest_ts.clone())
                .with_latest(latest_ts.clone())
                .with_limit(200)
                .opt_cursor(msg_cursor);

            let response =
                with_rate_limit_retry!(session.conversations_history(&request), rate_limit_cb)?;

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

        // Count messages with replies for progress reporting
        let messages_with_thread: Vec<_> = messages
            .iter()
            .filter(|m| m.parent.reply_count.map(|c| c > 0).unwrap_or(false))
            .collect();
        let total_threads = messages_with_thread.len();

        // Fetch thread replies for messages that have them
        let mut messages_with_replies: Vec<serde_json::Value> = Vec::new();
        let mut thread_idx = 0;

        for message in messages {
            let mut msg_value = serde_json::to_value(&message)
                .map_err(|e| AppError::JsonSerialize(e.to_string()))?;

            // Check if message has replies
            if let Some(reply_count) = message.parent.reply_count
                && reply_count > 0
            {
                thread_idx += 1;
                callbacks.report_progress(
                    thread_idx,
                    total_threads,
                    &format!("#{} - fetching thread {}/{}", channel_name, thread_idx, total_threads),
                );

                let mut replies: Vec<SlackHistoryMessage> = Vec::new();
                let mut reply_cursor: Option<SlackCursorId> = None;

                loop {
                    let request = SlackApiConversationsRepliesRequest::new(
                        channel_id.clone(),
                        message.origin.ts.clone(),
                    )
                    .with_limit(200)
                    .opt_cursor(reply_cursor);

                    let response = with_rate_limit_retry!(
                        session.conversations_replies(&request),
                        rate_limit_cb
                    )?;

                    // Skip the first message (parent) if it matches our message ts
                    let thread_replies: Vec<_> = response
                        .messages
                        .into_iter()
                        .filter(|m| m.origin.ts != message.origin.ts)
                        .collect();
                    replies.extend(thread_replies);

                    match response.response_metadata {
                        Some(meta)
                            if meta.next_cursor.is_some()
                                && !meta.next_cursor.as_ref().unwrap().0.is_empty() =>
                        {
                            reply_cursor = meta.next_cursor;
                        }
                        _ => break,
                    }
                }

                if !replies.is_empty() {
                    msg_value["thread_replies"] = serde_json::to_value(&replies)
                        .map_err(|e| AppError::JsonSerialize(e.to_string()))?;
                }
            }

            messages_with_replies.push(msg_value);
        }

        if !messages_with_replies.is_empty() {
            all_conversations.push(ConversationExport {
                channel_id: channel_id.0.clone(),
                channel_name,
                messages: messages_with_replies,
            });
        }
    }

    callbacks.report_progress(total_channels, total_channels, "Writing output file...");

    let total_messages: usize = all_conversations.iter().map(|c| c.messages.len()).sum();

    match format {
        OutputFormat::Json => {
            write_json(output_path, &all_conversations)?;
        }
        OutputFormat::Parquet => {
            let conversations_json: Vec<serde_json::Value> = all_conversations
                .iter()
                .map(serde_json::to_value)
                .collect::<std::result::Result<_, _>>()
                .map_err(|e| AppError::JsonSerialize(e.to_string()))?;
            parquet::write_conversations_parquet(output_path, &conversations_json)?;
        }
    }

    Ok(total_messages)
}

#[derive(Serialize)]
struct ConversationExport {
    channel_id: String,
    channel_name: String,
    messages: Vec<serde_json::Value>,
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
    pub errors: Vec<String>,
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
    progress_callback: ProgressCallback,
) -> Result<DownloadResult> {
    let files = extract_files_from_conversations(conversations_path)?;
    let total = files.len();

    if total == 0 {
        return Ok(DownloadResult {
            downloaded: 0,
            failed: 0,
            skipped: 0,
            errors: Vec::new(),
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
    let mut errors = Vec::new();

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
            errors.push(format!("Failed to create directory {}: {}", id_dir.display(), e));
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
                                errors.push(format!("Failed to write {}: {}", file_path.display(), e));
                                failed += 1;
                            } else {
                                downloaded += 1;
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Failed to read response for {}: {}", file_info.name, e));
                            failed += 1;
                        }
                    }
                } else {
                    errors.push(format!(
                        "HTTP {} for {}: {}",
                        response.status(),
                        file_info.name,
                        url
                    ));
                    failed += 1;
                }
            }
            Err(e) => {
                errors.push(format!("Failed to download {}: {}", file_info.name, e));
                failed += 1;
            }
        }
    }

    Ok(DownloadResult {
        downloaded,
        failed,
        skipped,
        errors,
    })
}

/// Result of fetching emojis
#[derive(Debug)]
pub struct EmojiResult {
    pub total: usize,
    pub downloaded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Fetch custom emojis from Slack and optionally download images
pub async fn fetch_emojis(
    token: &str,
    output_path: &Path,
    emojis_folder: &Path,
    progress_callback: ProgressCallback<'_>,
) -> Result<EmojiResult> {
    let report_progress = |current: usize, total: usize, msg: &str| {
        if let Some(cb) = progress_callback {
            cb(current, total, msg);
        }
    };

    report_progress(0, 0, "Fetching emoji list...");

    // Fetch emoji list using reqwest (emoji.list API) with rate limit retry
    let client = reqwest::Client::new();
    let mut retries = 0u32;
    let emoji_response: serde_json::Value = loop {
        let response = client
            .get("https://slack.com/api/emoji.list")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::SlackApi(format!("Failed to fetch emojis: {}", e)))?;

        // Handle rate limiting (HTTP 429)
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            retries += 1;
            if retries > MAX_RATE_LIMIT_RETRIES {
                return Err(AppError::SlackApi(format!(
                    "Rate limited {} times fetching emojis, giving up",
                    retries
                )));
            }
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);
            report_progress(0, 0, &format!("Rate limited, waiting {}s...", retry_after));
            tokio::time::sleep(Duration::from_secs(retry_after)).await;
            continue;
        }

        if !response.status().is_success() {
            return Err(AppError::SlackApi(format!(
                "HTTP {} fetching emojis",
                response.status()
            )));
        }

        break response
            .json()
            .await
            .map_err(|e| AppError::JsonParse(format!("Failed to parse emoji response: {}", e)))?;
    };

    if emoji_response.get("ok").and_then(|v: &serde_json::Value| v.as_bool()) != Some(true) {
        let error = emoji_response
            .get("error")
            .and_then(|e: &serde_json::Value| e.as_str())
            .unwrap_or("unknown error");
        return Err(AppError::SlackApi(format!("Emoji API error: {}", error)));
    }

    let emojis = emoji_response
        .get("emoji")
        .and_then(|e: &serde_json::Value| e.as_object())
        .ok_or_else(|| AppError::JsonParse("No emoji object in response".to_string()))?;

    let total = emojis.len();

    // Save emoji data to JSON file
    report_progress(0, total, "Saving emoji data...");
    let file = File::create(output_path).map_err(|e| AppError::WriteFile {
        path: output_path.display().to_string(),
        source: e,
    })?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &emojis)
        .map_err(|e| AppError::JsonSerialize(e.to_string()))?;

    // Create emojis folder
    std::fs::create_dir_all(emojis_folder).map_err(|e| AppError::WriteFile {
        path: emojis_folder.display().to_string(),
        source: e,
    })?;

    // Separate emojis into real ones and aliases
    let mut real_emojis: Vec<(&String, &str)> = Vec::new();
    let mut aliases: Vec<(&String, &str)> = Vec::new(); // (alias_name, target_name)

    for (name, url_value) in emojis.iter() {
        if let Some(url) = url_value.as_str() {
            if let Some(target) = url.strip_prefix("alias:") {
                aliases.push((name, target));
            } else {
                real_emojis.push((name, url));
            }
        }
    }

    // Download real emoji images and track their extensions
    let mut downloaded = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut copied = 0;
    let mut errors = Vec::new();
    let mut emoji_extensions: std::collections::HashMap<&str, String> =
        std::collections::HashMap::new();

    let real_count = real_emojis.len();
    for (idx, (name, url)) in real_emojis.iter().enumerate() {
        report_progress(idx + 1, total, name);

        // Extract file extension from URL
        let ext = url
            .split('.')
            .next_back()
            .and_then(|s: &str| s.split('?').next())
            .unwrap_or("png")
            .to_string();

        emoji_extensions.insert(name.as_str(), ext.clone());

        let filename = format!("{}.{}", name, ext);
        let file_path = emojis_folder.join(&filename);

        // Skip if already exists
        if file_path.exists() {
            skipped += 1;
            continue;
        }

        // Download emoji
        match client
            .get(*url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(bytes) => {
                            if let Err(e) = std::fs::write(&file_path, &bytes) {
                                errors.push(format!("Failed to write {}: {}", filename, e));
                                failed += 1;
                            } else {
                                downloaded += 1;
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Failed to read emoji {}: {}", name, e));
                            failed += 1;
                        }
                    }
                } else {
                    errors.push(format!("HTTP {} for emoji {}", response.status(), name));
                    failed += 1;
                }
            }
            Err(e) => {
                errors.push(format!("Failed to download emoji {}: {}", name, e));
                failed += 1;
            }
        }
    }

    // Copy files for aliases
    for (idx, (alias_name, target_name)) in aliases.iter().enumerate() {
        report_progress(real_count + idx + 1, total, &format!("{} -> {}", alias_name, target_name));

        // Resolve the target (follow alias chains)
        let mut current_target = *target_name;
        let mut visited = std::collections::HashSet::new();
        while let Some(url) = emojis.get(current_target).and_then(|v| v.as_str()) {
            if let Some(next_target) = url.strip_prefix("alias:") {
                if visited.contains(next_target) {
                    break; // Circular alias, stop
                }
                visited.insert(current_target);
                current_target = next_target;
            } else {
                break; // Found the real emoji
            }
        }

        // Get the extension of the target emoji
        let Some(ext) = emoji_extensions.get(current_target) else {
            errors.push(format!("Alias {} target {} not found", alias_name, current_target));
            failed += 1;
            continue;
        };

        let source_filename = format!("{}.{}", current_target, ext);
        let source_path = emojis_folder.join(&source_filename);
        let dest_filename = format!("{}.{}", alias_name, ext);
        let dest_path = emojis_folder.join(&dest_filename);

        // Skip if already exists
        if dest_path.exists() {
            skipped += 1;
            continue;
        }

        // Copy the file
        if source_path.exists() {
            if let Err(e) = std::fs::copy(&source_path, &dest_path) {
                errors.push(format!("Failed to copy {} to {}: {}", source_filename, dest_filename, e));
                failed += 1;
            } else {
                copied += 1;
            }
        } else {
            errors.push(format!("Source file {} not found for alias {}", source_filename, alias_name));
            failed += 1;
        }
    }

    Ok(EmojiResult {
        total,
        downloaded: downloaded + copied,
        failed,
        skipped,
        errors,
    })
}

/// Result of archiving a range of weeks
#[derive(Debug)]
pub struct ArchiveRangeResult {
    pub total_messages: usize,
    pub weeks_processed: usize,
    pub weeks_skipped: usize,
}

/// Generate all ISO weeks in a range (inclusive)
fn generate_weeks_in_range(
    from_year: i32,
    from_week: u32,
    to_year: i32,
    to_week: u32,
) -> Vec<(i32, u32)> {
    let mut weeks = Vec::new();
    let mut year = from_year;
    let mut week = from_week;

    loop {
        weeks.push((year, week));

        if year == to_year && week == to_week {
            break;
        }

        // Move to next week
        week += 1;

        // Check if we need to move to next year
        // ISO weeks can be 52 or 53 depending on the year
        let last_week_of_year = NaiveDate::from_ymd_opt(year, 12, 28)
            .map(|d| d.iso_week().week())
            .unwrap_or(52);

        if week > last_week_of_year {
            year += 1;
            week = 1;
        }

        // Safety limit to prevent infinite loops
        if weeks.len() > 520 {
            // 10 years of weeks
            break;
        }
    }

    weeks
}

/// Convert SlackClientError to AppError, properly extracting rate limit retry-after
fn slack_error_to_app_error(error: SlackClientError) -> AppError {
    match error {
        SlackClientError::RateLimitError(rate_err) => {
            // Extract retry-after from header, default to 60 seconds
            let retry_after_secs = rate_err
                .retry_after
                .map(|d| d.as_secs())
                .unwrap_or(60);
            AppError::SlackRateLimit { retry_after_secs }
        }
        other => AppError::SlackApi(other.to_string()),
    }
}

/// Archive conversations for a range of ISO weeks to parquet format
pub async fn archive_range(
    token: &str,
    from_year: i32,
    from_week: u32,
    to_year: i32,
    to_week: u32,
    output_path: &Path,
    callbacks: SlackApiCallbacks<'_>,
) -> Result<ArchiveRangeResult> {

    let weeks = generate_weeks_in_range(from_year, from_week, to_year, to_week);
    let total_weeks = weeks.len();

    // Capture which parquet files exist BEFORE we start processing.
    // This prevents skipping weeks that only have "overflow" messages from
    // thread replies written during this run.
    let pre_existing_files: HashSet<_> = weeks
        .iter()
        .filter_map(|(year, week)| {
            let parquet_file = output_path
                .join(format!("year={}/week={:02}", year, week))
                .join("threads.parquet");
            if parquet_file.exists() {
                Some((*year, *week))
            } else {
                None
            }
        })
        .collect();

    let mut total_messages = 0usize;
    let mut weeks_processed = 0usize;
    let mut weeks_skipped = 0usize;

    callbacks.report_progress(
        0,
        total_weeks,
        &format!("Archiving {} weeks...", total_weeks),
    );

    // Create callbacks for export_conversations without progress (we report at week level)
    // but with rate limit callback
    let export_callbacks = SlackApiCallbacks::new()
        .with_rate_limit(callbacks.on_rate_limit.unwrap_or(&|_, _, _| {}));

    for (idx, (year, week)) in weeks.iter().enumerate() {
        let week_label = format!("{}-W{:02}", year, week);

        // Only skip if file existed BEFORE this run started
        // (not created by overflow messages during this run)
        if pre_existing_files.contains(&(*year, *week)) {
            callbacks.report_progress(
                idx + 1,
                total_weeks,
                &format!("{} - already exists, skipping", week_label),
            );
            weeks_skipped += 1;
            continue;
        }

        callbacks.report_progress(idx + 1, total_weeks, &format!("{} - fetching...", week_label));

        // Convert week to date range
        let (from_date, to_date) = week_to_date_range(*year, *week)?;

        // Export conversations (rate limits handled at individual API call level)
        let count = export_conversations(
            token,
            from_date,
            to_date,
            output_path,
            None, // All channels
            export_callbacks,
            OutputFormat::Parquet,
        )
        .await?;

        total_messages += count;
        weeks_processed += 1;
        callbacks.report_progress(
            idx + 1,
            total_weeks,
            &format!("{} - {} messages", week_label, count),
        );
    }

    Ok(ArchiveRangeResult {
        total_messages,
        weeks_processed,
        weeks_skipped,
    })
}
