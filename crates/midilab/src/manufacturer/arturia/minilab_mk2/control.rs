use std::fmt;

use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::EnumIter;

use crate::manufacturer::arturia::minilab_mk2::ParamId;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::ButtonMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::ControlChannel;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::KnobMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::ModWheelMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::PadColor;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::PadMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::PitchBendMode;
use crate::manufacturer::arturia::minilab_mk2::control::value_kind::SustainPedalMode;
use crate::manufacturer::arturia::minilab_mk2::error::ButtonParseError;
use crate::manufacturer::arturia::minilab_mk2::error::KnobParseError;
use crate::manufacturer::arturia::minilab_mk2::error::ModWheelParseError;
use crate::manufacturer::arturia::minilab_mk2::error::PadParseError;
use crate::manufacturer::arturia::minilab_mk2::error::PitchBendParseError;
use crate::manufacturer::arturia::minilab_mk2::error::SustainPedalParseError;
use crate::manufacturer::arturia::minilab_mk2::raw::RawControl;
use crate::manufacturer::arturia::minilab_mk2::raw::RawPad;
use crate::midi::Note;
use crate::midi::Value;

pub mod value_kind;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive, IntoPrimitive, EnumIter)]
pub enum ControlId {
    Knob2 = 0x01,
    Knob3 = 0x02,
    Knob4 = 0x03,
    Knob5 = 0x04,
    Knob6 = 0x05,
    Knob7 = 0x06,
    Knob8 = 0x07,
    Knob10 = 0x08,
    Knob11 = 0x09,
    Knob12 = 0x0A,
    Knob13 = 0x0B,
    Knob14 = 0x0C,
    Knob15 = 0x0D,
    Knob16 = 0x0E,
    OctaveMinus = 0x10,
    OctavePlus = 0x11,
    Knob1 = 0x30,
    Knob1Switch = 0x31,
    Knob1Shift = 0x32,
    Knob9 = 0x33,
    Knob9Switch = 0x34,
    Knob9Shift = 0x35,
    ModWheel = 0x40,
    PitchBend = 0x41,
    SustainPedal = 0x50,
    Pad1 = 0x70,
    Pad2 = 0x71,
    Pad3 = 0x72,
    Pad4 = 0x73,
    Pad5 = 0x74,
    Pad6 = 0x75,
    Pad7 = 0x76,
    Pad8 = 0x77,
    Pad9 = 0x78,
    Pad10 = 0x79,
    Pad11 = 0x7A,
    Pad12 = 0x7B,
    Pad13 = 0x7C,
    Pad14 = 0x7D,
    Pad15 = 0x7E,
    Pad16 = 0x7F,
}

impl ControlId {
    pub const KNOBS: [ControlId; 16] = [
        ControlId::Knob1,
        ControlId::Knob2,
        ControlId::Knob3,
        ControlId::Knob4,
        ControlId::Knob5,
        ControlId::Knob6,
        ControlId::Knob7,
        ControlId::Knob8,
        ControlId::Knob9,
        ControlId::Knob10,
        ControlId::Knob11,
        ControlId::Knob12,
        ControlId::Knob13,
        ControlId::Knob14,
        ControlId::Knob15,
        ControlId::Knob16,
    ];

    pub const SHIFT_KNOBS: [ControlId; 2] = [ControlId::Knob1Shift, ControlId::Knob9Shift];

    pub const BUTTONS: [ControlId; 4] = [
        ControlId::Knob1Switch,
        ControlId::Knob9Switch,
        ControlId::OctaveMinus,
        ControlId::OctavePlus,
    ];

    pub const PADS: [ControlId; 16] = [
        ControlId::Pad1,
        ControlId::Pad2,
        ControlId::Pad3,
        ControlId::Pad4,
        ControlId::Pad5,
        ControlId::Pad6,
        ControlId::Pad7,
        ControlId::Pad8,
        ControlId::Pad9,
        ControlId::Pad10,
        ControlId::Pad11,
        ControlId::Pad12,
        ControlId::Pad13,
        ControlId::Pad14,
        ControlId::Pad15,
        ControlId::Pad16,
    ];
}

impl fmt::Display for ControlId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ControlId::Knob1 => "Knob 1",
            ControlId::Knob2 => "Knob 2",
            ControlId::Knob3 => "Knob 3",
            ControlId::Knob4 => "Knob 4",
            ControlId::Knob5 => "Knob 5",
            ControlId::Knob6 => "Knob 6",
            ControlId::Knob7 => "Knob 7",
            ControlId::Knob8 => "Knob 8",
            ControlId::Knob9 => "Knob 9",
            ControlId::Knob10 => "Knob 10",
            ControlId::Knob11 => "Knob 11",
            ControlId::Knob12 => "Knob 12",
            ControlId::Knob13 => "Knob 13",
            ControlId::Knob14 => "Knob 14",
            ControlId::Knob15 => "Knob 15",
            ControlId::Knob16 => "Knob 16",
            ControlId::Knob1Switch => "Knob 1 Switch",
            ControlId::Knob9Switch => "Knob 9 Switch",
            ControlId::Knob1Shift => "Knob 1 + Shift",
            ControlId::Knob9Shift => "Knob 9 + Shift",
            ControlId::OctaveMinus => "Oct -",
            ControlId::OctavePlus => "Oct +",
            ControlId::ModWheel => "Mod Wheel",
            ControlId::PitchBend => "Pitch Bend",
            ControlId::SustainPedal => "Sustain Pedal",
            ControlId::Pad1 => "Pad 1",
            ControlId::Pad2 => "Pad 2",
            ControlId::Pad3 => "Pad 3",
            ControlId::Pad4 => "Pad 4",
            ControlId::Pad5 => "Pad 5",
            ControlId::Pad6 => "Pad 6",
            ControlId::Pad7 => "Pad 7",
            ControlId::Pad8 => "Pad 8",
            ControlId::Pad9 => "Pad 9",
            ControlId::Pad10 => "Pad 10",
            ControlId::Pad11 => "Pad 11",
            ControlId::Pad12 => "Pad 12",
            ControlId::Pad13 => "Pad 13",
            ControlId::Pad14 => "Pad 14",
            ControlId::Pad15 => "Pad 15",
            ControlId::Pad16 => "Pad 16",
        };
        write!(f, "{s}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Knob {
    pub id: ControlId,
    pub mode: KnobMode,
    pub channel: ControlChannel,
    pub cc: Value,
    pub min: Value,
    pub max: Value,
    pub option: Value,
}

impl Knob {
    pub const PARAMS: [ParamId; 6] = [
        ParamId::Mode,
        ParamId::Channel,
        ParamId::Data1,
        ParamId::Data2,
        ParamId::Data3,
        ParamId::Option,
    ];

    pub fn new(id: ControlId) -> Self {
        Self {
            id,
            mode: KnobMode::default(),
            channel: ControlChannel::default(),
            cc: 0.into(),
            min: 0.into(),
            max: 127.into(),
            option: 0.into(),
        }
    }

    pub fn param_pairs(&self) -> Vec<(ParamId, u8)> {
        let raw = RawControl::from(self);
        Self::PARAMS
            .iter()
            .zip(raw.as_bytes())
            .map(|(p, v)| (*p, v))
            .collect()
    }
}

impl From<&Knob> for RawControl {
    fn from(knob: &Knob) -> Self {
        RawControl {
            mode: knob.mode.into(),
            channel: knob.channel.into(),
            data1: knob.cc.into(),
            data2: knob.min.into(),
            data3: knob.max.into(),
            option: knob.option.into(),
        }
    }
}

impl TryFrom<(ControlId, RawControl)> for Knob {
    type Error = KnobParseError;

    fn try_from((id, raw): (ControlId, RawControl)) -> Result<Self, Self::Error> {
        Ok(Knob {
            id,
            mode: KnobMode::try_from(raw.mode).map_err(KnobParseError::Mode)?,
            channel: ControlChannel::try_from(raw.channel).map_err(KnobParseError::Channel)?,
            cc: raw.data1.into(),
            min: raw.data2.into(),
            max: raw.data3.into(),
            option: raw.option.into(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Button {
    pub id: ControlId,
    pub mode: ButtonMode,
    pub channel: ControlChannel,
    pub note: Note,
    pub off_value: Value,
    pub on_value: Value,
    pub option: Value,
}

impl Button {
    pub const PARAMS: [ParamId; 6] = Knob::PARAMS;

    pub fn new(id: ControlId) -> Self {
        Self {
            id,
            mode: ButtonMode::default(),
            channel: ControlChannel::default(),
            note: Note::from(0),
            off_value: 0.into(),
            on_value: 127.into(),
            option: 0.into(),
        }
    }

    pub fn param_pairs(&self) -> Vec<(ParamId, u8)> {
        let raw = RawControl::from(self);
        Self::PARAMS
            .iter()
            .zip(raw.as_bytes())
            .map(|(p, v)| (*p, v))
            .collect()
    }
}

impl From<&Button> for RawControl {
    fn from(button: &Button) -> Self {
        RawControl {
            mode: button.mode.into(),
            channel: button.channel.into(),
            data1: button.note.into(),
            data2: button.off_value.into(),
            data3: button.on_value.into(),
            option: button.option.into(),
        }
    }
}

impl TryFrom<(ControlId, RawControl)> for Button {
    type Error = ButtonParseError;

    fn try_from((id, raw): (ControlId, RawControl)) -> Result<Self, Self::Error> {
        Ok(Button {
            id,
            mode: ButtonMode::try_from(raw.mode).map_err(ButtonParseError::Mode)?,
            channel: ControlChannel::try_from(raw.channel).map_err(ButtonParseError::Channel)?,
            note: raw.data1.into(),
            off_value: raw.data2.into(),
            on_value: raw.data3.into(),
            option: raw.option.into(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModWheel {
    pub mode: ModWheelMode,
    pub channel: ControlChannel,
    pub cc: Value,
    pub min: Value,
    pub max: Value,
    pub option: Value,
}

impl ModWheel {
    pub const PARAMS: [ParamId; 6] = Knob::PARAMS;

    pub fn param_pairs(&self) -> Vec<(ParamId, u8)> {
        let raw = RawControl::from(self);
        Self::PARAMS
            .iter()
            .zip(raw.as_bytes())
            .map(|(p, v)| (*p, v))
            .collect()
    }
}

impl Default for ModWheel {
    fn default() -> Self {
        Self {
            mode: ModWheelMode::default(),
            channel: ControlChannel::default(),
            cc: 1.into(),
            min: 0.into(),
            max: 127.into(),
            option: 0.into(),
        }
    }
}

impl From<&ModWheel> for RawControl {
    fn from(wheel: &ModWheel) -> Self {
        RawControl {
            mode: wheel.mode.into(),
            channel: wheel.channel.into(),
            data1: wheel.cc.into(),
            data2: wheel.min.into(),
            data3: wheel.max.into(),
            option: wheel.option.into(),
        }
    }
}

impl TryFrom<RawControl> for ModWheel {
    type Error = ModWheelParseError;

    fn try_from(raw: RawControl) -> Result<Self, Self::Error> {
        Ok(ModWheel {
            mode: ModWheelMode::try_from(raw.mode).map_err(ModWheelParseError::Mode)?,
            channel: ControlChannel::try_from(raw.channel).map_err(ModWheelParseError::Channel)?,
            cc: raw.data1.into(),
            min: raw.data2.into(),
            max: raw.data3.into(),
            option: raw.option.into(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PitchBend {
    pub mode: PitchBendMode,
    pub channel: ControlChannel,
    pub option: Value,
}

impl PitchBend {
    pub const PARAMS: [ParamId; 3] = [ParamId::Mode, ParamId::Channel, ParamId::Option];

    pub fn param_pairs(&self) -> Vec<(ParamId, u8)> {
        vec![
            (ParamId::Mode, self.mode.into()),
            (ParamId::Channel, self.channel.into()),
            (ParamId::Option, self.option.into()),
        ]
    }
}

impl Default for PitchBend {
    fn default() -> Self {
        Self {
            mode: PitchBendMode::default(),
            channel: ControlChannel::default(),
            option: 0.into(),
        }
    }
}

impl From<&PitchBend> for RawControl {
    fn from(bend: &PitchBend) -> Self {
        RawControl {
            mode: bend.mode.into(),
            channel: bend.channel.into(),
            data1: 0,
            data2: 0,
            data3: 0,
            option: bend.option.into(),
        }
    }
}

impl TryFrom<RawControl> for PitchBend {
    type Error = PitchBendParseError;

    fn try_from(raw: RawControl) -> Result<Self, Self::Error> {
        Ok(PitchBend {
            mode: PitchBendMode::try_from(raw.mode).map_err(PitchBendParseError::Mode)?,
            channel: ControlChannel::try_from(raw.channel).map_err(PitchBendParseError::Channel)?,
            option: raw.option.into(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SustainPedal {
    pub mode: SustainPedalMode,
    pub channel: ControlChannel,
    pub cc: Value,
    pub off_value: Value,
    pub on_value: Value,
    pub option: Value,
}

impl SustainPedal {
    pub const PARAMS: [ParamId; 6] = Knob::PARAMS;

    pub fn param_pairs(&self) -> Vec<(ParamId, u8)> {
        let raw = RawControl::from(self);
        Self::PARAMS
            .iter()
            .zip(raw.as_bytes())
            .map(|(p, v)| (*p, v))
            .collect()
    }
}

impl Default for SustainPedal {
    fn default() -> Self {
        Self {
            mode: SustainPedalMode::default(),
            channel: ControlChannel::default(),
            cc: 64.into(),
            off_value: 0.into(),
            on_value: 127.into(),
            option: 0.into(),
        }
    }
}

impl From<&SustainPedal> for RawControl {
    fn from(pedal: &SustainPedal) -> Self {
        RawControl {
            mode: pedal.mode.into(),
            channel: pedal.channel.into(),
            data1: pedal.cc.into(),
            data2: pedal.off_value.into(),
            data3: pedal.on_value.into(),
            option: pedal.option.into(),
        }
    }
}

impl TryFrom<RawControl> for SustainPedal {
    type Error = SustainPedalParseError;

    fn try_from(raw: RawControl) -> Result<Self, Self::Error> {
        Ok(SustainPedal {
            mode: SustainPedalMode::try_from(raw.mode).map_err(SustainPedalParseError::Mode)?,
            channel: ControlChannel::try_from(raw.channel)
                .map_err(SustainPedalParseError::Channel)?,
            cc: raw.data1.into(),
            off_value: raw.data2.into(),
            on_value: raw.data3.into(),
            option: raw.option.into(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pad {
    pub id: ControlId,
    pub mode: PadMode,
    pub channel: ControlChannel,
    pub note: Note,
    pub off_value: Value,
    pub on_value: Value,
    pub option: Value,
    pub color: PadColor,
}

impl Pad {
    pub const PARAMS: [ParamId; 7] = [
        ParamId::Mode,
        ParamId::Channel,
        ParamId::Data1,
        ParamId::Data2,
        ParamId::Data3,
        ParamId::Option,
        ParamId::PadColor,
    ];

    pub fn new(id: ControlId) -> Self {
        Self {
            id,
            mode: PadMode::default(),
            channel: ControlChannel::default(),
            note: Note::from(36),
            off_value: 0.into(),
            on_value: 127.into(),
            option: 0.into(),
            color: PadColor::default(),
        }
    }

    pub fn param_pairs(&self) -> Vec<(ParamId, u8)> {
        let raw = RawPad::from(self);
        Self::PARAMS
            .iter()
            .zip(raw.as_bytes())
            .map(|(p, v)| (*p, v))
            .collect()
    }
}

impl From<&Pad> for RawPad {
    fn from(pad: &Pad) -> Self {
        RawPad {
            mode: pad.mode.into(),
            channel: pad.channel.into(),
            data1: pad.note.into(),
            data2: pad.off_value.into(),
            data3: pad.on_value.into(),
            option: pad.option.into(),
            color: pad.color.into(),
        }
    }
}

impl TryFrom<(ControlId, RawPad)> for Pad {
    type Error = PadParseError;

    fn try_from((id, raw): (ControlId, RawPad)) -> Result<Self, Self::Error> {
        Ok(Pad {
            id,
            mode: PadMode::try_from(raw.mode).map_err(PadParseError::Mode)?,
            channel: ControlChannel::try_from(raw.channel).map_err(PadParseError::Channel)?,
            note: raw.data1.into(),
            off_value: raw.data2.into(),
            on_value: raw.data3.into(),
            option: raw.option.into(),
            color: PadColor::try_from(raw.color).map_err(PadParseError::Color)?,
        })
    }
}
