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

    #[error("Slack rate limit error: retry after {retry_after_secs}s")]
    SlackRateLimit { retry_after_secs: u64 },

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

    #[error("invalid output format: {0}")]
    InvalidFormat(String),

    #[error("Parquet error: {0}")]
    Parquet(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_missing_token_display() {
        let err = AppError::MissingToken;
        assert_eq!(err.to_string(), "SLACK_TOKEN environment variable not set");
    }

    #[test]
    fn test_io_error_display() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = AppError::Io(io_err);
        assert!(err.to_string().starts_with("IO error:"));
    }

    #[test]
    fn test_io_error_from_conversion() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let err: AppError = io_err.into();
        assert!(matches!(err, AppError::Io(_)));
    }

    #[test]
    fn test_invalid_date_display() {
        let err = AppError::InvalidDate("not-a-date".to_string());
        assert_eq!(err.to_string(), "invalid date format: not-a-date");
    }

    #[test]
    fn test_slack_api_display() {
        let err = AppError::SlackApi("rate limited".to_string());
        assert_eq!(err.to_string(), "Slack API error: rate limited");
    }

    #[test]
    fn test_slack_rate_limit_display() {
        let err = AppError::SlackRateLimit { retry_after_secs: 30 };
        assert_eq!(err.to_string(), "Slack rate limit error: retry after 30s");
    }

    #[test]
    fn test_read_file_display() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "no such file");
        let err = AppError::ReadFile {
            path: "/path/to/file.json".to_string(),
            source: io_err,
        };
        assert!(err.to_string().contains("/path/to/file.json"));
        assert!(err.to_string().contains("failed to read file"));
    }

    #[test]
    fn test_read_file_source() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "no such file");
        let err = AppError::ReadFile {
            path: "/path/to/file.json".to_string(),
            source: io_err,
        };
        assert!(err.source().is_some());
    }

    #[test]
    fn test_write_file_display() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let err = AppError::WriteFile {
            path: "/path/to/output.json".to_string(),
            source: io_err,
        };
        assert!(err.to_string().contains("/path/to/output.json"));
        assert!(err.to_string().contains("failed to write file"));
    }

    #[test]
    fn test_write_file_source() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let err = AppError::WriteFile {
            path: "/path/to/output.json".to_string(),
            source: io_err,
        };
        assert!(err.source().is_some());
    }

    #[test]
    fn test_json_serialize_display() {
        let err = AppError::JsonSerialize("invalid utf-8".to_string());
        assert_eq!(err.to_string(), "JSON serialization error: invalid utf-8");
    }

    #[test]
    fn test_json_parse_display() {
        let err = AppError::JsonParse("unexpected token".to_string());
        assert_eq!(err.to_string(), "JSON parse error: unexpected token");
    }

    #[test]
    fn test_toml_parse_display() {
        let err = AppError::TomlParse("invalid toml".to_string());
        assert_eq!(err.to_string(), "TOML parse error: invalid toml");
    }

    #[test]
    fn test_toml_serialize_display() {
        let err = AppError::TomlSerialize("serialization failed".to_string());
        assert_eq!(err.to_string(), "TOML serialization error: serialization failed");
    }

    #[test]
    fn test_meilisearch_display() {
        let err = AppError::Meilisearch("connection refused".to_string());
        assert_eq!(err.to_string(), "Meilisearch error: connection refused");
    }

    #[test]
    fn test_error_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AppError>();
    }

    #[test]
    fn test_error_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<AppError>();
    }

    #[test]
    fn test_error_debug() {
        let err = AppError::MissingToken;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("MissingToken"));
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(AppError::MissingToken);
        assert!(result.is_err());
    }
}
