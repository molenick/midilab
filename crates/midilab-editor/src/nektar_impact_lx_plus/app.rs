use std::time::Instant;

use midilab::manufacturer::nektar::impact_lx_plus::Dump;

use crate::nektar_impact_lx_plus::config::AppConfig;
use crate::nektar_impact_lx_plus::message::AppEffect;
use crate::nektar_impact_lx_plus::message::AppMsg;
use crate::nektar_impact_lx_plus::message::DeviceEvent;
use crate::nektar_impact_lx_plus::message::DeviceMsg;
use crate::nektar_impact_lx_plus::message::IoEffect;
use crate::nektar_impact_lx_plus::message::IoMsg;
use crate::nektar_impact_lx_plus::message::UiEffect;
use crate::nektar_impact_lx_plus::message::UiMsg;
use crate::nektar_impact_lx_plus::message::UserError;
use crate::nektar_impact_lx_plus::message::UserMsg;
use crate::nektar_impact_lx_plus::message::UserMsgKind;

pub struct AppState {
    pub dump: Dump,
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
            dump: Dump::default(),
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
            UiEffect::WriteDump(dump) => {
                self.dump = *dump;
                vec![AppEffect::Device(DeviceMsg::WriteDump(Box::new(self.dump)))]
            }
            UiEffect::WritePreset { id, preset } => {
                self.dump.presets[id as usize - 1] = *preset;
                vec![AppEffect::Device(DeviceMsg::WritePreset { id, preset })]
            }
            UiEffect::WritePadMap { id, map } => {
                self.dump.pad_maps[id as usize - 1] = map;
                vec![AppEffect::Device(DeviceMsg::WritePadMap { id, map })]
            }
            UiEffect::WriteGlobalSettings(settings) => {
                self.dump.settings = settings;
                vec![AppEffect::Device(DeviceMsg::WriteGlobalSettings(settings))]
            }
            UiEffect::WriteGlobalControls(controls) => {
                self.dump.controls = controls;
                vec![AppEffect::Device(DeviceMsg::WriteGlobalControls(controls))]
            }
            UiEffect::Reconnect => vec![AppEffect::Device(DeviceMsg::Reconnect)],
            UiEffect::PersistDump { dump, path } => {
                self.dump = *dump;
                vec![AppEffect::Io(Box::new(IoMsg::SaveDump {
                    dump: Box::new(self.dump),
                    path,
                }))]
            }
            UiEffect::ShowDumpSaveDialog => {
                let path = std::env::temp_dir().join(self.dump.default_filename());
                vec![AppEffect::Ui(UiMsg::SaveDumpDialog(path))]
            }
            UiEffect::ShowDumpLoadDialog => {
                vec![AppEffect::Ui(UiMsg::LoadDumpDialog)]
            }
            UiEffect::LoadDumpFromFile { path } => {
                vec![AppEffect::Io(Box::new(IoMsg::LoadDump { path }))]
            }
        }
    }

    fn update_device(&mut self, event: DeviceEvent) -> Vec<AppEffect> {
        match event {
            DeviceEvent::DumpStarted => vec![status("Receiving memory dump from device...")],
            DeviceEvent::DumpReceived(dump) => {
                self.dump = *dump;
                vec![
                    AppEffect::Ui(UiMsg::UpdateDump(Box::new(self.dump))),
                    status("Memory dump received from device"),
                ]
            }
            DeviceEvent::DumpWritten => vec![status(
                "Full dump written (presets & pads activate on next load from the panel)",
            )],
            DeviceEvent::PresetWritten(id) => vec![status(format!(
                "{id} written to stored memory (activates next time it is loaded on the device)"
            ))],
            DeviceEvent::PadMapWritten(id) => vec![status(format!(
                "{id} written to stored memory (activates next time it is loaded on the device)"
            ))],
            DeviceEvent::GlobalSettingsWritten => {
                vec![status("Global settings written (applied instantly)")]
            }
            DeviceEvent::GlobalControlsWritten => {
                vec![status("Wheels & transport written (applied instantly)")]
            }
            DeviceEvent::Reconnected => vec![status("Device connected")],
        }
    }

    fn update_io(&mut self, effect: IoEffect) -> Vec<AppEffect> {
        match effect {
            IoEffect::DumpSaveResult(Ok(path)) => vec![status(format!("Dump saved: {path}"))],
            IoEffect::DumpSaveResult(Err(e)) => vec![error(format!("Dump save failed: {e}"))],
            IoEffect::DumpLoadResult(Ok(dump)) => {
                self.dump = *dump;
                vec![
                    AppEffect::Ui(UiMsg::UpdateDump(Box::new(self.dump))),
                    status("Dump loaded from file"),
                ]
            }
            IoEffect::DumpLoadResult(Err(e)) => vec![error(format!("Dump load failed: {e}"))],
            IoEffect::PersistConfigResult(Ok(())) => vec![],
            IoEffect::PersistConfigResult(Err(e)) => {
                vec![error(format!("Config persist failed: {e}"))]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use midilab::manufacturer::nektar::impact_lx_plus::Preset;
    use midilab::manufacturer::nektar::impact_lx_plus::control::PresetId;

    use super::*;

    #[test]
    fn test_write_preset_updates_state() {
        let mut state = AppState::new(AppConfig::default());
        let mut preset = Preset::default();
        preset.faders[0].cc = 42.into();

        let effects = state.update(AppMsg::Ui(UiEffect::WritePreset {
            id: PresetId::Preset3,
            preset: Box::new(preset),
        }));

        assert_eq!(state.dump.presets[2].faders[0].cc, 42.into());
        assert!(matches!(
            effects[0],
            AppEffect::Device(DeviceMsg::WritePreset {
                id: PresetId::Preset3,
                ..
            })
        ));
    }

    #[test]
    fn test_dump_received_updates_state_and_ui() {
        let mut state = AppState::new(AppConfig::default());
        let mut dump = Dump::default();
        dump.presets[0].pots[3].cc = 99.into();

        let effects = state.update(AppMsg::Device(DeviceEvent::DumpReceived(Box::new(dump))));

        assert_eq!(state.dump.presets[0].pots[3].cc, 99.into());
        assert_eq!(effects.len(), 2);
        assert!(matches!(effects[0], AppEffect::Ui(UiMsg::UpdateDump(_))));
    }

    #[test]
    fn test_write_global_settings_produces_device_effect() {
        let mut state = AppState::new(AppConfig::default());
        let settings = state.dump.settings;

        let effects = state.update(AppMsg::Ui(UiEffect::WriteGlobalSettings(settings)));

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            AppEffect::Device(DeviceMsg::WriteGlobalSettings(_))
        ));
    }
}
