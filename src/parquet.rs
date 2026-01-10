use std::collections::HashMap;
use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanArray, Int32Array, Int64Array, StringBuilder};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use chrono::{DateTime, Datelike};
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;

use crate::{AppError, Result};

/// Write users data to a parquet file
pub fn write_users_parquet(path: &Path, users: &[serde_json::Value]) -> Result<()> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("real_name", DataType::Utf8, true),
        Field::new("display_name", DataType::Utf8, true),
        Field::new("email", DataType::Utf8, true),
        Field::new("is_bot", DataType::Boolean, true),
        Field::new("is_admin", DataType::Boolean, true),
        Field::new("tz", DataType::Utf8, true),
    ]));

    let mut id_builder = StringBuilder::new();
    let mut name_builder = StringBuilder::new();
    let mut real_name_builder = StringBuilder::new();
    let mut display_name_builder = StringBuilder::new();
    let mut email_builder = StringBuilder::new();
    let mut is_bot_builder: Vec<Option<bool>> = Vec::new();
    let mut is_admin_builder: Vec<Option<bool>> = Vec::new();
    let mut tz_builder = StringBuilder::new();

    for user in users {
        id_builder.append_value(user.get("id").and_then(|v| v.as_str()).unwrap_or(""));
        name_builder.append_option(user.get("name").and_then(|v| v.as_str()));
        real_name_builder.append_option(user.get("real_name").and_then(|v| v.as_str()));
        display_name_builder.append_option(
            user.get("profile")
                .and_then(|p| p.get("display_name"))
                .and_then(|v| v.as_str()),
        );
        email_builder.append_option(
            user.get("profile")
                .and_then(|p| p.get("email"))
                .and_then(|v| v.as_str()),
        );
        is_bot_builder.push(user.get("is_bot").and_then(|v| v.as_bool()));
        is_admin_builder.push(user.get("is_admin").and_then(|v| v.as_bool()));
        tz_builder.append_option(user.get("tz").and_then(|v| v.as_str()));
    }

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(id_builder.finish()) as ArrayRef,
            Arc::new(name_builder.finish()) as ArrayRef,
            Arc::new(real_name_builder.finish()) as ArrayRef,
            Arc::new(display_name_builder.finish()) as ArrayRef,
            Arc::new(email_builder.finish()) as ArrayRef,
            Arc::new(BooleanArray::from(is_bot_builder)) as ArrayRef,
            Arc::new(BooleanArray::from(is_admin_builder)) as ArrayRef,
            Arc::new(tz_builder.finish()) as ArrayRef,
        ],
    )
    .map_err(|e| AppError::Parquet(e.to_string()))?;

    write_parquet_file(path, &schema, &[batch])
}

/// Write channels data to a parquet file
pub fn write_channels_parquet(path: &Path, channels: &[serde_json::Value]) -> Result<()> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("topic", DataType::Utf8, true),
        Field::new("purpose", DataType::Utf8, true),
        Field::new("is_private", DataType::Boolean, true),
        Field::new("is_archived", DataType::Boolean, true),
        Field::new("created", DataType::Int64, true),
        Field::new("num_members", DataType::Int32, true),
    ]));

    let mut id_builder = StringBuilder::new();
    let mut name_builder = StringBuilder::new();
    let mut topic_builder = StringBuilder::new();
    let mut purpose_builder = StringBuilder::new();
    let mut is_private_builder: Vec<Option<bool>> = Vec::new();
    let mut is_archived_builder: Vec<Option<bool>> = Vec::new();
    let mut created_builder: Vec<Option<i64>> = Vec::new();
    let mut num_members_builder: Vec<Option<i32>> = Vec::new();

    for channel in channels {
        id_builder.append_value(channel.get("id").and_then(|v| v.as_str()).unwrap_or(""));
        name_builder.append_option(channel.get("name").and_then(|v| v.as_str()));
        topic_builder.append_option(
            channel
                .get("topic")
                .and_then(|t| t.get("value"))
                .and_then(|v| v.as_str()),
        );
        purpose_builder.append_option(
            channel
                .get("purpose")
                .and_then(|p| p.get("value"))
                .and_then(|v| v.as_str()),
        );
        is_private_builder.push(channel.get("is_private").and_then(|v| v.as_bool()));
        is_archived_builder.push(channel.get("is_archived").and_then(|v| v.as_bool()));
        created_builder.push(channel.get("created").and_then(|v| v.as_i64()));
        num_members_builder.push(
            channel
                .get("num_members")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32),
        );
    }

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(id_builder.finish()) as ArrayRef,
            Arc::new(name_builder.finish()) as ArrayRef,
            Arc::new(topic_builder.finish()) as ArrayRef,
            Arc::new(purpose_builder.finish()) as ArrayRef,
            Arc::new(BooleanArray::from(is_private_builder)) as ArrayRef,
            Arc::new(BooleanArray::from(is_archived_builder)) as ArrayRef,
            Arc::new(Int64Array::from(created_builder)) as ArrayRef,
            Arc::new(Int32Array::from(num_members_builder)) as ArrayRef,
        ],
    )
    .map_err(|e| AppError::Parquet(e.to_string()))?;

    write_parquet_file(path, &schema, &[batch])
}

/// Flattened message for parquet export
struct FlatMessage {
    ts: String,
    user: Option<String>,
    text: Option<String>,
    channel_id: String,
    channel_name: String,
    thread_ts: Option<String>,
    is_reply: bool,
    date: String,
    year: i32,
    week: i32,
    blocks: Option<String>,
}

/// Write conversations data to partitioned parquet files (Hive-style: year=YYYY/week=WW)
pub fn write_conversations_parquet(
    base_path: &Path,
    conversations: &[serde_json::Value],
) -> Result<usize> {
    // Flatten all messages and group by year/week
    let mut messages_by_partition: HashMap<(i32, i32), Vec<FlatMessage>> = HashMap::new();

    for conv in conversations {
        let channel_id = conv
            .get("channel_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let channel_name = conv
            .get("channel_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if let Some(messages) = conv.get("messages").and_then(|m| m.as_array()) {
            for msg in messages {
                // Process parent message
                if let Some(flat) =
                    flatten_message(msg, &channel_id, &channel_name, None, false)
                {
                    let key = (flat.year, flat.week);
                    messages_by_partition.entry(key).or_default().push(flat);
                }

                // Process thread replies
                if let Some(replies) = msg.get("thread_replies").and_then(|r| r.as_array()) {
                    let parent_ts = msg.get("ts").and_then(|v| v.as_str()).map(|s| s.to_string());
                    for reply in replies {
                        if let Some(flat) =
                            flatten_message(reply, &channel_id, &channel_name, parent_ts.clone(), true)
                        {
                            let key = (flat.year, flat.week);
                            messages_by_partition.entry(key).or_default().push(flat);
                        }
                    }
                }
            }
        }
    }

    let mut total_written = 0;

    // Write each partition
    for ((year, week), messages) in messages_by_partition {
        let partition_path = base_path.join(format!("year={}/week={:02}", year, week));
        fs::create_dir_all(&partition_path).map_err(|e| AppError::WriteFile {
            path: partition_path.display().to_string(),
            source: e,
        })?;

        let file_path = partition_path.join("threads.parquet");
        write_messages_parquet(&file_path, &messages)?;
        total_written += messages.len();
    }

    Ok(total_written)
}

fn flatten_message(
    msg: &serde_json::Value,
    channel_id: &str,
    channel_name: &str,
    parent_ts: Option<String>,
    is_reply: bool,
) -> Option<FlatMessage> {
    let ts = msg.get("ts").and_then(|v| v.as_str())?.to_string();

    // Parse timestamp to get date info
    let ts_float: f64 = ts.parse().ok()?;
    let datetime = DateTime::from_timestamp(ts_float as i64, 0)?.naive_utc();
    let date = datetime.format("%Y-%m-%d").to_string();
    let year = datetime.iso_week().year();
    let week = datetime.iso_week().week() as i32;

    let user = msg.get("user").and_then(|v| v.as_str()).map(|s| s.to_string());
    let text = msg.get("text").and_then(|v| v.as_str()).map(|s| s.to_string());
    let thread_ts = if is_reply {
        parent_ts
    } else {
        msg.get("thread_ts")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    let blocks = msg
        .get("blocks")
        .map(|b| serde_json::to_string(b).unwrap_or_default());

    Some(FlatMessage {
        ts,
        user,
        text,
        channel_id: channel_id.to_string(),
        channel_name: channel_name.to_string(),
        thread_ts,
        is_reply,
        date,
        year,
        week,
        blocks,
    })
}

fn write_messages_parquet(path: &Path, messages: &[FlatMessage]) -> Result<()> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("ts", DataType::Utf8, false),
        Field::new("user", DataType::Utf8, true),
        Field::new("text", DataType::Utf8, true),
        Field::new("channel_id", DataType::Utf8, false),
        Field::new("channel_name", DataType::Utf8, false),
        Field::new("thread_ts", DataType::Utf8, true),
        Field::new("is_reply", DataType::Boolean, false),
        Field::new("date", DataType::Utf8, false),
        Field::new("year", DataType::Int32, false),
        Field::new("week", DataType::Int32, false),
        Field::new("blocks", DataType::Utf8, true),
    ]));

    let mut ts_builder = StringBuilder::new();
    let mut user_builder = StringBuilder::new();
    let mut text_builder = StringBuilder::new();
    let mut channel_id_builder = StringBuilder::new();
    let mut channel_name_builder = StringBuilder::new();
    let mut thread_ts_builder = StringBuilder::new();
    let mut is_reply_builder: Vec<bool> = Vec::new();
    let mut date_builder = StringBuilder::new();
    let mut year_builder: Vec<i32> = Vec::new();
    let mut week_builder: Vec<i32> = Vec::new();
    let mut blocks_builder = StringBuilder::new();

    for msg in messages {
        ts_builder.append_value(&msg.ts);
        user_builder.append_option(msg.user.as_deref());
        text_builder.append_option(msg.text.as_deref());
        channel_id_builder.append_value(&msg.channel_id);
        channel_name_builder.append_value(&msg.channel_name);
        thread_ts_builder.append_option(msg.thread_ts.as_deref());
        is_reply_builder.push(msg.is_reply);
        date_builder.append_value(&msg.date);
        year_builder.push(msg.year);
        week_builder.push(msg.week);
        blocks_builder.append_option(msg.blocks.as_deref());
    }

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(ts_builder.finish()) as ArrayRef,
            Arc::new(user_builder.finish()) as ArrayRef,
            Arc::new(text_builder.finish()) as ArrayRef,
            Arc::new(channel_id_builder.finish()) as ArrayRef,
            Arc::new(channel_name_builder.finish()) as ArrayRef,
            Arc::new(thread_ts_builder.finish()) as ArrayRef,
            Arc::new(BooleanArray::from(is_reply_builder)) as ArrayRef,
            Arc::new(date_builder.finish()) as ArrayRef,
            Arc::new(Int32Array::from(year_builder)) as ArrayRef,
            Arc::new(Int32Array::from(week_builder)) as ArrayRef,
            Arc::new(blocks_builder.finish()) as ArrayRef,
        ],
    )
    .map_err(|e| AppError::Parquet(e.to_string()))?;

    write_parquet_file(path, &schema, &[batch])
}

fn write_parquet_file(path: &Path, schema: &Arc<Schema>, batches: &[RecordBatch]) -> Result<()> {
    let file = File::create(path).map_err(|e| AppError::WriteFile {
        path: path.display().to_string(),
        source: e,
    })?;

    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();

    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))
        .map_err(|e| AppError::Parquet(e.to_string()))?;

    for batch in batches {
        writer
            .write(batch)
            .map_err(|e| AppError::Parquet(e.to_string()))?;
    }

    writer
        .close()
        .map_err(|e| AppError::Parquet(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_write_users_parquet_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("users.parquet");

        let result = write_users_parquet(&path, &[]);
        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_write_users_parquet_with_data() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("users.parquet");

        let users = vec![serde_json::json!({
            "id": "U123",
            "name": "testuser",
            "real_name": "Test User",
            "profile": {
                "display_name": "Test",
                "email": "test@example.com"
            },
            "is_bot": false,
            "is_admin": true,
            "tz": "America/New_York"
        })];

        let result = write_users_parquet(&path, &users);
        assert!(result.is_ok());
        assert!(path.exists());
        assert!(fs::metadata(&path).unwrap().len() > 0);
    }

    #[test]
    fn test_write_channels_parquet_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("channels.parquet");

        let result = write_channels_parquet(&path, &[]);
        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_write_channels_parquet_with_data() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("channels.parquet");

        let channels = vec![serde_json::json!({
            "id": "C123",
            "name": "general",
            "topic": {"value": "General discussion"},
            "purpose": {"value": "Company-wide announcements"},
            "is_private": false,
            "is_archived": false,
            "created": 1609459200,
            "num_members": 100
        })];

        let result = write_channels_parquet(&path, &channels);
        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_write_conversations_parquet_empty() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().join("threads");

        let result = write_conversations_parquet(&base_path, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_conversations_parquet_with_data() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().join("threads");

        // Use a timestamp for 2024-01-15 (week 3)
        let ts = "1705312800.000000"; // 2024-01-15 10:00:00 UTC

        let conversations = vec![serde_json::json!({
            "channel_id": "C123",
            "channel_name": "general",
            "messages": [
                {
                    "ts": ts,
                    "user": "U123",
                    "text": "Hello world"
                }
            ]
        })];

        let result = write_conversations_parquet(&base_path, &conversations);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // Check partition structure
        let partition_path = base_path.join("year=2024").join("week=03");
        assert!(partition_path.exists());
        assert!(partition_path.join("threads.parquet").exists());
    }

    #[test]
    fn test_write_conversations_parquet_with_thread_replies() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().join("threads");

        let parent_ts = "1705312800.000000";
        let reply_ts = "1705313400.000000";

        let conversations = vec![serde_json::json!({
            "channel_id": "C123",
            "channel_name": "general",
            "messages": [
                {
                    "ts": parent_ts,
                    "user": "U123",
                    "text": "Parent message",
                    "thread_replies": [
                        {
                            "ts": reply_ts,
                            "user": "U456",
                            "text": "Reply message"
                        }
                    ]
                }
            ]
        })];

        let result = write_conversations_parquet(&base_path, &conversations);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2); // Parent + reply
    }

    #[test]
    fn test_flatten_message() {
        let msg = serde_json::json!({
            "ts": "1705312800.000000",
            "user": "U123",
            "text": "Test message"
        });

        let result = flatten_message(&msg, "C123", "general", None, false);
        assert!(result.is_some());

        let flat = result.unwrap();
        assert_eq!(flat.ts, "1705312800.000000");
        assert_eq!(flat.user, Some("U123".to_string()));
        assert_eq!(flat.text, Some("Test message".to_string()));
        assert_eq!(flat.channel_id, "C123");
        assert_eq!(flat.channel_name, "general");
        assert!(!flat.is_reply);
        assert_eq!(flat.date, "2024-01-15");
        assert_eq!(flat.year, 2024);
        assert_eq!(flat.week, 3);
    }

    #[test]
    fn test_flatten_message_reply() {
        let msg = serde_json::json!({
            "ts": "1705312800.000000",
            "user": "U456",
            "text": "Reply"
        });

        let result = flatten_message(
            &msg,
            "C123",
            "general",
            Some("1705300000.000000".to_string()),
            true,
        );
        assert!(result.is_some());

        let flat = result.unwrap();
        assert!(flat.is_reply);
        assert_eq!(flat.thread_ts, Some("1705300000.000000".to_string()));
    }

    #[test]
    fn test_flatten_message_missing_ts() {
        let msg = serde_json::json!({
            "user": "U123",
            "text": "No timestamp"
        });

        let result = flatten_message(&msg, "C123", "general", None, false);
        assert!(result.is_none());
    }
}
