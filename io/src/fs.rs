use std::path::Path;

use bytemuck::PodCastError;
use midilab::manufacturer::akai;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::error::PresetParseError;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    FileSys(#[from] std::io::Error),
    #[error(transparent)]
    RawPresetDeserialization(#[from] PodCastError),
    #[error(transparent)]
    PresetDeserialization(#[from] PresetParseError),
}

pub async fn save_akai_mpd226_preset(
    preset: akai::mpd226::Preset,
    path: &Path,
) -> Result<(), Error> {
    let raw = RawPreset::from(&preset);
    let payload = bytemuck::bytes_of(&raw).to_vec();

    Ok(tokio::fs::write(path, payload).await?)
}

pub async fn load_akai_mpd226_preset_from_sysex(
    path: &Path,
) -> Result<akai::mpd226::Preset, Error> {
    let bytes = tokio::fs::read(path).await?;
    let raw: RawPreset = *bytemuck::try_from_bytes(&bytes)?;
    let preset = Preset::try_from(raw)?;

    Ok(preset)
}
