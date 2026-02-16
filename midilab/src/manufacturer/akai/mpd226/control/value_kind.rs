use std::fmt;
use std::ops::Deref;

use enumeric::range_enum;
use num_enum::Default;
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::Display;
use strum_macros::EnumIter;

use crate::manufacturer::akai::mpd226::error::PresetParseError;
use crate::manufacturer::akai::mpd226::error::PresetSettingsParseError;
use crate::sysex::pack_u14;
use crate::sysex::unpack_u14;

const DEFAULT_TEMPO: u16 = 120;

#[range_enum]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive, EnumIter, Display)]
pub enum MidiChannel {
    COMMON,
    #[range(1..=16)]
    A,
    #[range(1..=16)]
    B,
}

#[expect(
    clippy::derivable_impls,
    reason = "can't derive default in combination with range_enum"
)]
impl Default for MidiChannel {
    fn default() -> Self {
        Self::COMMON
    }
}

#[range_enum]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive, EnumIter, Display)]
pub enum UsbChannel {
    #[range(1..=16)]
    A,
    #[range(1..=16)]
    B,
}
#[expect(
    clippy::derivable_impls,
    reason = "can't derive default in combination with range_enum"
)]
impl Default for UsbChannel {
    fn default() -> Self {
        Self::A1
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, EnumIter, Display)]
pub enum SwitchKind {
    #[default]
    CC = 0,
    Aftertouch = 1,
    Program = 2,
    Bank = 3,
    Key = 4,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, EnumIter)]
#[expect(non_camel_case_types, reason = "annoying to camelcase and little-used")]
#[allow(clippy::upper_case_acronyms)]
pub enum KeyModifier {
    #[default]
    NONE = 0,
    CTRL = 1,
    SHIFT = 2,
    ALT = 3,
    OPT = 4,
    CTRL_SHIFT = 5,
    CTRL_ALT = 6,
    CTRL_OPT = 7,
    SHIFT_ALT = 8,
    SHIFT_OPT = 9,
    ALT_OPT = 10,
    CTRL_ALT_OP = 11,
    CTRL_SH_ALT = 12,
    CTRL_SH_OPT = 13,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresetName(pub [u8; 8]);
impl Default for PresetName {
    fn default() -> Self {
        Self(*b"Midilab ")
    }
}
impl PresetName {
    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}
impl fmt::Display for PresetName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name_bytes = self.as_bytes();
        let s = String::from_utf8_lossy(&name_bytes);

        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tempo(pub u16);

impl Tempo {
    pub fn to_packed_bytes(&self) -> [u8; 2] {
        pack_u14(self.0)
    }

    pub fn from_packed_bytes(bytes: [u8; 2]) -> Self {
        Self(unpack_u14(bytes))
    }
}

impl Default for Tempo {
    fn default() -> Self {
        Self(DEFAULT_TEMPO)
    }
}

#[repr(u8)]
#[derive(Default, Clone, Copy, TryFromPrimitive, Debug, PartialEq, Display, EnumIter)]
pub enum PresetSlot {
    #[default]
    #[allow(clippy::upper_case_acronyms)]
    RAM = 0,
    Slot1 = 1,
    Slot2 = 2,
    Slot3 = 3,
    Slot4 = 4,
    Slot5 = 5,
    Slot6 = 6,
    Slot7 = 7,
    Slot8 = 8,
    Slot9 = 9,
    Slot10 = 10,
    Slot11 = 11,
    Slot12 = 12,
    Slot13 = 13,
    Slot14 = 14,
    Slot15 = 15,
    Slot16 = 16,
    Slot17 = 17,
    Slot18 = 18,
    Slot19 = 19,
    Slot20 = 20,
}

impl TryFrom<&[u8]> for PresetSlot {
    type Error = PresetParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        const PRESET_ACK_LENGTH: usize = 2;

        if value.len() != PRESET_ACK_LENGTH {
            Err(PresetParseError::PresetSettings(
                PresetSettingsParseError::PresetSlotData(value.to_vec()),
            ))
        } else {
            PresetSlot::try_from_primitive(value[0]).map_err(|e| {
                PresetParseError::PresetSettings(PresetSettingsParseError::PresetSlot(e))
            })
        }
    }
}

#[repr(u8)]
#[derive(Default, Clone, Copy, TryFromPrimitive, Debug, PartialEq, EnumIter, Display)]
pub enum TriggerKind {
    #[default]
    Momentary = 0,
    Toggle = 1,
}

#[repr(u8)]
#[derive(Default, Clone, Copy, TryFromPrimitive, Debug, PartialEq, EnumIter)]
pub enum TimeDivision {
    Div1_4 = 0,
    Div1_4t = 1,
    Div1_8 = 2,
    Div1_8t = 3,
    #[default]
    Div1_16 = 4,
    Div1_16t = 5,
    Div1_32 = 6,
    Div1_32t = 7,
}
impl std::fmt::Display for TimeDivision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            TimeDivision::Div1_4 => "1/4",
            TimeDivision::Div1_4t => "1/4t",
            TimeDivision::Div1_8 => "1/8",
            TimeDivision::Div1_8t => "1/8t",
            TimeDivision::Div1_16 => "1/16",
            TimeDivision::Div1_16t => "1/16t",
            TimeDivision::Div1_32 => "1/32",
            TimeDivision::Div1_32t => "1/32t",
        };

        write!(f, "{out}")
    }
}

#[repr(u8)]
#[derive(Default)]
#[range_enum]
#[derive(Clone, Copy, TryFromPrimitive, Debug, PartialEq, EnumIter, Display)]
pub enum SwingKind {
    #[default]
    Off = 50,

    #[range(51..=75)]
    Swing,
}

/// Represents gate values, saturates at boundaries 1..=99
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gate {
    pub value: u8,
}
impl Gate {
    pub const MIN_CLAMP: u8 = 1;
    pub const MAX_CLAMP: u8 = 99;
}

impl From<u8> for Gate {
    fn from(value: u8) -> Self {
        let value = value.clamp(Self::MIN_CLAMP, Self::MAX_CLAMP);

        Self { value }
    }
}

impl From<Gate> for u8 {
    fn from(val: Gate) -> Self {
        val.value
    }
}

#[repr(u8)]
#[derive(Default, Clone, Copy, TryFromPrimitive, Debug, PartialEq, EnumIter, Display)]
#[allow(clippy::upper_case_acronyms)]
pub enum TransportKind {
    MCC = 0,
    MCCMIDI = 1,
    MIDI = 2,
    #[default]
    CTRL = 3,
    PTEX = 4,
}

#[repr(u8)]
#[derive(Default, Clone, Copy, TryFromPrimitive, Debug, PartialEq, EnumIter, Display)]
pub enum PadKind {
    #[default]
    Note = 0,
    Prog = 1,
    Bank = 2,
}

#[repr(u8)]
#[derive(Default, Clone, Copy, TryFromPrimitive, Debug, PartialEq, EnumIter, Display)]
pub enum AfterTouchKind {
    Off = 0,
    #[default]
    Channel = 1,
    Poly = 2,
}

#[repr(u8)]
#[derive(Default, Clone, Copy, TryFromPrimitive, Debug, PartialEq, EnumIter, Display)]
pub enum PadColor {
    #[default]
    Off = 0,
    Red = 1,
    Orange = 2,
    Amber = 3,
    Yellow = 4,
    Green = 5,
    GreenBlue = 6,
    Aqua = 7,
    LightBlue = 8,
    Blue = 9,
    Purple = 10,
    Pink = 11,
    HotPink = 12,
    LightPurple = 13,
    LightGreen = 14,
    LightPink = 15,
    Grey = 16,
}
impl PadColor {
    pub fn as_rgb_color(&self) -> RGBColor {
        match self {
            Self::Off => RGBColor((0, 0, 0)),
            Self::Red => RGBColor((255, 0, 0)),
            Self::Orange => RGBColor((255, 100, 0)),
            Self::Amber => RGBColor((255, 176, 0)),
            Self::Yellow => RGBColor((255, 255, 0)),
            Self::Green => RGBColor((0, 255, 0)),
            Self::GreenBlue => RGBColor((0, 255, 128)),
            Self::Aqua => RGBColor((0, 255, 255)),
            Self::LightBlue => RGBColor((0, 176, 255)),
            Self::Blue => RGBColor((0, 0, 255)),
            Self::Purple => RGBColor((128, 0, 255)),
            Self::Pink => RGBColor((255, 105, 180)),
            Self::HotPink => RGBColor((255, 20, 147)),
            Self::LightPurple => RGBColor((200, 162, 255)),
            Self::LightGreen => RGBColor((128, 255, 128)),
            Self::LightPink => RGBColor((255, 182, 193)),
            Self::Grey => RGBColor((128, 128, 128)),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RGBColor((u8, u8, u8));
impl Deref for RGBColor {
    type Target = (u8, u8, u8);

    fn deref(&self) -> &(u8, u8, u8) {
        &self.0
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, EnumIter, Display)]
pub enum FaderKind {
    #[default]
    CC = 0,
    Aftertouch = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, EnumIter, Display)]
pub enum DialKind {
    #[default]
    CC = 0,
    Aftertouch = 1,
    IncDec1 = 2,
    IncDec2 = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, EnumIter, Display)]
pub enum ActiveState {
    #[default]
    Off = 0,
    On = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, EnumIter, Display)]
pub enum NoteDisplay {
    #[default]
    Value = 0,
    Number = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum TapAverage {
    Tap2 = 2,
    #[default]
    Tap3 = 3,
    Tap4 = 4,
}

impl std::fmt::Display for TapAverage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = match self {
            TapAverage::Tap2 => "2",
            TapAverage::Tap3 => "3",
            TapAverage::Tap4 => "4",
        };
        write!(f, "{n}")
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum PadCurve {
    #[default]
    Linear = 0,
    SCurve = 1,
    Log1 = 2,
    Log2 = 3,
    Exp1 = 4,
    Exp2 = 5,
}

impl std::fmt::Display for PadCurve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PadCurve::Linear => "Linear",
            PadCurve::SCurve => "S-Curve",
            PadCurve::Log1 => "Log 1",
            PadCurve::Log2 => "Log 2",
            PadCurve::Exp1 => "Exp 1",
            PadCurve::Exp2 => "Exp 2",
        };
        write!(f, "{s}")
    }
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum MidiClock {
    #[default]
    Internal = 0,
    External = 1,
}
