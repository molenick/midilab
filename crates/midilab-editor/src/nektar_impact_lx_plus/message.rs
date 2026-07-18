use std::path::PathBuf;
use std::time::Instant;

use midilab::error::MidiError;
use midilab::manufacturer::nektar::impact_lx_plus::Dump;
use midilab::manufacturer::nektar::impact_lx_plus::GlobalControls;
use midilab::manufacturer::nektar::impact_lx_plus::GlobalSettings;
use midilab::manufacturer::nektar::impact_lx_plus::PadMap;
use midilab::manufacturer::nektar::impact_lx_plus::Preset;
use midilab::manufacturer::nektar::impact_lx_plus::control::PadMapId;
use midilab::manufacturer::nektar::impact_lx_plus::control::PresetId;

use crate::nektar_impact_lx_plus::config::AppConfig;

pub enum AppMsg {
    Device(DeviceEvent),
    Ui(UiEffect),
    UserError(UserError),
    Io(Box<IoEffect>),
}

pub enum DeviceEvent {
    /// A panel-triggered memory dump started arriving.
    DumpStarted,
    /// A complete 182-message memory dump was received and assembled.
    DumpReceived(Box<Dump>),
    DumpWritten,
    PresetWritten(PresetId),
    PadMapWritten(PadMapId),
    GlobalSettingsWritten,
    GlobalControlsWritten,
    Reconnected,
}

pub enum UserError {
    Midi(MidiError),
    Parse(String),
}

pub enum IoMsg {
    PersistConfig { config: AppConfig, path: PathBuf },
    SaveDump { dump: Box<Dump>, path: PathBuf },
    LoadDump { path: PathBuf },
}

pub enum IoEffect {
    PersistConfigResult(Result<(), String>),
    DumpSaveResult(Result<String, String>),
    DumpLoadResult(Result<Box<Dump>, String>),
}

pub enum AppEffect {
    Ui(UiMsg),
    Device(DeviceMsg),
    Io(Box<IoMsg>),
}

pub enum UiMsg {
    UpdateDump(Box<Dump>),
    UserMsg(UserMsg),
    SaveDumpDialog(PathBuf),
    LoadDumpDialog,
}

pub enum UiEffect {
    WriteDump(Box<Dump>),
    WritePreset { id: PresetId, preset: Box<Preset> },
    WritePadMap { id: PadMapId, map: PadMap },
    WriteGlobalSettings(GlobalSettings),
    WriteGlobalControls(GlobalControls),
    Reconnect,
    PersistDump { dump: Box<Dump>, path: PathBuf },
    ShowDumpSaveDialog,
    ShowDumpLoadDialog,
    LoadDumpFromFile { path: PathBuf },
}

pub enum DeviceMsg {
    WriteDump(Box<Dump>),
    WritePreset { id: PresetId, preset: Box<Preset> },
    WritePadMap { id: PadMapId, map: PadMap },
    WriteGlobalSettings(GlobalSettings),
    WriteGlobalControls(GlobalControls),
    Reconnect,
}

pub struct UserMsg {
    pub msg: String,
    pub kind: UserMsgKind,
    pub received_at: Instant,
}

pub enum UserMsgKind {
    Status,
    Error,
}
