use std::path::Path;

use midilab::manufacturer::korg::r3::wrappers::FormantMotion;
use midilab::manufacturer::korg::r3::wrappers::Global;
use midilab::manufacturer::korg::r3::wrappers::Program;
use midilab::manufacturer::korg::r3::wrappers::RawGlobal;
use midilab::manufacturer::korg::r3::wrappers::RawProgram;

use crate::config::AppConfig;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    FileSys(#[from] std::io::Error),
    #[error(transparent)]
    JsonSerialization(#[from] serde_json::Error),
    #[error("program deserialization: {0}")]
    ProgramDeserialization(String),
    #[error("global deserialization: {0}")]
    GlobalDeserialization(String),
    #[error("formant motion deserialization: {0}")]
    FormantDeserialization(String),
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

pub async fn save_program(program: Program, path: &Path) -> Result<String, Error> {
    let payload = program.as_bytes();
    tokio::fs::write(path, payload).await?;

    Ok(path.to_string_lossy().to_string())
}

pub async fn load_program_from_file(path: &Path) -> Result<Program, Error> {
    let bytes = tokio::fs::read(path).await?;
    let raw: &RawProgram = bytemuck::try_from_bytes(bytes.as_slice())
        .map_err(|e| Error::ProgramDeserialization(e.to_string()))?;
    let program = Program::try_from(*raw).map_err(Error::ProgramDeserialization)?;

    Ok(program)
}

pub async fn save_global(global: Global, path: &Path) -> Result<String, Error> {
    let payload = global.as_bytes();
    tokio::fs::write(path, payload).await?;

    Ok(path.to_string_lossy().to_string())
}

pub async fn load_global_from_file(path: &Path) -> Result<Global, Error> {
    let bytes = tokio::fs::read(path).await?;
    let raw: &RawGlobal = bytemuck::try_from_bytes(bytes.as_slice())
        .map_err(|e| Error::GlobalDeserialization(e.to_string()))?;
    let global = Global::try_from(*raw).map_err(Error::GlobalDeserialization)?;

    Ok(global)
}

pub async fn save_formant_motion(motion: FormantMotion, path: &Path) -> Result<String, Error> {
    tokio::fs::write(path, motion.as_bytes()).await?;

    Ok(path.to_string_lossy().to_string())
}

pub async fn load_formant_motion_from_file(path: &Path) -> Result<FormantMotion, Error> {
    let bytes = tokio::fs::read(path).await?;
    FormantMotion::from_bytes(None, &bytes).map_err(Error::FormantDeserialization)
}
