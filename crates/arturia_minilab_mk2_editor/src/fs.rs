use std::path::Path;

use midilab::manufacturer::arturia::minilab_mk2::Global;
use midilab::manufacturer::arturia::minilab_mk2::Preset;

use crate::config::AppConfig;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    FileSys(#[from] std::io::Error),
    #[error(transparent)]
    JsonSerialization(#[from] serde_json::Error),
    #[error("preset deserialization: {0}")]
    PresetDeserialization(String),
    #[error("global deserialization: {0}")]
    GlobalDeserialization(String),
}

pub async fn load_app_config(path: &Path) -> Result<AppConfig, Error> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str::<AppConfig>(&content)?)
}

pub async fn persist_config(config: AppConfig, path: &Path) -> Result<(), Error> {
    Ok(tokio::fs::write(path, serde_json::to_vec(&config)?).await?)
}

pub async fn persist_user_settings(config: AppConfig, path: &Path) -> Result<(), Error> {
    Ok(tokio::fs::write(path, serde_json::to_vec(&config)?).await?)
}

pub async fn save_preset(preset: Preset, path: &Path) -> Result<String, Error> {
    tokio::fs::write(path, preset.as_bytes()).await?;

    Ok(path.to_string_lossy().to_string())
}

pub async fn load_preset_from_file(path: &Path) -> Result<Preset, Error> {
    let bytes = tokio::fs::read(path).await?;
    Preset::try_from(bytes.as_slice()).map_err(|e| Error::PresetDeserialization(e.to_string()))
}

pub async fn save_global(global: Global, path: &Path) -> Result<String, Error> {
    tokio::fs::write(path, global.as_bytes()).await?;

    Ok(path.to_string_lossy().to_string())
}

pub async fn load_global_from_file(path: &Path) -> Result<Global, Error> {
    let bytes = tokio::fs::read(path).await?;
    Global::try_from(bytes.as_slice()).map_err(|e| Error::GlobalDeserialization(e.to_string()))
}
