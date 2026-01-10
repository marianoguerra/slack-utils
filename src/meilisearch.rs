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
