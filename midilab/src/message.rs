use std::path::PathBuf;
use std::time::Instant;

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
    MidiError(MidiError),             // this comes from midi layer, it's an io error
    SysexParseError(SysexParseError), // this comes from parsing wire layer - it's not sysex
    DeviceStatusParseError(DeviceStatusParseError), // this comes from parsing into a known device status - it's not a device status
}

pub enum IoMsg {
    SavePreset { preset: Box<Preset>, path: PathBuf },
    LoadPreset { path: PathBuf },
}

pub enum IoEffect {
    PresetSaveResult(Result<(), String>),
    PresetLoadResult(Result<Box<Preset>, String>),
}

pub enum SubsystemError {
    Midi(MidiError),
    SysexParse(SysexParseError),
    DeviceStatusParseEr(DeviceStatusParseError),
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
    ShowDirectoryPicker { for_action: PendingFileAction },
    DirectoryConfigured(PathBuf),
}

#[derive(Debug, Clone, Copy)]
pub enum PendingFileAction {
    Save,
    Load,
    ManualSet,
}

pub enum UiEffect {
    WritePreset(Box<Preset>),
    DumpPreset(PresetSlot),
    LoadPersistedPreset,
    PersistPreset(Box<Preset>),
    SetPresetDirectory,
    PresetDirectorySelected(PathBuf),
    SendGlobalToDevice(Box<Global>),
    RequestGlobalFromDevice,
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
