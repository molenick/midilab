use std::io;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Config IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Config JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub persistence_path: Option<PathBuf>,
}

impl AppConfig {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("midilab").join("nektar_impact_lx_plus.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_persistence_path() {
        let config = AppConfig::default();
        assert!(config.persistence_path.is_none());
    }

    #[test]
    fn config_path_uses_config_dir() {
        let path = AppConfig::config_path();
        if let Some(p) = path {
            assert!(p.ends_with("midilab/nektar_impact_lx_plus.json"));
        }
    }
}
