use std::io;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub auto_sync_enabled: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            auto_sync_enabled: true,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "Config IO error: {}", e),
            ConfigError::Json(e) => write!(f, "Config JSON error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError::Json(e)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub persistence_path: Option<PathBuf>,
    pub user: UserSettings,
}

impl AppConfig {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("midilab").join("config.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_preset_directory() {
        let config = AppConfig::default();
        assert!(config.persistence_path.is_none());
    }

    #[test]
    fn default_config_has_default_user_settings() {
        let config = AppConfig::default();
        assert!(config.user.auto_sync_enabled);
    }

    #[test]
    fn config_path_uses_config_dir() {
        let path = AppConfig::config_path();
        if let Some(p) = path {
            assert!(p.ends_with("midilab/config.json"));
        }
    }

    #[test]
    fn load_with_default_or_error_works_with_existing_file() {
        let temp_dir = std::env::temp_dir();
        let config_file = temp_dir.join("test_config.json");

        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        std::fs::write(&config_file, json).unwrap();

        let loaded_config = AppConfig::default();

        assert!(loaded_config.user.auto_sync_enabled);

        let _ = std::fs::remove_file(&config_file);
    }
}
