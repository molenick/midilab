use std::time::Instant;

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
}

/// Notifies ui of updates
pub enum UiMsg {
    UpdatePreset(Box<Preset>),
    UserMsg(UserMsg),
}

pub enum UiEffect {
    SendPresetToDevice(Box<Preset>),
    RequestPresetFromDevice(PresetSlot),
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
}
impl Default for AppState {
    fn default() -> Self {
        Self {
            preset: Preset::generic_preset(),
        }
    }
}

impl AppState {
    #[must_use]
    pub fn update(&mut self, msg: AppMsg) -> Vec<AppEffect> {
        match msg {
            AppMsg::Ui(UiEffect::SendPresetToDevice(preset)) => {
                self.preset = *preset;
                vec![AppEffect::Device(DeviceMsg::SendPreset(Box::new(
                    self.preset,
                )))]
            }
            AppMsg::Ui(UiEffect::RequestPresetFromDevice(slot)) => {
                vec![AppEffect::Device(DeviceMsg::RequestPreset(slot))]
            }
            AppMsg::Device(msg) => match msg {
                DeviceStatus::PresetData(preset) => {
                    let slot = preset.global.preset_slot;
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DeviceStatusDeserializationError;
    use crate::error::SysexDeserializationError;
    use crate::manufacturer::akai::mpd226::control::Global;

    fn preset_with_slot(slot: PresetSlot) -> Preset {
        Preset {
            global: Global {
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

        assert_eq!(app.preset.global.preset_slot, PresetSlot::Slot3);
        let effect = effects.into_iter().next().unwrap();
        assert!(matches!(
            effect,
            AppEffect::Device(DeviceMsg::SendPreset(p)) if p.global.preset_slot == PresetSlot::Slot3
        ));
    }

    #[test]
    fn request_preset_from_device() {
        let mut app = AppState::default();
        let original_slot = app.preset.global.preset_slot;

        let effects = app.update(AppMsg::Ui(UiEffect::RequestPresetFromDevice(
            PresetSlot::Slot4,
        )));

        assert_eq!(app.preset.global.preset_slot, original_slot);
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

        assert_eq!(app.preset.global.preset_slot, PresetSlot::Slot1);
        assert_eq!(effects.len(), 2);
        assert!(matches!(
            &effects[0],
            AppEffect::Ui(UiMsg::UpdatePreset(p)) if p.global.preset_slot == PresetSlot::Slot1
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
        let original_slot = app.preset.global.preset_slot;

        let effects = app.update(AppMsg::Device(DeviceStatus::ReceivedPresetAck(
            PresetSlot::Slot7,
        )));

        assert_eq!(app.preset.global.preset_slot, original_slot);
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
        let original_slot = app.preset.global.preset_slot;

        let effects = app.update(AppMsg::MidiError(MidiError::ResponseTimeout));

        assert_eq!(app.preset.global.preset_slot, original_slot);
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
        let original_slot = app.preset.global.preset_slot;

        let effects = app.update(AppMsg::SysexParseError(
            SysexDeserializationError::InvalidStart(0x00),
        ));

        assert_eq!(app.preset.global.preset_slot, original_slot);
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
        let original_slot = app.preset.global.preset_slot;

        let effects = app.update(AppMsg::DeviceStatusParseError(
            DeviceStatusDeserializationError::InvalidCommand(0xFF),
        ));

        assert_eq!(app.preset.global.preset_slot, original_slot);
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
