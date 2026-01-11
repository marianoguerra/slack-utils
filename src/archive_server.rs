//! Archive server module providing core logic for serving Slack archive parquet files.
//!
//! This module contains configuration structures and logic for finding and serving
//! parquet files from a Slack archive. The logic is separated from HTTP endpoints
//! to enable testing without starting a server.

use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::{AppError, Result};

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Host address to bind to (e.g., "127.0.0.1" or "0.0.0.0")
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Path to static assets directory to serve
    pub static_assets: Option<String>,
}

/// Slack archive configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackArchiveConfig {
    /// Base path where users.parquet, channels.parquet, and conversations/ are located
    pub base_path: String,
}

/// Meilisearch configuration for search functionality
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MeilisearchConfig {
    /// Meilisearch server URL (e.g., "http://localhost:7700")
    pub url: String,
    /// API key for authentication
    #[serde(rename = "api-key")]
    pub api_key: String,
    /// Index name to search
    #[serde(rename = "index-name")]
    pub index_name: String,
}

/// Complete server configuration file structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(rename = "slack-archive")]
    pub slack_archive: SlackArchiveConfig,
    /// Optional meilisearch configuration for search functionality
    #[serde(default)]
    pub meilisearch: Option<MeilisearchConfig>,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| AppError::ReadFile {
            path: path.display().to_string(),
            source: e,
        })?;
        toml::from_str(&content).map_err(|e| AppError::TomlParse(e.to_string()))
    }
}

/// Represents a year/week pair for thread partitions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct YearWeek {
    pub year: i32,
    pub week: u32,
}

impl YearWeek {
    pub fn new(year: i32, week: u32) -> Self {
        Self { year, week }
    }
}

/// Archive service providing access to parquet files
#[derive(Debug, Clone)]
pub struct ArchiveService {
    base_path: PathBuf,
}

impl ArchiveService {
    /// Create a new archive service with the given base path
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// Get the path to the users parquet file
    pub fn users_path(&self) -> PathBuf {
        self.base_path.join("users.parquet")
    }

    /// Get the path to the channels parquet file
    pub fn channels_path(&self) -> PathBuf {
        self.base_path.join("channels.parquet")
    }

    /// Get the path to a threads parquet file for a specific year/week
    pub fn threads_path(&self, year: i32, week: u32) -> PathBuf {
        self.base_path
            .join("conversations")
            .join(format!("year={}", year))
            .join(format!("week={:02}", week))
            .join("threads.parquet")
    }

    /// Check if the users parquet file exists
    pub fn users_exists(&self) -> bool {
        self.users_path().exists()
    }

    /// Check if the channels parquet file exists
    pub fn channels_exists(&self) -> bool {
        self.channels_path().exists()
    }

    /// Check if a threads parquet file exists for the given year/week
    pub fn threads_exists(&self, year: i32, week: u32) -> bool {
        self.threads_path(year, week).exists()
    }

    /// Get all year/week partitions that have existing parquet files within a date range.
    ///
    /// Returns only partitions where the parquet file actually exists on disk.
    pub fn threads_in_range(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<YearWeek>> {
        if from > to {
            return Err(AppError::InvalidDate(format!(
                "from date ({}) must be before or equal to to date ({})",
                from, to
            )));
        }

        let mut result = Vec::new();
        let mut current = from;

        // Track which year/weeks we've already checked to avoid duplicates
        let mut seen = std::collections::HashSet::new();

        while current <= to {
            let iso_week = current.iso_week();
            let year = iso_week.year();
            let week = iso_week.week();
            let key = (year, week);

            if !seen.contains(&key) {
                seen.insert(key);
                if self.threads_exists(year, week) {
                    result.push(YearWeek::new(year, week));
                }
            }

            // Move to the next day
            current = current
                .succ_opt()
                .ok_or_else(|| AppError::InvalidDate("date overflow".to_string()))?;
        }

        // Sort by year and week
        result.sort_by(|a, b| (a.year, a.week).cmp(&(b.year, b.week)));

        Ok(result)
    }

    /// Get the base path for this archive service
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_archive() -> (tempfile::TempDir, ArchiveService) {
        let dir = tempdir().unwrap();
        let service = ArchiveService::new(dir.path());
        (dir, service)
    }

    fn create_threads_partition(dir: &Path, year: i32, week: u32) {
        let partition_path = dir
            .join("conversations")
            .join(format!("year={}", year))
            .join(format!("week={:02}", week));
        fs::create_dir_all(&partition_path).unwrap();
        fs::write(partition_path.join("threads.parquet"), b"test").unwrap();
    }

    #[test]
    fn test_config_from_file() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let config_content = r#"
[server]
host = "127.0.0.1"
port = 8080
static_assets = "/var/www/static"

[slack-archive]
base_path = "/data/slack-archive"
"#;
        fs::write(&config_path, config_content).unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(
            config.server.static_assets,
            Some("/var/www/static".to_string())
        );
        assert_eq!(config.slack_archive.base_path, "/data/slack-archive");
    }

    #[test]
    fn test_config_from_file_minimal() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let config_content = r#"
[server]
host = "0.0.0.0"
port = 3000

[slack-archive]
base_path = "./archive"
"#;
        fs::write(&config_path, config_content).unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        assert!(config.server.static_assets.is_none());
        assert_eq!(config.slack_archive.base_path, "./archive");
        assert!(config.meilisearch.is_none());
    }

    #[test]
    fn test_config_from_file_with_meilisearch() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let config_content = r#"
[server]
host = "127.0.0.1"
port = 8080

[slack-archive]
base_path = "/data/archive"

[meilisearch]
url = "http://localhost:7700"
api-key = "secret-api-key"
index-name = "slack-messages"
"#;
        fs::write(&config_path, config_content).unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert!(config.meilisearch.is_some());
        let ms = config.meilisearch.unwrap();
        assert_eq!(ms.url, "http://localhost:7700");
        assert_eq!(ms.api_key, "secret-api-key");
        assert_eq!(ms.index_name, "slack-messages");
    }

    #[test]
    fn test_config_from_file_not_found() {
        let result = Config::from_file(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_file_invalid_toml() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, "invalid toml {{{").unwrap();

        let result = Config::from_file(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_archive_service_paths() {
        let (_dir, service) = create_test_archive();

        assert!(service.users_path().ends_with("users.parquet"));
        assert!(service.channels_path().ends_with("channels.parquet"));

        let threads_path = service.threads_path(2024, 3);
        assert!(threads_path.to_string_lossy().contains("year=2024"));
        assert!(threads_path.to_string_lossy().contains("week=03"));
        assert!(threads_path.ends_with("threads.parquet"));
    }

    #[test]
    fn test_users_exists_false() {
        let (_dir, service) = create_test_archive();
        assert!(!service.users_exists());
    }

    #[test]
    fn test_users_exists_true() {
        let (dir, service) = create_test_archive();
        fs::write(dir.path().join("users.parquet"), b"test").unwrap();
        assert!(service.users_exists());
    }

    #[test]
    fn test_channels_exists_false() {
        let (_dir, service) = create_test_archive();
        assert!(!service.channels_exists());
    }

    #[test]
    fn test_channels_exists_true() {
        let (dir, service) = create_test_archive();
        fs::write(dir.path().join("channels.parquet"), b"test").unwrap();
        assert!(service.channels_exists());
    }

    #[test]
    fn test_threads_exists_false() {
        let (_dir, service) = create_test_archive();
        assert!(!service.threads_exists(2024, 3));
    }

    #[test]
    fn test_threads_exists_true() {
        let (dir, service) = create_test_archive();
        create_threads_partition(dir.path(), 2024, 3);
        assert!(service.threads_exists(2024, 3));
    }

    #[test]
    fn test_threads_in_range_empty() {
        let (_dir, service) = create_test_archive();
        let from = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 1, 21).unwrap();

        let result = service.threads_in_range(from, to).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_threads_in_range_with_data() {
        let (dir, service) = create_test_archive();

        // Create partitions for weeks 3 and 4 of 2024
        create_threads_partition(dir.path(), 2024, 3);
        create_threads_partition(dir.path(), 2024, 4);

        // Query range that spans both weeks
        let from = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(); // Week 3
        let to = NaiveDate::from_ymd_opt(2024, 1, 28).unwrap(); // Week 4

        let result = service.threads_in_range(from, to).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], YearWeek::new(2024, 3));
        assert_eq!(result[1], YearWeek::new(2024, 4));
    }

    #[test]
    fn test_threads_in_range_partial_data() {
        let (dir, service) = create_test_archive();

        // Create partition only for week 3
        create_threads_partition(dir.path(), 2024, 3);

        // Query range that spans weeks 3 and 4, but only week 3 exists
        let from = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 1, 28).unwrap();

        let result = service.threads_in_range(from, to).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], YearWeek::new(2024, 3));
    }

    #[test]
    fn test_threads_in_range_single_day() {
        let (dir, service) = create_test_archive();
        create_threads_partition(dir.path(), 2024, 3);

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let result = service.threads_in_range(date, date).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], YearWeek::new(2024, 3));
    }

    #[test]
    fn test_threads_in_range_invalid_order() {
        let (_dir, service) = create_test_archive();
        let from = NaiveDate::from_ymd_opt(2024, 1, 28).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let result = service.threads_in_range(from, to);
        assert!(result.is_err());
    }

    #[test]
    fn test_threads_in_range_cross_year() {
        let (dir, service) = create_test_archive();

        // Create partitions across year boundary
        create_threads_partition(dir.path(), 2023, 52);
        create_threads_partition(dir.path(), 2024, 1);

        let from = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();

        let result = service.threads_in_range(from, to).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], YearWeek::new(2023, 52));
        assert_eq!(result[1], YearWeek::new(2024, 1));
    }

    #[test]
    fn test_year_week_equality() {
        let yw1 = YearWeek::new(2024, 3);
        let yw2 = YearWeek::new(2024, 3);
        let yw3 = YearWeek::new(2024, 4);

        assert_eq!(yw1, yw2);
        assert_ne!(yw1, yw3);
    }

    #[test]
    fn test_threads_path_week_padding() {
        let (_dir, service) = create_test_archive();

        // Week 3 should be zero-padded to "03"
        let path = service.threads_path(2024, 3);
        assert!(path.to_string_lossy().contains("week=03"));

        // Week 42 should remain "42"
        let path = service.threads_path(2024, 42);
        assert!(path.to_string_lossy().contains("week=42"));
    }
}
