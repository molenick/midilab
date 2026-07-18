use std::path::Path;

use midilab::manufacturer::nektar::impact_lx_plus::Dump;

use crate::config::AppConfig;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    FileSys(#[from] std::io::Error),
    #[error(transparent)]
    JsonSerialization(#[from] serde_json::Error),
    #[error("dump deserialization: {0}")]
    DumpDeserialization(String),
}

pub async fn load_app_config(path: &Path) -> Result<AppConfig, Error> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str::<AppConfig>(&content)?)
}

pub async fn persist_config(config: AppConfig, path: &Path) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    Ok(tokio::fs::write(path, serde_json::to_vec(&config)?).await?)
}

pub async fn save_dump(dump: Dump, path: &Path) -> Result<String, Error> {
    tokio::fs::write(path, dump.as_bytes()).await?;

    Ok(path.to_string_lossy().to_string())
}

pub async fn load_dump_from_file(path: &Path) -> Result<Dump, Error> {
    let bytes = tokio::fs::read(path).await?;
    Dump::try_from(bytes.as_slice()).map_err(|e| Error::DumpDeserialization(e.to_string()))
}
