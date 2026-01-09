use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use slack_blocks_render::{render_blocks_as_markdown, SlackReferences};
use slack_morphism::prelude::{SlackBlock, SlackChannelId, SlackUserId};
use webpage::{Webpage, WebpageOptions};

use crate::error::{AppError, Result};
use crate::ProgressCallback;

/// Truncate a URL for display
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len - 3])
    }
}

/// Resolve a better title for a URL if the current title equals the URL
fn resolve_title_if_needed(title: &str, url: &str) -> String {
    // Check if title is the same as URL (or very similar)
    let needs_resolution =
        title == url || title.is_empty() || url.contains(title) || title.contains("http");

    if needs_resolution
        && let Ok(page) = Webpage::from_url(url, WebpageOptions::default())
        && let Some(page_title) = page.html.title
        && !page_title.is_empty()
    {
        return page_title;
    }
    title.to_string()
}

/// Export selected conversations to markdown format
pub fn export_conversations_to_markdown(
    conversations_path: &str,
    users_path: &str,
    channels_path: &str,
    output_path: &str,
) -> Result<usize> {
    export_conversations_to_markdown_with_progress(
        conversations_path,
        users_path,
        channels_path,
        output_path,
        None,
    )
}

/// Export selected conversations to markdown format with progress reporting
pub fn export_conversations_to_markdown_with_progress(
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

    // Load selected-conversations.json
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

    report_progress(0, total_messages, "Exporting messages...");

    // Open output file
    let output_file = File::create(output_path).map_err(|e| AppError::WriteFile {
        path: output_path.to_string(),
        source: e,
    })?;
    let mut writer = BufWriter::new(output_file);

    let mut message_count = 0;
    let mut current_channel_id: Option<String> = None;

    // Process each channel entry in the conversations file
    for channel_entry in &conversations {
        let channel_id = channel_entry
            .get("channel_id")
            .and_then(|id| id.as_str())
            .unwrap_or("");

        let messages = channel_entry
            .get("messages")
            .and_then(|m| m.as_array())
            .map(|a| a.as_slice())
            .unwrap_or(&[]);

        let messages_len = messages.len();
        for (msg_idx, message) in messages.iter().enumerate() {
            // Check if this is a new channel
            if current_channel_id.as_deref() != Some(channel_id) {
                current_channel_id = Some(channel_id.to_string());

                // Add channel heading
                let channel_name = channel_names
                    .get(channel_id)
                    .map(|s| s.as_str())
                    .unwrap_or(channel_id);

                if message_count > 0 {
                    writeln!(writer).map_err(|e| AppError::WriteFile {
                        path: output_path.to_string(),
                        source: e,
                    })?;
                }
                writeln!(writer, "# {}\n", channel_name).map_err(|e| AppError::WriteFile {
                    path: output_path.to_string(),
                    source: e,
                })?;
            }

            // Report progress for this message
            report_progress(message_count + 1, total_messages, "Processing messages...");

            // Get user name
            let user_id = message.get("user").and_then(|u| u.as_str()).unwrap_or("");
            let user_name = user_names
                .get(user_id)
                .map(|s| s.as_str())
                .unwrap_or(user_id);

            // Get main link if present, resolving title if it equals the URL
            let main_link = message.get("main_link").and_then(|ml| {
                let title = ml.get("title").and_then(|t| t.as_str())?;
                let url = ml.get("url").and_then(|u| u.as_str())?;
                // Check if we need to resolve title
                let needs_resolution = title == url
                    || title.is_empty()
                    || url.contains(title)
                    || title.contains("http");
                if needs_resolution {
                    report_progress(
                        message_count + 1,
                        total_messages,
                        &format!("Resolving: {}", truncate_url(url, 40)),
                    );
                }
                let resolved_title = resolve_title_if_needed(title, url);
                Some((resolved_title, url.to_string()))
            });

            // Write message header: **Username**: [Title](url) or just **Username**
            let header = match main_link {
                Some((title, url)) => format!("💬 **{}**: [{}]({})", user_name, title, url),
                None => format!("💬 **{}**", user_name),
            };
            writeln!(writer, "{}\n", header).map_err(|e| AppError::WriteFile {
                path: output_path.to_string(),
                source: e,
            })?;

            // Render the message content using slack-blocks-render
            let markdown = render_message_to_markdown(message, &slack_references);
            if !markdown.is_empty() {
                writeln!(writer, "{}", markdown).map_err(|e| AppError::WriteFile {
                    path: output_path.to_string(),
                    source: e,
                })?;
            }

            // Collect resources (files and links)
            let mut resources: Vec<(String, String)> = Vec::new();

            // Add files if present
            if let Some(files) = message.get("files").and_then(|f| f.as_array()) {
                for file in files {
                    let title = file
                        .get("title")
                        .or_else(|| file.get("name"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("Untitled file");
                    let url = file
                        .get("url_private")
                        .or_else(|| file.get("permalink"))
                        .and_then(|u| u.as_str());
                    if let Some(url) = url {
                        resources.push((title.to_string(), url.to_string()));
                    }
                }
            }

            // Add selected_links if present, resolving titles if they equal the URL
            if let Some(links) = message.get("selected_links").and_then(|l| l.as_array()) {
                for link in links {
                    let title = link
                        .get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Untitled link");
                    let url = link.get("url").and_then(|u| u.as_str());
                    if let Some(url) = url {
                        // Check if we need to resolve title and report progress
                        let needs_resolution = title == url
                            || title.is_empty()
                            || url.contains(title)
                            || title.contains("http");
                        if needs_resolution {
                            report_progress(
                                message_count + 1,
                                total_messages,
                                &format!("Resolving: {}", truncate_url(url, 40)),
                            );
                        }
                        let resolved_title = resolve_title_if_needed(title, url);
                        resources.push((resolved_title, url.to_string()));
                    }
                }
            }

            // Write resources section if there are any
            if !resources.is_empty() {
                writeln!(writer, "\n📑 Resources\n").map_err(|e| AppError::WriteFile {
                    path: output_path.to_string(),
                    source: e,
                })?;
                for (title, url) in resources {
                    writeln!(writer, "- [{}]({})", title, url).map_err(|e| AppError::WriteFile {
                        path: output_path.to_string(),
                        source: e,
                    })?;
                }
            }

            // Render link unfurls (attachments) if present
            if let Some(attachments) = message.get("attachments").and_then(|a| a.as_array()) {
                for attachment in attachments {
                    let unfurl = render_link_unfurl(attachment);
                    if !unfurl.is_empty() {
                        writeln!(writer, "\n{}", unfurl).map_err(|e| AppError::WriteFile {
                            path: output_path.to_string(),
                            source: e,
                        })?;
                    }
                }
            }

            writeln!(writer).map_err(|e| AppError::WriteFile {
                path: output_path.to_string(),
                source: e,
            })?;

            // Add separator between messages, but not after the last one in each channel
            if msg_idx < messages_len - 1 {
                writeln!(writer, "---\n").map_err(|e| AppError::WriteFile {
                    path: output_path.to_string(),
                    source: e,
                })?;
            }

            message_count += 1;
        }
    }

    writer.flush().map_err(|e| AppError::WriteFile {
        path: output_path.to_string(),
        source: e,
    })?;

    Ok(message_count)
}

/// Render a single message to markdown using slack-blocks-render
fn render_message_to_markdown(
    message: &serde_json::Value,
    slack_references: &SlackReferences,
) -> String {
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
                return rendered;
            }
        }
    }

    // Fall back to plain text field
    message
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string()
}

/// Render a link unfurl (attachment) to markdown
fn render_link_unfurl(attachment: &serde_json::Value) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Get title with optional link
    let title = attachment.get("title").and_then(|t| t.as_str());
    let title_link = attachment.get("title_link").and_then(|l| l.as_str());
    let original_url = attachment.get("original_url").and_then(|u| u.as_str());

    match (title, title_link.or(original_url)) {
        (Some(t), Some(link)) => parts.push(format!("> **[{}]({})**", t, link)),
        (Some(t), None) => parts.push(format!("> **{}**", t)),
        (None, Some(link)) => parts.push(format!("> {}", link)),
        (None, None) => {}
    }

    // Add author if present
    if let Some(author) = attachment.get("author_name").and_then(|a| a.as_str()) {
        let author_link = attachment.get("author_link").and_then(|l| l.as_str());
        let author_text = match author_link {
            Some(link) => format!("> *By [{}]({})*", author, link),
            None => format!("> *By {}*", author),
        };
        parts.push(author_text);
    }

    // Add service name if present and different from author
    if let Some(service) = attachment.get("service_name").and_then(|s| s.as_str()) {
        let author = attachment
            .get("author_name")
            .and_then(|a| a.as_str())
            .unwrap_or("");
        if service != author {
            parts.push(format!("> *From {}*", service));
        }
    }

    // Add description from text or description field
    let description = attachment
        .get("text")
        .or_else(|| attachment.get("description"))
        .and_then(|t| t.as_str())
        .filter(|s| !s.is_empty());

    if let Some(desc) = description {
        // Quote each line of the description
        let quoted: String = desc
            .lines()
            .map(|line| format!("> {}", line))
            .collect::<Vec<_>>()
            .join("\n");
        parts.push(quoted);
    }

    // Add preview image (check multiple possible fields for video thumbnails and images)
    let image_url = attachment
        .get("image_url")
        .or_else(|| attachment.get("thumb_url"))
        .or_else(|| attachment.get("video_thumbnail_url"))
        .or_else(|| attachment.get("thumb_720"))
        .or_else(|| attachment.get("thumb_480"))
        .or_else(|| attachment.get("thumb_360"))
        .and_then(|u| u.as_str());

    if let Some(img) = image_url {
        // Use the title as alt text if available, otherwise use "preview"
        let alt_text = title.unwrap_or("preview");
        parts.push(format!("> ![{}]({})", alt_text, img));
    }

    // Add footer if present
    if let Some(footer) = attachment.get("footer").and_then(|f| f.as_str()) {
        parts.push(format!("> _{}_", footer));
    }

    parts.join("\n")
}
