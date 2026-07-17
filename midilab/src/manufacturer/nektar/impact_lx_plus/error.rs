use num_enum::TryFromPrimitiveError;

use crate::error::SysexParseError;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::ButtonKind;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::ContinuousKind;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::ControlChannel;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::MidiChannel;

#[derive(Debug, thiserror::Error)]
pub enum DeviceStatusParseError {
    #[error("invalid sysex: {0}")]
    InvalidSysex(#[from] SysexParseError),
    #[error("invalid header")]
    InvalidHeader,
    #[error("invalid length: {0}")]
    InvalidLength(usize),
    #[error("checksum mismatch: expected {expected:#04x}, got {actual:#04x}")]
    ChecksumMismatch { expected: u8, actual: u8 },
    #[error("invalid object type: {0:#04x}")]
    InvalidObjectType(u8),
    #[error("invalid preset: {0:#04x}")]
    InvalidPreset(u8),
    #[error("invalid pad map: {0:#04x}")]
    InvalidPadMap(u8),
    #[error("invalid section: {0:#04x}")]
    InvalidSection(u8),
    #[error("invalid preset control id: {0:#04x}")]
    InvalidPresetControlId(u8),
    #[error("invalid pad id: {0:#04x}")]
    InvalidPadId(u8),
    #[error("invalid global setting id: {0:#04x}")]
    InvalidSettingId(u8),
    #[error("invalid global control id: {0:#04x}")]
    InvalidGlobalControlId(u8),
    #[error("malformed TLV parameter block")]
    InvalidTlv,
    #[error("unsupported TLV parameter length: {0}")]
    InvalidParamLength(u8),
    #[error("missing param {0:#04x}")]
    MissingParam(u8),
}

#[derive(Debug, thiserror::Error)]
pub enum ControlParseError {
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<ControlChannel>),
    #[error("invalid continuous kind: {0}")]
    ContinuousKind(TryFromPrimitiveError<ContinuousKind>),
    #[error("invalid button kind: {0}")]
    ButtonKind(TryFromPrimitiveError<ButtonKind>),
}

#[derive(Debug, thiserror::Error)]
pub enum DumpParseError {
    #[error("missing preset {preset} control {control:#04x}")]
    MissingPresetControl { preset: u8, control: u8 },
    #[error("missing pad map {map} pad {pad:#04x}")]
    MissingPad { map: u8, pad: u8 },
    #[error("missing global setting {setting:#04x}")]
    MissingSetting { setting: u8 },
    #[error("missing global control {control:#04x}")]
    MissingGlobalControl { control: u8 },
    #[error("preset {preset} control {control:#04x}: {source}")]
    PresetControl {
        preset: u8,
        control: u8,
        source: ControlParseError,
    },
    #[error("pad map {map} pad {pad:#04x}: {source}")]
    Pad {
        map: u8,
        pad: u8,
        source: ControlParseError,
    },
    #[error("invalid global MIDI channel: {0}")]
    GlobalMidiChannel(TryFromPrimitiveError<MidiChannel>),
    #[error("global control {control:#04x}: {source}")]
    GlobalControl {
        control: u8,
        source: ControlParseError,
    },
    #[error("invalid length: {0}")]
    InvalidLength(usize),
}
