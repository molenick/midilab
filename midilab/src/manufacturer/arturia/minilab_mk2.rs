use std::collections::HashMap;

use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::manufacturer::arturia::SYSEX_MANUFACTURER_ID;
use crate::manufacturer::arturia::minilab_mk2::control::Button;
use crate::manufacturer::arturia::minilab_mk2::control::ControlId;
use crate::manufacturer::arturia::minilab_mk2::control::Knob;
use crate::manufacturer::arturia::minilab_mk2::control::ModWheel;
use crate::manufacturer::arturia::minilab_mk2::control::Pad;
use crate::manufacturer::arturia::minilab_mk2::control::PitchBend;
use crate::manufacturer::arturia::minilab_mk2::control::SustainPedal;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::KnobAcceleration;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::MemorySlot;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::MidiChannel;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::PadColor;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::ToggleState;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::VelocityCurve;
use crate::manufacturer::arturia::minilab_mk2::error::DeviceStatusParseError;
use crate::manufacturer::arturia::minilab_mk2::error::GlobalParseError;
use crate::manufacturer::arturia::minilab_mk2::error::PresetParseError;
use crate::manufacturer::arturia::minilab_mk2::raw::RawControl;
use crate::manufacturer::arturia::minilab_mk2::raw::RawGlobal;
use crate::manufacturer::arturia::minilab_mk2::raw::RawPad;
use crate::manufacturer::arturia::minilab_mk2::raw::RawPreset;
use crate::manufacturer::arturia::minilab_mk2::repository::ButtonRepository;
use crate::manufacturer::arturia::minilab_mk2::repository::KnobRepository;
use crate::manufacturer::arturia::minilab_mk2::repository::PadRepository;
use crate::sysex::Sysex;

pub mod control;
pub mod error;
pub mod raw;
pub mod repository;

pub const PORT_NAME: &str = "Arturia MiniLab mkII";
pub const TOTAL_KNOBS: usize = 16;
pub const TOTAL_SHIFT_KNOBS: usize = 2;
pub const TOTAL_PADS: usize = 16;

pub const SYSEX_COMMAND_HEADER: [u8; 5] = [
    SYSEX_MANUFACTURER_ID[0],
    SYSEX_MANUFACTURER_ID[1],
    SYSEX_MANUFACTURER_ID[2],
    0x7F,
    0x42,
];

const GLOBAL_PARAM_MARKER: u8 = 0x40;
const IDENTITY_HEADER: u8 = 0x7E;
const IDENTITY_REPLY_LEN: usize = 15;

#[repr(u8)]
#[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Clone, Copy)]
pub enum OpCode {
    ReadParam = 0x01,
    WriteParam = 0x02,
    RecallMemory = 0x05,
    StoreMemory = 0x06,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive, EnumIter)]
pub enum ParamId {
    Mode = 0x01,
    Channel = 0x02,
    Data1 = 0x03,
    Data2 = 0x04,
    Data3 = 0x05,
    Option = 0x06,
    PadColorLive = 0x10,
    PadColor = 0x11,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive, EnumIter)]
pub enum GlobalParamId {
    KeyboardChannel = 0x06,
    KeyVelocityCurve = 0x19,
    PadVelocityCurve = 0x1A,
    KnobAcceleration = 0x1B,
    OctaveButtonBlink = 0x1D,
    PadOffBacklight = 0x1E,
}

fn command_message(body: &[u8]) -> Vec<u8> {
    let mut payload = SYSEX_COMMAND_HEADER.to_vec();
    payload.extend_from_slice(body);
    Sysex::new(payload).as_bytes()
}

pub fn read_param_message(param: ParamId, control: ControlId) -> Vec<u8> {
    command_message(&[OpCode::ReadParam.into(), 0x00, param.into(), control.into()])
}

pub fn write_param_message(param: ParamId, control: ControlId, value: u8) -> Vec<u8> {
    write_value_message(param.into(), control.into(), value)
}

pub fn write_value_message(param: u8, control: u8, value: u8) -> Vec<u8> {
    command_message(&[OpCode::WriteParam.into(), 0x00, param, control, value])
}

pub fn read_global_message(param: GlobalParamId) -> Vec<u8> {
    command_message(&[
        OpCode::ReadParam.into(),
        0x00,
        GLOBAL_PARAM_MARKER,
        param.into(),
    ])
}

pub fn write_global_message(param: GlobalParamId, value: u8) -> Vec<u8> {
    write_value_message(GLOBAL_PARAM_MARKER, param.into(), value)
}

pub fn recall_memory_message(slot: MemorySlot) -> Vec<u8> {
    command_message(&[OpCode::RecallMemory.into(), slot.into()])
}

pub fn store_memory_message(slot: MemorySlot) -> Vec<u8> {
    command_message(&[OpCode::StoreMemory.into(), slot.into()])
}

pub fn identity_request_message() -> Vec<u8> {
    Sysex::new(vec![IDENTITY_HEADER, 0x7F, 0x06, 0x01]).as_bytes()
}

pub fn identity_reply_message(firmware: [u8; 4]) -> Vec<u8> {
    let mut payload = vec![IDENTITY_HEADER, 0x00, 0x06, 0x02];
    payload.extend_from_slice(&SYSEX_MANUFACTURER_ID);
    payload.extend_from_slice(&[0x02, 0x00, 0x04, 0x02]);
    payload.extend_from_slice(&firmware);
    Sysex::new(payload).as_bytes()
}

pub fn set_pad_live_color_message(pad: ControlId, color: PadColor) -> Vec<u8> {
    write_param_message(ParamId::PadColorLive, pad, color.into())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamValue {
    pub param: ParamId,
    pub control: ControlId,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalValue {
    pub param: GlobalParamId,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdentityReply {
    pub firmware: [u8; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceStatus {
    ParamValue(ParamValue),
    GlobalValue(GlobalValue),
    IdentityReply(IdentityReply),
}

impl TryFrom<&[u8]> for DeviceStatus {
    type Error = DeviceStatusParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let sysex = Sysex::try_from(value)?;
        DeviceStatus::try_from(sysex)
    }
}

impl TryFrom<Sysex> for DeviceStatus {
    type Error = DeviceStatusParseError;

    fn try_from(value: Sysex) -> Result<Self, Self::Error> {
        let payload = value.payload();

        if payload.first() == Some(&IDENTITY_HEADER) {
            return parse_identity_reply(payload);
        }

        if !payload.starts_with(&SYSEX_COMMAND_HEADER) {
            return Err(DeviceStatusParseError::InvalidHeader);
        }

        let body = &payload[SYSEX_COMMAND_HEADER.len()..];
        let [op, _, param, control, value] = body else {
            return Err(DeviceStatusParseError::InvalidLength(body.len()));
        };

        let op = OpCode::try_from(*op).map_err(|_| DeviceStatusParseError::InvalidOpCode(*op))?;
        if op != OpCode::WriteParam {
            return Err(DeviceStatusParseError::InvalidOpCode(op.into()));
        }

        if *param == GLOBAL_PARAM_MARKER {
            let param = GlobalParamId::try_from(*control)
                .map_err(|_| DeviceStatusParseError::InvalidGlobalParam(*control))?;
            return Ok(DeviceStatus::GlobalValue(GlobalValue {
                param,
                value: *value,
            }));
        }

        Ok(DeviceStatus::ParamValue(ParamValue {
            param: ParamId::try_from(*param)
                .map_err(|_| DeviceStatusParseError::InvalidParam(*param))?,
            control: ControlId::try_from(*control)
                .map_err(|_| DeviceStatusParseError::InvalidControl(*control))?,
            value: *value,
        }))
    }
}

fn parse_identity_reply(payload: &[u8]) -> Result<DeviceStatus, DeviceStatusParseError> {
    if payload.len() != IDENTITY_REPLY_LEN {
        return Err(DeviceStatusParseError::InvalidLength(payload.len()));
    }

    if payload[2] != 0x06 || payload[3] != 0x02 {
        return Err(DeviceStatusParseError::InvalidHeader);
    }

    if payload[4..7] != SYSEX_MANUFACTURER_ID {
        return Err(DeviceStatusParseError::InvalidHeader);
    }

    let mut firmware = [0u8; 4];
    firmware.copy_from_slice(&payload[11..15]);

    Ok(DeviceStatus::IdentityReply(IdentityReply { firmware }))
}

#[derive(Debug, Clone, Default)]
pub struct ParamStore(HashMap<(u8, u8), u8>);

impl ParamStore {
    pub fn apply(&mut self, status: &DeviceStatus) {
        match status {
            DeviceStatus::ParamValue(pv) => {
                self.0
                    .insert((pv.param.into(), pv.control.into()), pv.value);
            }
            DeviceStatus::GlobalValue(gv) => {
                self.0
                    .insert((GLOBAL_PARAM_MARKER, gv.param.into()), gv.value);
            }
            DeviceStatus::IdentityReply(_) => {}
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, param: u8, control: u8) -> Option<u8> {
        self.0.get(&(param, control)).copied()
    }

    pub fn set(&mut self, param: u8, control: u8, value: u8) {
        self.0.insert((param, control), value);
    }

    fn require(&self, param: ParamId, control: ControlId) -> Result<u8, PresetParseError> {
        self.0
            .get(&(param.into(), control.into()))
            .copied()
            .ok_or(PresetParseError::MissingParam {
                param: param.into(),
                control: control.into(),
            })
    }

    fn raw_control(
        &self,
        control: ControlId,
        params: &[ParamId],
    ) -> Result<RawControl, PresetParseError> {
        let mut raw = RawControl::default();
        for param in params {
            let value = self.require(*param, control)?;
            match param {
                ParamId::Mode => raw.mode = value,
                ParamId::Channel => raw.channel = value,
                ParamId::Data1 => raw.data1 = value,
                ParamId::Data2 => raw.data2 = value,
                ParamId::Data3 => raw.data3 = value,
                ParamId::Option => raw.option = value,
                ParamId::PadColorLive | ParamId::PadColor => {}
            }
        }
        Ok(raw)
    }

    fn raw_pad(&self, control: ControlId) -> Result<RawPad, PresetParseError> {
        let raw = self.raw_control(control, &Knob::PARAMS)?;
        let color = self.require(ParamId::PadColor, control)?;
        Ok(RawPad {
            mode: raw.mode,
            channel: raw.channel,
            data1: raw.data1,
            data2: raw.data2,
            data3: raw.data3,
            option: raw.option,
            color,
        })
    }

    pub fn try_into_raw_preset(&self) -> Result<RawPreset, PresetParseError> {
        let mut raw = RawPreset::default();

        for (index, id) in ControlId::KNOBS.iter().enumerate() {
            raw.knobs[index] = self.raw_control(*id, &Knob::PARAMS)?;
        }

        for (index, id) in ControlId::SHIFT_KNOBS.iter().enumerate() {
            raw.shift_knobs[index] = self.raw_control(*id, &Knob::PARAMS)?;
        }

        for (index, id) in ControlId::BUTTONS.iter().enumerate() {
            raw.buttons[index] = self.raw_control(*id, &Button::PARAMS)?;
        }

        raw.mod_wheel = self.raw_control(ControlId::ModWheel, &ModWheel::PARAMS)?;
        raw.pitch_bend = self.raw_control(ControlId::PitchBend, &PitchBend::PARAMS)?;
        raw.sustain_pedal = self.raw_control(ControlId::SustainPedal, &SustainPedal::PARAMS)?;

        for (index, id) in ControlId::PADS.iter().enumerate() {
            raw.pads[index] = self.raw_pad(*id)?;
        }

        Ok(raw)
    }

    pub fn try_into_preset(&self) -> Result<Preset, PresetParseError> {
        Preset::try_from(self.try_into_raw_preset()?)
    }

    pub fn try_into_raw_global(&self) -> Result<RawGlobal, GlobalParseError> {
        let mut values = [0u8; 6];
        for (slot, param) in values.iter_mut().zip(GlobalParamId::iter()) {
            *slot = self
                .0
                .get(&(GLOBAL_PARAM_MARKER, param.into()))
                .copied()
                .ok_or(GlobalParseError::MissingParam {
                    param: param.into(),
                })?;
        }

        Ok(RawGlobal {
            keyboard_channel: values[0],
            key_velocity_curve: values[1],
            pad_velocity_curve: values[2],
            knob_acceleration: values[3],
            octave_button_blink: values[4],
            pad_off_backlight: values[5],
        })
    }

    pub fn try_into_global(&self) -> Result<Global, GlobalParseError> {
        Global::try_from(self.try_into_raw_global()?)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    pub knobs: KnobRepository,
    pub buttons: ButtonRepository,
    pub mod_wheel: ModWheel,
    pub pitch_bend: PitchBend,
    pub sustain_pedal: SustainPedal,
    pub pads: PadRepository,
}

impl Preset {
    const GENERIC_KNOB_CC: [u8; TOTAL_KNOBS] = [
        102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117,
    ];
    const GENERIC_SHIFT_KNOB_CC: [u8; TOTAL_SHIFT_KNOBS] = [118, 119];

    pub fn read_messages() -> Vec<Vec<u8>> {
        let mut messages = Vec::new();

        for id in ControlId::KNOBS.iter().chain(ControlId::SHIFT_KNOBS.iter()) {
            for param in Knob::PARAMS {
                messages.push(read_param_message(param, *id));
            }
        }

        for id in ControlId::BUTTONS {
            for param in Button::PARAMS {
                messages.push(read_param_message(param, id));
            }
        }

        for param in ModWheel::PARAMS {
            messages.push(read_param_message(param, ControlId::ModWheel));
        }

        for param in PitchBend::PARAMS {
            messages.push(read_param_message(param, ControlId::PitchBend));
        }

        for param in SustainPedal::PARAMS {
            messages.push(read_param_message(param, ControlId::SustainPedal));
        }

        for id in ControlId::PADS {
            for param in Pad::PARAMS {
                messages.push(read_param_message(param, id));
            }
        }

        messages
    }

    pub fn send_messages(&self) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();

        for knob in self.knobs.knobs.iter().chain(self.knobs.shift_knobs.iter()) {
            for (param, value) in knob.param_pairs() {
                messages.push(write_param_message(param, knob.id, value));
            }
        }

        for button in &self.buttons.buttons {
            for (param, value) in button.param_pairs() {
                messages.push(write_param_message(param, button.id, value));
            }
        }

        for (param, value) in self.mod_wheel.param_pairs() {
            messages.push(write_param_message(param, ControlId::ModWheel, value));
        }

        for (param, value) in self.pitch_bend.param_pairs() {
            messages.push(write_param_message(param, ControlId::PitchBend, value));
        }

        for (param, value) in self.sustain_pedal.param_pairs() {
            messages.push(write_param_message(param, ControlId::SustainPedal, value));
        }

        for pad in &self.pads.pads {
            for (param, value) in pad.param_pairs() {
                messages.push(write_param_message(param, pad.id, value));
            }
        }

        messages
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let raw = RawPreset::from(self);
        bytemuck::bytes_of(&raw).to_vec()
    }

    pub fn default_filename(&self) -> String {
        "arturia_minilab_mk2.preset".to_string()
    }
}

impl Default for Preset {
    fn default() -> Self {
        let mut knobs = KnobRepository::with_cc_values(Self::GENERIC_KNOB_CC);
        for (knob, cc) in knobs
            .shift_knobs
            .iter_mut()
            .zip(Self::GENERIC_SHIFT_KNOB_CC)
        {
            knob.cc = cc.into();
        }

        Self {
            knobs,
            buttons: ButtonRepository::default(),
            mod_wheel: ModWheel::default(),
            pitch_bend: PitchBend::default(),
            sustain_pedal: SustainPedal::default(),
            pads: PadRepository::default(),
        }
    }
}

impl TryFrom<&[u8]> for Preset {
    type Error = PresetParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let raw: RawPreset = *bytemuck::try_from_bytes(value)
            .map_err(|_| PresetParseError::InvalidLength(value.len()))?;
        Preset::try_from(raw)
    }
}

impl TryFrom<RawPreset> for Preset {
    type Error = PresetParseError;

    fn try_from(raw: RawPreset) -> Result<Self, Self::Error> {
        Ok(Preset {
            knobs: KnobRepository::try_from((&raw.knobs, &raw.shift_knobs))?,
            buttons: ButtonRepository::try_from(&raw.buttons)?,
            mod_wheel: ModWheel::try_from(raw.mod_wheel)?,
            pitch_bend: PitchBend::try_from(raw.pitch_bend)?,
            sustain_pedal: SustainPedal::try_from(raw.sustain_pedal)?,
            pads: PadRepository::try_from(&raw.pads)?,
        })
    }
}

impl From<&Preset> for RawPreset {
    fn from(preset: &Preset) -> Self {
        let mut raw = RawPreset::default();

        for (slot, knob) in raw.knobs.iter_mut().zip(preset.knobs.knobs.iter()) {
            *slot = RawControl::from(knob);
        }

        for (slot, knob) in raw
            .shift_knobs
            .iter_mut()
            .zip(preset.knobs.shift_knobs.iter())
        {
            *slot = RawControl::from(knob);
        }

        for (slot, button) in raw.buttons.iter_mut().zip(preset.buttons.buttons.iter()) {
            *slot = RawControl::from(button);
        }

        raw.mod_wheel = RawControl::from(&preset.mod_wheel);
        raw.pitch_bend = RawControl::from(&preset.pitch_bend);
        raw.sustain_pedal = RawControl::from(&preset.sustain_pedal);

        for (slot, pad) in raw.pads.iter_mut().zip(preset.pads.pads.iter()) {
            *slot = RawPad::from(pad);
        }

        raw
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Global {
    pub keyboard_channel: MidiChannel,
    pub key_velocity_curve: VelocityCurve,
    pub pad_velocity_curve: VelocityCurve,
    pub knob_acceleration: KnobAcceleration,
    pub octave_button_blink: ToggleState,
    pub pad_off_backlight: ToggleState,
}

impl Global {
    pub fn read_messages() -> Vec<Vec<u8>> {
        GlobalParamId::iter().map(read_global_message).collect()
    }

    pub fn send_messages(&self) -> Vec<Vec<u8>> {
        let raw = RawGlobal::from(self);
        GlobalParamId::iter()
            .zip(raw.as_bytes())
            .map(|(param, value)| write_global_message(param, value))
            .collect()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let raw = RawGlobal::from(self);
        bytemuck::bytes_of(&raw).to_vec()
    }
}

impl Default for Global {
    fn default() -> Self {
        Self {
            keyboard_channel: MidiChannel::default(),
            key_velocity_curve: VelocityCurve::default(),
            pad_velocity_curve: VelocityCurve::default(),
            knob_acceleration: KnobAcceleration::default(),
            octave_button_blink: ToggleState::On,
            pad_off_backlight: ToggleState::Off,
        }
    }
}

impl TryFrom<&[u8]> for Global {
    type Error = GlobalParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let raw: RawGlobal = *bytemuck::try_from_bytes(value)
            .map_err(|_| GlobalParseError::InvalidLength(value.len()))?;
        Global::try_from(raw)
    }
}

impl TryFrom<RawGlobal> for Global {
    type Error = GlobalParseError;

    fn try_from(raw: RawGlobal) -> Result<Self, Self::Error> {
        Ok(Global {
            keyboard_channel: MidiChannel::try_from(raw.keyboard_channel)
                .map_err(GlobalParseError::KeyboardChannel)?,
            key_velocity_curve: VelocityCurve::try_from(raw.key_velocity_curve)
                .map_err(GlobalParseError::KeyVelocityCurve)?,
            pad_velocity_curve: VelocityCurve::try_from(raw.pad_velocity_curve)
                .map_err(GlobalParseError::PadVelocityCurve)?,
            knob_acceleration: KnobAcceleration::try_from(raw.knob_acceleration)
                .map_err(GlobalParseError::KnobAcceleration)?,
            octave_button_blink: ToggleState::try_from(raw.octave_button_blink)
                .map_err(GlobalParseError::OctaveButtonBlink)?,
            pad_off_backlight: ToggleState::try_from(raw.pad_off_backlight)
                .map_err(GlobalParseError::PadOffBacklight)?,
        })
    }
}

impl From<&Global> for RawGlobal {
    fn from(global: &Global) -> Self {
        RawGlobal {
            keyboard_channel: global.keyboard_channel.into(),
            key_velocity_curve: global.key_velocity_curve.into(),
            pad_velocity_curve: global.pad_velocity_curve.into(),
            knob_acceleration: global.knob_acceleration.into(),
            octave_button_blink: global.octave_button_blink.into(),
            pad_off_backlight: global.pad_off_backlight.into(),
        }
    }
}

impl RawGlobal {
    pub fn as_bytes(&self) -> [u8; 6] {
        [
            self.keyboard_channel,
            self.key_velocity_curve,
            self.pad_velocity_curve,
            self.knob_acceleration,
            self.octave_button_blink,
            self.pad_off_backlight,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manufacturer::arturia::minilab_mk2::control::value_kind::ControlChannel;
    use crate::manufacturer::arturia::minilab_mk2::control::value_kind::KnobMode;
    use crate::manufacturer::arturia::minilab_mk2::control::value_kind::PadMode;

    #[test]
    fn test_read_param_message_format() {
        let msg = read_param_message(ParamId::Mode, ControlId::Knob2);

        assert_eq!(
            msg,
            vec![
                0xF0, 0x00, 0x20, 0x6B, 0x7F, 0x42, 0x01, 0x00, 0x01, 0x01, 0xF7
            ]
        );
    }

    #[test]
    fn test_write_param_message_format() {
        let msg = write_param_message(ParamId::PadColor, ControlId::Pad1, PadColor::Yellow.into());

        assert_eq!(
            msg,
            vec![
                0xF0, 0x00, 0x20, 0x6B, 0x7F, 0x42, 0x02, 0x00, 0x11, 0x70, 0x05, 0xF7
            ]
        );
    }

    #[test]
    fn test_read_global_message_format() {
        let msg = read_global_message(GlobalParamId::KeyboardChannel);

        assert_eq!(
            msg,
            vec![
                0xF0, 0x00, 0x20, 0x6B, 0x7F, 0x42, 0x01, 0x00, 0x40, 0x06, 0xF7
            ]
        );
    }

    #[test]
    fn test_write_global_message_format() {
        let msg = write_global_message(GlobalParamId::KnobAcceleration, 0x02);

        assert_eq!(
            msg,
            vec![
                0xF0, 0x00, 0x20, 0x6B, 0x7F, 0x42, 0x02, 0x00, 0x40, 0x1B, 0x02, 0xF7
            ]
        );
    }

    #[test]
    fn test_recall_memory_message_format() {
        let msg = recall_memory_message(MemorySlot::Slot2);

        assert_eq!(
            msg,
            vec![0xF0, 0x00, 0x20, 0x6B, 0x7F, 0x42, 0x05, 0x02, 0xF7]
        );
    }

    #[test]
    fn test_store_memory_message_format() {
        let msg = store_memory_message(MemorySlot::Slot8);

        assert_eq!(
            msg,
            vec![0xF0, 0x00, 0x20, 0x6B, 0x7F, 0x42, 0x06, 0x08, 0xF7]
        );
    }

    #[test]
    fn test_identity_request_message_format() {
        assert_eq!(
            identity_request_message(),
            vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]
        );
    }

    #[test]
    fn test_set_pad_live_color_message_format() {
        let msg = set_pad_live_color_message(ControlId::Pad16, PadColor::Cyan);

        assert_eq!(
            msg,
            vec![
                0xF0, 0x00, 0x20, 0x6B, 0x7F, 0x42, 0x02, 0x00, 0x10, 0x7F, 0x14, 0xF7
            ]
        );
    }

    #[test]
    fn test_device_status_param_value_parse() {
        let bytes = vec![
            0xF0, 0x00, 0x20, 0x6B, 0x7F, 0x42, 0x02, 0x00, 0x03, 0x01, 0x4A, 0xF7,
        ];

        let status = DeviceStatus::try_from(bytes.as_slice()).unwrap();

        assert_eq!(
            status,
            DeviceStatus::ParamValue(ParamValue {
                param: ParamId::Data1,
                control: ControlId::Knob2,
                value: 0x4A,
            })
        );
    }

    #[test]
    fn test_device_status_global_value_parse() {
        let bytes = vec![
            0xF0, 0x00, 0x20, 0x6B, 0x7F, 0x42, 0x02, 0x00, 0x40, 0x19, 0x01, 0xF7,
        ];

        let status = DeviceStatus::try_from(bytes.as_slice()).unwrap();

        assert_eq!(
            status,
            DeviceStatus::GlobalValue(GlobalValue {
                param: GlobalParamId::KeyVelocityCurve,
                value: 0x01,
            })
        );
    }

    #[test]
    fn test_device_status_identity_reply_parse() {
        let bytes = vec![
            0xF0, 0x7E, 0x00, 0x06, 0x02, 0x00, 0x20, 0x6B, 0x02, 0x00, 0x04, 0x02, 0x01, 0x00,
            0x02, 0x05, 0xF7,
        ];

        let status = DeviceStatus::try_from(bytes.as_slice()).unwrap();

        assert_eq!(
            status,
            DeviceStatus::IdentityReply(IdentityReply {
                firmware: [0x01, 0x00, 0x02, 0x05],
            })
        );
    }

    #[test]
    fn test_identity_reply_message_round_trip() {
        let firmware = [0x01, 0x00, 0x02, 0x05];
        let bytes = identity_reply_message(firmware);

        let status = DeviceStatus::try_from(bytes.as_slice()).unwrap();
        assert_eq!(
            status,
            DeviceStatus::IdentityReply(IdentityReply { firmware })
        );
    }

    #[test]
    fn test_device_status_rejects_unknown_header() {
        let bytes = vec![0xF0, 0x47, 0x00, 0x35, 0x24, 0xF7];

        assert!(matches!(
            DeviceStatus::try_from(bytes.as_slice()),
            Err(DeviceStatusParseError::InvalidHeader)
        ));
    }

    #[test]
    fn test_preset_message_counts() {
        let expected = 18 * 6 + 4 * 6 + 6 + 3 + 6 + 16 * 7;

        assert_eq!(Preset::read_messages().len(), expected);
        assert_eq!(Preset::default().send_messages().len(), expected);
    }

    #[test]
    fn test_global_message_counts() {
        assert_eq!(Global::read_messages().len(), 6);
        assert_eq!(Global::default().send_messages().len(), 6);
    }

    #[test]
    fn test_preset_raw_round_trip() {
        let mut preset = Preset::default();
        preset.knobs.knobs[0].mode = KnobMode::Nrpn;
        preset.knobs.knobs[0].channel = ControlChannel::Ch5;
        preset.pads.pads[3].mode = PadMode::PatchChange;
        preset.pads.pads[3].color = PadColor::Purple;
        preset.pitch_bend.channel = ControlChannel::Ch16;

        let raw = RawPreset::from(&preset);
        let restored = Preset::try_from(raw).unwrap();

        assert_eq!(preset, restored);
    }

    #[test]
    fn test_preset_as_bytes_round_trip() {
        let preset = Preset::default();
        let bytes = preset.as_bytes();

        assert_eq!(bytes.len(), std::mem::size_of::<RawPreset>());
        assert_eq!(Preset::try_from(bytes.as_slice()).unwrap(), preset);
    }

    #[test]
    fn test_param_store_preset_round_trip() {
        let mut preset = Preset::default();
        preset.knobs.knobs[7].cc = 42.into();
        preset.knobs.shift_knobs[1].mode = KnobMode::Off;
        preset.pads.pads[15].color = PadColor::White;

        let mut store = ParamStore::default();
        for message in preset.send_messages() {
            let status = DeviceStatus::try_from(message.as_slice()).unwrap();
            store.apply(&status);
        }

        let restored = store.try_into_preset().unwrap();
        assert_eq!(preset, restored);
    }

    #[test]
    fn test_param_store_missing_param_errors() {
        let store = ParamStore::default();

        assert!(matches!(
            store.try_into_preset(),
            Err(PresetParseError::MissingParam { .. })
        ));
    }

    #[test]
    fn test_param_store_global_round_trip() {
        let global = Global {
            keyboard_channel: MidiChannel::Ch3,
            key_velocity_curve: VelocityCurve::Exponential,
            pad_velocity_curve: VelocityCurve::Full,
            knob_acceleration: KnobAcceleration::Fast,
            octave_button_blink: ToggleState::Off,
            pad_off_backlight: ToggleState::On,
        };

        let mut store = ParamStore::default();
        for message in global.send_messages() {
            let status = DeviceStatus::try_from(message.as_slice()).unwrap();
            store.apply(&status);
        }

        let restored = store.try_into_global().unwrap();
        assert_eq!(global, restored);
    }

    #[test]
    fn test_global_raw_field_order_matches_param_order() {
        let raw = RawGlobal {
            keyboard_channel: 0,
            key_velocity_curve: 1,
            pad_velocity_curve: 2,
            knob_acceleration: 2,
            octave_button_blink: 0x7F,
            pad_off_backlight: 0,
        };

        let params: Vec<GlobalParamId> = GlobalParamId::iter().collect();
        assert_eq!(params[0], GlobalParamId::KeyboardChannel);
        assert_eq!(params[5], GlobalParamId::PadOffBacklight);
        assert_eq!(raw.as_bytes(), [0, 1, 2, 2, 0x7F, 0]);
    }
}
