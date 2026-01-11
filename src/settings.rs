use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{AppError, Result};

const SETTINGS_FILE: &str = "settings.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub ui: UiSettings,
    #[serde(default)]
    pub meilisearch: MeilisearchSettings,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UiSettings {
    #[serde(default, rename = "selected-channels")]
    pub selected_channels: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MeilisearchSettings {
    #[serde(default)]
    pub url: String,
    #[serde(default, rename = "api-key")]
    pub api_key: String,
    #[serde(default, rename = "index-name")]
    pub index_name: String,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let path = Path::new(SETTINGS_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(|e| AppError::ReadFile {
            path: SETTINGS_FILE.to_string(),
            source: e,
        })?;

        toml::from_str(&content).map_err(|e| AppError::TomlParse(e.to_string()))
    }

    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(self).map_err(|e| AppError::TomlSerialize(e.to_string()))?;
        fs::write(SETTINGS_FILE, content).map_err(|e| AppError::WriteFile {
            path: SETTINGS_FILE.to_string(),
            source: e,
        })?;
        Ok(())
    }
}

#[cfg(feature = "tui")]
impl Settings {
    pub fn selected_channels_set(&self) -> std::collections::HashSet<String> {
        self.ui.selected_channels.iter().cloned().collect()
    }

    pub fn set_selected_channels(&mut self, channels: Vec<String>) {
        self.ui.selected_channels = channels;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_file_constant() {
        assert_eq!(SETTINGS_FILE, "settings.toml");
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();

        assert!(settings.ui.selected_channels.is_empty());
        assert!(settings.meilisearch.url.is_empty());
        assert!(settings.meilisearch.api_key.is_empty());
        assert!(settings.meilisearch.index_name.is_empty());
    }

    #[test]
    fn test_ui_settings_default() {
        let ui = UiSettings::default();

        assert!(ui.selected_channels.is_empty());
    }

    #[test]
    fn test_meilisearch_settings_default() {
        let ms = MeilisearchSettings::default();

        assert!(ms.url.is_empty());
        assert!(ms.api_key.is_empty());
        assert!(ms.index_name.is_empty());
    }

    #[test]
    fn test_settings_serialization() {
        let mut settings = Settings::default();
        settings.ui.selected_channels = vec!["general".to_string(), "random".to_string()];
        settings.meilisearch.url = "http://localhost:7700".to_string();
        settings.meilisearch.api_key = "secret-key".to_string();
        settings.meilisearch.index_name = "slack".to_string();

        let toml = toml::to_string(&settings).unwrap();

        assert!(toml.contains("selected-channels"));
        assert!(toml.contains("general"));
        assert!(toml.contains("random"));
        assert!(toml.contains("url"));
        assert!(toml.contains("http://localhost:7700"));
        assert!(toml.contains("api-key"));
        assert!(toml.contains("secret-key"));
        assert!(toml.contains("index-name"));
        assert!(toml.contains("slack"));
    }

    #[test]
    fn test_settings_deserialization() {
        let toml_content = r#"
[ui]
selected-channels = ["general", "random"]

[meilisearch]
url = "http://localhost:7700"
api-key = "test-key"
index-name = "test-index"
"#;

        let settings: Settings = toml::from_str(toml_content).unwrap();

        assert_eq!(settings.ui.selected_channels.len(), 2);
        assert_eq!(settings.ui.selected_channels[0], "general");
        assert_eq!(settings.ui.selected_channels[1], "random");
        assert_eq!(settings.meilisearch.url, "http://localhost:7700");
        assert_eq!(settings.meilisearch.api_key, "test-key");
        assert_eq!(settings.meilisearch.index_name, "test-index");
    }

    #[test]
    fn test_settings_deserialization_empty() {
        let toml_content = "";

        let settings: Settings = toml::from_str(toml_content).unwrap();

        assert!(settings.ui.selected_channels.is_empty());
        assert!(settings.meilisearch.url.is_empty());
    }

    #[test]
    fn test_settings_deserialization_partial_ui_only() {
        let toml_content = r#"
[ui]
selected-channels = ["announcements"]
"#;

        let settings: Settings = toml::from_str(toml_content).unwrap();

        assert_eq!(settings.ui.selected_channels.len(), 1);
        assert_eq!(settings.ui.selected_channels[0], "announcements");
        assert!(settings.meilisearch.url.is_empty());
    }

    #[test]
    fn test_settings_deserialization_partial_meilisearch_only() {
        let toml_content = r#"
[meilisearch]
url = "http://example.com:7700"
"#;

        let settings: Settings = toml::from_str(toml_content).unwrap();

        assert!(settings.ui.selected_channels.is_empty());
        assert_eq!(settings.meilisearch.url, "http://example.com:7700");
        assert!(settings.meilisearch.api_key.is_empty());
        assert!(settings.meilisearch.index_name.is_empty());
    }

    #[test]
    #[cfg(feature = "tui")]
    fn test_selected_channels_set_empty() {
        let settings = Settings::default();
        let set = settings.selected_channels_set();

        assert!(set.is_empty());
    }

    #[test]
    #[cfg(feature = "tui")]
    fn test_selected_channels_set_with_values() {
        let mut settings = Settings::default();
        settings.ui.selected_channels = vec![
            "general".to_string(),
            "random".to_string(),
            "dev".to_string(),
        ];

        let set = settings.selected_channels_set();

        assert_eq!(set.len(), 3);
        assert!(set.contains("general"));
        assert!(set.contains("random"));
        assert!(set.contains("dev"));
    }

    #[test]
    #[cfg(feature = "tui")]
    fn test_selected_channels_set_deduplicates() {
        let mut settings = Settings::default();
        settings.ui.selected_channels = vec![
            "general".to_string(),
            "general".to_string(),
            "random".to_string(),
        ];

        let set = settings.selected_channels_set();

        assert_eq!(set.len(), 2);
        assert!(set.contains("general"));
        assert!(set.contains("random"));
    }

    #[test]
    #[cfg(feature = "tui")]
    fn test_set_selected_channels() {
        let mut settings = Settings::default();
        assert!(settings.ui.selected_channels.is_empty());

        settings.set_selected_channels(vec!["channel1".to_string(), "channel2".to_string()]);

        assert_eq!(settings.ui.selected_channels.len(), 2);
        assert_eq!(settings.ui.selected_channels[0], "channel1");
        assert_eq!(settings.ui.selected_channels[1], "channel2");
    }

    #[test]
    #[cfg(feature = "tui")]
    fn test_set_selected_channels_replaces() {
        let mut settings = Settings::default();
        settings.ui.selected_channels = vec!["old1".to_string(), "old2".to_string()];

        settings.set_selected_channels(vec!["new1".to_string()]);

        assert_eq!(settings.ui.selected_channels.len(), 1);
        assert_eq!(settings.ui.selected_channels[0], "new1");
    }

    #[test]
    fn test_meilisearch_settings_clone() {
        let ms = MeilisearchSettings {
            url: "http://localhost:7700".to_string(),
            api_key: "key".to_string(),
            index_name: "index".to_string(),
        };

        let cloned = ms.clone();

        assert_eq!(cloned.url, ms.url);
        assert_eq!(cloned.api_key, ms.api_key);
        assert_eq!(cloned.index_name, ms.index_name);
    }

    #[test]
    fn test_settings_roundtrip() {
        let mut settings = Settings::default();
        settings.ui.selected_channels = vec!["ch1".to_string(), "ch2".to_string()];
        settings.meilisearch = MeilisearchSettings {
            url: "http://localhost:7700".to_string(),
            api_key: "api-key-123".to_string(),
            index_name: "my-index".to_string(),
        };

        let toml = toml::to_string(&settings).unwrap();
        let deserialized: Settings = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.ui.selected_channels, settings.ui.selected_channels);
        assert_eq!(deserialized.meilisearch.url, settings.meilisearch.url);
        assert_eq!(deserialized.meilisearch.api_key, settings.meilisearch.api_key);
        assert_eq!(deserialized.meilisearch.index_name, settings.meilisearch.index_name);
    }
}
