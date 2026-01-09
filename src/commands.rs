use std::path::Path;

use crate::error::Result;
use crate::markdown::export_conversations_to_markdown;
use crate::slack;
use crate::{default_from_date, default_to_date, load_token, parse_date};

pub async fn run_export_conversations(
    from: Option<String>,
    to: Option<String>,
    output: &str,
) -> Result<()> {
    let token = load_token()?;

    let from_date = match from {
        Some(s) => parse_date(&s)?,
        None => default_from_date(),
    };
    let to_date = match to {
        Some(s) => parse_date(&s)?,
        None => default_to_date(),
    };

    println!(
        "Exporting conversations from {} to {} to {}...",
        from_date, to_date, output
    );

    let count =
        slack::export_conversations(&token, from_date, to_date, Path::new(output), None).await?;

    println!(
        "Export completed successfully! {} messages exported.",
        count
    );
    Ok(())
}

pub async fn run_export_users(output: &str) -> Result<()> {
    let token = load_token()?;

    println!("Exporting users to {}...", output);

    let count = slack::export_users(&token, Path::new(output)).await?;

    println!("Export completed successfully! {} users exported.", count);
    Ok(())
}

pub async fn run_export_channels(output: &str) -> Result<()> {
    let token = load_token()?;

    println!("Exporting channels to {}...", output);

    let count = slack::export_channels(&token, Path::new(output)).await?;

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
