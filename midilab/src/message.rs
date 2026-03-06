use std::path::PathBuf;
use std::time::Instant;

use crate::config::AppConfig;
use crate::error::DeviceStatusParseError;
use crate::error::MidiError;
use crate::error::SysexParseError;
use crate::manufacturer::akai::mpd226::DeviceStatus;
use crate::manufacturer::akai::mpd226::Global;
use crate::manufacturer::akai::mpd226::Preset;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetSlot;

/// Application system messages that are processed into AppEffects
pub enum AppMsg {
    Device(DeviceStatus),
    Ui(UiEffect),
    UserError(UserError),
    Io(Box<IoEffect>),
}

pub enum UserError {
    Midi(MidiError),
    SysexParse(SysexParseError),
    DeviceStatusParse(DeviceStatusParseError),
}

pub enum IoMsg {
    PersistConfig { config: AppConfig, path: PathBuf },
    SavePreset { preset: Box<Preset>, path: PathBuf },
    LoadPreset { path: PathBuf },
    SaveGlobal { global: Box<Global>, path: PathBuf },
    LoadGlobal { path: PathBuf },
    PersistUserSettings { config: AppConfig, path: PathBuf },
}

pub enum IoEffect {
    PersistConfigResult(Result<(), String>),
    PresetSaveResult(Result<String, String>),
    PresetLoadResult(Result<Box<Preset>, String>),
    GlobalSaveResult(Result<String, String>),
    GlobalLoadResult(Result<Box<Global>, String>),
    PersistUserSettingsResult(Result<(), String>),
}

/// Application system effects produced from processing AppMsgs
pub enum AppEffect {
    Ui(UiMsg),
    Device(DeviceMsg),
    Io(Box<IoMsg>),
}

/// Notifies ui of updates
pub enum UiMsg {
    UpdatePreset(Box<Preset>),
    UpdateGlobal(Box<Global>),
    UserMsg(UserMsg),
    LoadPresetDialog,
    DirectoryConfigured(PathBuf),
    SavePresetDialog(PathBuf),
    LoadGlobalDialog,
    DirectoryConfiguredGlobal(PathBuf),
    SaveGlobalDialog(PathBuf),
    ShowSettingsModal,
    UpdateUserSettings(crate::config::UserSettings),
    AutoSync,
}

pub enum UiEffect {
    WritePreset(Box<Preset>),
    DumpPreset(PresetSlot),
    PersistPreset { preset: Box<Preset>, path: PathBuf },
    ShowPresetSaveDialog,
    ShowPresetLoadDialog,
    LoadPresetFromFile { path: PathBuf },
    SendGlobalToDevice(Box<Global>),
    RequestGlobalFromDevice,
    PersistGlobal { global: Box<Global>, path: PathBuf },
    ShowGlobalSaveDialog,
    ShowGlobalLoadDialog,
    LoadGlobalFromFile { path: PathBuf },
    ShowSettingsModal,
    PersistUserSettings { config: AppConfig, path: PathBuf },
    AutoSync,
}

pub enum DeviceMsg {
    DumpPreset(PresetSlot),
    WritePreset(Box<Preset>),
    DumpGlobal,
    WriteGlobal(Box<Global>),
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
