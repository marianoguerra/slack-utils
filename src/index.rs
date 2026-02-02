use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufWriter;

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use crate::slack_render::{render_blocks_as_markdown, SlackReferences};
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
    let users_data: Vec<serde_json::Value> = crate::load_json_file(users_path)?;

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
    let channels_data: Vec<serde_json::Value> = crate::load_json_file(channels_path)?;

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
    let conversations: Vec<serde_json::Value> = crate::load_json_file(conversations_path)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_slack_ts_to_iso8601_valid() {
        let ts = "1767636991.559059";
        let result = slack_ts_to_iso8601(ts);
        assert_eq!(result, "2026-01-05T18:16:31+00:00");
    }

    #[test]
    fn test_slack_ts_to_iso8601_integer_only() {
        let ts = "1767636991";
        let result = slack_ts_to_iso8601(ts);
        assert_eq!(result, "2026-01-05T18:16:31+00:00");
    }

    #[test]
    fn test_slack_ts_to_iso8601_zero() {
        let ts = "0";
        let result = slack_ts_to_iso8601(ts);
        assert_eq!(result, "1970-01-01T00:00:00+00:00");
    }

    #[test]
    fn test_slack_ts_to_iso8601_empty() {
        let ts = "";
        let result = slack_ts_to_iso8601(ts);
        assert_eq!(result, "1970-01-01T00:00:00+00:00");
    }

    #[test]
    fn test_slack_ts_to_iso8601_invalid() {
        let ts = "invalid";
        let result = slack_ts_to_iso8601(ts);
        assert_eq!(result, "1970-01-01T00:00:00+00:00");
    }

    #[test]
    fn test_index_entry_serialization() {
        let entry = IndexEntry {
            id: "1234567890_123456".to_string(),
            ts: "1234567890.123456".to_string(),
            date: "2009-02-13T23:31:30+00:00".to_string(),
            text: "Hello world".to_string(),
            users: vec![IndexUser {
                id: "U123".to_string(),
                name: "testuser".to_string(),
            }],
            channel: IndexChannel {
                id: "C456".to_string(),
                name: "general".to_string(),
            },
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: IndexEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, entry.id);
        assert_eq!(deserialized.ts, entry.ts);
        assert_eq!(deserialized.date, entry.date);
        assert_eq!(deserialized.text, entry.text);
        assert_eq!(deserialized.users.len(), 1);
        assert_eq!(deserialized.users[0].id, "U123");
        assert_eq!(deserialized.users[0].name, "testuser");
        assert_eq!(deserialized.channel.id, "C456");
        assert_eq!(deserialized.channel.name, "general");
    }

    #[test]
    fn test_index_user_serialization() {
        let user = IndexUser {
            id: "U123".to_string(),
            name: "Test User".to_string(),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"id\":\"U123\""));
        assert!(json.contains("\"name\":\"Test User\""));

        let deserialized: IndexUser = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, user.id);
        assert_eq!(deserialized.name, user.name);
    }

    #[test]
    fn test_index_channel_serialization() {
        let channel = IndexChannel {
            id: "C456".to_string(),
            name: "random".to_string(),
        };

        let json = serde_json::to_string(&channel).unwrap();
        assert!(json.contains("\"id\":\"C456\""));
        assert!(json.contains("\"name\":\"random\""));

        let deserialized: IndexChannel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, channel.id);
        assert_eq!(deserialized.name, channel.name);
    }

    #[test]
    fn test_render_message_to_markdown_plain_text() {
        let message = json!({
            "user": "U123",
            "text": "Hello, world!"
        });
        let slack_references = SlackReferences::default();
        let mut user_names = HashMap::new();
        user_names.insert("U123".to_string(), "TestUser".to_string());

        let result = render_message_to_markdown(&message, &slack_references, &user_names);

        assert!(result.contains("**TestUser**"));
        assert!(result.contains("Hello, world!"));
    }

    #[test]
    fn test_render_message_to_markdown_unknown_user() {
        let message = json!({
            "user": "U999",
            "text": "Message from unknown"
        });
        let slack_references = SlackReferences::default();
        let user_names = HashMap::new();

        let result = render_message_to_markdown(&message, &slack_references, &user_names);

        assert!(result.contains("**U999**"));
        assert!(result.contains("Message from unknown"));
    }

    #[test]
    fn test_render_message_to_markdown_empty_text() {
        let message = json!({
            "user": "U123",
            "text": ""
        });
        let slack_references = SlackReferences::default();
        let mut user_names = HashMap::new();
        user_names.insert("U123".to_string(), "TestUser".to_string());

        let result = render_message_to_markdown(&message, &slack_references, &user_names);

        assert!(result.contains("**TestUser**"));
    }

    #[test]
    fn test_render_message_to_markdown_with_rich_text_block() {
        let message = json!({
            "user": "U123",
            "text": "fallback text",
            "blocks": [
                {
                    "type": "rich_text",
                    "elements": [
                        {
                            "type": "rich_text_section",
                            "elements": [
                                {
                                    "type": "text",
                                    "text": "Rich text content"
                                }
                            ]
                        }
                    ]
                }
            ]
        });
        let slack_references = SlackReferences::default();
        let mut user_names = HashMap::new();
        user_names.insert("U123".to_string(), "TestUser".to_string());

        let result = render_message_to_markdown(&message, &slack_references, &user_names);

        assert!(result.contains("**TestUser**"));
        assert!(result.contains("Rich text content"));
    }

    #[test]
    fn test_index_entry_id_sanitization() {
        // Verify that dots are replaced with underscores in the id
        let ts = "1767636991.559059";
        let id = ts.replace('.', "_");
        assert_eq!(id, "1767636991_559059");
        assert!(!id.contains('.'));
    }
}
