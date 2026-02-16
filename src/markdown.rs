use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Duration;

use crate::slack_render::{render_blocks_as_markdown_with_options, MarkdownRenderOptions, SlackReferences};
use slack_morphism::prelude::{SlackBlock, SlackChannelId, SlackUserId};
use webpage::{Webpage, WebpageOptions};

use crate::error::{AppError, Result};
use crate::formatter::{format_attachment, format_file, format_permalink, format_prefix, format_suffix, FormatterStats, MarkdownExportOptions};
use crate::ProgressCallback;

/// Maximum bytes to fetch when resolving link titles (32KB should be enough for <title>)
const MAX_FETCH_BYTES: usize = 32 * 1024;

/// Truncate a URL for display
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len - 3])
    }
}

/// Create WebpageOptions with size and time limits
fn limited_webpage_options() -> WebpageOptions {
    let mut options = WebpageOptions::default();
    options.timeout = Duration::from_secs(5);
    // Request only the first N bytes - enough to get the <title> tag
    options.headers = vec![format!("Range: bytes=0-{}", MAX_FETCH_BYTES - 1)];
    options
}

/// A link with optional rich metadata from Slack attachment unfurls
struct RichLink {
    url: String,
    title: String,
    description: Option<String>,
    author: Option<String>,
    author_link: Option<String>,
    service: Option<String>,
    image_url: Option<String>,
    fields: Vec<(String, String)>,
    footer: Option<String>,
}

impl RichLink {
    fn new(url: String, title: String, attachment: Option<&serde_json::Value>) -> Self {
        let mut link = Self {
            url,
            title,
            description: None,
            author: None,
            author_link: None,
            service: None,
            image_url: None,
            fields: Vec::new(),
            footer: None,
        };

        if let Some(att) = attachment {
            // Extract description
            link.description = att
                .get("text")
                .or_else(|| att.get("description"))
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            // Extract author
            link.author = att
                .get("author_name")
                .and_then(|a| a.as_str())
                .map(|s| s.to_string());
            link.author_link = att
                .get("author_link")
                .and_then(|l| l.as_str())
                .map(|s| s.to_string());

            // Extract service name (if different from author)
            let service = att.get("service_name").and_then(|s| s.as_str());
            let author = link.author.as_deref().unwrap_or("");
            if let Some(s) = service
                && s != author
            {
                link.service = Some(s.to_string());
            }

            // Extract image
            link.image_url = att
                .get("image_url")
                .or_else(|| att.get("thumb_url"))
                .or_else(|| att.get("video_thumbnail_url"))
                .or_else(|| att.get("thumb_720"))
                .or_else(|| att.get("thumb_480"))
                .or_else(|| att.get("thumb_360"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string());

            // Extract fields
            if let Some(fields_arr) = att.get("fields").and_then(|f| f.as_array()) {
                for field in fields_arr {
                    if let (Some(title), Some(value)) = (
                        field.get("title").and_then(|t| t.as_str()),
                        field.get("value").and_then(|v| v.as_str()),
                    ) {
                        link.fields.push((title.to_string(), value.to_string()));
                    }
                }
            }

            // Extract footer
            link.footer = att
                .get("footer")
                .and_then(|f| f.as_str())
                .map(|s| s.to_string());
        }

        link
    }

    fn render(&self) -> String {
        let mut output = format!("- [{}]({})\n", self.title, self.url);

        // Add author/service info
        if let Some(author) = &self.author {
            let author_text = match &self.author_link {
                Some(link) => format!("  *By [{}]({})*\n", author, link),
                None => format!("  *By {}*\n", author),
            };
            output.push_str(&author_text);
        }

        if let Some(service) = &self.service {
            output.push_str(&format!("  *From {}*\n", service));
        }

        // Add description as indented text
        if let Some(desc) = &self.description {
            let indented: String = desc
                .lines()
                .map(|line| format!("  > {}", line))
                .collect::<Vec<_>>()
                .join("\n");
            output.push_str(&indented);
            output.push('\n');
        }

        // Add fields
        for (title, value) in &self.fields {
            output.push_str(&format!("  **{}:** {}\n", title, value));
        }

        // Add image preview
        if let Some(img) = &self.image_url {
            output.push_str(&format!("  ![{}]({})\n", self.title, img));
        }

        // Add footer
        if let Some(footer) = &self.footer {
            output.push_str(&format!("  _{}_\n", footer));
        }

        output
    }
}

/// Resolve a better title for a URL if the current title equals the URL
fn resolve_title_if_needed(title: &str, url: &str) -> String {
    // Check if title is the same as URL (or very similar)
    let needs_resolution =
        title == url || title.is_empty() || url.contains(title) || title.contains("http");

    if needs_resolution
        && let Ok(page) = Webpage::from_url(url, limited_webpage_options())
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
    let (count, _stats) = export_conversations_to_markdown_with_options(
        conversations_path,
        users_path,
        channels_path,
        output_path,
        None,
        &MarkdownExportOptions::default(),
    )?;
    Ok(count)
}

/// Export selected conversations to markdown format with progress reporting
pub fn export_conversations_to_markdown_with_progress(
    conversations_path: &str,
    users_path: &str,
    channels_path: &str,
    output_path: &str,
    progress_callback: ProgressCallback,
) -> Result<usize> {
    let (count, _stats) = export_conversations_to_markdown_with_options(
        conversations_path,
        users_path,
        channels_path,
        output_path,
        progress_callback,
        &MarkdownExportOptions::default(),
    )?;
    Ok(count)
}

/// Export selected conversations to markdown format with options and formatter support
pub fn export_conversations_to_markdown_with_options(
    conversations_path: &str,
    users_path: &str,
    channels_path: &str,
    output_path: &str,
    progress_callback: ProgressCallback,
    options: &MarkdownExportOptions,
) -> Result<(usize, FormatterStats)> {
    let mut formatter_stats = FormatterStats::new();
    let report_progress = |current: usize, total: usize, msg: &str| {
        if let Some(cb) = progress_callback {
            cb(current, total, msg);
        }
    };

    // Create render options from export options
    let render_options = MarkdownRenderOptions {
        backslash_line_breaks: options.backslash_line_breaks,
    };

    report_progress(1, 4, "Loading users...");

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
                .or_else(|| {
                    user.get("profile")
                        .and_then(|p| p.get("real_name"))
                        .and_then(|n| n.as_str())
                        .filter(|s| !s.is_empty())
                })
                .or_else(|| user.get("name").and_then(|n| n.as_str()))
                .unwrap_or(&id)
                .to_string();
            Some((id, name))
        })
        .collect();

    report_progress(2, 4, "Loading channels...");

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

    report_progress(3, 4, "Loading conversations...");

    // Load selected-conversations.json
    let conversations: Vec<serde_json::Value> = crate::load_json_file(conversations_path)?;

    // Count total messages for progress reporting
    let total_messages: usize = conversations
        .iter()
        .filter_map(|ch| ch.get("messages").and_then(|m| m.as_array()))
        .map(|msgs| msgs.len())
        .sum();

    report_progress(4, 4, "Starting export...");

    // Open output file
    let output_file = File::create(output_path).map_err(|e| AppError::WriteFile {
        path: output_path.to_string(),
        source: e,
    })?;
    let mut writer = BufWriter::new(output_file);

    // Call formatter for prefix content if script is configured
    if let Some(script_path) = &options.formatter_script
        && let Some(prefix_content) = format_prefix(script_path, &conversations, &mut formatter_stats)
    {
        write!(writer, "{}", prefix_content).map_err(|e| AppError::WriteFile {
            path: output_path.to_string(),
            source: e,
        })?;
    }

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

        // Get channel name for progress and formatter
        let channel_name = channel_names
            .get(channel_id)
            .map(|s| s.as_str())
            .unwrap_or(channel_id);

        let messages_len = messages.len();
        for (msg_idx, message) in messages.iter().enumerate() {
            // Report progress for this message
            report_progress(message_count + 1, total_messages, channel_name);

            // Check if this is a new channel - write heading
            if current_channel_id.as_deref() != Some(channel_id) {
                current_channel_id = Some(channel_id.to_string());

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

            // Get message timestamp for formatter
            let message_ts = message
                .get("ts")
                .and_then(|ts| ts.as_str())
                .unwrap_or("");

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

            // Call formatter for permalink if script is configured
            if let Some(script_path) = &options.formatter_script
                && let Some(permalink_response) = format_permalink(
                    script_path,
                    channel_id,
                    channel_name,
                    message_ts,
                    message,
                    &mut formatter_stats,
                )
            {
                writeln!(
                    writer,
                    "[{}]({})\n",
                    permalink_response.label, permalink_response.url
                )
                .map_err(|e| AppError::WriteFile {
                    path: output_path.to_string(),
                    source: e,
                })?;
            }

            // Render the message content using slack-blocks-render
            let markdown = render_message_to_markdown(message, &slack_references, &render_options);
            if !markdown.is_empty() {
                writeln!(writer, "{}", markdown).map_err(|e| AppError::WriteFile {
                    path: output_path.to_string(),
                    source: e,
                })?;
            }

            // Build attachment lookup by URL for merging with links
            let attachments = message
                .get("attachments")
                .and_then(|a| a.as_array())
                .map(|arr| arr.as_slice())
                .unwrap_or(&[]);

            let attachment_by_url: HashMap<&str, &serde_json::Value> = attachments
                .iter()
                .filter_map(|att| {
                    let url = att
                        .get("original_url")
                        .or_else(|| att.get("from_url"))
                        .or_else(|| att.get("title_link"))
                        .and_then(|u| u.as_str())?;
                    Some((url, att))
                })
                .collect();

            // Collect files
            let mut files: Vec<(String, String)> = Vec::new();
            if let Some(file_arr) = message.get("files").and_then(|f| f.as_array()) {
                for file in file_arr {
                    // Try to format file with external script first
                    let (final_title, final_url) = if let Some(script_path) = &options.formatter_script
                        && let Some(response) = format_file(
                            script_path,
                            channel_id,
                            channel_name,
                            file,
                            &mut formatter_stats,
                        )
                    {
                        (response.label, response.url)
                    } else {
                        let title = file
                            .get("title")
                            .or_else(|| file.get("name"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("Untitled file");
                        let url = file
                            .get("url_private")
                            .or_else(|| file.get("permalink"))
                            .and_then(|u| u.as_str())
                            .unwrap_or("");
                        (title.to_string(), url.to_string())
                    };
                    if !final_url.is_empty() {
                        files.push((final_title, final_url));
                    }
                }
            }

            // Collect links with their unfurl data merged
            let mut rich_links: Vec<RichLink> = Vec::new();
            let mut used_attachment_urls: std::collections::HashSet<&str> =
                std::collections::HashSet::new();

            if let Some(links) = message.get("selected_links").and_then(|l| l.as_array()) {
                for link in links {
                    let link_title = link
                        .get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Untitled link");
                    let url = match link.get("url").and_then(|u| u.as_str()) {
                        Some(u) => u,
                        None => continue,
                    };

                    // Look for matching attachment
                    let attachment = attachment_by_url.get(url).copied();
                    if attachment.is_some() {
                        used_attachment_urls.insert(url);
                    }

                    // Get title from attachment if available, otherwise resolve
                    let title = attachment
                        .and_then(|att| att.get("title").and_then(|t| t.as_str()))
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| {
                            let needs_resolution = link_title == url
                                || link_title.is_empty()
                                || url.contains(link_title)
                                || link_title.contains("http");
                            if needs_resolution {
                                report_progress(
                                    message_count + 1,
                                    total_messages,
                                    &format!("Resolving: {}", truncate_url(url, 40)),
                                );
                            }
                            resolve_title_if_needed(link_title, url)
                        });

                    rich_links.push(RichLink::new(url.to_string(), title, attachment));
                }
            }

            // Add any attachments that weren't matched to selected_links
            for att in attachments {
                let url = att
                    .get("original_url")
                    .or_else(|| att.get("from_url"))
                    .or_else(|| att.get("title_link"))
                    .and_then(|u| u.as_str());

                if let Some(url) = url
                    && !used_attachment_urls.contains(url)
                {
                    // Try to format attachment with external script first
                    let (final_title, final_url) = if let Some(script_path) = &options.formatter_script
                        && let Some(response) = format_attachment(
                            script_path,
                            channel_id,
                            channel_name,
                            att,
                            &mut formatter_stats,
                        )
                    {
                        (response.label, response.url)
                    } else {
                        let title = att
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("Untitled")
                            .to_string();
                        (title, url.to_string())
                    };
                    rich_links.push(RichLink::new(final_url, final_title, Some(att)));
                }
            }

            // Write resources section if there are any files or links
            if !files.is_empty() || !rich_links.is_empty() {
                writeln!(writer, "\n📑 Resources\n").map_err(|e| AppError::WriteFile {
                    path: output_path.to_string(),
                    source: e,
                })?;

                // Write files
                for (title, url) in &files {
                    writeln!(writer, "- [{}]({})", title, url).map_err(|e| AppError::WriteFile {
                        path: output_path.to_string(),
                        source: e,
                    })?;
                }

                // Write rich links with metadata
                for link in &rich_links {
                    write!(writer, "{}", link.render()).map_err(|e| AppError::WriteFile {
                        path: output_path.to_string(),
                        source: e,
                    })?;
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

    // Call formatter for suffix content if script is configured
    if let Some(script_path) = &options.formatter_script
        && let Some(suffix_content) = format_suffix(script_path, &conversations, &mut formatter_stats)
    {
        write!(writer, "{}", suffix_content).map_err(|e| AppError::WriteFile {
            path: output_path.to_string(),
            source: e,
        })?;
    }

    writer.flush().map_err(|e| AppError::WriteFile {
        path: output_path.to_string(),
        source: e,
    })?;

    Ok((message_count, formatter_stats))
}

/// Render a single message to markdown using slack-blocks-render
fn render_message_to_markdown(
    message: &serde_json::Value,
    slack_references: &SlackReferences,
    render_options: &MarkdownRenderOptions,
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
            let rendered = render_blocks_as_markdown_with_options(
                blocks,
                slack_references.clone(),
                Some("**".to_string()),
                render_options,
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
