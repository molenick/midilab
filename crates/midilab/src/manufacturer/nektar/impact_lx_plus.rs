//! Nektar Impact LX+ series (LX25+/49+/61+/88+) sysex protocol.
//!
//! Nektar publishes no sysex documentation; this protocol was reverse
//! engineered and HIL-verified on an Impact LX61+ (2026-07-16).
//!
//! Every message, both directions, is framed as
//! `F0 00 01 77 7F 01 <type> <section> <id> <TLV...> <checksum> F7` where the
//! TLV block is a sequence of `[param, 0x01, value]` groups separated by
//! `0x00`.
//!
//! Reading device memory is panel-triggered only ([Setup] → *Memory Dump*
//! key): no dump-request sysex is known. One dump is exactly
//! [`DUMP_MESSAGE_COUNT`] messages, assembled with [`DumpAssembler`].
//!
//! Writes are silent (no ack) and are accepted one message at a time. Writes
//! to the global block apply to the live device state instantly; writes to
//! presets and pad maps only update stored memory and take effect the next
//! time that preset or pad map is loaded from the panel.

use std::collections::HashMap;

use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::manufacturer::nektar::SYSEX_MANUFACTURER_ID;
use crate::manufacturer::nektar::impact_lx_plus::control::Button;
use crate::manufacturer::nektar::impact_lx_plus::control::Continuous;
use crate::manufacturer::nektar::impact_lx_plus::control::GlobalControlId;
use crate::manufacturer::nektar::impact_lx_plus::control::Pad;
use crate::manufacturer::nektar::impact_lx_plus::control::PadId;
use crate::manufacturer::nektar::impact_lx_plus::control::PadMapId;
use crate::manufacturer::nektar::impact_lx_plus::control::PresetControlId;
use crate::manufacturer::nektar::impact_lx_plus::control::PresetId;
use crate::manufacturer::nektar::impact_lx_plus::error::DeviceStatusParseError;
use crate::manufacturer::nektar::impact_lx_plus::error::DumpParseError;
use crate::manufacturer::nektar::impact_lx_plus::raw::RawControl;
use crate::manufacturer::nektar::impact_lx_plus::raw::RawDump;
use crate::manufacturer::nektar::impact_lx_plus::raw::RawGlobalControls;
use crate::manufacturer::nektar::impact_lx_plus::raw::RawGlobalSettings;
use crate::manufacturer::nektar::impact_lx_plus::raw::RawPad;
use crate::manufacturer::nektar::impact_lx_plus::raw::RawPadMap;
use crate::manufacturer::nektar::impact_lx_plus::raw::RawPreset;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::ButtonKind;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::ContinuousKind;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::ControlChannel;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::MidiChannel;
use crate::midi::Value;
use crate::sysex::SysEx;
use crate::sysex::sysex;

pub mod control;
pub mod error;
pub mod raw;
pub mod value_kind;

/// Sysex I/O port of the LX61+ model. Other LX+ models substitute their key
/// count. The `MIDI2` port is the DAW-integration port and carries no sysex.
pub const PORT_NAME: &str = "Impact LX61+ MIDI1";

pub const TOTAL_FADERS: usize = 9;
pub const TOTAL_POTS: usize = 8;
pub const TOTAL_FADER_BUTTONS: usize = 9;
pub const TOTAL_PRESET_CONTROLS: usize = TOTAL_FADERS + TOTAL_POTS + TOTAL_FADER_BUTTONS;
pub const TOTAL_PADS: usize = 8;
pub const TOTAL_PRESETS: usize = 5;
pub const TOTAL_PAD_MAPS: usize = 4;
pub const TOTAL_SETTINGS: usize = 11;
pub const TOTAL_TRANSPORT_BUTTONS: usize = 6;
pub const TOTAL_GLOBAL_CONTROLS: usize = TOTAL_TRANSPORT_BUTTONS + 3;

/// Number of messages in one full memory dump.
pub const DUMP_MESSAGE_COUNT: usize = TOTAL_PRESETS * TOTAL_PRESET_CONTROLS
    + TOTAL_PAD_MAPS * TOTAL_PADS
    + TOTAL_SETTINGS
    + TOTAL_GLOBAL_CONTROLS;

pub const SYSEX_COMMAND_HEADER: [u8; 5] = [
    SYSEX_MANUFACTURER_ID[0],
    SYSEX_MANUFACTURER_ID[1],
    SYSEX_MANUFACTURER_ID[2],
    0x7F,
    0x01,
];

/// Returns whether a CoreMIDI port name is the sysex port of an Impact LX+
/// device (any model), as opposed to its DAW-integration `MIDI2` port.
pub fn is_sysex_port(name: &str) -> bool {
    name.contains("Impact LX") && name.contains("MIDI1")
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
pub enum ObjectType {
    PresetControl = 0x01,
    Pad = 0x02,
    Global = 0x05,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive, EnumIter)]
pub enum ParamId {
    Channel = 0x01,
    Kind = 0x02,
    Data1 = 0x03,
    Min = 0x04,
    Max = 0x05,
    Note = 0x06,
}

pub const CONTROL_PARAMS: [ParamId; 5] = [
    ParamId::Channel,
    ParamId::Kind,
    ParamId::Data1,
    ParamId::Min,
    ParamId::Max,
];

pub const PAD_PARAMS: [ParamId; 6] = [
    ParamId::Channel,
    ParamId::Kind,
    ParamId::Data1,
    ParamId::Min,
    ParamId::Max,
    ParamId::Note,
];

/// Global setting ids (`type 05`, message id `01`). Declaration order is the
/// dump order (`0x0F` is dumped last, after `0x12`).
///
/// Only `MidiChannel` is mapped; the `Unknown*` settings round-trip as opaque
/// values. Candidates from the Setup menu: velocity curves, program, bank
/// LSB/MSB, USB port setup.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive, EnumIter)]
pub enum GlobalSettingId {
    MidiChannel = 0x01,
    Unknown04 = 0x04,
    Unknown05 = 0x05,
    Unknown06 = 0x06,
    Unknown07 = 0x07,
    Unknown08 = 0x08,
    Unknown09 = 0x09,
    Unknown10 = 0x10,
    Unknown11 = 0x11,
    Unknown12 = 0x12,
    Unknown0F = 0x0F,
}

/// 7-bit two's complement checksum over the object-type byte through the last
/// TLV byte.
pub fn checksum(body: &[u8]) -> u8 {
    let sum = body.iter().fold(0u8, |acc, b| acc.wrapping_add(*b));
    !sum & 0x7F
}

fn command_message(object: ObjectType, section: u8, id: u8, params: &[(u8, u8)]) -> SysEx {
    let mut body = vec![object.into(), section, id];
    for (index, (param, value)) in params.iter().enumerate() {
        if index > 0 {
            body.push(0x00);
        }
        body.extend_from_slice(&[*param, 0x01, *value]);
    }
    body.push(checksum(&body));

    let mut payload = SYSEX_COMMAND_HEADER.to_vec();
    payload.extend_from_slice(&body);
    sysex(payload)
}

fn control_params(value: &RawControl) -> Vec<(u8, u8)> {
    CONTROL_PARAMS
        .into_iter()
        .map(u8::from)
        .zip(value.as_bytes())
        .collect()
}

pub fn preset_control_message(
    preset: PresetId,
    control: PresetControlId,
    value: &RawControl,
) -> SysEx {
    command_message(
        ObjectType::PresetControl,
        preset.into(),
        control.into(),
        &control_params(value),
    )
}

pub fn pad_message(map: PadMapId, pad: PadId, value: &RawPad) -> SysEx {
    let params: Vec<(u8, u8)> = PAD_PARAMS
        .into_iter()
        .map(u8::from)
        .zip(value.as_bytes())
        .collect();
    command_message(ObjectType::Pad, map.into(), pad.into(), &params)
}

/// Global settings are single-TLV messages whose param id is the setting id
/// itself (message id is always `0x01`).
pub fn global_setting_message(setting: GlobalSettingId, value: u8) -> SysEx {
    command_message(ObjectType::Global, 0x00, 0x01, &[(setting.into(), value)])
}

pub fn global_control_message(control: GlobalControlId, value: &RawControl) -> SysEx {
    command_message(
        ObjectType::Global,
        0x00,
        control.into(),
        &control_params(value),
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceStatus {
    PresetControl {
        preset: PresetId,
        control: PresetControlId,
        value: RawControl,
    },
    Pad {
        map: PadMapId,
        pad: PadId,
        value: RawPad,
    },
    GlobalSetting {
        setting: GlobalSettingId,
        value: u8,
    },
    GlobalControl {
        control: GlobalControlId,
        value: RawControl,
    },
}

impl DeviceStatus {
    pub fn message(&self) -> SysEx {
        match self {
            DeviceStatus::PresetControl {
                preset,
                control,
                value,
            } => preset_control_message(*preset, *control, value),
            DeviceStatus::Pad { map, pad, value } => pad_message(*map, *pad, value),
            DeviceStatus::GlobalSetting { setting, value } => {
                global_setting_message(*setting, *value)
            }
            DeviceStatus::GlobalControl { control, value } => {
                global_control_message(*control, value)
            }
        }
    }
}

fn parse_tlv(mut tlv: &[u8]) -> Result<Vec<(u8, u8)>, DeviceStatusParseError> {
    let mut params = Vec::new();
    while !tlv.is_empty() {
        let [param, len, rest @ ..] = tlv else {
            return Err(DeviceStatusParseError::InvalidTlv);
        };
        if *len != 0x01 {
            return Err(DeviceStatusParseError::InvalidParamLength(*len));
        }
        let [value, rest @ ..] = rest else {
            return Err(DeviceStatusParseError::InvalidTlv);
        };
        params.push((*param, *value));
        tlv = match rest {
            [0x00, rest @ ..] => rest,
            rest => rest,
        };
    }
    Ok(params)
}

fn require(params: &[(u8, u8)], param: ParamId) -> Result<u8, DeviceStatusParseError> {
    let id: u8 = param.into();
    params
        .iter()
        .find(|(p, _)| *p == id)
        .map(|(_, v)| *v)
        .ok_or(DeviceStatusParseError::MissingParam(id))
}

impl RawControl {
    fn try_from_params(params: &[(u8, u8)]) -> Result<Self, DeviceStatusParseError> {
        Ok(RawControl {
            channel: require(params, ParamId::Channel)?,
            kind: require(params, ParamId::Kind)?,
            data1: require(params, ParamId::Data1)?,
            min: require(params, ParamId::Min)?,
            max: require(params, ParamId::Max)?,
        })
    }
}

impl RawPad {
    fn try_from_params(params: &[(u8, u8)]) -> Result<Self, DeviceStatusParseError> {
        Ok(RawPad {
            channel: require(params, ParamId::Channel)?,
            kind: require(params, ParamId::Kind)?,
            data1: require(params, ParamId::Data1)?,
            min: require(params, ParamId::Min)?,
            max: require(params, ParamId::Max)?,
            note: require(params, ParamId::Note)?,
        })
    }
}

impl TryFrom<&[u8]> for DeviceStatus {
    type Error = DeviceStatusParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let sysex = SysEx::try_from(value)?;
        DeviceStatus::try_from(sysex)
    }
}

impl TryFrom<SysEx> for DeviceStatus {
    type Error = DeviceStatusParseError;

    fn try_from(value: SysEx) -> Result<Self, Self::Error> {
        let payload = value.bytes();

        if !payload.starts_with(&SYSEX_COMMAND_HEADER) {
            return Err(DeviceStatusParseError::InvalidHeader);
        }

        // header(5) + type/section/id(3) + at least one TLV group(3) + checksum(1)
        if payload.len() < SYSEX_COMMAND_HEADER.len() + 7 {
            return Err(DeviceStatusParseError::InvalidLength(payload.len()));
        }

        let body = &payload[SYSEX_COMMAND_HEADER.len()..payload.len() - 1];
        let actual = payload[payload.len() - 1];
        let expected = checksum(body);
        if actual != expected {
            return Err(DeviceStatusParseError::ChecksumMismatch { expected, actual });
        }

        let [object, section, id, tlv @ ..] = body else {
            return Err(DeviceStatusParseError::InvalidLength(payload.len()));
        };
        let object = ObjectType::try_from(*object)
            .map_err(|_| DeviceStatusParseError::InvalidObjectType(*object))?;
        let params = parse_tlv(tlv)?;

        match object {
            ObjectType::PresetControl => Ok(DeviceStatus::PresetControl {
                preset: PresetId::try_from(*section)
                    .map_err(|_| DeviceStatusParseError::InvalidPreset(*section))?,
                control: PresetControlId::try_from(*id)
                    .map_err(|_| DeviceStatusParseError::InvalidPresetControlId(*id))?,
                value: RawControl::try_from_params(&params)?,
            }),
            ObjectType::Pad => Ok(DeviceStatus::Pad {
                map: PadMapId::try_from(*section)
                    .map_err(|_| DeviceStatusParseError::InvalidPadMap(*section))?,
                pad: PadId::try_from(*id).map_err(|_| DeviceStatusParseError::InvalidPadId(*id))?,
                value: RawPad::try_from_params(&params)?,
            }),
            ObjectType::Global => {
                if *section != 0x00 {
                    return Err(DeviceStatusParseError::InvalidSection(*section));
                }
                if *id == 0x01 {
                    let [(setting, value)] = params.as_slice() else {
                        return Err(DeviceStatusParseError::InvalidTlv);
                    };
                    return Ok(DeviceStatus::GlobalSetting {
                        setting: GlobalSettingId::try_from(*setting)
                            .map_err(|_| DeviceStatusParseError::InvalidSettingId(*setting))?,
                        value: *value,
                    });
                }
                Ok(DeviceStatus::GlobalControl {
                    control: GlobalControlId::try_from(*id)
                        .map_err(|_| DeviceStatusParseError::InvalidGlobalControlId(*id))?,
                    value: RawControl::try_from_params(&params)?,
                })
            }
        }
    }
}

/// Accumulates dump messages (in any order) until a full [`Dump`] can be
/// assembled.
#[derive(Debug, Clone, Default)]
pub struct DumpAssembler {
    preset_controls: HashMap<(u8, u8), RawControl>,
    pads: HashMap<(u8, u8), RawPad>,
    settings: HashMap<u8, u8>,
    global_controls: HashMap<u8, RawControl>,
}

impl DumpAssembler {
    pub fn apply(&mut self, status: &DeviceStatus) {
        match status {
            DeviceStatus::PresetControl {
                preset,
                control,
                value,
            } => {
                self.preset_controls
                    .insert(((*preset).into(), (*control).into()), *value);
            }
            DeviceStatus::Pad { map, pad, value } => {
                self.pads.insert(((*map).into(), (*pad).into()), *value);
            }
            DeviceStatus::GlobalSetting { setting, value } => {
                self.settings.insert((*setting).into(), *value);
            }
            DeviceStatus::GlobalControl { control, value } => {
                self.global_controls.insert((*control).into(), *value);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.preset_controls.len()
            + self.pads.len()
            + self.settings.len()
            + self.global_controls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_complete(&self) -> bool {
        self.preset_controls.len() == TOTAL_PRESETS * TOTAL_PRESET_CONTROLS
            && self.pads.len() == TOTAL_PAD_MAPS * TOTAL_PADS
            && self.settings.len() == TOTAL_SETTINGS
            && self.global_controls.len() == TOTAL_GLOBAL_CONTROLS
    }

    pub fn try_into_raw_dump(&self) -> Result<RawDump, DumpParseError> {
        let mut raw = RawDump::default();

        for (preset_index, preset) in PresetId::iter().enumerate() {
            let slot = &mut raw.presets[preset_index];
            let all = PresetControlId::FADERS
                .iter()
                .chain(PresetControlId::POTS.iter())
                .chain(PresetControlId::FADER_BUTTONS.iter());
            for (control_index, control) in all.enumerate() {
                let value = self
                    .preset_controls
                    .get(&(preset.into(), (*control).into()))
                    .copied()
                    .ok_or(DumpParseError::MissingPresetControl {
                        preset: preset.into(),
                        control: (*control).into(),
                    })?;
                if control_index < TOTAL_FADERS {
                    slot.faders[control_index] = value;
                } else if control_index < TOTAL_FADERS + TOTAL_POTS {
                    slot.pots[control_index - TOTAL_FADERS] = value;
                } else {
                    slot.fader_buttons[control_index - TOTAL_FADERS - TOTAL_POTS] = value;
                }
            }
        }

        for (map_index, map) in PadMapId::iter().enumerate() {
            for (pad_index, pad) in PadId::ALL.iter().enumerate() {
                raw.pad_maps[map_index].pads[pad_index] =
                    self.pads.get(&(map.into(), (*pad).into())).copied().ok_or(
                        DumpParseError::MissingPad {
                            map: map.into(),
                            pad: (*pad).into(),
                        },
                    )?;
            }
        }

        for (setting_index, setting) in GlobalSettingId::iter().enumerate() {
            raw.settings.values[setting_index] =
                self.settings.get(&setting.into()).copied().ok_or(
                    DumpParseError::MissingSetting {
                        setting: setting.into(),
                    },
                )?;
        }

        for (control_index, control) in GlobalControlId::iter().enumerate() {
            let value = self.global_controls.get(&control.into()).copied().ok_or(
                DumpParseError::MissingGlobalControl {
                    control: control.into(),
                },
            )?;
            match control {
                GlobalControlId::PitchWheel => raw.controls.pitch_wheel = value,
                GlobalControlId::ModWheel => raw.controls.mod_wheel = value,
                GlobalControlId::FootSwitch => raw.controls.foot_switch = value,
                _ => raw.controls.transport[control_index] = value,
            }
        }

        Ok(raw)
    }

    pub fn try_into_dump(&self) -> Result<Dump, DumpParseError> {
        Dump::try_from(&self.try_into_raw_dump()?)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    pub faders: [Continuous; TOTAL_FADERS],
    pub pots: [Continuous; TOTAL_POTS],
    pub fader_buttons: [Button; TOTAL_FADER_BUTTONS],
}

impl Preset {
    const GM_FADER_CC: [u8; TOTAL_FADERS] = [73, 75, 72, 91, 92, 93, 94, 95, 7];
    const GM_POT_CC: [u8; TOTAL_POTS] = [74, 71, 5, 84, 78, 76, 77, 10];
    const GM_BUTTON_CC: [u8; TOTAL_FADER_BUTTONS] = [0, 2, 3, 4, 6, 8, 9, 11, 65];
    const CC_FADER_CC: [u8; TOTAL_FADERS] = [80, 81, 82, 83, 85, 86, 87, 88, 3];
    const CC_POT_CC: [u8; TOTAL_POTS] = [89, 90, 96, 97, 116, 117, 118, 119];
    const CC_BUTTON_CC: [u8; TOTAL_FADER_BUTTONS] = [66, 67, 68, 69, 98, 99, 100, 101, 65];

    /// The factory content of a preset slot, as shipped:
    /// 1 = GM instrument CCs, 2/3 = mixer pages on channels 1–8/9–16,
    /// 4/5 = a generic CC set with toggle/momentary buttons.
    pub fn factory(id: PresetId) -> Self {
        match id {
            PresetId::Preset1 => {
                Self::with_ccs(Self::GM_FADER_CC, Self::GM_POT_CC, Self::GM_BUTTON_CC)
            }
            PresetId::Preset2 => Self::mixer(1),
            PresetId::Preset3 => Self::mixer(9),
            PresetId::Preset4 => {
                Self::with_ccs(Self::CC_FADER_CC, Self::CC_POT_CC, Self::CC_BUTTON_CC)
            }
            PresetId::Preset5 => {
                let mut preset =
                    Self::with_ccs(Self::CC_FADER_CC, Self::CC_POT_CC, Self::CC_BUTTON_CC);
                for button in preset.fader_buttons.iter_mut() {
                    button.kind = ButtonKind::CcMomentary;
                }
                preset
            }
        }
    }

    fn with_ccs(
        fader_cc: [u8; TOTAL_FADERS],
        pot_cc: [u8; TOTAL_POTS],
        button_cc: [u8; TOTAL_FADER_BUTTONS],
    ) -> Self {
        Self {
            faders: fader_cc.map(Continuous::with_cc),
            pots: pot_cc.map(Continuous::with_cc),
            fader_buttons: button_cc.map(Button::with_cc),
        }
    }

    fn mixer(first_channel: u8) -> Self {
        let channel = |offset: usize| {
            ControlChannel::try_from(first_channel + offset as u8)
                .expect("mixer channels stay within 1-16")
        };

        let mut preset = Self::with_ccs(
            [7; TOTAL_FADERS],
            [10; TOTAL_POTS],
            [12, 12, 12, 12, 12, 12, 12, 12, 65],
        );
        for (index, fader) in preset.faders.iter_mut().take(8).enumerate() {
            fader.channel = channel(index);
        }
        for (index, pot) in preset.pots.iter_mut().enumerate() {
            pot.channel = channel(index);
        }
        for (index, button) in preset.fader_buttons.iter_mut().take(8).enumerate() {
            button.channel = channel(index);
        }
        preset
    }

    /// The 26 stored-memory write messages for this preset, in dump order.
    ///
    /// The device applies them silently to stored memory; the live state
    /// updates the next time the preset is loaded from the panel.
    pub fn send_messages(&self, preset: PresetId) -> Vec<SysEx> {
        let raw = RawPreset::from(self);
        let mut messages = Vec::with_capacity(TOTAL_PRESET_CONTROLS);
        for (id, value) in PresetControlId::FADERS.iter().zip(raw.faders.iter()) {
            messages.push(preset_control_message(preset, *id, value));
        }
        for (id, value) in PresetControlId::POTS.iter().zip(raw.pots.iter()) {
            messages.push(preset_control_message(preset, *id, value));
        }
        for (id, value) in PresetControlId::FADER_BUTTONS
            .iter()
            .zip(raw.fader_buttons.iter())
        {
            messages.push(preset_control_message(preset, *id, value));
        }
        messages
    }
}

impl Default for Preset {
    fn default() -> Self {
        Self::factory(PresetId::Preset1)
    }
}

impl From<&Preset> for RawPreset {
    fn from(preset: &Preset) -> Self {
        let mut raw = RawPreset::default();
        for (slot, fader) in raw.faders.iter_mut().zip(preset.faders.iter()) {
            *slot = RawControl::from(fader);
        }
        for (slot, pot) in raw.pots.iter_mut().zip(preset.pots.iter()) {
            *slot = RawControl::from(pot);
        }
        for (slot, button) in raw
            .fader_buttons
            .iter_mut()
            .zip(preset.fader_buttons.iter())
        {
            *slot = RawControl::from(button);
        }
        raw
    }
}

impl TryFrom<(PresetId, &RawPreset)> for Preset {
    type Error = DumpParseError;

    fn try_from((id, raw): (PresetId, &RawPreset)) -> Result<Self, Self::Error> {
        let context = |control: PresetControlId| {
            move |source| DumpParseError::PresetControl {
                preset: id.into(),
                control: control.into(),
                source,
            }
        };

        let mut preset = Preset::default();
        for (index, value) in raw.faders.iter().enumerate() {
            preset.faders[index] =
                Continuous::try_from(*value).map_err(context(PresetControlId::FADERS[index]))?;
        }
        for (index, value) in raw.pots.iter().enumerate() {
            preset.pots[index] =
                Continuous::try_from(*value).map_err(context(PresetControlId::POTS[index]))?;
        }
        for (index, value) in raw.fader_buttons.iter().enumerate() {
            preset.fader_buttons[index] =
                Button::try_from(*value).map_err(context(PresetControlId::FADER_BUTTONS[index]))?;
        }
        Ok(preset)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadMap {
    pub pads: [Pad; TOTAL_PADS],
}

impl PadMap {
    const FACTORY_NOTES: [[u8; TOTAL_PADS]; TOTAL_PAD_MAPS] = [
        [36, 37, 38, 39, 40, 41, 42, 43],
        [44, 45, 46, 47, 48, 49, 50, 51],
        [60, 62, 64, 65, 67, 69, 71, 72],
        [36, 38, 42, 46, 43, 45, 37, 49],
    ];

    /// The factory content of a pad map: 1 = notes 36–43, 2 = 44–51,
    /// 3 = white keys 60–72, 4 = a GM drum layout.
    pub fn factory(id: PadMapId) -> Self {
        let notes = Self::FACTORY_NOTES[id as usize - 1];
        Self {
            pads: notes.map(Pad::with_note),
        }
    }

    /// The 8 stored-memory write messages for this pad map, in dump order.
    pub fn send_messages(&self, map: PadMapId) -> Vec<SysEx> {
        let raw = RawPadMap::from(self);
        PadId::ALL
            .iter()
            .zip(raw.pads.iter())
            .map(|(id, value)| pad_message(map, *id, value))
            .collect()
    }
}

impl Default for PadMap {
    fn default() -> Self {
        Self::factory(PadMapId::Map1)
    }
}

impl From<&PadMap> for RawPadMap {
    fn from(map: &PadMap) -> Self {
        let mut raw = RawPadMap::default();
        for (slot, pad) in raw.pads.iter_mut().zip(map.pads.iter()) {
            *slot = RawPad::from(pad);
        }
        raw
    }
}

impl TryFrom<(PadMapId, &RawPadMap)> for PadMap {
    type Error = DumpParseError;

    fn try_from((id, raw): (PadMapId, &RawPadMap)) -> Result<Self, Self::Error> {
        let mut map = PadMap::default();
        for (index, value) in raw.pads.iter().enumerate() {
            map.pads[index] = Pad::try_from(*value).map_err(|source| DumpParseError::Pad {
                map: id.into(),
                pad: PadId::ALL[index].into(),
                source,
            })?;
        }
        Ok(map)
    }
}

/// The global settings block (`type 05`, message id `01`).
///
/// Only the global MIDI channel is mapped; the other ten settings round-trip
/// as opaque values, ordered as in [`GlobalSettingId`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobalSettings {
    pub midi_channel: MidiChannel,
    pub unknown: [Value; TOTAL_SETTINGS - 1],
}

impl GlobalSettings {
    /// Writes to global settings apply to the live device state instantly.
    pub fn send_messages(&self) -> Vec<SysEx> {
        let raw = RawGlobalSettings::from(self);
        GlobalSettingId::iter()
            .zip(raw.values)
            .map(|(setting, value)| global_setting_message(setting, value))
            .collect()
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            midi_channel: MidiChannel::Ch1,
            unknown: [0, 0, 0, 1, 1, 0, 1, 0, 0, 0].map(Value::from),
        }
    }
}

impl From<&GlobalSettings> for RawGlobalSettings {
    fn from(settings: &GlobalSettings) -> Self {
        let mut values = [0u8; TOTAL_SETTINGS];
        values[0] = settings.midi_channel.into();
        for (slot, value) in values[1..].iter_mut().zip(settings.unknown.iter()) {
            *slot = (*value).into();
        }
        RawGlobalSettings { values }
    }
}

impl TryFrom<&RawGlobalSettings> for GlobalSettings {
    type Error = DumpParseError;

    fn try_from(raw: &RawGlobalSettings) -> Result<Self, Self::Error> {
        let mut unknown = [Value::default(); TOTAL_SETTINGS - 1];
        for (slot, value) in unknown.iter_mut().zip(raw.values[1..].iter()) {
            *slot = (*value).into();
        }
        Ok(GlobalSettings {
            midi_channel: MidiChannel::try_from(raw.values[0])
                .map_err(DumpParseError::GlobalMidiChannel)?,
            unknown,
        })
    }
}

/// The global controls block: wheels, foot switch and transport buttons.
/// These are not part of presets and survive preset switching.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobalControls {
    pub transport: [Button; TOTAL_TRANSPORT_BUTTONS],
    pub pitch_wheel: Continuous,
    pub mod_wheel: Continuous,
    pub foot_switch: Button,
}

impl GlobalControls {
    /// Writes to global controls apply to the live device state instantly.
    pub fn send_messages(&self) -> Vec<SysEx> {
        let raw = RawGlobalControls::from(self);
        let mut messages = Vec::with_capacity(TOTAL_GLOBAL_CONTROLS);
        for (id, value) in GlobalControlId::TRANSPORT.iter().zip(raw.transport.iter()) {
            messages.push(global_control_message(*id, value));
        }
        messages.push(global_control_message(
            GlobalControlId::PitchWheel,
            &raw.pitch_wheel,
        ));
        messages.push(global_control_message(
            GlobalControlId::ModWheel,
            &raw.mod_wheel,
        ));
        messages.push(global_control_message(
            GlobalControlId::FootSwitch,
            &raw.foot_switch,
        ));
        messages
    }
}

impl Default for GlobalControls {
    fn default() -> Self {
        let mut transport = [Button::default(); TOTAL_TRANSPORT_BUTTONS];
        for (index, button) in transport.iter_mut().enumerate() {
            *button = Button {
                channel: ControlChannel::Ch16,
                kind: ButtonKind::CcMomentary,
                data1: (102 + index as u8).into(),
                ..Button::default()
            };
        }

        Self {
            transport,
            pitch_wheel: Continuous {
                kind: ContinuousKind::PitchBend,
                ..Continuous::default()
            },
            mod_wheel: Continuous::with_cc(1),
            foot_switch: Button {
                kind: ButtonKind::CcMomentary,
                data1: 64.into(),
                ..Button::default()
            },
        }
    }
}

impl From<&GlobalControls> for RawGlobalControls {
    fn from(controls: &GlobalControls) -> Self {
        let mut raw = RawGlobalControls::default();
        for (slot, button) in raw.transport.iter_mut().zip(controls.transport.iter()) {
            *slot = RawControl::from(button);
        }
        raw.pitch_wheel = RawControl::from(&controls.pitch_wheel);
        raw.mod_wheel = RawControl::from(&controls.mod_wheel);
        raw.foot_switch = RawControl::from(&controls.foot_switch);
        raw
    }
}

impl TryFrom<&RawGlobalControls> for GlobalControls {
    type Error = DumpParseError;

    fn try_from(raw: &RawGlobalControls) -> Result<Self, Self::Error> {
        let context = |control: GlobalControlId| {
            move |source| DumpParseError::GlobalControl {
                control: control.into(),
                source,
            }
        };

        let mut controls = GlobalControls::default();
        for (index, value) in raw.transport.iter().enumerate() {
            controls.transport[index] =
                Button::try_from(*value).map_err(context(GlobalControlId::TRANSPORT[index]))?;
        }
        controls.pitch_wheel =
            Continuous::try_from(raw.pitch_wheel).map_err(context(GlobalControlId::PitchWheel))?;
        controls.mod_wheel =
            Continuous::try_from(raw.mod_wheel).map_err(context(GlobalControlId::ModWheel))?;
        controls.foot_switch =
            Button::try_from(raw.foot_switch).map_err(context(GlobalControlId::FootSwitch))?;
        Ok(controls)
    }
}

/// The full device memory: 5 presets, 4 pad maps, global settings and global
/// controls. [`Default`] is the factory state, HIL-verified byte-perfect
/// against an unmodified LX61+.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dump {
    pub presets: [Preset; TOTAL_PRESETS],
    pub pad_maps: [PadMap; TOTAL_PAD_MAPS],
    pub settings: GlobalSettings,
    pub controls: GlobalControls,
}

impl Dump {
    /// All 182 write messages in canonical dump order. Replaying them to the
    /// device restores its memory byte-perfectly.
    pub fn to_messages(&self) -> Vec<SysEx> {
        let mut messages = Vec::with_capacity(DUMP_MESSAGE_COUNT);
        for (id, preset) in PresetId::iter().zip(self.presets.iter()) {
            messages.extend(preset.send_messages(id));
        }
        for (id, map) in PadMapId::iter().zip(self.pad_maps.iter()) {
            messages.extend(map.send_messages(id));
        }
        messages.extend(self.settings.send_messages());
        messages.extend(self.controls.send_messages());
        messages
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let raw = RawDump::from(self);
        bytemuck::bytes_of(&raw).to_vec()
    }

    pub fn default_filename(&self) -> String {
        "nektar_impact_lx_plus.dump".to_string()
    }
}

impl Default for Dump {
    fn default() -> Self {
        let mut presets = [Preset::default(); TOTAL_PRESETS];
        for (slot, id) in presets.iter_mut().zip(PresetId::iter()) {
            *slot = Preset::factory(id);
        }
        let mut pad_maps = [PadMap::default(); TOTAL_PAD_MAPS];
        for (slot, id) in pad_maps.iter_mut().zip(PadMapId::iter()) {
            *slot = PadMap::factory(id);
        }
        Self {
            presets,
            pad_maps,
            settings: GlobalSettings::default(),
            controls: GlobalControls::default(),
        }
    }
}

impl From<&Dump> for RawDump {
    fn from(dump: &Dump) -> Self {
        let mut raw = RawDump::default();
        for (slot, preset) in raw.presets.iter_mut().zip(dump.presets.iter()) {
            *slot = RawPreset::from(preset);
        }
        for (slot, map) in raw.pad_maps.iter_mut().zip(dump.pad_maps.iter()) {
            *slot = RawPadMap::from(map);
        }
        raw.settings = RawGlobalSettings::from(&dump.settings);
        raw.controls = RawGlobalControls::from(&dump.controls);
        raw
    }
}

impl TryFrom<&RawDump> for Dump {
    type Error = DumpParseError;

    fn try_from(raw: &RawDump) -> Result<Self, Self::Error> {
        let mut dump = Dump::default();
        for ((slot, id), value) in dump
            .presets
            .iter_mut()
            .zip(PresetId::iter())
            .zip(raw.presets.iter())
        {
            *slot = Preset::try_from((id, value))?;
        }
        for ((slot, id), value) in dump
            .pad_maps
            .iter_mut()
            .zip(PadMapId::iter())
            .zip(raw.pad_maps.iter())
        {
            *slot = PadMap::try_from((id, value))?;
        }
        dump.settings = GlobalSettings::try_from(&raw.settings)?;
        dump.controls = GlobalControls::try_from(&raw.controls)?;
        Ok(dump)
    }
}

impl TryFrom<&[u8]> for Dump {
    type Error = DumpParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let raw: RawDump = *bytemuck::try_from_bytes(value)
            .map_err(|_| DumpParseError::InvalidLength(value.len()))?;
        Dump::try_from(&raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Captured from an unmodified LX61+ (factory preset 1, fader 1 = CC73).
    const FACTORY_FADER1: [u8; 30] = [
        0xF0, 0x00, 0x01, 0x77, 0x7F, 0x01, 0x01, 0x01, 0x02, 0x01, 0x01, 0x00, 0x00, 0x02, 0x01,
        0x00, 0x00, 0x03, 0x01, 0x49, 0x00, 0x04, 0x01, 0x00, 0x00, 0x05, 0x01, 0x7F, 0x1F, 0xF7,
    ];

    // Captured global setting message: global MIDI channel = 1.
    const FACTORY_MIDI_CHANNEL: [u8; 14] = [
        0xF0, 0x00, 0x01, 0x77, 0x7F, 0x01, 0x05, 0x00, 0x01, 0x01, 0x01, 0x01, 0x76, 0xF7,
    ];

    // Captured global control message: mod wheel = CC1, follow global channel.
    const FACTORY_MOD_WHEEL: [u8; 30] = [
        0xF0, 0x00, 0x01, 0x77, 0x7F, 0x01, 0x05, 0x00, 0x09, 0x01, 0x01, 0x00, 0x00, 0x02, 0x01,
        0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x04, 0x01, 0x00, 0x00, 0x05, 0x01, 0x7F, 0x5D, 0xF7,
    ];

    // Captured pad message: pad map 1, pad 1 = note 36.
    const FACTORY_PAD1: [u8; 34] = [
        0xF0, 0x00, 0x01, 0x77, 0x7F, 0x01, 0x02, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00, 0x02, 0x01,
        0x02, 0x00, 0x03, 0x01, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x05, 0x01, 0x7F, 0x00, 0x06,
        0x01, 0x24, 0x39, 0xF7,
    ];

    #[test]
    fn test_preset_control_message_format() {
        let value = RawControl {
            channel: 0,
            kind: 0,
            data1: 73,
            min: 0,
            max: 127,
        };
        let msg = preset_control_message(PresetId::Preset1, PresetControlId::Fader1, &value)
            .to_wire_bytes();
        assert_eq!(msg, FACTORY_FADER1);
    }

    #[test]
    fn test_global_setting_message_format() {
        let msg = global_setting_message(GlobalSettingId::MidiChannel, 0x01).to_wire_bytes();
        assert_eq!(msg, FACTORY_MIDI_CHANNEL);
    }

    #[test]
    fn test_global_control_message_format() {
        let value = RawControl {
            channel: 0,
            kind: 0,
            data1: 1,
            min: 0,
            max: 127,
        };
        let msg = global_control_message(GlobalControlId::ModWheel, &value).to_wire_bytes();
        assert_eq!(msg, FACTORY_MOD_WHEEL);
    }

    #[test]
    fn test_pad_message_format() {
        let value = RawPad {
            channel: 0,
            kind: 2,
            data1: 0,
            min: 0,
            max: 127,
            note: 36,
        };
        let msg = pad_message(PadMapId::Map1, PadId::Pad1, &value).to_wire_bytes();
        assert_eq!(msg, FACTORY_PAD1);
    }

    #[test]
    fn test_device_status_preset_control_parse() {
        let status = DeviceStatus::try_from(FACTORY_FADER1.as_slice()).unwrap();
        assert_eq!(
            status,
            DeviceStatus::PresetControl {
                preset: PresetId::Preset1,
                control: PresetControlId::Fader1,
                value: RawControl {
                    channel: 0,
                    kind: 0,
                    data1: 73,
                    min: 0,
                    max: 127,
                },
            }
        );
    }

    #[test]
    fn test_device_status_global_setting_parse() {
        let status = DeviceStatus::try_from(FACTORY_MIDI_CHANNEL.as_slice()).unwrap();
        assert_eq!(
            status,
            DeviceStatus::GlobalSetting {
                setting: GlobalSettingId::MidiChannel,
                value: 0x01,
            }
        );
    }

    #[test]
    fn test_device_status_pad_parse() {
        let status = DeviceStatus::try_from(FACTORY_PAD1.as_slice()).unwrap();
        assert_eq!(
            status,
            DeviceStatus::Pad {
                map: PadMapId::Map1,
                pad: PadId::Pad1,
                value: RawPad {
                    channel: 0,
                    kind: 2,
                    data1: 0,
                    min: 0,
                    max: 127,
                    note: 36,
                },
            }
        );
    }

    #[test]
    fn test_device_status_rejects_bad_checksum() {
        let mut bytes = FACTORY_FADER1;
        bytes[28] ^= 0x01;
        assert!(matches!(
            DeviceStatus::try_from(bytes.as_slice()),
            Err(DeviceStatusParseError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn test_device_status_rejects_unknown_header() {
        let bytes = [0xF0, 0x47, 0x00, 0x35, 0x24, 0xF7];
        assert!(matches!(
            DeviceStatus::try_from(bytes.as_slice()),
            Err(DeviceStatusParseError::InvalidHeader)
        ));
    }

    #[test]
    fn test_device_status_message_round_trip() {
        let dump = Dump::default();
        for message in dump.to_messages() {
            let status = DeviceStatus::try_from(message.clone()).unwrap();
            assert_eq!(status.message(), message);
        }
    }

    #[test]
    fn test_message_counts() {
        assert_eq!(
            Preset::default().send_messages(PresetId::Preset1).len(),
            TOTAL_PRESET_CONTROLS
        );
        assert_eq!(
            PadMap::default().send_messages(PadMapId::Map1).len(),
            TOTAL_PADS
        );
        assert_eq!(
            GlobalSettings::default().send_messages().len(),
            TOTAL_SETTINGS
        );
        assert_eq!(
            GlobalControls::default().send_messages().len(),
            TOTAL_GLOBAL_CONTROLS
        );
        assert_eq!(Dump::default().to_messages().len(), DUMP_MESSAGE_COUNT);
        assert_eq!(DUMP_MESSAGE_COUNT, 182);
    }

    #[test]
    fn test_dump_assembler_round_trip() {
        let dump = Dump::default();
        let mut assembler = DumpAssembler::default();
        assert!(assembler.is_empty());

        for message in dump.to_messages() {
            let status = DeviceStatus::try_from(message).unwrap();
            assembler.apply(&status);
        }

        assert_eq!(assembler.len(), DUMP_MESSAGE_COUNT);
        assert!(assembler.is_complete());
        assert_eq!(assembler.try_into_dump().unwrap(), dump);
    }

    #[test]
    fn test_dump_assembler_missing_message_errors() {
        let assembler = DumpAssembler::default();
        assert!(!assembler.is_complete());
        assert!(matches!(
            assembler.try_into_dump(),
            Err(DumpParseError::MissingPresetControl { .. })
        ));
    }

    #[test]
    fn test_dump_as_bytes_round_trip() {
        let mut dump = Dump::default();
        dump.presets[2].faders[4].cc = 42.into();
        dump.presets[4].fader_buttons[8].kind = ButtonKind::Note;
        dump.pad_maps[3].pads[7].note = 99.into();
        dump.settings.midi_channel = MidiChannel::Ch7;
        dump.controls.mod_wheel.kind = ContinuousKind::PitchBend;

        let bytes = dump.as_bytes();
        assert_eq!(bytes.len(), std::mem::size_of::<RawDump>());
        assert_eq!(Dump::try_from(bytes.as_slice()).unwrap(), dump);
    }

    #[test]
    fn test_factory_mixer_presets() {
        let preset2 = Preset::factory(PresetId::Preset2);
        assert_eq!(preset2.faders[0].channel, ControlChannel::Ch1);
        assert_eq!(preset2.faders[7].channel, ControlChannel::Ch8);
        assert_eq!(preset2.faders[8].channel, ControlChannel::Follow);
        assert_eq!(preset2.fader_buttons[8].data1, 65.into());

        let preset3 = Preset::factory(PresetId::Preset3);
        assert_eq!(preset3.pots[0].channel, ControlChannel::Ch9);
        assert_eq!(preset3.pots[7].channel, ControlChannel::Ch16);
    }

    #[test]
    fn test_global_setting_iter_order_is_dump_order() {
        let order: Vec<u8> = GlobalSettingId::iter().map(u8::from).collect();
        assert_eq!(
            order,
            vec![
                0x01, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11, 0x12, 0x0F
            ]
        );
    }

    #[test]
    fn test_is_sysex_port() {
        assert!(is_sysex_port("Impact LX61+ MIDI1"));
        assert!(is_sysex_port("Impact LX49+ MIDI1"));
        assert!(!is_sysex_port("Impact LX61+ MIDI2"));
        assert!(!is_sysex_port("Arturia MiniLab mkII"));
    }
}
