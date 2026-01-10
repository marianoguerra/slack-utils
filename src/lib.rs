use std::io;
use std::time::Duration;

use chrono::{Local, NaiveDate};
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;

mod app;
mod cli;
mod commands;
mod error;
mod index;
mod input;
mod markdown;
mod meilisearch;
mod parquet;
mod settings;
mod slack;
mod ui;
mod widgets;

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

// Re-export command functions for main.rs compatibility
pub use commands::run_download_attachments;
pub use commands::run_export_channels as run_export_channels_async;
pub use commands::run_export_conversations as run_export_conversations_async;
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

/// Run the terminal UI
pub fn run_ui() -> Result<()> {
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
