use std::path::Path;

use bytemuck::PodCastError;
use midilab::manufacturer::akai::mpd226::Global;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::error::GlobalParseError;
use midilab::manufacturer::akai::mpd226::error::PresetParseError;
use midilab::manufacturer::akai::mpd226::raw::RawGlobal;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;

use crate::config::AppConfig;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    FileSys(#[from] std::io::Error),
    #[error(transparent)]
    JsonSerialization(#[from] serde_json::Error),
    #[error(transparent)]
    RawPresetDeserialization(#[from] PodCastError),
    #[error(transparent)]
    PresetDeserialization(#[from] PresetParseError),
    #[error(transparent)]
    GlobalDeserialization(#[from] GlobalParseError),
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
    let raw = RawPreset::from(&preset);
    let payload = bytemuck::bytes_of(&raw).to_vec();
    tokio::fs::write(path, payload).await?;

    Ok(path.to_string_lossy().to_string())
}

pub async fn load_preset_from_file(path: &Path) -> Result<Preset, Error> {
    let bytes = tokio::fs::read(path).await?;
    let raw: RawPreset = *bytemuck::try_from_bytes(&bytes)?;
    let preset = Preset::try_from(raw)?;

    Ok(preset)
}

pub async fn save_global(global: Global, path: &Path) -> Result<String, Error> {
    let raw = RawGlobal::from(&global);
    let payload = bytemuck::bytes_of(&raw).to_vec();
    tokio::fs::write(path, payload).await?;

    Ok(path.to_string_lossy().to_string())
}

pub async fn load_global_from_file(path: &Path) -> Result<Global, Error> {
    let bytes = tokio::fs::read(path).await?;
    let raw: RawGlobal = *bytemuck::try_from_bytes(&bytes)?;
    let global = Global::try_from(raw)?;

    Ok(global)
}
