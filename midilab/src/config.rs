use std::fs;
use std::io;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

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
    pub preset_directory: Option<PathBuf>,
}

impl AppConfig {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("midilab").join("config.json"))
    }

    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| fs::read_to_string(&path).ok())
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path().ok_or_else(|| {
            ConfigError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine config directory",
            ))
        })?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn preset_path(&self) -> Option<PathBuf> {
        self.preset_directory
            .as_ref()
            .map(|dir| dir.join("akai_mpd226_preset"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_preset_directory() {
        let config = AppConfig::default();
        assert!(config.preset_directory.is_none());
        assert!(config.preset_path().is_none());
    }

    #[test]
    fn preset_path_appends_filename() {
        let config = AppConfig {
            preset_directory: Some(PathBuf::from("/some/dir")),
        };
        assert_eq!(
            config.preset_path(),
            Some(PathBuf::from("/some/dir/akai_mpd226_preset"))
        );
    }

    #[test]
    fn config_path_uses_config_dir() {
        let path = AppConfig::config_path();
        // Should return Some on most systems
        if let Some(p) = path {
            assert!(p.ends_with("midilab/config.json"));
        }
    }
}
