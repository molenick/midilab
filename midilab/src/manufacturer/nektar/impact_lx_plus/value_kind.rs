use std::fmt;

use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::Display;
use strum_macros::EnumIter;

/// Channel assignment for a single control. `0` means the control follows the
/// device's global MIDI channel; `1`–`16` pin it to a fixed channel.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum ControlChannel {
    #[default]
    Follow = 0,
    Ch1 = 1,
    Ch2 = 2,
    Ch3 = 3,
    Ch4 = 4,
    Ch5 = 5,
    Ch6 = 6,
    Ch7 = 7,
    Ch8 = 8,
    Ch9 = 9,
    Ch10 = 10,
    Ch11 = 11,
    Ch12 = 12,
    Ch13 = 13,
    Ch14 = 14,
    Ch15 = 15,
    Ch16 = 16,
}

impl fmt::Display for ControlChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlChannel::Follow => write!(f, "Global"),
            ch => write!(f, "{}", *ch as u8),
        }
    }
}

/// The device's global MIDI channel (global setting `0x01`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum MidiChannel {
    #[default]
    Ch1 = 1,
    Ch2 = 2,
    Ch3 = 3,
    Ch4 = 4,
    Ch5 = 5,
    Ch6 = 6,
    Ch7 = 7,
    Ch8 = 8,
    Ch9 = 9,
    Ch10 = 10,
    Ch11 = 11,
    Ch12 = 12,
    Ch13 = 13,
    Ch14 = 14,
    Ch15 = 15,
    Ch16 = 16,
}

impl fmt::Display for MidiChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

/// Assignment type for continuous controls (faders, pots, wheels).
#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum ContinuousKind {
    #[default]
    #[strum(serialize = "MIDI CC")]
    Cc = 0,
    #[strum(serialize = "Pitch Bend")]
    PitchBend = 1,
}

/// Assignment type for button-like controls (fader buttons, transport
/// buttons, foot switch, pads).
///
/// `NoteToggle` and `Mmc` are inferred from the user guide's option order;
/// their stored values round-trip on hardware but their live behavior has not
/// been verified.
#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive, IntoPrimitive, EnumIter, Display,
)]
pub enum ButtonKind {
    #[default]
    #[strum(serialize = "MIDI CC (Toggle)")]
    CcToggle = 0,
    #[strum(serialize = "MIDI CC (Momentary)")]
    CcMomentary = 1,
    #[strum(serialize = "Note")]
    Note = 2,
    #[strum(serialize = "Note (Toggle)")]
    NoteToggle = 3,
    #[strum(serialize = "MMC")]
    Mmc = 4,
}
