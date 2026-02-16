use std::io::Write;
use std::process::{Command, Stdio};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

/// Request headers for the formatter script
#[derive(Debug, Clone, Serialize)]
pub struct FormatterHeaders {
    pub action: String,
    pub channel_id: String,
    pub channel_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_ts: Option<String>,
}

/// Request sent to the formatter script via stdin
#[derive(Debug, Clone, Serialize)]
pub struct FormatterRequest {
    pub headers: FormatterHeaders,
    pub body: serde_json::Value,
}

/// Response expected from the formatter script via stdout
#[derive(Debug, Clone, Deserialize)]
pub struct FormatterResponse {
    pub label: String,
    pub url: String,
}

/// Response for prefix/suffix actions - returns content to insert
#[derive(Debug, Clone, Deserialize)]
pub struct ContentResponse {
    pub content: String,
}

/// Output from calling the formatter script, including stderr
#[derive(Debug, Clone)]
pub struct FormatterOutput {
    pub response: FormatterResponse,
    pub stderr: Option<String>,
}

/// Identifies which formatter action a stat counter belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatKind {
    Permalink = 0,
    Attachment = 1,
    File = 2,
    Prefix = 3,
    Suffix = 4,
}

impl StatKind {
    const COUNT: usize = 5;

    fn index(self) -> usize {
        self as usize
    }
}

/// Counters for a single formatter action (calls, successes, failures)
#[derive(Debug, Clone, Copy, Default)]
pub struct ActionCounters {
    pub calls: usize,
    pub successes: usize,
    pub failures: usize,
}

/// Statistics for formatter script calls
#[derive(Debug, Clone, Default)]
pub struct FormatterStats {
    counters: [ActionCounters; StatKind::COUNT],
    pub stderr_count: usize,
    pub stderr_outputs: Vec<String>,
}

impl FormatterStats {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::indexing_slicing)] // StatKind variants are 0..4, array is size 5
    pub fn get(&self, kind: StatKind) -> &ActionCounters {
        &self.counters[kind.index()]
    }

    #[allow(clippy::indexing_slicing)] // StatKind variants are 0..4, array is size 5
    pub fn get_mut(&mut self, kind: StatKind) -> &mut ActionCounters {
        &mut self.counters[kind.index()]
    }

    pub fn total_calls(&self) -> usize {
        self.counters.iter().map(|c| c.calls).sum()
    }

    pub fn total_successes(&self) -> usize {
        self.counters.iter().map(|c| c.successes).sum()
    }

    pub fn total_failures(&self) -> usize {
        self.counters.iter().map(|c| c.failures).sum()
    }

    /// Add stderr output to the stats if non-empty
    pub fn add_stderr(&mut self, stderr: Option<String>) {
        if let Some(s) = stderr {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                self.stderr_count += 1;
                self.stderr_outputs.push(trimmed.to_string());
            }
        }
    }

    /// Get all stderr outputs joined as a single string
    pub fn stderr_combined(&self) -> String {
        self.stderr_outputs.join("\n")
    }

    /// Check if there are any stderr outputs
    pub fn has_stderr(&self) -> bool {
        !self.stderr_outputs.is_empty()
    }
}

impl std::fmt::Display for FormatterStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let p = self.get(StatKind::Permalink);
        let a = self.get(StatKind::Attachment);
        let fi = self.get(StatKind::File);
        write!(
            f,
            "Formatter: {} calls ({} success, {} failed, {} with stderr) [permalinks: {}/{}, attachments: {}/{}, files: {}/{}]",
            self.total_calls(),
            self.total_successes(),
            self.total_failures(),
            self.stderr_count,
            p.successes, p.calls,
            a.successes, a.calls,
            fi.successes, fi.calls,
        )
    }
}

/// Run the external formatter script and deserialize its stdout as type T.
/// Returns the deserialized response along with any stderr output.
fn run_formatter_script<T: DeserializeOwned>(
    script_path: &str,
    request: &FormatterRequest,
) -> Result<(T, Option<String>)> {
    let request_json =
        serde_json::to_string(request).map_err(|e| AppError::JsonSerialize(e.to_string()))?;

    let mut child = Command::new(script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::FormatterScript(format!("failed to spawn script: {}", e)))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(request_json.as_bytes())
            .map_err(|e| AppError::FormatterScript(format!("failed to write to stdin: {}", e)))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| AppError::FormatterScript(format!("failed to wait for script: {}", e)))?;

    // Capture stderr (even on success)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr_opt = if stderr.trim().is_empty() {
        None
    } else {
        Some(stderr.trim().to_string())
    };

    if !output.status.success() {
        return Err(AppError::FormatterScript(format!(
            "script exited with status {}: {}",
            output.status,
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: T = serde_json::from_str(&stdout)
        .map_err(|e| AppError::FormatterScript(format!("invalid JSON response: {}: {}", e, stdout)))?;

    Ok((response, stderr_opt))
}

/// Call the external formatter script with the given request.
/// Returns the response along with any stderr output.
#[cfg(test)]
pub fn call_formatter(script_path: &str, request: &FormatterRequest) -> Result<FormatterOutput> {
    let (response, stderr) = run_formatter_script::<FormatterResponse>(script_path, request)?;
    Ok(FormatterOutput { response, stderr })
}

/// Format an item (permalink, attachment, or file) using the external formatter script.
/// Tracks call/success/failure counters on the given StatKind.
#[allow(clippy::too_many_arguments)]
fn format_item(
    script_path: &str,
    action: &str,
    channel_id: &str,
    channel_name: &str,
    message_ts: Option<&str>,
    body: &serde_json::Value,
    stats: &mut FormatterStats,
    kind: StatKind,
) -> Option<FormatterResponse> {
    stats.get_mut(kind).calls += 1;

    let request = FormatterRequest {
        headers: FormatterHeaders {
            action: action.to_string(),
            channel_id: channel_id.to_string(),
            channel_name: channel_name.to_string(),
            message_ts: message_ts.map(|ts| ts.to_string()),
        },
        body: body.clone(),
    };

    match run_formatter_script::<FormatterResponse>(script_path, &request) {
        Ok((response, stderr)) => {
            stats.get_mut(kind).successes += 1;
            stats.add_stderr(stderr);
            Some(response)
        }
        Err(_) => {
            stats.get_mut(kind).failures += 1;
            None
        }
    }
}

/// Format a permalink for a message using the external formatter script
pub fn format_permalink(
    script_path: &str,
    channel_id: &str,
    channel_name: &str,
    message_ts: &str,
    message: &serde_json::Value,
    stats: &mut FormatterStats,
) -> Option<FormatterResponse> {
    format_item(
        script_path, "format-permalink", channel_id, channel_name,
        Some(message_ts), message, stats, StatKind::Permalink,
    )
}

/// Format an attachment using the external formatter script
pub fn format_attachment(
    script_path: &str,
    channel_id: &str,
    channel_name: &str,
    attachment: &serde_json::Value,
    stats: &mut FormatterStats,
) -> Option<FormatterResponse> {
    format_item(
        script_path, "format-attachment", channel_id, channel_name,
        None, attachment, stats, StatKind::Attachment,
    )
}

/// Format a file using the external formatter script
pub fn format_file(
    script_path: &str,
    channel_id: &str,
    channel_name: &str,
    file: &serde_json::Value,
    stats: &mut FormatterStats,
) -> Option<FormatterResponse> {
    format_item(
        script_path, "format-file", channel_id, channel_name,
        None, file, stats, StatKind::File,
    )
}

/// Format content (prefix or suffix) using the external formatter script.
/// Returns the content string if non-empty, None otherwise.
fn format_content(
    script_path: &str,
    action: &str,
    threads: &[serde_json::Value],
    stats: &mut FormatterStats,
    kind: StatKind,
) -> Option<String> {
    stats.get_mut(kind).calls += 1;

    let request = FormatterRequest {
        headers: FormatterHeaders {
            action: action.to_string(),
            channel_id: String::new(),
            channel_name: String::new(),
            message_ts: None,
        },
        body: serde_json::json!({ "threads": threads }),
    };

    match run_formatter_script::<ContentResponse>(script_path, &request) {
        Ok((response, stderr)) => {
            stats.get_mut(kind).successes += 1;
            stats.add_stderr(stderr);
            if response.content.is_empty() {
                None
            } else {
                Some(response.content)
            }
        }
        Err(_) => {
            stats.get_mut(kind).failures += 1;
            None
        }
    }
}

/// Get prefix content to insert before the markdown using the external formatter script.
/// The threads parameter contains all conversation threads to be exported.
pub fn format_prefix(
    script_path: &str,
    threads: &[serde_json::Value],
    stats: &mut FormatterStats,
) -> Option<String> {
    format_content(script_path, "prefix", threads, stats, StatKind::Prefix)
}

/// Get suffix content to insert after the markdown using the external formatter script.
/// The threads parameter contains all conversation threads that were exported.
pub fn format_suffix(
    script_path: &str,
    threads: &[serde_json::Value],
    stats: &mut FormatterStats,
) -> Option<String> {
    format_content(script_path, "suffix", threads, stats, StatKind::Suffix)
}

/// Options for markdown export with optional formatter script
#[derive(Debug, Clone, Default)]
pub struct MarkdownExportOptions {
    pub formatter_script: Option<String>,
    /// When true, newlines in rich text are converted to `\` + newline for hard line breaks.
    /// Default is false (no backslashes).
    pub backslash_line_breaks: bool,
}

impl MarkdownExportOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_formatter_script(mut self, script: Option<String>) -> Self {
        self.formatter_script = script;
        self
    }

    pub fn with_backslash_line_breaks(mut self, enabled: bool) -> Self {
        self.backslash_line_breaks = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_stats_default() {
        let stats = FormatterStats::default();
        for kind in [StatKind::Permalink, StatKind::Attachment, StatKind::File, StatKind::Prefix, StatKind::Suffix] {
            let c = stats.get(kind);
            assert_eq!(c.calls, 0);
            assert_eq!(c.successes, 0);
            assert_eq!(c.failures, 0);
        }
    }

    #[test]
    fn test_formatter_stats_totals() {
        let mut stats = FormatterStats::new();
        *stats.get_mut(StatKind::Permalink) = ActionCounters { calls: 5, successes: 4, failures: 1 };
        *stats.get_mut(StatKind::Attachment) = ActionCounters { calls: 3, successes: 2, failures: 1 };
        *stats.get_mut(StatKind::File) = ActionCounters { calls: 2, successes: 2, failures: 0 };
        assert_eq!(stats.total_calls(), 10);
        assert_eq!(stats.total_successes(), 8);
        assert_eq!(stats.total_failures(), 2);
    }

    #[test]
    fn test_formatter_stats_display() {
        let mut stats = FormatterStats::new();
        *stats.get_mut(StatKind::Permalink) = ActionCounters { calls: 5, successes: 4, failures: 1 };
        *stats.get_mut(StatKind::Attachment) = ActionCounters { calls: 3, successes: 2, failures: 1 };
        *stats.get_mut(StatKind::File) = ActionCounters { calls: 2, successes: 2, failures: 0 };
        stats.stderr_count = 2;
        let display = format!("{}", stats);
        assert!(display.contains("10 calls"));
        assert!(display.contains("8 success"));
        assert!(display.contains("2 failed"));
        assert!(display.contains("2 with stderr"));
        assert!(display.contains("files: 2/2"));
    }

    #[test]
    fn test_formatter_request_serialization() {
        let request = FormatterRequest {
            headers: FormatterHeaders {
                action: "format-permalink".to_string(),
                channel_id: "C123".to_string(),
                channel_name: "general".to_string(),
                message_ts: Some("1234567890.123456".to_string()),
            },
            body: serde_json::json!({"text": "hello"}),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("format-permalink"));
        assert!(json.contains("C123"));
        assert!(json.contains("general"));
        assert!(json.contains("1234567890.123456"));
    }

    #[test]
    fn test_formatter_request_serialization_no_message_ts() {
        let request = FormatterRequest {
            headers: FormatterHeaders {
                action: "format-attachment".to_string(),
                channel_id: "C123".to_string(),
                channel_name: "general".to_string(),
                message_ts: None,
            },
            body: serde_json::json!({}),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("message_ts"));
    }

    #[test]
    fn test_formatter_response_deserialization() {
        let json = r#"{"label": "View in Slack", "url": "https://slack.com/archives/C123"}"#;
        let response: FormatterResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.label, "View in Slack");
        assert_eq!(response.url, "https://slack.com/archives/C123");
    }

    #[test]
    fn test_markdown_export_options_default() {
        let options = MarkdownExportOptions::default();
        assert!(options.formatter_script.is_none());
    }

    #[test]
    fn test_markdown_export_options_with_script() {
        let options = MarkdownExportOptions::new()
            .with_formatter_script(Some("./format.py".to_string()));
        assert_eq!(options.formatter_script, Some("./format.py".to_string()));
    }

    // Integration tests for the default formatter script
    // These tests require the scripts/format-links.py script to be present

    fn get_script_path() -> Option<String> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
        let script_path = std::path::Path::new(&manifest_dir).join("scripts/format-links.py");
        if script_path.exists() {
            Some(script_path.to_string_lossy().to_string())
        } else {
            None
        }
    }

    #[test]
    fn test_integration_format_permalink() {
        let script_path = match get_script_path() {
            Some(p) => p,
            None => {
                eprintln!("Skipping integration test: scripts/format-links.py not found");
                return;
            }
        };

        let request = FormatterRequest {
            headers: FormatterHeaders {
                action: "format-permalink".to_string(),
                channel_id: "C123".to_string(),
                channel_name: "test".to_string(),
                message_ts: Some("1234567890.123456".to_string()),
            },
            body: serde_json::json!({}),
        };

        let output = call_formatter(&script_path, &request).expect("call_formatter should succeed");

        assert_eq!(output.response.label, "Conversation permalink");
        assert_eq!(output.response.url, "https://app.slack.com/archives/C123/p1234567890123456");
    }

    #[test]
    fn test_integration_format_attachment() {
        let script_path = match get_script_path() {
            Some(p) => p,
            None => {
                eprintln!("Skipping integration test: scripts/format-links.py not found");
                return;
            }
        };

        let request = FormatterRequest {
            headers: FormatterHeaders {
                action: "format-attachment".to_string(),
                channel_id: "C123".to_string(),
                channel_name: "test".to_string(),
                message_ts: None,
            },
            body: serde_json::json!({
                "title": "My Article",
                "original_url": "https://example.com/article"
            }),
        };

        let output = call_formatter(&script_path, &request).expect("call_formatter should succeed");

        assert_eq!(output.response.label, "My Article");
        assert_eq!(output.response.url, "https://example.com/article");
    }

    #[test]
    fn test_integration_format_attachment_fallback_fields() {
        let script_path = match get_script_path() {
            Some(p) => p,
            None => {
                eprintln!("Skipping integration test: scripts/format-links.py not found");
                return;
            }
        };

        // Test with from_url instead of original_url
        let request = FormatterRequest {
            headers: FormatterHeaders {
                action: "format-attachment".to_string(),
                channel_id: "C456".to_string(),
                channel_name: "general".to_string(),
                message_ts: None,
            },
            body: serde_json::json!({
                "name": "Document Name",
                "from_url": "https://example.com/doc"
            }),
        };

        let output = call_formatter(&script_path, &request).expect("call_formatter should succeed");

        assert_eq!(output.response.label, "Document Name");
        assert_eq!(output.response.url, "https://example.com/doc");
    }

    #[test]
    fn test_integration_format_permalink_with_stats() {
        let script_path = match get_script_path() {
            Some(p) => p,
            None => {
                eprintln!("Skipping integration test: scripts/format-links.py not found");
                return;
            }
        };

        let mut stats = FormatterStats::new();
        let message = serde_json::json!({"text": "hello world"});

        let response = format_permalink(
            &script_path,
            "C789",
            "random",
            "9876543210.654321",
            &message,
            &mut stats,
        );

        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.label, "Conversation permalink");
        assert!(response.url.contains("C789"));
        assert!(response.url.contains("9876543210654321"));

        let p = stats.get(StatKind::Permalink);
        assert_eq!(p.calls, 1);
        assert_eq!(p.successes, 1);
        assert_eq!(p.failures, 0);
    }

    #[test]
    fn test_integration_format_attachment_with_stats() {
        let script_path = match get_script_path() {
            Some(p) => p,
            None => {
                eprintln!("Skipping integration test: scripts/format-links.py not found");
                return;
            }
        };

        let mut stats = FormatterStats::new();
        let attachment = serde_json::json!({
            "title": "Test Link",
            "original_url": "https://test.com/page"
        });

        let response = format_attachment(
            &script_path,
            "C111",
            "dev",
            &attachment,
            &mut stats,
        );

        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.label, "Test Link");
        assert_eq!(response.url, "https://test.com/page");

        let a = stats.get(StatKind::Attachment);
        assert_eq!(a.calls, 1);
        assert_eq!(a.successes, 1);
        assert_eq!(a.failures, 0);
    }

    #[test]
    fn test_integration_invalid_script_path() {
        let mut stats = FormatterStats::new();
        let message = serde_json::json!({});

        let response = format_permalink(
            "/nonexistent/path/to/script.py",
            "C123",
            "test",
            "1234567890.123456",
            &message,
            &mut stats,
        );

        assert!(response.is_none());
        let p = stats.get(StatKind::Permalink);
        assert_eq!(p.calls, 1);
        assert_eq!(p.successes, 0);
        assert_eq!(p.failures, 1);
    }
}
