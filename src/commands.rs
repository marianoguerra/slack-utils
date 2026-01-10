use std::path::Path;

use crate::error::Result;
use crate::index::export_conversations_to_index;
use crate::markdown::export_conversations_to_markdown;
use crate::meilisearch::{import_index_to_meilisearch, query_meilisearch};
use crate::slack;
use crate::{
    current_iso_week, default_from_date, default_to_date, load_token, parse_date,
    week_to_date_range, OutputFormat,
};

/// Derive output path based on format
fn derive_output_path(base: &str, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => format!("{}.json", base),
        OutputFormat::Parquet => format!("{}.parquet", base),
    }
}

pub async fn run_export_conversations(
    from: Option<String>,
    to: Option<String>,
    output: &str,
    format_str: &str,
) -> Result<()> {
    let token = load_token()?;
    let format: OutputFormat = format_str.parse()?;

    let from_date = match from {
        Some(s) => parse_date(&s)?,
        None => default_from_date(),
    };
    let to_date = match to {
        Some(s) => parse_date(&s)?,
        None => default_to_date(),
    };

    // For parquet, output is a directory; for json, output is a file
    let output_path = match format {
        OutputFormat::Json => derive_output_path(output, format),
        OutputFormat::Parquet => output.to_string(), // Keep as directory path
    };

    println!(
        "Exporting conversations from {} to {} to {} (format: {})...",
        from_date, to_date, output_path, format
    );

    let progress_callback = |current: usize, total: usize, name: &str| {
        if total > 0 {
            println!("  [{}/{}] {}", current, total, name);
        } else {
            println!("  {}", name);
        }
    };

    let count = slack::export_conversations(
        &token,
        from_date,
        to_date,
        Path::new(&output_path),
        None,
        Some(progress_callback),
        format,
    )
    .await?;

    println!(
        "Export completed successfully! {} messages exported.",
        count
    );
    Ok(())
}

pub async fn run_export_conversations_week(
    year: Option<i32>,
    week: Option<u32>,
    output: &str,
    format_str: &str,
) -> Result<()> {
    let token = load_token()?;
    let format: OutputFormat = format_str.parse()?;

    // Default to current ISO week
    let (default_year, default_week) = current_iso_week();
    let year = year.unwrap_or(default_year);
    let week = week.unwrap_or(default_week);

    // Convert year/week to date range
    let (from_date, to_date) = week_to_date_range(year, week)?;

    // For parquet, output is a directory; for json, output is a file
    let output_path = match format {
        OutputFormat::Json => derive_output_path(output, format),
        OutputFormat::Parquet => output.to_string(),
    };

    println!(
        "Exporting conversations for {}-W{:02} ({} to {}) to {} (format: {})...",
        year, week, from_date, to_date, output_path, format
    );

    let progress_callback = |current: usize, total: usize, name: &str| {
        if total > 0 {
            println!("  [{}/{}] {}", current, total, name);
        } else {
            println!("  {}", name);
        }
    };

    let count = slack::export_conversations(
        &token,
        from_date,
        to_date,
        Path::new(&output_path),
        None,
        Some(progress_callback),
        format,
    )
    .await?;

    println!(
        "Export completed successfully! {} messages exported.",
        count
    );
    Ok(())
}

pub async fn run_export_users(output: &str, format_str: &str) -> Result<()> {
    let token = load_token()?;
    let format: OutputFormat = format_str.parse()?;
    let output_path = derive_output_path(output, format);

    println!("Exporting users to {} (format: {})...", output_path, format);

    let count = slack::export_users(&token, Path::new(&output_path), format).await?;

    println!("Export completed successfully! {} users exported.", count);
    Ok(())
}

pub async fn run_export_channels(output: &str, format_str: &str) -> Result<()> {
    let token = load_token()?;
    let format: OutputFormat = format_str.parse()?;
    let output_path = derive_output_path(output, format);

    println!("Exporting channels to {} (format: {})...", output_path, format);

    let count = slack::export_channels(&token, Path::new(&output_path), format).await?;

    println!(
        "Export completed successfully! {} channels exported.",
        count
    );
    Ok(())
}

pub fn run_download_attachments(input: &str, output: &str) -> Result<()> {
    let token = load_token()?;

    println!("Downloading attachments from {} to {}...", input, output);

    let result = slack::download_attachments(
        &token,
        input,
        Path::new(output),
        Some(&|current, total, name| {
            println!("  [{}/{}] {}", current, total, name);
        }),
    )?;

    println!(
        "Download completed! {} files downloaded, {} skipped, {} failed.",
        result.downloaded, result.skipped, result.failed
    );
    for error in &result.errors {
        eprintln!("  {}", error);
    }
    Ok(())
}

pub fn run_export_markdown(
    conversations: &str,
    users: &str,
    channels: &str,
    output: &str,
) -> Result<()> {
    println!("Exporting selected conversations to markdown...");

    let count = export_conversations_to_markdown(conversations, users, channels, output)?;

    println!(
        "Export completed successfully! {} messages exported to {}",
        count, output
    );
    Ok(())
}

pub async fn run_export_emojis(output: &str, folder: &str) -> Result<()> {
    let token = load_token()?;

    println!("Exporting custom emojis to {} (images to {})...", output, folder);

    let result = slack::fetch_emojis(
        &token,
        Path::new(output),
        Path::new(folder),
        Some(&|current, total, name| {
            if total > 0 {
                println!("  [{}/{}] {}", current, total, name);
            } else {
                println!("  {}", name);
            }
        }),
    )
    .await?;

    println!(
        "Export completed! {} emojis total ({} downloaded, {} skipped, {} failed).",
        result.total, result.downloaded, result.skipped, result.failed
    );
    for error in &result.errors {
        eprintln!("  {}", error);
    }
    Ok(())
}

pub fn run_export_index(
    conversations: &str,
    users: &str,
    channels: &str,
    output: &str,
) -> Result<()> {
    println!("Exporting conversations to index...");

    let count = export_conversations_to_index(conversations, users, channels, output)?;

    println!(
        "Export completed successfully! {} messages exported to {}",
        count, output
    );
    Ok(())
}

pub async fn run_import_index_meilisearch(
    input: &str,
    url: &str,
    api_key: &str,
    index_name: &str,
    clear: bool,
) -> Result<()> {
    println!(
        "Importing index to Meilisearch at {} (index: {})...",
        url, index_name
    );
    if clear {
        println!("  Index will be cleared (using swap operation)");
    }

    let progress_callback = |current: usize, total: usize, name: &str| {
        if total > 0 {
            println!("  [{}/{}] {}", current, total, name);
        } else {
            println!("  {}", name);
        }
    };

    let result = import_index_to_meilisearch(
        input,
        url,
        api_key,
        index_name,
        clear,
        Some(&progress_callback),
    )
    .await?;

    println!(
        "Import completed successfully! {} documents imported to index '{}'",
        result.total, result.index_name
    );
    Ok(())
}

pub async fn run_query_meilisearch(
    url: &str,
    api_key: &str,
    index_name: &str,
    query: &str,
    limit: usize,
) -> Result<()> {
    println!("Searching '{}' in index '{}'...\n", query, index_name);

    let result = query_meilisearch(url, api_key, index_name, query, limit).await?;

    if result.hits.is_empty() {
        println!("No results found.");
    } else {
        println!(
            "Found {} results (showing {}, {}ms):\n",
            result.estimated_total_hits.unwrap_or(result.hits.len()),
            result.hits.len(),
            result.processing_time_ms
        );

        for (i, hit) in result.hits.iter().enumerate() {
            println!("{}. [{}] #{}", i + 1, hit.date, hit.channel.name);
            println!("   Users: {}", hit.users.iter().map(|u| u.name.as_str()).collect::<Vec<_>>().join(", "));

            // Show first 200 chars of text
            let preview: String = hit.text.chars().take(200).collect();
            let preview = preview.replace('\n', " ");
            if hit.text.len() > 200 {
                println!("   {}...\n", preview);
            } else {
                println!("   {}\n", preview);
            }
        }
    }

    Ok(())
}
