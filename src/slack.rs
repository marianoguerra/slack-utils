use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;

use crate::{AppError, Result};

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
