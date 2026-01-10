use std::fs::File;
use std::io::BufReader;

use meilisearch_sdk::client::{Client, SwapIndexes};
use meilisearch_sdk::indexes::Index;
use meilisearch_sdk::task_info::TaskInfo;
use meilisearch_sdk::tasks::Task;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::index::IndexEntry;
use crate::settings::{MeilisearchSettings, Settings};
use crate::ProgressCallback;

/// Entry with sanitized ID for Meilisearch (no dots allowed)
#[derive(Debug, Clone, serde::Serialize)]
struct MeilisearchEntry {
    pub id: String,
    pub ts: String,
    pub date: String,
    pub text: String,
    pub users: Vec<crate::index::IndexUser>,
    pub channel: crate::index::IndexChannel,
}

impl From<IndexEntry> for MeilisearchEntry {
    fn from(entry: IndexEntry) -> Self {
        Self {
            // id is already sanitized in IndexEntry now
            id: entry.id,
            ts: entry.ts,
            date: entry.date,
            text: entry.text,
            users: entry.users,
            channel: entry.channel,
        }
    }
}

const BATCH_SIZE: usize = 100;
const TEMP_INDEX_PREFIX: &str = "slack_utils_temp_";

/// Result of importing to Meilisearch
#[derive(Debug)]
pub struct MeilisearchImportResult {
    pub total: usize,
    pub index_name: String,
}

/// Import conversation index to Meilisearch
pub async fn import_index_to_meilisearch(
    index_path: &str,
    url: &str,
    api_key: &str,
    index_name: &str,
    clear_index: bool,
    progress_callback: ProgressCallback<'_>,
) -> Result<MeilisearchImportResult> {
    let report_progress = |current: usize, total: usize, msg: &str| {
        if let Some(cb) = progress_callback {
            cb(current, total, msg);
        }
    };

    // Save settings
    report_progress(0, 0, "Saving settings...");
    save_meilisearch_settings(url, api_key, index_name)?;

    // Load the index file
    report_progress(0, 0, "Loading index file...");
    let file = File::open(index_path).map_err(|e| AppError::ReadFile {
        path: index_path.to_string(),
        source: e,
    })?;
    let reader = BufReader::new(file);
    let entries: Vec<IndexEntry> =
        serde_json::from_reader(reader).map_err(|e| AppError::JsonParse(e.to_string()))?;

    let total = entries.len();
    if total == 0 {
        return Ok(MeilisearchImportResult {
            total: 0,
            index_name: index_name.to_string(),
        });
    }

    // Create Meilisearch client
    report_progress(0, total, "Connecting to Meilisearch...");
    let client = Client::new(url, Some(api_key)).map_err(|e| AppError::Meilisearch(e.to_string()))?;

    // Convert to MeilisearchEntry with sanitized IDs
    let entries: Vec<MeilisearchEntry> = entries.into_iter().map(MeilisearchEntry::from).collect();

    if clear_index {
        import_with_swap(&client, index_name, entries, &report_progress).await
    } else {
        import_direct(&client, index_name, entries, &report_progress).await
    }
}

/// Import directly to the target index
async fn import_direct<F>(
    client: &Client,
    index_name: &str,
    entries: Vec<MeilisearchEntry>,
    report_progress: &F,
) -> Result<MeilisearchImportResult>
where
    F: Fn(usize, usize, &str),
{
    let total = entries.len();
    report_progress(0, total, "Getting or creating index...");

    let index = client.index(index_name);

    // Import in batches
    let mut imported = 0;
    for (batch_num, batch) in entries.chunks(BATCH_SIZE).enumerate() {
        report_progress(
            imported,
            total,
            &format!("Importing batch {}/{}", batch_num + 1, total.div_ceil(BATCH_SIZE)),
        );

        let task = index
            .add_documents(batch, Some("id"))
            .await
            .map_err(|e| AppError::Meilisearch(e.to_string()))?;

        wait_for_task(client, &task, report_progress, imported, total).await?;
        imported += batch.len();
    }

    report_progress(total, total, "Import complete");

    Ok(MeilisearchImportResult {
        total,
        index_name: index_name.to_string(),
    })
}

/// Import using a temp index and swap (for clearing)
async fn import_with_swap<F>(
    client: &Client,
    index_name: &str,
    entries: Vec<MeilisearchEntry>,
    report_progress: &F,
) -> Result<MeilisearchImportResult>
where
    F: Fn(usize, usize, &str),
{
    let total = entries.len();
    let temp_index_name = format!("{}{}", TEMP_INDEX_PREFIX, Uuid::new_v4());

    report_progress(0, total, "Creating temporary index...");

    // Create temp index
    let task = client
        .create_index(&temp_index_name, Some("id"))
        .await
        .map_err(|e| AppError::Meilisearch(e.to_string()))?;

    wait_for_task(client, &task, report_progress, 0, total).await?;

    let temp_index = client.index(&temp_index_name);

    // Copy settings from original index if it exists, or create the target index
    let target_exists = match get_index_if_exists(client, index_name).await {
        Ok(original_index) => {
            report_progress(0, total, "Copying index settings...");
            copy_index_settings(&original_index, &temp_index).await?;
            true
        }
        Err(_) => {
            // Target index doesn't exist, create it so swap works
            report_progress(0, total, "Creating target index...");
            let task = client
                .create_index(index_name, Some("id"))
                .await
                .map_err(|e| AppError::Meilisearch(e.to_string()))?;
            wait_for_task(client, &task, report_progress, 0, total).await?;
            false
        }
    };
    let _ = target_exists; // silence unused warning

    // Import in batches to temp index
    let mut imported = 0;
    for (batch_num, batch) in entries.chunks(BATCH_SIZE).enumerate() {
        report_progress(
            imported,
            total,
            &format!("Importing batch {}/{}", batch_num + 1, total.div_ceil(BATCH_SIZE)),
        );

        let task = temp_index
            .add_documents(batch, Some("id"))
            .await
            .map_err(|e| AppError::Meilisearch(e.to_string()))?;

        wait_for_task(client, &task, report_progress, imported, total).await?;
        imported += batch.len();
    }

    // Swap indexes
    report_progress(total, total, "Swapping indexes...");
    let task = client
        .swap_indexes([&SwapIndexes {
            indexes: (index_name.to_string(), temp_index_name.clone()),
            rename: None,
        }])
        .await
        .map_err(|e| AppError::Meilisearch(e.to_string()))?;

    wait_for_task(client, &task, report_progress, total, total).await?;

    // Delete temp index (which now contains old data)
    report_progress(total, total, "Cleaning up temporary index...");
    let task = client
        .index(&temp_index_name)
        .delete()
        .await
        .map_err(|e| AppError::Meilisearch(e.to_string()))?;

    wait_for_task(client, &task, report_progress, total, total).await?;

    report_progress(total, total, "Import complete");

    Ok(MeilisearchImportResult {
        total,
        index_name: index_name.to_string(),
    })
}

/// Wait for a Meilisearch task to complete
async fn wait_for_task<F>(
    client: &Client,
    task: &TaskInfo,
    report_progress: &F,
    current: usize,
    total: usize,
) -> Result<()>
where
    F: Fn(usize, usize, &str),
{
    loop {
        let status = client
            .get_task(task)
            .await
            .map_err(|e| AppError::Meilisearch(e.to_string()))?;

        match status {
            Task::Succeeded { .. } => return Ok(()),
            Task::Failed { content } => {
                let error_msg = &content.error.error_message;
                return Err(AppError::Meilisearch(format!("Task failed: {}", error_msg)));
            }
            Task::Enqueued { .. } | Task::Processing { .. } => {
                report_progress(current, total, "Waiting for Meilisearch...");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// Get an index if it exists
async fn get_index_if_exists(client: &Client, index_name: &str) -> Result<Index> {
    client
        .get_index(index_name)
        .await
        .map_err(|e| AppError::Meilisearch(e.to_string()))
}

/// Copy settings from one index to another
async fn copy_index_settings(source: &Index, target: &Index) -> Result<()> {
    // Get settings from source
    let settings = source
        .get_settings()
        .await
        .map_err(|e| AppError::Meilisearch(e.to_string()))?;

    // Apply to target
    target
        .set_settings(&settings)
        .await
        .map_err(|e| AppError::Meilisearch(e.to_string()))?;

    Ok(())
}

/// Save Meilisearch settings to settings.toml
fn save_meilisearch_settings(url: &str, api_key: &str, index_name: &str) -> Result<()> {
    let mut settings = Settings::load().unwrap_or_default();
    settings.meilisearch = MeilisearchSettings {
        url: url.to_string(),
        api_key: api_key.to_string(),
        index_name: index_name.to_string(),
    };
    settings.save()
}

/// Search result from Meilisearch
#[derive(Debug)]
pub struct MeilisearchSearchResult {
    pub hits: Vec<IndexEntry>,
    pub processing_time_ms: usize,
    pub estimated_total_hits: Option<usize>,
}

/// Query Meilisearch index
pub async fn query_meilisearch(
    url: &str,
    api_key: &str,
    index_name: &str,
    query: &str,
    limit: usize,
) -> Result<MeilisearchSearchResult> {
    let client = Client::new(url, Some(api_key)).map_err(|e| AppError::Meilisearch(e.to_string()))?;
    let index = client.index(index_name);

    let results = index
        .search()
        .with_query(query)
        .with_limit(limit)
        .execute::<IndexEntry>()
        .await
        .map_err(|e| AppError::Meilisearch(e.to_string()))?;

    Ok(MeilisearchSearchResult {
        hits: results.hits.into_iter().map(|h| h.result).collect(),
        processing_time_ms: results.processing_time_ms,
        estimated_total_hits: results.estimated_total_hits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{IndexChannel, IndexEntry, IndexUser};

    #[test]
    fn test_batch_size_constant() {
        assert_eq!(BATCH_SIZE, 100);
    }

    #[test]
    fn test_temp_index_prefix_constant() {
        assert_eq!(TEMP_INDEX_PREFIX, "slack_utils_temp_");
    }

    #[test]
    fn test_meilisearch_entry_from_index_entry() {
        let index_entry = IndexEntry {
            id: "1234567890_123456".to_string(),
            ts: "1234567890.123456".to_string(),
            date: "2009-02-13T23:31:30+00:00".to_string(),
            text: "Test message".to_string(),
            users: vec![IndexUser {
                id: "U123".to_string(),
                name: "testuser".to_string(),
            }],
            channel: IndexChannel {
                id: "C456".to_string(),
                name: "general".to_string(),
            },
        };

        let ms_entry = MeilisearchEntry::from(index_entry.clone());

        assert_eq!(ms_entry.id, index_entry.id);
        assert_eq!(ms_entry.ts, index_entry.ts);
        assert_eq!(ms_entry.date, index_entry.date);
        assert_eq!(ms_entry.text, index_entry.text);
        assert_eq!(ms_entry.users.len(), 1);
        assert_eq!(ms_entry.users[0].id, "U123");
        assert_eq!(ms_entry.users[0].name, "testuser");
        assert_eq!(ms_entry.channel.id, "C456");
        assert_eq!(ms_entry.channel.name, "general");
    }

    #[test]
    fn test_meilisearch_entry_serialization() {
        let entry = MeilisearchEntry {
            id: "1234567890_123456".to_string(),
            ts: "1234567890.123456".to_string(),
            date: "2009-02-13T23:31:30+00:00".to_string(),
            text: "Test message".to_string(),
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

        assert!(json.contains("\"id\":\"1234567890_123456\""));
        assert!(json.contains("\"ts\":\"1234567890.123456\""));
        assert!(json.contains("\"date\":\"2009-02-13T23:31:30+00:00\""));
        assert!(json.contains("\"text\":\"Test message\""));
        assert!(json.contains("\"name\":\"testuser\""));
        assert!(json.contains("\"name\":\"general\""));
    }

    #[test]
    fn test_meilisearch_entry_id_has_no_dots() {
        let index_entry = IndexEntry {
            id: "1234567890_123456".to_string(), // Already sanitized
            ts: "1234567890.123456".to_string(),
            date: "2009-02-13T23:31:30+00:00".to_string(),
            text: "Test".to_string(),
            users: vec![],
            channel: IndexChannel {
                id: "C456".to_string(),
                name: "general".to_string(),
            },
        };

        let ms_entry = MeilisearchEntry::from(index_entry);

        // ID should not contain dots (Meilisearch requirement)
        assert!(!ms_entry.id.contains('.'));
    }

    #[test]
    fn test_meilisearch_entry_preserves_original_ts() {
        let index_entry = IndexEntry {
            id: "1234567890_123456".to_string(),
            ts: "1234567890.123456".to_string(), // Original with dot
            date: "2009-02-13T23:31:30+00:00".to_string(),
            text: "Test".to_string(),
            users: vec![],
            channel: IndexChannel {
                id: "C456".to_string(),
                name: "general".to_string(),
            },
        };

        let ms_entry = MeilisearchEntry::from(index_entry);

        // ts should preserve the original format with dot
        assert!(ms_entry.ts.contains('.'));
        assert_eq!(ms_entry.ts, "1234567890.123456");
    }

    #[test]
    fn test_meilisearch_entry_with_multiple_users() {
        let index_entry = IndexEntry {
            id: "123_456".to_string(),
            ts: "123.456".to_string(),
            date: "2009-02-13T23:31:30+00:00".to_string(),
            text: "Thread message".to_string(),
            users: vec![
                IndexUser {
                    id: "U001".to_string(),
                    name: "alice".to_string(),
                },
                IndexUser {
                    id: "U002".to_string(),
                    name: "bob".to_string(),
                },
                IndexUser {
                    id: "U003".to_string(),
                    name: "charlie".to_string(),
                },
            ],
            channel: IndexChannel {
                id: "C789".to_string(),
                name: "random".to_string(),
            },
        };

        let ms_entry = MeilisearchEntry::from(index_entry);

        assert_eq!(ms_entry.users.len(), 3);
        assert_eq!(ms_entry.users[0].name, "alice");
        assert_eq!(ms_entry.users[1].name, "bob");
        assert_eq!(ms_entry.users[2].name, "charlie");
    }

    #[test]
    fn test_meilisearch_entry_with_empty_users() {
        let index_entry = IndexEntry {
            id: "123_456".to_string(),
            ts: "123.456".to_string(),
            date: "2009-02-13T23:31:30+00:00".to_string(),
            text: "System message".to_string(),
            users: vec![],
            channel: IndexChannel {
                id: "C789".to_string(),
                name: "announcements".to_string(),
            },
        };

        let ms_entry = MeilisearchEntry::from(index_entry);

        assert!(ms_entry.users.is_empty());
    }

    #[test]
    fn test_meilisearch_import_result() {
        let result = MeilisearchImportResult {
            total: 100,
            index_name: "test-index".to_string(),
        };

        assert_eq!(result.total, 100);
        assert_eq!(result.index_name, "test-index");
    }

    #[test]
    fn test_meilisearch_search_result() {
        let result = MeilisearchSearchResult {
            hits: vec![IndexEntry {
                id: "123_456".to_string(),
                ts: "123.456".to_string(),
                date: "2009-02-13T23:31:30+00:00".to_string(),
                text: "Found message".to_string(),
                users: vec![],
                channel: IndexChannel {
                    id: "C789".to_string(),
                    name: "general".to_string(),
                },
            }],
            processing_time_ms: 5,
            estimated_total_hits: Some(42),
        };

        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.processing_time_ms, 5);
        assert_eq!(result.estimated_total_hits, Some(42));
    }

    #[test]
    fn test_meilisearch_search_result_no_estimated_hits() {
        let result = MeilisearchSearchResult {
            hits: vec![],
            processing_time_ms: 1,
            estimated_total_hits: None,
        };

        assert!(result.hits.is_empty());
        assert_eq!(result.processing_time_ms, 1);
        assert_eq!(result.estimated_total_hits, None);
    }

    #[test]
    fn test_temp_index_name_format() {
        let uuid = Uuid::new_v4();
        let temp_name = format!("{}{}", TEMP_INDEX_PREFIX, uuid);

        assert!(temp_name.starts_with("slack_utils_temp_"));
        assert!(temp_name.len() > TEMP_INDEX_PREFIX.len());
    }
}
