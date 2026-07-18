use std::path::PathBuf;
use std::time::Instant;

use midilab::error::MidiError;
use midilab::manufacturer::arturia::minilab_mk2::Global;
use midilab::manufacturer::arturia::minilab_mk2::Preset;
use midilab::manufacturer::arturia::minilab_mk2::control::ControlId;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::MemorySlot;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::PadColor;

use crate::config::AppConfig;
use crate::config::UserSettings;

pub enum AppMsg {
    Device(DeviceEvent),
    Ui(UiEffect),
    UserError(UserError),
    Io(Box<IoEffect>),
}

pub enum DeviceEvent {
    PresetRead(Box<Preset>),
    GlobalRead(Global),
    PresetWritten,
    GlobalWritten,
    MemoryRecalled(MemorySlot),
    MemoryStored(MemorySlot),
    IdentityReceived([u8; 4]),
    LiveColorSent,
}

pub enum UserError {
    Midi(MidiError),
    Parse(String),
}

pub enum IoMsg {
    PersistConfig { config: AppConfig, path: PathBuf },
    PersistUserSettings { config: AppConfig, path: PathBuf },
    SavePreset { preset: Box<Preset>, path: PathBuf },
    LoadPreset { path: PathBuf },
    SaveGlobal { global: Global, path: PathBuf },
    LoadGlobal { path: PathBuf },
}

pub enum IoEffect {
    PersistConfigResult(Result<(), String>),
    PersistUserSettingsResult(Result<(), String>),
    PresetSaveResult(Result<String, String>),
    PresetLoadResult(Result<Box<Preset>, String>),
    GlobalSaveResult(Result<String, String>),
    GlobalLoadResult(Result<Global, String>),
}

pub enum AppEffect {
    Ui(UiMsg),
    Device(DeviceMsg),
    Io(Box<IoMsg>),
}

pub enum UiMsg {
    UpdatePreset(Box<Preset>),
    UpdateGlobal(Box<Global>),
    UserMsg(UserMsg),
    DirectoryConfigured(PathBuf),
    LoadPresetDialog,
    SavePresetDialog(PathBuf),
    LoadGlobalDialog,
    SaveGlobalDialog(PathBuf),
    ShowSettingsModal,
    UpdateUserSettings(UserSettings),
    AutoSync,
}

pub enum UiEffect {
    ReadPreset,
    WritePreset(Box<Preset>),
    ReadGlobal,
    WriteGlobal(Global),
    RecallMemory(MemorySlot),
    StoreMemory(MemorySlot),
    LivePadColor { pad: ControlId, color: PadColor },
    PersistPreset { preset: Box<Preset>, path: PathBuf },
    ShowPresetSaveDialog,
    ShowPresetLoadDialog,
    LoadPresetFromFile { path: PathBuf },
    PersistGlobal { global: Global, path: PathBuf },
    ShowGlobalSaveDialog,
    ShowGlobalLoadDialog,
    LoadGlobalFromFile { path: PathBuf },
    ShowSettingsModal,
    PersistUserSettings { config: AppConfig, path: PathBuf },
    AutoSync,
}

pub enum DeviceMsg {
    ReadPreset,
    WritePreset(Box<Preset>),
    ReadGlobal,
    WriteGlobal(Global),
    RecallMemory(MemorySlot),
    StoreMemory(MemorySlot),
    SetLivePadColor { pad: ControlId, color: PadColor },
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
