use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{AppError, Result};

const SETTINGS_FILE: &str = "settings.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub ui: UiSettings,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UiSettings {
    #[serde(default, rename = "selected-channels")]
    pub selected_channels: Vec<String>,
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

    pub fn selected_channels_set(&self) -> HashSet<String> {
        self.ui.selected_channels.iter().cloned().collect()
    }

    pub fn set_selected_channels(&mut self, channels: Vec<String>) {
        self.ui.selected_channels = channels;
    }
}
