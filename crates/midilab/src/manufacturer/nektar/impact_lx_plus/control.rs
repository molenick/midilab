use std::fmt;

use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::EnumIter;

use crate::manufacturer::nektar::impact_lx_plus::error::ControlParseError;
use crate::manufacturer::nektar::impact_lx_plus::raw::RawControl;
use crate::manufacturer::nektar::impact_lx_plus::raw::RawPad;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::ButtonKind;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::ContinuousKind;
use crate::manufacturer::nektar::impact_lx_plus::value_kind::ControlChannel;
use crate::midi::Note;
use crate::midi::Value;

/// User preset slot (message section byte for preset control messages).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum PresetId {
    Preset1 = 1,
    Preset2 = 2,
    Preset3 = 3,
    Preset4 = 4,
    Preset5 = 5,
}

impl fmt::Display for PresetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Preset {}", *self as u8)
    }
}

/// Pad map slot (message section byte for pad messages).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum PadMapId {
    Map1 = 1,
    Map2 = 2,
    Map3 = 3,
    Map4 = 4,
}

impl fmt::Display for PadMapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pad Map {}", *self as u8)
    }
}

/// Control id within a preset. Confirmed against the user guide's factory
/// preset tables (CC-for-CC match).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum PresetControlId {
    Fader1 = 0x02,
    Fader2 = 0x03,
    Fader3 = 0x04,
    Fader4 = 0x05,
    Fader5 = 0x06,
    Fader6 = 0x07,
    Fader7 = 0x08,
    Fader8 = 0x09,
    Fader9 = 0x0A,
    Pot1 = 0x0B,
    Pot2 = 0x0C,
    Pot3 = 0x0D,
    Pot4 = 0x0E,
    Pot5 = 0x0F,
    Pot6 = 0x10,
    Pot7 = 0x11,
    Pot8 = 0x12,
    FaderButton1 = 0x13,
    FaderButton2 = 0x14,
    FaderButton3 = 0x15,
    FaderButton4 = 0x16,
    FaderButton5 = 0x17,
    FaderButton6 = 0x18,
    FaderButton7 = 0x19,
    FaderButton8 = 0x1A,
    FaderButton9 = 0x1B,
}

impl PresetControlId {
    pub const FADERS: [PresetControlId; 9] = [
        PresetControlId::Fader1,
        PresetControlId::Fader2,
        PresetControlId::Fader3,
        PresetControlId::Fader4,
        PresetControlId::Fader5,
        PresetControlId::Fader6,
        PresetControlId::Fader7,
        PresetControlId::Fader8,
        PresetControlId::Fader9,
    ];

    pub const POTS: [PresetControlId; 8] = [
        PresetControlId::Pot1,
        PresetControlId::Pot2,
        PresetControlId::Pot3,
        PresetControlId::Pot4,
        PresetControlId::Pot5,
        PresetControlId::Pot6,
        PresetControlId::Pot7,
        PresetControlId::Pot8,
    ];

    pub const FADER_BUTTONS: [PresetControlId; 9] = [
        PresetControlId::FaderButton1,
        PresetControlId::FaderButton2,
        PresetControlId::FaderButton3,
        PresetControlId::FaderButton4,
        PresetControlId::FaderButton5,
        PresetControlId::FaderButton6,
        PresetControlId::FaderButton7,
        PresetControlId::FaderButton8,
        PresetControlId::FaderButton9,
    ];
}

impl fmt::Display for PresetControlId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = *self as u8;
        if let Some(index) = Self::FADERS.iter().position(|c| c == self) {
            write!(f, "Fader {}", index + 1)
        } else if let Some(index) = Self::POTS.iter().position(|c| c == self) {
            write!(f, "Pot {}", index + 1)
        } else if let Some(index) = Self::FADER_BUTTONS.iter().position(|c| c == self) {
            write!(f, "Fader Button {}", index + 1)
        } else {
            write!(f, "Control {id:#04x}")
        }
    }
}

/// Pad id within a pad map. The physical bottom row is pads 1–4, the top row
/// pads 5–8.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum PadId {
    Pad1 = 0x03,
    Pad2 = 0x04,
    Pad3 = 0x05,
    Pad4 = 0x06,
    Pad5 = 0x07,
    Pad6 = 0x08,
    Pad7 = 0x09,
    Pad8 = 0x0A,
}

impl PadId {
    pub const ALL: [PadId; 8] = [
        PadId::Pad1,
        PadId::Pad2,
        PadId::Pad3,
        PadId::Pad4,
        PadId::Pad5,
        PadId::Pad6,
        PadId::Pad7,
        PadId::Pad8,
    ];
}

impl fmt::Display for PadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pad {}", *self as u8 - 0x02)
    }
}

/// Control id within the global block. Wheels, foot switch and transport
/// buttons live here rather than in presets, so they survive preset switches.
///
/// The mapping of `Transport1`–`Transport6` to physical transport buttons
/// (factory ch 16, CC 102–107) has not been individually verified; the wheels
/// and foot switch are HIL-confirmed.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum GlobalControlId {
    Transport1 = 0x02,
    Transport2 = 0x03,
    Transport3 = 0x04,
    Transport4 = 0x05,
    Transport5 = 0x06,
    Transport6 = 0x07,
    PitchWheel = 0x08,
    ModWheel = 0x09,
    FootSwitch = 0x0A,
}

impl GlobalControlId {
    pub const TRANSPORT: [GlobalControlId; 6] = [
        GlobalControlId::Transport1,
        GlobalControlId::Transport2,
        GlobalControlId::Transport3,
        GlobalControlId::Transport4,
        GlobalControlId::Transport5,
        GlobalControlId::Transport6,
    ];
}

impl fmt::Display for GlobalControlId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GlobalControlId::PitchWheel => write!(f, "Pitch Wheel"),
            GlobalControlId::ModWheel => write!(f, "Mod Wheel"),
            GlobalControlId::FootSwitch => write!(f, "Foot Switch"),
            transport => write!(f, "Transport {}", *transport as u8 - 0x01),
        }
    }
}

/// A continuous control (fader, pot or wheel).
///
/// `min`/`max` rescale the control's full physical travel into `[min, max]`;
/// they do not clamp.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Continuous {
    pub channel: ControlChannel,
    pub kind: ContinuousKind,
    pub cc: Value,
    pub min: Value,
    pub max: Value,
}

impl Default for Continuous {
    fn default() -> Self {
        Self {
            channel: ControlChannel::default(),
            kind: ContinuousKind::default(),
            cc: 0.into(),
            min: 0.into(),
            max: 127.into(),
        }
    }
}

impl Continuous {
    pub fn with_cc(cc: u8) -> Self {
        Self {
            cc: cc.into(),
            ..Self::default()
        }
    }
}

impl From<&Continuous> for RawControl {
    fn from(control: &Continuous) -> Self {
        RawControl {
            channel: control.channel.into(),
            kind: control.kind.into(),
            data1: control.cc.into(),
            min: control.min.into(),
            max: control.max.into(),
        }
    }
}

impl TryFrom<RawControl> for Continuous {
    type Error = ControlParseError;

    fn try_from(raw: RawControl) -> Result<Self, Self::Error> {
        Ok(Continuous {
            channel: ControlChannel::try_from(raw.channel).map_err(ControlParseError::Channel)?,
            kind: ContinuousKind::try_from(raw.kind).map_err(ControlParseError::ContinuousKind)?,
            cc: raw.data1.into(),
            min: raw.min.into(),
            max: raw.max.into(),
        })
    }
}

/// A button-like control (fader button, transport button or foot switch).
///
/// `data1` is the CC number for CC kinds and the note number for note kinds.
/// `min`/`max` are the release/press values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Button {
    pub channel: ControlChannel,
    pub kind: ButtonKind,
    pub data1: Value,
    pub min: Value,
    pub max: Value,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            channel: ControlChannel::default(),
            kind: ButtonKind::default(),
            data1: 0.into(),
            min: 0.into(),
            max: 127.into(),
        }
    }
}

impl Button {
    pub fn with_cc(cc: u8) -> Self {
        Self {
            data1: cc.into(),
            ..Self::default()
        }
    }
}

impl From<&Button> for RawControl {
    fn from(button: &Button) -> Self {
        RawControl {
            channel: button.channel.into(),
            kind: button.kind.into(),
            data1: button.data1.into(),
            min: button.min.into(),
            max: button.max.into(),
        }
    }
}

impl TryFrom<RawControl> for Button {
    type Error = ControlParseError;

    fn try_from(raw: RawControl) -> Result<Self, Self::Error> {
        Ok(Button {
            channel: ControlChannel::try_from(raw.channel).map_err(ControlParseError::Channel)?,
            kind: ButtonKind::try_from(raw.kind).map_err(ControlParseError::ButtonKind)?,
            data1: raw.data1.into(),
            min: raw.min.into(),
            max: raw.max.into(),
        })
    }
}

/// A drum pad. Behaves like a [`Button`] with an additional note number
/// (TLV param `0x06`) used when the pad is in a note kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pad {
    pub channel: ControlChannel,
    pub kind: ButtonKind,
    pub data1: Value,
    pub min: Value,
    pub max: Value,
    pub note: Note,
}

impl Default for Pad {
    fn default() -> Self {
        Self {
            channel: ControlChannel::default(),
            kind: ButtonKind::Note,
            data1: 0.into(),
            min: 0.into(),
            max: 127.into(),
            note: Note::from(36),
        }
    }
}

impl Pad {
    pub fn with_note(note: u8) -> Self {
        Self {
            note: Note::from(note),
            ..Self::default()
        }
    }
}

impl From<&Pad> for RawPad {
    fn from(pad: &Pad) -> Self {
        RawPad {
            channel: pad.channel.into(),
            kind: pad.kind.into(),
            data1: pad.data1.into(),
            min: pad.min.into(),
            max: pad.max.into(),
            note: pad.note.into(),
        }
    }
}

impl TryFrom<RawPad> for Pad {
    type Error = ControlParseError;

    fn try_from(raw: RawPad) -> Result<Self, Self::Error> {
        Ok(Pad {
            channel: ControlChannel::try_from(raw.channel).map_err(ControlParseError::Channel)?,
            kind: ButtonKind::try_from(raw.kind).map_err(ControlParseError::ButtonKind)?,
            data1: raw.data1.into(),
            min: raw.min.into(),
            max: raw.max.into(),
            note: raw.note.into(),
        })
    }
}
