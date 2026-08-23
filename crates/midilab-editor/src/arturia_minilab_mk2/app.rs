use std::time::Instant;

use midilab::manufacturer::arturia::minilab_mk2::Global;
use midilab::manufacturer::arturia::minilab_mk2::Preset;

use crate::arturia_minilab_mk2::config::AppConfig;
use crate::arturia_minilab_mk2::message::AppEffect;
use crate::arturia_minilab_mk2::message::AppMsg;
use crate::arturia_minilab_mk2::message::DeviceEvent;
use crate::arturia_minilab_mk2::message::DeviceMsg;
use crate::arturia_minilab_mk2::message::IoEffect;
use crate::arturia_minilab_mk2::message::IoMsg;
use crate::arturia_minilab_mk2::message::UiEffect;
use crate::arturia_minilab_mk2::message::UiMsg;
use crate::arturia_minilab_mk2::message::UserError;
use crate::arturia_minilab_mk2::message::UserMsg;
use crate::arturia_minilab_mk2::message::UserMsgKind;

pub struct AppState {
    pub preset: Preset,
    pub global: Global,
    pub config: AppConfig,
}

fn status(msg: impl Into<String>) -> AppEffect {
    AppEffect::Ui(UiMsg::UserMsg(UserMsg {
        msg: msg.into(),
        kind: UserMsgKind::Status,
        received_at: Instant::now(),
    }))
}

fn error(msg: impl Into<String>) -> AppEffect {
    AppEffect::Ui(UiMsg::UserMsg(UserMsg {
        msg: msg.into(),
        kind: UserMsgKind::Error,
        received_at: Instant::now(),
    }))
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
            AppMsg::Ui(msg) => self.update_ui(msg),
            AppMsg::Device(event) => self.update_device(event),
            AppMsg::UserError(e) => match e {
                UserError::Midi(e) => vec![error(format!("MIDI error: {e}"))],
                UserError::Parse(e) => vec![error(format!("Parse error: {e}"))],
            },
            AppMsg::Io(effect) => self.update_io(*effect),
        }
    }

    fn update_ui(&mut self, msg: UiEffect) -> Vec<AppEffect> {
        match msg {
            UiEffect::ReadPreset => vec![AppEffect::Device(DeviceMsg::ReadPreset)],
            UiEffect::WritePreset(preset) => {
                self.preset = *preset;
                vec![AppEffect::Device(DeviceMsg::WritePreset(Box::new(
                    self.preset,
                )))]
            }
            UiEffect::ReadGlobal => vec![AppEffect::Device(DeviceMsg::ReadGlobal)],
            UiEffect::WriteGlobal(global) => {
                self.global = global;
                vec![AppEffect::Device(DeviceMsg::WriteGlobal(global))]
            }
            UiEffect::RecallMemory(slot) => {
                vec![
                    AppEffect::Device(DeviceMsg::RecallMemory(slot)),
                    AppEffect::Device(DeviceMsg::ReadPreset),
                ]
            }
            UiEffect::StoreMemory(slot) => {
                vec![AppEffect::Device(DeviceMsg::StoreMemory(slot))]
            }
            UiEffect::LivePadColor { pad, color } => {
                vec![AppEffect::Device(DeviceMsg::SetLivePadColor { pad, color })]
            }
            UiEffect::PersistPreset { preset, path } => {
                self.preset = *preset;
                vec![AppEffect::Io(Box::new(IoMsg::SavePreset {
                    preset: Box::new(self.preset),
                    path,
                }))]
            }
            UiEffect::ShowPresetSaveDialog => {
                let path = std::env::temp_dir().join(self.preset.default_filename());
                vec![AppEffect::Ui(UiMsg::SavePresetDialog(path))]
            }
            UiEffect::ShowPresetLoadDialog => {
                vec![AppEffect::Ui(UiMsg::LoadPresetDialog)]
            }
            UiEffect::LoadPresetFromFile { path } => {
                vec![AppEffect::Io(Box::new(IoMsg::LoadPreset { path }))]
            }
            UiEffect::PersistGlobal { global, path } => {
                self.global = global;
                vec![AppEffect::Io(Box::new(IoMsg::SaveGlobal { global, path }))]
            }
            UiEffect::ShowGlobalSaveDialog => {
                let path = std::env::temp_dir().join("arturia_minilab_mk2.global");
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
                self.config = config.clone();
                vec![AppEffect::Io(Box::new(IoMsg::PersistUserSettings {
                    config,
                    path,
                }))]
            }
            UiEffect::AutoSync => {
                vec![
                    AppEffect::Device(DeviceMsg::ReadPreset),
                    AppEffect::Device(DeviceMsg::ReadGlobal),
                ]
            }
        }
    }

    fn update_device(&mut self, event: DeviceEvent) -> Vec<AppEffect> {
        match event {
            DeviceEvent::PresetRead(preset) => {
                self.preset = *preset;
                vec![
                    AppEffect::Ui(UiMsg::UpdatePreset(Box::new(self.preset))),
                    status("Preset loaded from device"),
                ]
            }
            DeviceEvent::GlobalRead(global) => {
                self.global = global;
                vec![
                    AppEffect::Ui(UiMsg::UpdateGlobal(Box::new(global))),
                    status("Global loaded from device"),
                ]
            }
            DeviceEvent::PresetWritten => vec![status("Preset written to device")],
            DeviceEvent::GlobalWritten => vec![status("Global written to device")],
            DeviceEvent::MemoryRecalled(slot) => vec![status(format!("Recalled memory {slot}"))],
            DeviceEvent::MemoryStored(slot) => vec![status(format!("Stored memory {slot}"))],
            DeviceEvent::IdentityReceived(fw) => vec![status(format!("Device firmware: {fw:?}"))],
            DeviceEvent::LiveColorSent => vec![],
        }
    }

    fn update_io(&mut self, effect: IoEffect) -> Vec<AppEffect> {
        match effect {
            IoEffect::PresetSaveResult(Ok(path)) => vec![status(format!("Preset saved: {path}"))],
            IoEffect::PresetSaveResult(Err(e)) => vec![error(format!("Preset save failed: {e}"))],
            IoEffect::PresetLoadResult(Ok(preset)) => {
                self.preset = *preset;
                vec![
                    AppEffect::Ui(UiMsg::UpdatePreset(Box::new(self.preset))),
                    status("Preset loaded from file"),
                ]
            }
            IoEffect::PresetLoadResult(Err(e)) => vec![error(format!("Preset load failed: {e}"))],
            IoEffect::GlobalSaveResult(Ok(path)) => vec![status(format!("Global saved: {path}"))],
            IoEffect::GlobalSaveResult(Err(e)) => vec![error(format!("Global save failed: {e}"))],
            IoEffect::GlobalLoadResult(Ok(global)) => {
                self.global = global;
                vec![
                    AppEffect::Ui(UiMsg::UpdateGlobal(Box::new(global))),
                    status("Global loaded from file"),
                ]
            }
            IoEffect::GlobalLoadResult(Err(e)) => vec![error(format!("Global load failed: {e}"))],
            IoEffect::PersistConfigResult(Ok(())) => vec![],
            IoEffect::PersistConfigResult(Err(e)) => {
                vec![error(format!("Config persist failed: {e}"))]
            }
            IoEffect::PersistUserSettingsResult(Ok(())) => vec![],
            IoEffect::PersistUserSettingsResult(Err(e)) => {
                vec![error(format!("Settings persist failed: {e}"))]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::MemorySlot;

    use super::*;

    #[test]
    fn test_read_preset_produces_device_effect() {
        let mut state = AppState::new(AppConfig::default());

        let effects = state.update(AppMsg::Ui(UiEffect::ReadPreset));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Device(DeviceMsg::ReadPreset)
        ));
    }

    #[test]
    fn test_write_preset_updates_state() {
        let mut state = AppState::new(AppConfig::default());
        let mut preset = Preset::default();
        preset.knobs.knobs[0].cc = 42.into();

        let effects = state.update(AppMsg::Ui(UiEffect::WritePreset(Box::new(preset))));

        assert_eq!(state.preset.knobs.knobs[0].cc, 42.into());
        assert!(matches!(
            effects[0],
            AppEffect::Device(DeviceMsg::WritePreset(_))
        ));
    }

    #[test]
    fn test_recall_memory_triggers_read_back() {
        let mut state = AppState::new(AppConfig::default());

        let effects = state.update(AppMsg::Ui(UiEffect::RecallMemory(MemorySlot::Slot3)));

        assert_eq!(effects.len(), 2);
        assert!(matches!(
            effects[0],
            AppEffect::Device(DeviceMsg::RecallMemory(MemorySlot::Slot3))
        ));
        assert!(matches!(
            effects[1],
            AppEffect::Device(DeviceMsg::ReadPreset)
        ));
    }

    #[test]
    fn test_device_preset_read_updates_state_and_ui() {
        let mut state = AppState::new(AppConfig::default());
        let mut preset = Preset::default();
        preset.knobs.knobs[3].cc = 99.into();

        let effects = state.update(AppMsg::Device(DeviceEvent::PresetRead(Box::new(preset))));

        assert_eq!(state.preset.knobs.knobs[3].cc, 99.into());
        assert_eq!(effects.len(), 2);
        assert!(matches!(effects[0], AppEffect::Ui(UiMsg::UpdatePreset(_))));
    }

    #[test]
    fn test_autosync_reads_preset_and_global() {
        let mut state = AppState::new(AppConfig::default());

        let effects = state.update(AppMsg::Ui(UiEffect::AutoSync));

        assert_eq!(effects.len(), 2);
        assert!(matches!(
            effects[0],
            AppEffect::Device(DeviceMsg::ReadPreset)
        ));
        assert!(matches!(
            effects[1],
            AppEffect::Device(DeviceMsg::ReadGlobal)
        ));
    }
}
