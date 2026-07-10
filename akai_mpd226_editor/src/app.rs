use std::boxed::Box;
use std::time::Instant;

use midilab::manufacturer::akai::mpd226::DeviceStatus;
use midilab::manufacturer::akai::mpd226::Global;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::control::value_kind::PresetSlot;

use crate::config::AppConfig;
use crate::message::AppEffect;
use crate::message::AppMsg;
use crate::message::DeviceMsg;
use crate::message::IoEffect;
use crate::message::IoMsg;
use crate::message::UiEffect;
use crate::message::UiMsg;
use crate::message::UserError;
use crate::message::UserMsg;
use crate::message::UserMsgKind;

pub struct AppState {
    pub preset: Preset,
    pub global: Global,
    pub config: AppConfig,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            preset: Preset::default(),
            global: Global::default(),
            config,
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
                UiEffect::PersistPreset { preset, path } => {
                    self.preset = *preset;
                    self.config.persistence_path =
                        Some(path.parent().unwrap_or(&path).to_path_buf());

                    let config_path = AppConfig::config_path().expect("Failed to get config path");

                    vec![
                        AppEffect::Io(Box::new(IoMsg::SavePreset { preset, path })),
                        AppEffect::Io(Box::new(IoMsg::PersistConfig {
                            config: self.config.clone(),
                            path: config_path,
                        })),
                    ]
                }
                UiEffect::ShowPresetSaveDialog => {
                    let path = std::env::temp_dir().join("akai_mpd226.preset");
                    vec![AppEffect::Ui(UiMsg::SavePresetDialog(path))]
                }
                UiEffect::ShowPresetLoadDialog => {
                    vec![AppEffect::Ui(UiMsg::LoadPresetDialog)]
                }
                UiEffect::LoadPresetFromFile { path } => {
                    vec![AppEffect::Io(Box::new(IoMsg::LoadPreset { path }))]
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
                UiEffect::PersistGlobal { global, path } => {
                    self.global = *global;
                    self.config.persistence_path =
                        Some(path.parent().unwrap_or(&path).to_path_buf());

                    let config_path = AppConfig::config_path().expect("Failed to get config path");

                    vec![
                        AppEffect::Io(Box::new(IoMsg::SaveGlobal { global, path })),
                        AppEffect::Io(Box::new(IoMsg::PersistConfig {
                            config: self.config.clone(),
                            path: config_path,
                        })),
                    ]
                }
                UiEffect::ShowGlobalSaveDialog => {
                    let path = std::env::temp_dir().join("akai_mpd226.global");
                    vec![AppEffect::Ui(UiMsg::SaveGlobalDialog(path))]
                }
                UiEffect::ShowGlobalLoadDialog => {
                    vec![AppEffect::Ui(UiMsg::LoadGlobalDialog)]
                }
                UiEffect::LoadGlobalFromFile { path } => {
                    vec![AppEffect::Io(Box::new(IoMsg::LoadGlobal { path }))]
                }
                UiEffect::ShowSettingsModal => {
                    vec![AppEffect::Ui(UiMsg::ShowSettingsModal)]
                }
                UiEffect::PersistUserSettings { config, path } => {
                    vec![AppEffect::Io(Box::new(IoMsg::PersistUserSettings {
                        config,
                        path,
                    }))]
                }
                UiEffect::AutoSync => {
                    vec![
                        AppEffect::Device(DeviceMsg::DumpPreset(PresetSlot::RAM)),
                        AppEffect::Device(DeviceMsg::DumpGlobal),
                    ]
                }
            },
            AppMsg::Device(msg) => match msg {
                DeviceStatus::PresetData(preset) => {
                    let slot = preset.settings.slot;
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
                            msg: "Wrote global settings to device".to_string(),
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
                    Ok(preset_path) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Saved preset {}", preset_path),
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
                    Ok(preset) => {
                        self.preset = *preset.clone();
                        vec![
                            AppEffect::Ui(UiMsg::UpdatePreset(preset)),
                            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                                msg: "Preset loaded".to_string(),
                                kind: UserMsgKind::Status,
                                received_at: Instant::now(),
                            })),
                        ]
                    }
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Preset load failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::GlobalSaveResult(result) => match result {
                    Ok(global_path) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Saved global {}", global_path),
                        kind: UserMsgKind::Status,
                        received_at: Instant::now(),
                    }))],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Global save failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::GlobalLoadResult(result) => match result {
                    Ok(global) => {
                        self.global = *global.clone();
                        vec![
                            AppEffect::Ui(UiMsg::UpdateGlobal(global)),
                            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                                msg: "Global loaded".to_string(),
                                kind: UserMsgKind::Status,
                                received_at: Instant::now(),
                            })),
                        ]
                    }
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("Global load failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::PersistConfigResult(result) => match result {
                    Ok(_) => vec![],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("App config save failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
                IoEffect::PersistUserSettingsResult(result) => match result {
                    Ok(_) => vec![],
                    Err(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                        msg: format!("User settings save failed: {e}"),
                        kind: UserMsgKind::Error,
                        received_at: Instant::now(),
                    }))],
                },
            },
            AppMsg::UserError(e) => match e {
                UserError::Midi(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                    msg: e.to_string(),
                    received_at: Instant::now(),
                    kind: UserMsgKind::Error,
                }))],
                UserError::SysexParse(e) => vec![AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                    msg: e.to_string(),
                    received_at: Instant::now(),
                    kind: UserMsgKind::Error,
                }))],
                UserError::DeviceStatusParse(e) => {
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
    use midilab::error::DeviceStatusParseError;
    use midilab::error::MidiError;
    use midilab::error::SysexParseError;
    use midilab::manufacturer::akai::mpd226::GlobalParamAck;
    use midilab::manufacturer::akai::mpd226::GlobalParamCmdId;
    use midilab::manufacturer::akai::mpd226::PresetAck;
    use midilab::manufacturer::akai::mpd226::control::PresetSettings;

    use super::*;
    use crate::config::UserSettings;

    fn preset_with_slot(slot: PresetSlot) -> Preset {
        Preset {
            settings: PresetSettings {
                slot,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn send_preset_to_device() {
        let mut app = AppState::new(AppConfig::default());
        let preset = preset_with_slot(PresetSlot::Slot3);

        let effects = app.update(AppMsg::Ui(UiEffect::WritePreset(Box::new(preset))));

        assert_eq!(app.preset.settings.slot, PresetSlot::Slot3);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Device(DeviceMsg::WritePreset(p)) if p.settings.slot == PresetSlot::Slot3
        ));
    }

    #[test]
    fn dump_preset() {
        let mut app = AppState::new(AppConfig::default());
        let original_slot = app.preset.settings.slot;

        let effects = app.update(AppMsg::Ui(UiEffect::DumpPreset(PresetSlot::Slot4)));

        assert_eq!(app.preset.settings.slot, original_slot);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Device(DeviceMsg::DumpPreset(PresetSlot::Slot4))
        ));
    }

    #[test]
    fn device_preset_data() {
        let mut app = AppState::new(AppConfig::default());
        let preset = preset_with_slot(PresetSlot::Slot1);

        let effects = app.update(AppMsg::Device(DeviceStatus::PresetData(Box::new(preset))));

        assert_eq!(app.preset.settings.slot, PresetSlot::Slot1);
        assert_eq!(effects.len(), 2);
        assert!(matches!(
            &effects[0],
            AppEffect::Ui(UiMsg::UpdatePreset(p)) if p.settings.slot == PresetSlot::Slot1
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
        let mut app = AppState::new(AppConfig::default());
        let original_slot = app.preset.settings.slot;

        let effects = app.update(AppMsg::Device(DeviceStatus::ReceivedPresetAck(PresetAck {
            slot: PresetSlot::Slot7,
        })));

        assert_eq!(app.preset.settings.slot, original_slot);
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
        let mut app = AppState::new(AppConfig::default());
        let original_slot = app.preset.settings.slot;

        let effects = app.update(AppMsg::UserError(UserError::Midi(
            MidiError::ResponseTimeout,
        )));

        assert_eq!(app.preset.settings.slot, original_slot);
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
        let mut app = AppState::new(AppConfig::default());
        let original_slot = app.preset.settings.slot;

        let effects = app.update(AppMsg::UserError(UserError::SysexParse(
            SysexParseError::InvalidStart(0x00),
        )));

        assert_eq!(app.preset.settings.slot, original_slot);
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
        let mut app = AppState::new(AppConfig::default());
        let original_slot = app.preset.settings.slot;

        let effects = app.update(AppMsg::UserError(UserError::DeviceStatusParse(
            DeviceStatusParseError::InvalidCommand(0xFF),
        )));

        assert_eq!(app.preset.settings.slot, original_slot);
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
        let mut app = AppState::new(AppConfig::default());
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
        let mut app = AppState::new(AppConfig::default());
        let original_contrast = app.global.lcd_contrast;

        let effects = app.update(AppMsg::Ui(UiEffect::RequestGlobalFromDevice));

        assert_eq!(app.global.lcd_contrast, original_contrast);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(effect, AppEffect::Device(DeviceMsg::DumpGlobal)));
    }

    #[test]
    fn device_global_data() {
        let mut app = AppState::new(AppConfig::default());
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
        let mut app = AppState::new(AppConfig::default());

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
        let mut app = AppState::new(AppConfig::default());

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
    fn persist_preset_with_config_path() {
        let persistence_path = std::env::temp_dir();

        let config = AppConfig {
            persistence_path: Some(persistence_path.clone()),
            user: UserSettings::default(),
        };
        let mut app = AppState {
            config,
            preset: Preset::default(),
            global: Global::default(),
        };
        let preset = preset_with_slot(PresetSlot::Slot2);

        let effects = app.update(AppMsg::Ui(UiEffect::PersistPreset {
            preset: Box::new(preset),
            path: persistence_path.join("test.preset"),
        }));

        assert_eq!(app.config.persistence_path, Some(persistence_path.clone()));
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn load_preset_from_file() {
        let persistence_path = std::env::temp_dir();

        let mut app = AppState::new(AppConfig::default());

        let mut effects = app.update(AppMsg::Ui(UiEffect::LoadPresetFromFile {
            path: persistence_path.join("test.preset"),
        }));

        assert_eq!(effects.len(), 1);
        let effect = effects.pop().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Io(msg) if matches!(msg.as_ref(), IoMsg::LoadPreset { path } if path == &persistence_path.join("test.preset"))
        ));
    }

    #[test]
    fn io_preset_save_success() {
        let persistence_path = std::env::temp_dir();

        let config = AppConfig {
            persistence_path: Some(persistence_path),
            user: UserSettings::default(),
        };
        let mut app = AppState {
            config,
            preset: Preset::default(),
            global: Global::default(),
        };

        let effects = app.update(AppMsg::Io(Box::new(IoEffect::PresetSaveResult(Ok(
            "saved".to_string(),
        )))));

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
        let mut app = AppState::new(AppConfig::default());

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
        let persistence_path = std::env::temp_dir();

        let config = AppConfig {
            persistence_path: Some(persistence_path),
            user: UserSettings::default(),
        };
        let mut app = AppState {
            config,
            preset: Preset::default(),
            global: Global::default(),
        };
        let preset = preset_with_slot(PresetSlot::Slot6);

        let effects = app.update(AppMsg::Io(Box::new(IoEffect::PresetLoadResult(Ok(
            Box::new(preset),
        )))));

        assert_eq!(effects.len(), 2);
        assert!(matches!(
            effects[1],
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Status,
                msg: ref m,
                ..
            })) if m.contains("loaded")
        ));
    }

    #[test]
    fn io_preset_load_failure() {
        let mut app = AppState::new(AppConfig::default());

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

    #[test]
    fn persist_global_with_config_path() {
        let persistence_path = std::env::temp_dir();

        let config = AppConfig {
            persistence_path: Some(persistence_path.clone()),
            user: UserSettings::default(),
        };
        let mut app = AppState {
            config,
            preset: Preset::default(),
            global: Global::default(),
        };
        let global = Global {
            lcd_contrast: 42,
            ..Default::default()
        };

        let effects = app.update(AppMsg::Ui(UiEffect::PersistGlobal {
            global: Box::new(global),
            path: persistence_path.join("test.global"),
        }));

        assert_eq!(app.config.persistence_path, Some(persistence_path.clone()));
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn load_global_from_file() {
        let persistence_path = std::env::temp_dir();

        let mut app = AppState::new(AppConfig::default());

        let mut effects = app.update(AppMsg::Ui(UiEffect::LoadGlobalFromFile {
            path: persistence_path.join("test.global"),
        }));

        assert_eq!(effects.len(), 1);
        let effect = effects.pop().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Io(msg) if matches!(msg.as_ref(), IoMsg::LoadGlobal { path } if path == &persistence_path.join("test.global"))
        ));
    }

    #[test]
    fn io_global_save_success() {
        let persistence_path = std::env::temp_dir();

        let config = AppConfig {
            persistence_path: Some(persistence_path),
            user: UserSettings::default(),
        };
        let mut app = AppState {
            config,
            preset: Preset::default(),
            global: Global::default(),
        };

        let effects = app.update(AppMsg::Io(Box::new(IoEffect::GlobalSaveResult(Ok(
            "saved".to_string(),
        )))));

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
    fn io_global_save_failure() {
        let mut app = AppState::new(AppConfig::default());

        let effects = app.update(AppMsg::Io(Box::new(IoEffect::GlobalSaveResult(Err(
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
    fn io_global_load_success() {
        let persistence_path = std::env::temp_dir();

        let config = AppConfig {
            persistence_path: Some(persistence_path),
            user: UserSettings::default(),
        };
        let mut app = AppState {
            config,
            preset: Preset::default(),
            global: Global::default(),
        };
        let global = Global {
            lcd_contrast: 35,
            ..Default::default()
        };

        let effects = app.update(AppMsg::Io(Box::new(IoEffect::GlobalLoadResult(Ok(
            Box::new(global),
        )))));

        assert_eq!(effects.len(), 2);
        assert!(matches!(
            effects[1],
            AppEffect::Ui(UiMsg::UserMsg(UserMsg {
                kind: UserMsgKind::Status,
                msg: ref m,
                ..
            })) if m.contains("loaded")
        ));
    }

    #[test]
    fn io_global_load_failure() {
        let mut app = AppState::new(AppConfig::default());

        let effects = app.update(AppMsg::Io(Box::new(IoEffect::GlobalLoadResult(Err(
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

    #[test]
    fn auto_sync_sends_both_dumps() {
        let mut app = AppState::new(AppConfig::default());

        let effects = app.update(AppMsg::Ui(UiEffect::AutoSync));

        assert_eq!(effects.len(), 2);
        assert!(matches!(
            &effects[0],
            AppEffect::Device(DeviceMsg::DumpPreset(slot)) if *slot == PresetSlot::RAM
        ));
        assert!(matches!(
            &effects[1],
            AppEffect::Device(DeviceMsg::DumpGlobal)
        ));
    }
}
