use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter};

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use slack_blocks_render::{render_blocks_as_markdown, SlackReferences};
use slack_morphism::prelude::{SlackBlock, SlackChannelId, SlackUserId};

use crate::error::{AppError, Result};
use crate::ProgressCallback;

/// A user entry in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexUser {
    pub id: String,
    pub name: String,
}

/// Channel information in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexChannel {
    pub id: String,
    pub name: String,
}

/// A single entry in the conversation index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Unique identifier (ts with dots replaced for Meilisearch compatibility)
    pub id: String,
    /// Original Slack timestamp (e.g., "1767636991.559059")
    pub ts: String,
    /// ISO 8601 datetime of the message
    pub date: String,
    /// Markdown rendering of the message blocks including thread replies
    pub text: String,
    /// List of users involved in this thread
    pub users: Vec<IndexUser>,
    /// Channel information
    pub channel: IndexChannel,
}

/// Convert a Slack timestamp to ISO 8601 datetime string
fn slack_ts_to_iso8601(ts: &str) -> String {
    // Slack ts format: "1767636991.559059" (seconds.microseconds)
    let secs: i64 = ts
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    Utc.timestamp_opt(secs, 0)
        .single()
        .map(|dt: DateTime<Utc>| dt.to_rfc3339())
        .unwrap_or_default()
}

/// Export conversations to an index JSON file
pub fn export_conversations_to_index(
    conversations_path: &str,
    users_path: &str,
    channels_path: &str,
    output_path: &str,
) -> Result<usize> {
    export_conversations_to_index_with_progress(
        conversations_path,
        users_path,
        channels_path,
        output_path,
        None,
    )
}

/// Export conversations to an index JSON file with progress reporting
pub fn export_conversations_to_index_with_progress(
    conversations_path: &str,
    users_path: &str,
    channels_path: &str,
    output_path: &str,
    progress_callback: ProgressCallback,
) -> Result<usize> {
    let report_progress = |current: usize, total: usize, msg: &str| {
        if let Some(cb) = progress_callback {
            cb(current, total, msg);
        }
    };

    report_progress(0, 100, "Loading users...");

    // Load users.json to build user_id -> display_name map
    let users_file = File::open(users_path).map_err(|e| AppError::ReadFile {
        path: users_path.to_string(),
        source: e,
    })?;
    let users_reader = BufReader::new(users_file);
    let users_data: Vec<serde_json::Value> =
        serde_json::from_reader(users_reader).map_err(|e| AppError::JsonParse(e.to_string()))?;

    let user_names: HashMap<String, String> = users_data
        .iter()
        .filter_map(|user| {
            let id = user.get("id")?.as_str()?.to_string();
            // Prefer display_name from profile, fall back to name, then id
            let name = user
                .get("profile")
                .and_then(|p| p.get("display_name"))
                .and_then(|n| n.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| user.get("name").and_then(|n| n.as_str()))
                .unwrap_or(&id)
                .to_string();
            Some((id, name))
        })
        .collect();

    report_progress(0, 100, "Loading channels...");

    // Load channels.json to build channel_id -> channel_name map
    let channels_file = File::open(channels_path).map_err(|e| AppError::ReadFile {
        path: channels_path.to_string(),
        source: e,
    })?;
    let channels_reader = BufReader::new(channels_file);
    let channels_data: Vec<serde_json::Value> =
        serde_json::from_reader(channels_reader).map_err(|e| AppError::JsonParse(e.to_string()))?;

    let channel_names: HashMap<String, String> = channels_data
        .iter()
        .filter_map(|ch| {
            let id = ch.get("id")?.as_str()?.to_string();
            let name = ch
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string();
            Some((id, name))
        })
        .collect();

    // Build SlackReferences for block rendering
    let slack_references = SlackReferences {
        users: user_names
            .iter()
            .map(|(id, name)| (SlackUserId::new(id.clone()), Some(name.clone())))
            .collect(),
        channels: channel_names
            .iter()
            .map(|(id, name)| (SlackChannelId::new(id.clone()), Some(name.clone())))
            .collect(),
        ..SlackReferences::default()
    };

    report_progress(0, 100, "Loading conversations...");

    // Load conversations.json
    let conv_file = File::open(conversations_path).map_err(|e| AppError::ReadFile {
        path: conversations_path.to_string(),
        source: e,
    })?;
    let conv_reader = BufReader::new(conv_file);
    let conversations: Vec<serde_json::Value> =
        serde_json::from_reader(conv_reader).map_err(|e| AppError::JsonParse(e.to_string()))?;

    // Count total messages for progress reporting
    let total_messages: usize = conversations
        .iter()
        .filter_map(|ch| ch.get("messages").and_then(|m| m.as_array()))
        .map(|msgs| msgs.len())
        .sum();

    report_progress(0, total_messages, "Processing messages...");

    let mut index_entries: Vec<IndexEntry> = Vec::new();
    let mut message_count = 0;

    // Process each channel entry in the conversations file
    for channel_entry in &conversations {
        let channel_id = channel_entry
            .get("channel_id")
            .and_then(|id| id.as_str())
            .unwrap_or("")
            .to_string();

        let channel_name = channel_entry
            .get("channel_name")
            .and_then(|n| n.as_str())
            .or_else(|| channel_names.get(&channel_id).map(|s| s.as_str()))
            .unwrap_or("unknown")
            .to_string();

        let messages = channel_entry
            .get("messages")
            .and_then(|m| m.as_array())
            .map(|a| a.as_slice())
            .unwrap_or(&[]);

        for message in messages {
            report_progress(message_count + 1, total_messages, "Processing messages...");

            // Get the unique message ID (ts field)
            let message_id = message
                .get("ts")
                .and_then(|ts| ts.as_str())
                .unwrap_or("")
                .to_string();

            if message_id.is_empty() {
                continue;
            }

            // Collect users involved in this thread
            let mut thread_user_ids: HashSet<String> = HashSet::new();

            // Add the main message author
            if let Some(user_id) = message.get("user").and_then(|u| u.as_str()) {
                thread_user_ids.insert(user_id.to_string());
            }

            // Build the markdown text for the main message
            let mut full_text = render_message_to_markdown(message, &slack_references, &user_names);

            // Process thread replies if present
            if let Some(replies) = message.get("thread_replies").and_then(|r| r.as_array()) {
                for reply in replies {
                    // Add reply author to users set
                    if let Some(user_id) = reply.get("user").and_then(|u| u.as_str()) {
                        thread_user_ids.insert(user_id.to_string());
                    }

                    // Render the reply
                    let reply_text =
                        render_message_to_markdown(reply, &slack_references, &user_names);
                    if !reply_text.is_empty() {
                        full_text.push_str("\n\n---\n\n");
                        full_text.push_str(&reply_text);
                    }
                }
            }

            // Build the users list
            let users: Vec<IndexUser> = thread_user_ids
                .iter()
                .map(|id| IndexUser {
                    id: id.clone(),
                    name: user_names.get(id).cloned().unwrap_or_else(|| id.clone()),
                })
                .collect();

            // Create the index entry
            let entry = IndexEntry {
                id: message_id.replace('.', "_"),
                ts: message_id.clone(),
                date: slack_ts_to_iso8601(&message_id),
                text: full_text,
                users,
                channel: IndexChannel {
                    id: channel_id.clone(),
                    name: channel_name.clone(),
                },
            };

            index_entries.push(entry);
            message_count += 1;
        }
    }

    report_progress(message_count, total_messages, "Writing output file...");

    // Write the index to the output file
    let output_file = File::create(output_path).map_err(|e| AppError::WriteFile {
        path: output_path.to_string(),
        source: e,
    })?;
    let writer = BufWriter::new(output_file);
    serde_json::to_writer_pretty(writer, &index_entries)
        .map_err(|e| AppError::JsonSerialize(e.to_string()))?;

    Ok(message_count)
}

/// Render a single message to markdown
fn render_message_to_markdown(
    message: &serde_json::Value,
    slack_references: &SlackReferences,
    user_names: &HashMap<String, String>,
) -> String {
    let mut output = String::new();

    // Get user name for the header
    let user_id = message.get("user").and_then(|u| u.as_str()).unwrap_or("");
    let user_name = user_names
        .get(user_id)
        .map(|s| s.as_str())
        .unwrap_or(user_id);

    // Add message header with username
    output.push_str(&format!("**{}**\n\n", user_name));

    // Try to render blocks if available
    if let Some(blocks_array) = message.get("blocks").and_then(|b| b.as_array()) {
        let blocks: Vec<SlackBlock> = blocks_array
            .iter()
            .filter_map(|block| {
                let block_type = block.get("type").and_then(|t| t.as_str())?;
                match block_type {
                    "rich_text" => Some(SlackBlock::RichText(block.clone())),
                    _ => None,
                }
            })
            .collect();

        if !blocks.is_empty() {
            let rendered = render_blocks_as_markdown(
                blocks,
                slack_references.clone(),
                Some("**".to_string()),
            );
            if !rendered.is_empty() {
                output.push_str(&rendered);
                return output;
            }
        }
    }

    // Fall back to plain text field
    let text = message
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("");
    output.push_str(text);

    output
}
