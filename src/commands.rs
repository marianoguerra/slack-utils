use std::path::Path;

use crate::error::Result;
use crate::formatter::MarkdownExportOptions;
use crate::index::export_conversations_to_index;
use crate::markdown::export_conversations_to_markdown_with_options;
use crate::meilisearch::{import_index_to_meilisearch, query_meilisearch};
use crate::settings::Settings;
use crate::slack;
use crate::{
    cli_callbacks, cli_progress, current_iso_week, default_from_date, default_to_date,
    load_token, parse_date, week_to_date_range, OutputFormat,
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

    let count = slack::export_conversations(
        &token,
        from_date,
        to_date,
        Path::new(&output_path),
        None,
        cli_callbacks(),
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

    let count = slack::export_conversations(
        &token,
        from_date,
        to_date,
        Path::new(&output_path),
        None,
        cli_callbacks(),
        format,
    )
    .await?;

    println!(
        "Export completed successfully! {} messages exported.",
        count
    );
    Ok(())
}

pub async fn run_archive_range(
    from_year: i32,
    from_week: u32,
    to_year: Option<i32>,
    to_week: Option<u32>,
    output: &str,
) -> Result<()> {
    let token = load_token()?;

    // Default to current ISO week if from_year/from_week are 0
    let (default_year, default_week) = current_iso_week();
    let from_year = if from_year == 0 { default_year } else { from_year };
    let from_week = if from_week == 0 { default_week } else { from_week };

    // Default to_year/to_week to from values if not specified
    let to_year = to_year.unwrap_or(from_year);
    let to_week = to_week.unwrap_or(from_week);

    println!(
        "Archiving conversations from {}-W{:02} to {}-W{:02} to {}...",
        from_year, from_week, to_year, to_week, output
    );

    let result = slack::archive_range(
        &token,
        from_year,
        from_week,
        to_year,
        to_week,
        Path::new(output),
        cli_callbacks(),
    )
    .await?;

    println!(
        "Archive completed! {} messages in {} weeks ({} skipped).",
        result.total_messages, result.weeks_processed, result.weeks_skipped
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
        Some(&cli_progress),
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
    formatter_script: Option<&str>,
    backslash_line_breaks: bool,
) -> Result<()> {
    println!("Exporting selected conversations to markdown...");

    // Load settings and merge with CLI args (CLI takes precedence)
    let settings = Settings::load().unwrap_or_default();

    let effective_script = match formatter_script {
        Some(script) => Some(script.to_string()),
        None => settings.markdown_export.formatter_script,
    };

    // CLI flag takes precedence over settings
    let effective_backslash_line_breaks =
        backslash_line_breaks || settings.markdown_export.backslash_line_breaks;

    if let Some(script) = &effective_script {
        println!("  Using formatter script: {}", script);
    }
    if effective_backslash_line_breaks {
        println!("  Using backslash line breaks");
    }

    let options = MarkdownExportOptions::new()
        .with_formatter_script(effective_script)
        .with_backslash_line_breaks(effective_backslash_line_breaks);

    let (count, stats) = export_conversations_to_markdown_with_options(
        conversations,
        users,
        channels,
        output,
        None,
        &options,
    )?;

    println!(
        "Export completed successfully! {} messages exported to {}",
        count, output
    );

    if stats.total_calls() > 0 {
        println!("  {}", stats);
    }

    Ok(())
}

pub async fn run_export_emojis(output: &str, folder: &str) -> Result<()> {
    let token = load_token()?;

    println!("Exporting custom emojis to {} (images to {})...", output, folder);

    let result = slack::fetch_emojis(
        &token,
        Path::new(output),
        Path::new(folder),
        Some(&cli_progress),
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

    let result = import_index_to_meilisearch(
        input,
        url,
        api_key,
        index_name,
        clear,
        Some(&cli_progress),
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

pub fn run_md_to_html(
    input: &str,
    output: Option<&str>,
    options: &crate::md_to_html::MdToHtmlOptions,
) -> Result<()> {
    println!("Converting {} to HTML...", input);

    let output_path = crate::md_to_html::convert_md_file_to_html(input, output, options)?;

    println!("Successfully converted to {}", output_path);
    Ok(())
}
