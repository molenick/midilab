use std::fmt;
use std::ops::Deref;

use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::Display;
use strum_macros::EnumIter;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum ControlChannel {
    Ch1 = 0,
    Ch2 = 1,
    Ch3 = 2,
    Ch4 = 3,
    Ch5 = 4,
    Ch6 = 5,
    Ch7 = 6,
    Ch8 = 7,
    Ch9 = 8,
    Ch10 = 9,
    Ch11 = 10,
    Ch12 = 11,
    Ch13 = 12,
    Ch14 = 13,
    Ch15 = 14,
    Ch16 = 15,
    #[default]
    Keyboard = 0x41,
}

impl fmt::Display for ControlChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlChannel::Keyboard => write!(f, "Keyboard"),
            ch => write!(f, "{}", *ch as u8 + 1),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum MidiChannel {
    #[default]
    Ch1 = 0,
    Ch2 = 1,
    Ch3 = 2,
    Ch4 = 3,
    Ch5 = 4,
    Ch6 = 5,
    Ch7 = 6,
    Ch8 = 7,
    Ch9 = 8,
    Ch10 = 9,
    Ch11 = 10,
    Ch12 = 11,
    Ch13 = 12,
    Ch14 = 13,
    Ch15 = 14,
    Ch16 = 15,
}

impl fmt::Display for MidiChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8 + 1)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum KnobMode {
    Off = 0,
    #[default]
    Control = 1,
    Nrpn = 4,
}

impl fmt::Display for KnobMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            KnobMode::Off => "Off",
            KnobMode::Control => "Control",
            KnobMode::Nrpn => "NRPN/RPN",
        };
        write!(f, "{s}")
    }
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum ButtonMode {
    #[default]
    Off = 0,
    #[strum(serialize = "Switched Control")]
    SwitchedControl = 8,
    Note = 9,
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum PadMode {
    Off = 0,
    #[strum(serialize = "MMC")]
    Mmc = 7,
    #[strum(serialize = "Switched Control")]
    SwitchedControl = 8,
    #[default]
    Note = 9,
    #[strum(serialize = "Patch Change")]
    PatchChange = 11,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum ModWheelMode {
    Off = 0,
    #[default]
    Control = 1,
    Nrpn = 4,
    Aftertouch = 14,
}

impl fmt::Display for ModWheelMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ModWheelMode::Off => "Off",
            ModWheelMode::Control => "Control",
            ModWheelMode::Nrpn => "NRPN/RPN",
            ModWheelMode::Aftertouch => "Aftertouch",
        };
        write!(f, "{s}")
    }
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum PitchBendMode {
    Off = 0,
    #[default]
    #[strum(serialize = "Pitch Bend")]
    PitchBend = 16,
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum SustainPedalMode {
    Off = 0,
    Control = 1,
    #[default]
    #[strum(serialize = "Switched Control")]
    SwitchedControl = 8,
    Note = 9,
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum KnobOption {
    #[default]
    Absolute = 0,
    #[strum(serialize = "Relative #1")]
    Relative1 = 1,
    #[strum(serialize = "Relative #2")]
    Relative2 = 2,
    #[strum(serialize = "Relative #3")]
    Relative3 = 3,
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
#[allow(clippy::upper_case_acronyms)]
pub enum NrpnRpn {
    #[default]
    NRPN = 0,
    RPN = 1,
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum SwitchBehavior {
    #[default]
    Toggle = 0,
    Gate = 1,
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum PitchBendOption {
    #[default]
    Standard = 0,
    Hold = 1,
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum PadColor {
    #[default]
    #[strum(serialize = "No Color")]
    NoColor = 0,
    Red = 1,
    Green = 4,
    Yellow = 5,
    Blue = 16,
    Purple = 17,
    Cyan = 20,
    White = 127,
}

impl PadColor {
    pub fn as_rgb_color(&self) -> RGBColor {
        match self {
            Self::NoColor => RGBColor((0, 0, 0)),
            Self::Red => RGBColor((255, 0, 0)),
            Self::Green => RGBColor((0, 255, 0)),
            Self::Yellow => RGBColor((255, 255, 0)),
            Self::Blue => RGBColor((0, 0, 255)),
            Self::Purple => RGBColor((255, 0, 255)),
            Self::Cyan => RGBColor((0, 255, 255)),
            Self::White => RGBColor((255, 255, 255)),
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum VelocityCurve {
    #[default]
    Linear = 0,
    Logarithmic = 1,
    Exponential = 2,
    Full = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum KnobAcceleration {
    #[default]
    Slow = 0,
    Medium = 1,
    Fast = 2,
}

impl fmt::Display for KnobAcceleration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            KnobAcceleration::Slow => "Slow (Off)",
            KnobAcceleration::Medium => "Medium",
            KnobAcceleration::Fast => "Fast",
        };
        write!(f, "{s}")
    }
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum ToggleState {
    #[default]
    Off = 0,
    On = 0x7F,
}

#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum MemorySlot {
    #[default]
    Slot1 = 1,
    Slot2 = 2,
    Slot3 = 3,
    Slot4 = 4,
    Slot5 = 5,
    Slot6 = 6,
    Slot7 = 7,
    Slot8 = 8,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum DataEntryResolution {
    #[default]
    Coarse128 = 0,
    Res64 = 1,
    Res32 = 2,
    Res16 = 3,
    Res8 = 4,
    Res4 = 5,
    Res2 = 6,
    Fine1 = 7,
}

impl fmt::Display for DataEntryResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DataEntryResolution::Coarse128 => "1:128 (coarse)",
            DataEntryResolution::Res64 => "1:64",
            DataEntryResolution::Res32 => "1:32",
            DataEntryResolution::Res16 => "1:16",
            DataEntryResolution::Res8 => "1:8",
            DataEntryResolution::Res4 => "1:4",
            DataEntryResolution::Res2 => "1:2",
            DataEntryResolution::Fine1 => "1:1 (fine)",
        };
        write!(f, "{s}")
    }
}
