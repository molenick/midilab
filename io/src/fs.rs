use std::path::Path;

use bytemuck::PodCastError;
use midilab::config::AppConfig;
use midilab::manufacturer::akai;
use midilab::manufacturer::akai::mpd226::Global;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::error::GlobalParseError;
use midilab::manufacturer::akai::mpd226::error::PresetParseError;
use midilab::manufacturer::akai::mpd226::raw::RawGlobal;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    FileSys(#[from] std::io::Error),
    #[error(transparent)]
    RawPresetDeserialization(#[from] PodCastError),
    #[error(transparent)]
    PresetDeserialization(#[from] PresetParseError),
    #[error(transparent)]
    GlobalDeserialization(#[from] GlobalParseError),
    #[error(transparent)]
    JsonSerialization(#[from] serde_json::Error),
}

pub async fn persist_config(config: AppConfig, path: &Path) -> Result<(), Error> {
    Ok(tokio::fs::write(path, serde_json::to_vec(&config)?).await?)
}

pub async fn persist_user_settings(config: AppConfig, path: &Path) -> Result<(), Error> {
    Ok(tokio::fs::write(path, serde_json::to_vec(&config)?).await?)
}

pub async fn save_akai_mpd226_preset(
    preset: akai::mpd226::Preset,
    path: &Path,
) -> Result<String, Error> {
    let raw = RawPreset::from(&preset);
    let payload = bytemuck::bytes_of(&raw).to_vec();
    tokio::fs::write(path, payload).await?;

    let path = path.to_path_buf();

    Ok(path.to_string_lossy().to_string())
}

pub async fn load_akai_mpd226_preset_from_sysex(
    path: &Path,
) -> Result<akai::mpd226::Preset, Error> {
    let bytes = tokio::fs::read(path).await?;
    let raw: RawPreset = *bytemuck::try_from_bytes(&bytes)?;
    let preset = Preset::try_from(raw)?;

    Ok(preset)
}

pub async fn save_akai_mpd226_global(
    global: akai::mpd226::Global,
    path: &Path,
) -> Result<String, Error> {
    let raw = RawGlobal::from(&global);
    let payload = bytemuck::bytes_of(&raw).to_vec();
    tokio::fs::write(path, payload).await?;

    let path = path.to_path_buf();

    Ok(path.to_string_lossy().to_string())
}

pub async fn load_akai_mpd226_global_from_bytes(
    path: &Path,
) -> Result<akai::mpd226::Global, Error> {
    let bytes = tokio::fs::read(path).await?;
    let raw: RawGlobal = *bytemuck::try_from_bytes(&bytes)?;
    let global = Global::try_from(raw)?;

    Ok(global)
}
