use crate::config::AppConfig;
use crate::manufacturer::akai::mpd226::DeviceStatus;
use crate::manufacturer::akai::mpd226::Global;
use crate::manufacturer::akai::mpd226::Preset;
use crate::message::AppEffect;
use crate::message::AppMsg;
use crate::message::DeviceMsg;
use crate::message::IoEffect;
use crate::message::IoMsg;
use crate::message::PendingFileAction;
use crate::message::UiEffect;
use crate::message::UiMsg;
use crate::message::UserError;
use crate::message::UserMsg;
use crate::message::UserMsgKind;

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
                        })),
                    ]
                }
                DeviceStatus::ReceivedPresetAck(ack) => {
                    vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Sent to device preset slot {}", ack.slot),
                        kind: UserMsgKind::Status,
                    }))]
                }
                DeviceStatus::GlobalData(global) => {
                    self.global = *global.clone();
                    vec![
                        AppEffect::Ui(UiMsg::UpdateGlobal(global)),
                        AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: "Loaded global settings from device".to_string(),
                            kind: UserMsgKind::Status,
                        })),
                    ]
                }
                DeviceStatus::GlobalParamAck(ack) => {
                    if ack.status == 0 {
                        vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: "Wrote global settings from device".to_string(),
                            kind: UserMsgKind::Status,
                        }))]
                    } else {
                        let addr = ack.addr as u8;
                        let status = ack.status;
                        vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                            msg: format!("Global param {addr:#04x} write failed: status {status}"),
                            kind: UserMsgKind::Error,
                        }))]
                    }
                }
            },
            AppMsg::Io(io_effect) => match *io_effect {
                IoEffect::PresetSaveResult(result) => match result {
                    Ok(_) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: "Preset saved".to_string(),
                        kind: UserMsgKind::Status,
                    }))],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Preset save failed: {e}"),
                        kind: UserMsgKind::Error,
                    }))],
                },
                IoEffect::PresetLoadResult(result) => match result {
                    Ok(_) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: "Preset loaded".to_string(),
                        kind: UserMsgKind::Status,
                    }))],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Preset load failed: {e}"),
                        kind: UserMsgKind::Error,
                    }))],
                },
            },
            AppMsg::UserError(e) => match e {
                UserError::Midi(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                    msg: e.to_string(),
                    kind: UserMsgKind::Error,
                }))],
                UserError::SysexParse(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                    msg: e.to_string(),
                    kind: UserMsgKind::Error,
                }))],
                UserError::DeviceStatusParse(e) => {
                    vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: e.to_string(),

                        kind: UserMsgKind::Error,
                    }))]
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::error::DeviceStatusParseError;
    use crate::error::MidiError;
    use crate::error::SysexParseError;
    use crate::manufacturer::akai::mpd226::GlobalParamAck;
    use crate::manufacturer::akai::mpd226::GlobalParamCmdId;
    use crate::manufacturer::akai::mpd226::PresetAck;
    use crate::manufacturer::akai::mpd226::control::PresetSettings;
    use crate::manufacturer::akai::mpd226::control::value_kind::PresetSlot;

    fn preset_with_slot(slot: PresetSlot) -> Preset {
        Preset {
            settings: PresetSettings {
                preset_slot: slot,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn app_with_preset_dir(dir: Option<PathBuf>) -> AppState {
        let mut app = AppState::new();
        app.config.preset_directory = dir;
        app
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

        let effects = app.update(AppMsg::UserError(UserError::Midi(
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

        let effects = app.update(AppMsg::UserError(UserError::SysexParse(
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

        let effects = app.update(AppMsg::UserError(UserError::DeviceStatusParse(
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
    fn load_persisted_preset_with_config_path() {
        let mut app = app_with_preset_dir(Some(PathBuf::from("/test/path")));

        let effects = app.update(AppMsg::Ui(UiEffect::LoadPersistedPreset));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Io(ref msg) if matches!(**msg, IoMsg::LoadPreset { ref path } if path.ends_with("akai_mpd226_preset"))
        ));
    }

    #[test]
    fn load_persisted_preset_without_config_path() {
        let mut app = app_with_preset_dir(None);

        let effects = app.update(AppMsg::Ui(UiEffect::LoadPersistedPreset));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Ui(UiMsg::ShowDirectoryPicker {
                for_action: PendingFileAction::Load
            })
        ));
    }

    #[test]
    fn persist_preset_with_config_path() {
        let mut app = app_with_preset_dir(Some(PathBuf::from("/test/path")));
        let preset = preset_with_slot(PresetSlot::Slot2);

        let effects = app.update(AppMsg::Ui(UiEffect::PersistPreset(Box::new(preset))));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Io(ref msg) if matches!(**msg, IoMsg::SavePreset { ref path, .. } if path.ends_with("akai_mpd226_preset"))
        ));
        assert!(app.pending_save_preset.is_none());
    }

    #[test]
    fn persist_preset_without_config_path() {
        let mut app = app_with_preset_dir(None);
        let preset = preset_with_slot(PresetSlot::Slot2);

        let effects = app.update(AppMsg::Ui(UiEffect::PersistPreset(Box::new(preset))));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Ui(UiMsg::ShowDirectoryPicker {
                for_action: PendingFileAction::Save
            })
        ));
        assert!(app.pending_save_preset.is_some());
        assert_eq!(
            app.pending_save_preset
                .as_ref()
                .unwrap()
                .settings
                .preset_slot,
            PresetSlot::Slot2
        );
    }

    #[test]
    fn set_preset_directory() {
        let mut app = AppState::default();

        let effects = app.update(AppMsg::Ui(UiEffect::SetPresetDirectory));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Ui(UiMsg::ShowDirectoryPicker {
                for_action: PendingFileAction::ManualSet
            })
        ));
    }

    #[test]
    fn preset_directory_selected_without_pending_save() {
        let mut app = AppState::default();
        let dir = PathBuf::from("/new/path");

        let effects = app.update(AppMsg::Ui(UiEffect::PresetDirectorySelected(dir.clone())));

        assert_eq!(app.config.preset_directory, Some(dir.clone()));
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Ui(UiMsg::DirectoryConfigured(ref d)) if d == &dir
        ));
    }

    #[test]
    fn preset_directory_selected_with_pending_save() {
        let mut app = AppState::default();
        let preset = preset_with_slot(PresetSlot::Slot5);
        app.pending_save_preset = Some(Box::new(preset));
        let dir = PathBuf::from("/new/path");

        let effects = app.update(AppMsg::Ui(UiEffect::PresetDirectorySelected(dir.clone())));

        assert_eq!(app.config.preset_directory, Some(dir.clone()));
        assert_eq!(effects.len(), 2);
        assert!(matches!(
            effects[0],
            AppEffect::Ui(UiMsg::DirectoryConfigured(ref d)) if d == &dir
        ));
        assert!(matches!(
            effects[1],
            AppEffect::Io(ref msg) if matches!(**msg, IoMsg::SavePreset { ref preset, .. } if preset.settings.preset_slot == PresetSlot::Slot5)
        ));
        assert!(app.pending_save_preset.is_none());
    }

    #[test]
    fn io_preset_save_success() {
        let mut app = AppState::default();

        let effects = app.update(AppMsg::Io(Box::new(IoEffect::PresetSaveResult(Ok(())))));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Status,
                msg: ref m,
                ..
            })) if m.contains("saved")
        ));
    }

    #[test]
    fn io_preset_save_failure() {
        let mut app = AppState::default();

        let effects = app.update(AppMsg::Io(Box::new(IoEffect::PresetSaveResult(Err(
            "disk full".to_string(),
        )))));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Error,
                msg: ref m,
                ..
            })) if m.contains("disk full")
        ));
    }

    #[test]
    fn io_preset_load_success() {
        let mut app = AppState::default();
        let preset = preset_with_slot(PresetSlot::Slot6);

        let effects = app.update(AppMsg::Io(Box::new(IoEffect::PresetLoadResult(Ok(
            Box::new(preset),
        )))));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Status,
                msg: ref m,
                ..
            })) if m.contains("loaded")
        ));
    }

    #[test]
    fn io_preset_load_failure() {
        let mut app = AppState::default();

        let effects = app.update(AppMsg::Io(Box::new(IoEffect::PresetLoadResult(Err(
            "file not found".to_string(),
        )))));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Error,
                msg: ref m,
                ..
            })) if m.contains("file not found")
        ));
    }
}
