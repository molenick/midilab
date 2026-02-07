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

pub struct AppState {
    pub preset: Preset,
    pub global: Global,
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
            global: Global::default(),
            config: AppConfig::load(),
            pending_save_preset: None,
        }
    }

    #[must_use]
    pub fn update(&mut self, msg: AppMsg) -> Vec<AppEffect> {
        match msg {
            AppMsg::Ui(msg) => match msg {
                UiEffect::WritePreset(preset) => {
                    self.preset = *preset;
                    vec![AppEffect::Device(DeviceMsg::WritePreset(Box::new(
                        self.preset,
                    )))]
                }
                UiEffect::DumpPreset(slot) => {
                    vec![AppEffect::Device(DeviceMsg::DumpPreset(slot))]
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
                UiEffect::SendGlobalToDevice(global) => {
                    self.global = *global;
                    vec![AppEffect::Device(DeviceMsg::WriteGlobal(Box::new(
                        self.global,
                    )))]
                }
                UiEffect::RequestGlobalFromDevice => {
                    vec![AppEffect::Device(DeviceMsg::DumpGlobal)]
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
                DeviceStatus::ReceivedPresetAck(ack) => {
                    vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Sent to device preset slot {}", ack.slot),
                        kind: UserMsgKind::Status,
                        received_at: Instant::now(),
                    }))]
                }
                DeviceStatus::GlobalData(global) => {
                    self.global = *global.clone();
                    vec![
                        AppEffect::Ui(UiMsg::UpdateGlobal(global)),
                        AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: "Loaded global settings from device".to_string(),
                            kind: UserMsgKind::Status,
                            received_at: Instant::now(),
                        })),
                    ]
                }
                DeviceStatus::GlobalParamAck(ack) => {
                    if ack.status == 0 {
                        vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: "Wrote global settings from device".to_string(),
                            kind: UserMsgKind::Status,
                            received_at: Instant::now(),
                        }))]
                    } else {
                        let addr = ack.addr as u8;
                        let status = ack.status;
                        vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: format!("Global param {addr:#04x} write failed: status {status}"),
                            kind: UserMsgKind::Error,
                            received_at: Instant::now(),
                        }))]
                    }
                }
            },
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
            AppMsg::UserError(e) => match e {
                UserError::MidiError(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                    msg: e.to_string(),
                    received_at: Instant::now(),
                    kind: UserMsgKind::Error,
                }))],
                UserError::SysexParseError(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                    msg: e.to_string(),
                    received_at: Instant::now(),
                    kind: UserMsgKind::Error,
                }))],
                UserError::DeviceStatusParseError(e) => {
                    vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: e.to_string(),
                        received_at: Instant::now(),
                        kind: UserMsgKind::Error,
                    }))]
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DeviceStatusParseError;
    use crate::error::SysexParseError;
    use crate::manufacturer::akai::mpd226::GlobalParamAck;
    use crate::manufacturer::akai::mpd226::GlobalParamCmdId;
    use crate::manufacturer::akai::mpd226::PresetAck;
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

        let effects = app.update(AppMsg::Ui(UiEffect::WritePreset(Box::new(preset))));

        assert_eq!(app.preset.settings.preset_slot, PresetSlot::Slot3);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Device(DeviceMsg::WritePreset(p)) if p.settings.preset_slot == PresetSlot::Slot3
        ));
    }

    #[test]
    fn test_dump_preset() {
        let mut app = AppState::default();
        let original_slot = app.preset.settings.preset_slot;

        let effects = app.update(AppMsg::Ui(UiEffect::DumpPreset(PresetSlot::Slot4)));

        assert_eq!(app.preset.settings.preset_slot, original_slot);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Device(DeviceMsg::DumpPreset(PresetSlot::Slot4))
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

        let effects = app.update(AppMsg::Device(DeviceStatus::ReceivedPresetAck(PresetAck {
            slot: PresetSlot::Slot7,
        })));

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

        let effects = app.update(AppMsg::UserError(UserError::MidiError(
            MidiError::ResponseTimeout,
        )));

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

        let effects = app.update(AppMsg::UserError(UserError::SysexParseError(
            SysexParseError::InvalidStart(0x00),
        )));

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

        let effects = app.update(AppMsg::UserError(UserError::DeviceStatusParseError(
            DeviceStatusParseError::InvalidCommand(0xFF),
        )));

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
    fn send_global_to_device() {
        let mut app = AppState::default();
        let global = Global {
            lcd_contrast: 42,
            ..Default::default()
        };

        let effects = app.update(AppMsg::Ui(UiEffect::SendGlobalToDevice(Box::new(global))));

        assert_eq!(app.global.lcd_contrast, 42);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Device(DeviceMsg::WriteGlobal(g)) if g.lcd_contrast == 42
        ));
    }

    #[test]
    fn request_global_from_device() {
        let mut app = AppState::default();
        let original_contrast = app.global.lcd_contrast;

        let effects = app.update(AppMsg::Ui(UiEffect::RequestGlobalFromDevice));

        assert_eq!(app.global.lcd_contrast, original_contrast);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(effect, AppEffect::Device(DeviceMsg::DumpGlobal)));
    }

    #[test]
    fn device_global_data() {
        let mut app = AppState::default();
        let global = Global {
            lcd_contrast: 35,
            pad_threshold: 7,
            ..Default::default()
        };

        let effects = app.update(AppMsg::Device(DeviceStatus::GlobalData(Box::new(global))));

        assert_eq!(app.global.lcd_contrast, 35);
        assert_eq!(app.global.pad_threshold, 7);
        assert_eq!(effects.len(), 2);
        assert!(matches!(
            &effects[0],
            AppEffect::Ui(UiMsg::UpdateGlobal(g)) if g.lcd_contrast == 35
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
    fn device_global_param_ack_success() {
        let mut app = AppState::default();

        let mut effects = app.update(AppMsg::Device(DeviceStatus::GlobalParamAck(
            GlobalParamAck {
                addr: GlobalParamCmdId::try_from(0x02_u8).unwrap(),
                status: 0,
            },
        )));

        assert_eq!(effects.len(), 1);
        let effect = effects.pop().unwrap();

        assert!(matches!(
            effect,
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Status,
                ..
            }))
        ));
    }

    #[test]
    fn device_global_param_ack_failure() {
        let mut app = AppState::default();

        let effects = app.update(AppMsg::Device(DeviceStatus::GlobalParamAck(
            GlobalParamAck {
                addr: GlobalParamCmdId::try_from(0x02_u8).unwrap(),
                status: 1,
            },
        )));

        // Failure should produce an error message
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
