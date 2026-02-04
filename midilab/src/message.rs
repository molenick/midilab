use std::path::PathBuf;
use std::time::Instant;

use crate::config::AppConfig;
use crate::error::DeviceStatusDeserializationError;
use crate::error::MidiError;
use crate::error::SysexDeserializationError;
use crate::manufacturer::akai::mpd226::DeviceStatus;
use crate::manufacturer::akai::mpd226::Preset;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetSlot;

/// Application system messages that are processed into AppEffects
pub enum AppMsg {
    Device(DeviceStatus),
    Ui(UiEffect),
    // todo group + organize or refactor error comms
    MidiError(MidiError), // this comes from midi layer, it's an io error
    SysexParseError(SysexDeserializationError), // this comes from parsing wire layer - it's not sysex
    DeviceStatusParseError(DeviceStatusDeserializationError), // this comes from parsing into a known device status - it's not a device status
    Io(Box<IoEffect>),
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
    SysexParse(SysexDeserializationError),
    DeviceStatusParseEr(DeviceStatusDeserializationError),
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
    SendPresetToDevice(Box<Preset>),
    RequestPresetFromDevice(PresetSlot),
    LoadPersistedPreset,
    PersistPreset(Box<Preset>),
    SetPresetDirectory,
    PresetDirectorySelected(PathBuf),
}

pub enum DeviceMsg {
    RequestPreset(PresetSlot),
    SendPreset(Box<Preset>),
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

pub struct AppState {
    pub preset: Preset,
    pub config: AppConfig,
    pending_save_preset: Option<Box<Preset>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            preset: Preset::default(),
            config: AppConfig::load(),
            pending_save_preset: None,
        }
    }

    #[must_use]
    pub fn update(&mut self, msg: AppMsg) -> Vec<AppEffect> {
        match msg {
            AppMsg::Ui(msg) => match msg {
                UiEffect::SendPresetToDevice(preset) => {
                    self.preset = *preset;
                    vec![AppEffect::Device(DeviceMsg::SendPreset(Box::new(
                        self.preset,
                    )))]
                }
                UiEffect::RequestPresetFromDevice(slot) => {
                    vec![AppEffect::Device(DeviceMsg::RequestPreset(slot))]
                }
                UiEffect::LoadPersistedPreset => {
                    if let Some(path) = self.config.preset_path() {
                        vec![AppEffect::Io(Box::new(IoMsg::LoadPreset { path }))]
                    } else {
                        vec![AppEffect::Ui(UiMsg::ShowDirectoryPicker {
                            for_action: PendingFileAction::Load,
                        })]
                    }
                }
                UiEffect::PersistPreset(preset) => {
                    if let Some(path) = self.config.preset_path() {
                        vec![AppEffect::Io(Box::new(IoMsg::SavePreset { preset, path }))]
                    } else {
                        self.pending_save_preset = Some(preset);
                        vec![AppEffect::Ui(UiMsg::ShowDirectoryPicker {
                            for_action: PendingFileAction::Save,
                        })]
                    }
                }
                UiEffect::SetPresetDirectory => {
                    vec![AppEffect::Ui(UiMsg::ShowDirectoryPicker {
                        for_action: PendingFileAction::ManualSet,
                    })]
                }
                UiEffect::PresetDirectorySelected(dir) => {
                    self.config.preset_directory = Some(dir.clone());
                    let _ = self.config.save();

                    let mut effects = vec![AppEffect::Ui(UiMsg::DirectoryConfigured(dir))];

                    if let Some(preset) = self.pending_save_preset.take()
                        && let Some(path) = self.config.preset_path()
                    {
                        effects.push(AppEffect::Io(Box::new(IoMsg::SavePreset { preset, path })));
                    }

                    effects
                }
            },

            AppMsg::Device(msg) => match msg {
                DeviceStatus::PresetData(preset) => {
                    let slot = preset.settings.preset_slot;
                    self.preset = *preset.clone();
                    vec![
                        AppEffect::Ui(UiMsg::UpdatePreset(preset)),
                        AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: format!("Loaded preset slot {slot} from device"),
                            kind: UserMsgKind::Status,
                            received_at: Instant::now(),
                        })),
                    ]
                }
                DeviceStatus::ReceivedPresetAck(slot) => {
                    vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Sent to device preset slot {slot}"),
                        kind: UserMsgKind::Status,
                        received_at: Instant::now(),
                    }))]
                }
            },
            AppMsg::MidiError(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                msg: e.to_string(),
                received_at: Instant::now(),
                kind: UserMsgKind::Error,
            }))],
            AppMsg::SysexParseError(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                msg: e.to_string(),
                received_at: Instant::now(),
                kind: UserMsgKind::Error,
            }))],
            AppMsg::DeviceStatusParseError(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                msg: e.to_string(),
                received_at: Instant::now(),
                kind: UserMsgKind::Error,
            }))],
            AppMsg::Io(io_effect) => match *io_effect {
                IoEffect::PresetSaveResult(result) => match result {
                    Ok(_) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: "Preset saved".to_string(),
                        kind: UserMsgKind::Status,
                        received_at: Instant::now(),
                    }))],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Preset save failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::PresetLoadResult(result) => match result {
                    Ok(_) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: "Preset loaded".to_string(),
                        kind: UserMsgKind::Status,
                        received_at: Instant::now(),
                    }))],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Preset save failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DeviceStatusDeserializationError;
    use crate::error::SysexDeserializationError;
    use crate::manufacturer::akai::mpd226::control::PresetSettings;

    fn preset_with_slot(slot: PresetSlot) -> Preset {
        Preset {
            settings: PresetSettings {
                preset_slot: slot,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn send_preset_to_device() {
        let mut app = AppState::default();
        let preset = preset_with_slot(PresetSlot::Slot3);

        let effects = app.update(AppMsg::Ui(UiEffect::SendPresetToDevice(Box::new(preset))));

        assert_eq!(app.preset.settings.preset_slot, PresetSlot::Slot3);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Device(DeviceMsg::SendPreset(p)) if p.settings.preset_slot == PresetSlot::Slot3
        ));
    }

    #[test]
    fn request_preset_from_device() {
        let mut app = AppState::default();
        let original_slot = app.preset.settings.preset_slot;

        let effects = app.update(AppMsg::Ui(UiEffect::RequestPresetFromDevice(
            PresetSlot::Slot4,
        )));

        assert_eq!(app.preset.settings.preset_slot, original_slot);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Device(DeviceMsg::RequestPreset(PresetSlot::Slot4))
        ));
    }

    #[test]
    fn device_preset_data() {
        let mut app = AppState::default();
        let preset = preset_with_slot(PresetSlot::Slot1);

        let effects = app.update(AppMsg::Device(DeviceStatus::PresetData(Box::new(preset))));

        assert_eq!(app.preset.settings.preset_slot, PresetSlot::Slot1);
        assert_eq!(effects.len(), 2);
        assert!(matches!(
            &effects[0],
            AppEffect::Ui(UiMsg::UpdatePreset(p)) if p.settings.preset_slot == PresetSlot::Slot1
        ));
        assert!(matches!(
            &effects[1],
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Status,
                ..
            }))
        ));
    }

    #[test]
    fn device_received_preset_ack() {
        let mut app = AppState::default();
        let original_slot = app.preset.settings.preset_slot;

        let effects = app.update(AppMsg::Device(DeviceStatus::ReceivedPresetAck(
            PresetSlot::Slot7,
        )));

        assert_eq!(app.preset.settings.preset_slot, original_slot);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Status,
                ..
            }))
        ));
    }

    #[test]
    fn midi_error() {
        let mut app = AppState::default();
        let original_slot = app.preset.settings.preset_slot;

        let effects = app.update(AppMsg::MidiError(MidiError::ResponseTimeout));

        assert_eq!(app.preset.settings.preset_slot, original_slot);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Error,
                ..
            }))
        ));
    }

    #[test]
    fn sysex_parse_error() {
        let mut app = AppState::default();
        let original_slot = app.preset.settings.preset_slot;

        let effects = app.update(AppMsg::SysexParseError(
            SysexDeserializationError::InvalidStart(0x00),
        ));

        assert_eq!(app.preset.settings.preset_slot, original_slot);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Error,
                ..
            }))
        ));
    }

    #[test]
    fn device_status_parse_error() {
        let mut app = AppState::default();
        let original_slot = app.preset.settings.preset_slot;

        let effects = app.update(AppMsg::DeviceStatusParseError(
            DeviceStatusDeserializationError::InvalidCommand(0xFF),
        ));

        assert_eq!(app.preset.settings.preset_slot, original_slot);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Error,
                ..
            }))
        ));
    }
}
