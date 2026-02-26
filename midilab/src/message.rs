use std::path::PathBuf;
use std::time::Instant;

use crate::config::PendingAction;
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
    SavePreset { preset: Box<Preset>, path: PathBuf },
    LoadPreset { path: PathBuf },
}

pub enum IoEffect {
    PresetSaveResult(Result<(), String>),
    PresetLoadResult(Result<Box<Preset>, String>),
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
    ShowDirectoryPicker { for_action: PendingAction },
    DirectoryConfigured(PathBuf),
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
impl UiEffect {
    pub fn as_app_msg(self) -> AppMsg {
        AppMsg::Ui(self)
    }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ui_effect_as_app_msg() {
        let eff = UiEffect::RequestGlobalFromDevice;
        let msg = eff.as_app_msg();
        assert!(matches!(msg, AppMsg::Ui(UiEffect::RequestGlobalFromDevice)));
    }
}
