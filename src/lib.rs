use std::fs::File;
use std::io::BufReader;

use chrono::{Datelike, Local, NaiveDate};

mod cli;
mod commands;
mod error;
mod index;
mod markdown;
mod meilisearch;
mod parquet;
mod settings;
mod slack;

#[cfg(feature = "tui")]
mod app;
#[cfg(feature = "tui")]
mod input;
#[cfg(feature = "tui")]
mod ui;
#[cfg(feature = "tui")]
mod widgets;

#[cfg(feature = "duckdb")]
pub mod duckdb_query;

#[cfg(feature = "server")]
pub mod archive_server;

// Re-export meilisearch types for the server binary
#[cfg(feature = "server")]
pub use index::{IndexChannel, IndexEntry, IndexUser};
#[cfg(feature = "server")]
pub use meilisearch::query_meilisearch;

/// Output format for export commands
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Json,
    Parquet,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Parquet => write!(f, "parquet"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = AppError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "parquet" => Ok(OutputFormat::Parquet),
            _ => Err(AppError::InvalidFormat(s.to_string())),
        }
    }
}

// Re-export public API
pub use cli::{Cli, Commands};
pub use error::{AppError, Result};
pub use parquet::{write_channels_parquet, write_conversations_parquet, write_users_parquet};

/// Type alias for progress callback functions
pub type ProgressCallback<'a> = Option<&'a dyn Fn(usize, usize, &str)>;

/// Type alias for rate limit callback functions (wait_secs, attempt, max_attempts)
pub type RateLimitCallback<'a> = Option<&'a dyn Fn(u64, u32, u32)>;

/// Unified callbacks for Slack API operations
/// This struct provides a consistent way to handle progress and rate limit
/// notifications across both CLI and TUI contexts.
#[derive(Clone, Copy, Default)]
pub struct SlackApiCallbacks<'a> {
    /// Called to report progress (current, total, message)
    pub on_progress: ProgressCallback<'a>,
    /// Called when rate limited (wait_secs, attempt, max_attempts)
    pub on_rate_limit: RateLimitCallback<'a>,
}

impl<'a> SlackApiCallbacks<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_progress(mut self, callback: &'a dyn Fn(usize, usize, &str)) -> Self {
        self.on_progress = Some(callback);
        self
    }

    pub fn with_rate_limit(mut self, callback: &'a dyn Fn(u64, u32, u32)) -> Self {
        self.on_rate_limit = Some(callback);
        self
    }

    pub fn report_progress(&self, current: usize, total: usize, message: &str) {
        if let Some(cb) = self.on_progress {
            cb(current, total, message);
        }
    }

    pub fn report_rate_limit(&self, wait_secs: u64, attempt: u32, max_attempts: u32) {
        if let Some(cb) = self.on_rate_limit {
            cb(wait_secs, attempt, max_attempts);
        }
    }
}

// Re-export command functions for main.rs compatibility
pub use commands::run_archive_range as run_archive_range_async;
pub use commands::run_download_attachments;
pub use commands::run_export_channels as run_export_channels_async;
pub use commands::run_export_conversations as run_export_conversations_async;
pub use commands::run_export_conversations_week as run_export_conversations_week_async;
pub use commands::run_export_emojis as run_export_emojis_async;
pub use commands::run_export_index;
pub use commands::run_export_markdown;
pub use commands::run_export_users as run_export_users_async;
pub use commands::run_import_index_meilisearch as run_import_index_meilisearch_async;
pub use commands::run_query_meilisearch as run_query_meilisearch_async;

/// Constant for the channels file
pub const CHANNELS_FILE: &str = "channels.json";

/// Load the Slack token from environment
pub fn load_token() -> Result<String> {
    std::env::var("SLACK_TOKEN").map_err(|_| AppError::MissingToken)
}

/// Default from date (30 days ago)
pub fn default_from_date() -> NaiveDate {
    Local::now().date_naive() - chrono::Duration::days(30)
}

/// Default to date (today)
pub fn default_to_date() -> NaiveDate {
    Local::now().date_naive()
}

/// Parse a date string in YYYY-MM-DD format
pub fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| AppError::InvalidDate(s.to_string()))
}

/// Load and deserialize a JSON file
pub fn load_json_file<T: serde::de::DeserializeOwned>(path: &str) -> Result<T> {
    let file = File::open(path).map_err(|e| AppError::ReadFile {
        path: path.to_string(),
        source: e,
    })?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| AppError::JsonParse(e.to_string()))
}

/// Get current ISO year and week number
pub fn current_iso_week() -> (i32, u32) {
    let today = Local::now().date_naive();
    let iso_week = today.iso_week();
    (iso_week.year(), iso_week.week())
}

/// Convert ISO year and week to date range (Monday to Sunday)
pub fn week_to_date_range(year: i32, week: u32) -> Result<(NaiveDate, NaiveDate)> {
    // ISO week 1 is the week containing the first Thursday of the year
    // We use from_isoywd to get Monday of the given ISO week
    let monday = NaiveDate::from_isoywd_opt(year, week, chrono::Weekday::Mon)
        .ok_or_else(|| AppError::InvalidDate(format!("Invalid ISO week: {}-W{:02}", year, week)))?;
    let sunday = NaiveDate::from_isoywd_opt(year, week, chrono::Weekday::Sun)
        .ok_or_else(|| AppError::InvalidDate(format!("Invalid ISO week: {}-W{:02}", year, week)))?;
    Ok((monday, sunday))
}

/// Run the terminal UI
#[cfg(feature = "tui")]
pub fn run_ui() -> Result<()> {
    use std::io;
    use std::time::Duration;

    use crossterm::{
        event::{self, Event, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::Terminal;

    let token = load_token()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(token);

    loop {
        terminal.draw(|f| ui::ui(f, &mut app))?;

        app.check_async_result();
        app.check_progress();

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            input::handle_input(&mut app, key);
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
