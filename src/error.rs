use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("SLACK_TOKEN environment variable not set")]
    MissingToken,

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("invalid date format: {0}")]
    InvalidDate(String),

    #[error("Slack API error: {0}")]
    SlackApi(String),

    #[error("failed to read file at {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to write file at {path}: {source}")]
    WriteFile {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("JSON serialization error: {0}")]
    JsonSerialize(String),

    #[error("JSON parse error: {0}")]
    JsonParse(String),

    #[error("TOML parse error: {0}")]
    TomlParse(String),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(String),

    #[error("Meilisearch error: {0}")]
    Meilisearch(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
