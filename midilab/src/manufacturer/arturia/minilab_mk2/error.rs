use num_enum::TryFromPrimitiveError;

use crate::error::SysexParseError;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::ButtonMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::ControlChannel;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::KnobAcceleration;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::KnobMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::MidiChannel;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::ModWheelMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::PadColor;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::PadMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::PitchBendMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::SustainPedalMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::ToggleState;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::VelocityCurve;

#[derive(Debug, thiserror::Error)]
pub enum DeviceStatusParseError {
    #[error("invalid sysex: {0}")]
    InvalidSysex(#[from] SysexParseError),
    #[error("invalid header")]
    InvalidHeader,
    #[error("invalid op code: {0}")]
    InvalidOpCode(u8),
    #[error("invalid param: {0}")]
    InvalidParam(u8),
    #[error("invalid control: {0}")]
    InvalidControl(u8),
    #[error("invalid global param: {0}")]
    InvalidGlobalParam(u8),
    #[error("invalid length: {0}")]
    InvalidLength(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum PresetParseError {
    #[error("knob {index}: {source}")]
    Knob {
        index: usize,
        source: KnobParseError,
    },
    #[error("shift knob {index}: {source}")]
    ShiftKnob {
        index: usize,
        source: KnobParseError,
    },
    #[error("button {index}: {source}")]
    Button {
        index: usize,
        source: ButtonParseError,
    },
    #[error("mod wheel: {0}")]
    ModWheel(#[from] ModWheelParseError),
    #[error("pitch bend: {0}")]
    PitchBend(#[from] PitchBendParseError),
    #[error("sustain pedal: {0}")]
    SustainPedal(#[from] SustainPedalParseError),
    #[error("pad {index}: {source}")]
    Pad { index: usize, source: PadParseError },
    #[error("missing param {param:#04x} for control {control:#04x}")]
    MissingParam { param: u8, control: u8 },
    #[error("invalid length: {0}")]
    InvalidLength(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum KnobParseError {
    #[error("invalid mode: {0}")]
    Mode(TryFromPrimitiveError<KnobMode>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<ControlChannel>),
}

#[derive(Debug, thiserror::Error)]
pub enum ButtonParseError {
    #[error("invalid mode: {0}")]
    Mode(TryFromPrimitiveError<ButtonMode>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<ControlChannel>),
}

#[derive(Debug, thiserror::Error)]
pub enum ModWheelParseError {
    #[error("invalid mode: {0}")]
    Mode(TryFromPrimitiveError<ModWheelMode>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<ControlChannel>),
}

#[derive(Debug, thiserror::Error)]
pub enum PitchBendParseError {
    #[error("invalid mode: {0}")]
    Mode(TryFromPrimitiveError<PitchBendMode>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<ControlChannel>),
}

#[derive(Debug, thiserror::Error)]
pub enum SustainPedalParseError {
    #[error("invalid mode: {0}")]
    Mode(TryFromPrimitiveError<SustainPedalMode>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<ControlChannel>),
}

#[derive(Debug, thiserror::Error)]
pub enum PadParseError {
    #[error("invalid mode: {0}")]
    Mode(TryFromPrimitiveError<PadMode>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<ControlChannel>),
    #[error("invalid color: {0}")]
    Color(TryFromPrimitiveError<PadColor>),
}

#[derive(Debug, thiserror::Error)]
pub enum GlobalParseError {
    #[error("invalid keyboard_channel: {0}")]
    KeyboardChannel(TryFromPrimitiveError<MidiChannel>),
    #[error("invalid key_velocity_curve: {0}")]
    KeyVelocityCurve(TryFromPrimitiveError<VelocityCurve>),
    #[error("invalid pad_velocity_curve: {0}")]
    PadVelocityCurve(TryFromPrimitiveError<VelocityCurve>),
    #[error("invalid knob_acceleration: {0}")]
    KnobAcceleration(TryFromPrimitiveError<KnobAcceleration>),
    #[error("invalid octave_button_blink: {0}")]
    OctaveButtonBlink(TryFromPrimitiveError<ToggleState>),
    #[error("invalid pad_off_backlight: {0}")]
    PadOffBacklight(TryFromPrimitiveError<ToggleState>),
    #[error("missing global param {param:#04x}")]
    MissingParam { param: u8 },
    #[error("invalid length: {0}")]
    InvalidLength(usize),
}
