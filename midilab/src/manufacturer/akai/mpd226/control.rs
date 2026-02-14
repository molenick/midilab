use crate::manufacturer::akai::mpd226::control::value_kind::ActiveState;
use crate::manufacturer::akai::mpd226::control::value_kind::AfterTouchKind;
use crate::manufacturer::akai::mpd226::control::value_kind::DialKind;
use crate::manufacturer::akai::mpd226::control::value_kind::FaderKind;
use crate::manufacturer::akai::mpd226::control::value_kind::Gate;
use crate::manufacturer::akai::mpd226::control::value_kind::KeyModifier;
use crate::manufacturer::akai::mpd226::control::value_kind::MidiChannel;
use crate::manufacturer::akai::mpd226::control::value_kind::PadColor;
use crate::manufacturer::akai::mpd226::control::value_kind::PadKind;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetName;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetSlot;
use crate::manufacturer::akai::mpd226::control::value_kind::SwingKind;
use crate::manufacturer::akai::mpd226::control::value_kind::SwitchKind;
use crate::manufacturer::akai::mpd226::control::value_kind::Tempo;
use crate::manufacturer::akai::mpd226::control::value_kind::TimeDivision;
use crate::manufacturer::akai::mpd226::control::value_kind::TransportKind;
use crate::manufacturer::akai::mpd226::control::value_kind::TriggerKind;
use crate::manufacturer::akai::mpd226::raw::RawDial;
use crate::manufacturer::akai::mpd226::raw::RawFader;
use crate::manufacturer::akai::mpd226::raw::RawPad;
use crate::manufacturer::akai::mpd226::raw::RawSwitch;
use crate::midi::MidiValue;
use crate::midi::Note;

pub mod value_kind;

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Pad {
    pub id: usize,
    pub kind: PadKind,
    pub channel: MidiChannel,
    pub note: Note,
    pub midi2din: ActiveState,
    pub trigger: TriggerKind,
    pub aftertouch: AfterTouchKind,
    pub program: MidiValue,
    pub msb: MidiValue,
    pub lsb: MidiValue,
    pub off_color: PadColor,
    pub on_color: PadColor,
}

impl Pad {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn sysex_payload(&self) -> Vec<u8> {
        self.as_bytes()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        vec![
            self.kind as u8,
            self.channel as u8,
            self.note as u8,
            self.midi2din as u8,
            self.trigger as u8,
            self.aftertouch as u8,
            self.program.into(),
            self.msb.into(),
            self.lsb.into(),
            self.off_color as u8,
            self.on_color as u8,
        ]
    }
}

impl TryFrom<(usize, RawPad)> for Pad {
    type Error = super::error::PadParseError;

    fn try_from(value: (usize, RawPad)) -> Result<Self, Self::Error> {
        use super::error::PadParseError;
        let (index, raw) = value;
        Ok(Pad {
            id: index,
            kind: PadKind::try_from(raw.kind).map_err(PadParseError::Kind)?,
            channel: MidiChannel::try_from(raw.channel).map_err(PadParseError::Channel)?,
            note: Note::try_from(raw.note).map_err(PadParseError::Note)?,
            midi2din: ActiveState::try_from(raw.midi2din).map_err(PadParseError::Midi2Din)?,
            trigger: TriggerKind::try_from(raw.trigger).map_err(PadParseError::Trigger)?,
            aftertouch: AfterTouchKind::try_from(raw.aftertouch)
                .map_err(PadParseError::Aftertouch)?,
            program: raw.program.into(),
            msb: raw.msb.into(),
            lsb: raw.lsb.into(),
            off_color: PadColor::try_from(raw.off_color).map_err(PadParseError::OffColor)?,
            on_color: PadColor::try_from(raw.on_color).map_err(PadParseError::OnColor)?,
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dial {
    pub kind: DialKind,
    pub channel: MidiChannel,
    pub midicc: MidiValue,
    pub min: MidiValue,
    pub max: MidiValue,
    pub midi2din: ActiveState,
    pub msb: MidiValue,
    pub lsb: MidiValue,
    pub value: MidiValue,
}

impl Default for Dial {
    fn default() -> Self {
        Self {
            kind: DialKind::default(),
            channel: MidiChannel::default(),
            midicc: 0.into(),
            min: 0.into(),
            max: 127.into(),
            midi2din: ActiveState::default(),
            msb: 0.into(),
            lsb: 0.into(),
            value: 0.into(),
        }
    }
}

impl Dial {
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![
            self.kind as u8,
            self.channel as u8,
            self.midicc.into(),
            self.min.into(),
            self.max.into(),
            self.midi2din as u8,
            self.msb.into(),
            self.lsb.into(),
            self.value.into(),
        ]
    }
}

impl TryFrom<RawDial> for Dial {
    type Error = super::error::DialParseError;

    fn try_from(raw: RawDial) -> Result<Self, Self::Error> {
        use super::error::DialParseError;
        Ok(Dial {
            kind: DialKind::try_from(raw.kind).map_err(DialParseError::Kind)?,
            channel: MidiChannel::try_from(raw.channel).map_err(DialParseError::Channel)?,
            midicc: raw.midicc.into(),
            min: raw.min.into(),
            max: raw.max.into(),
            midi2din: ActiveState::try_from(raw.midi2din).map_err(DialParseError::Midi2Din)?,
            msb: raw.msb.into(),
            lsb: raw.lsb.into(),
            value: raw.value.into(),
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fader {
    pub kind: FaderKind,
    pub channel: MidiChannel,
    pub midicc: MidiValue,
    pub min: MidiValue,
    pub max: MidiValue,
    pub midi2din: ActiveState,
}

impl Default for Fader {
    fn default() -> Self {
        Self {
            kind: FaderKind::default(),
            channel: MidiChannel::default(),
            midicc: 0.into(),
            min: 0.into(),
            max: 127.into(),
            midi2din: ActiveState::default(),
        }
    }
}

impl Fader {
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![
            self.kind as u8,
            self.channel as u8,
            self.midicc.into(),
            self.min.into(),
            self.max.into(),
            self.midi2din as u8,
        ]
    }
}

impl TryFrom<RawFader> for Fader {
    type Error = super::error::FaderParseError;

    fn try_from(raw: RawFader) -> Result<Self, Self::Error> {
        use super::error::FaderParseError;
        Ok(Fader {
            kind: FaderKind::try_from(raw.kind).map_err(FaderParseError::Kind)?,
            channel: MidiChannel::try_from(raw.channel).map_err(FaderParseError::Channel)?,
            midicc: raw.midicc.into(),
            min: raw.min.into(),
            max: raw.max.into(),
            midi2din: ActiveState::try_from(raw.midi2din).map_err(FaderParseError::Midi2Din)?,
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Switch {
    pub kind: SwitchKind,
    pub channel: MidiChannel,
    pub midicc: MidiValue,
    pub mode: TriggerKind,
    pub prog: MidiValue,
    pub msb: MidiValue,
    pub lsb: MidiValue,
    pub midi2din: ActiveState,
    pub note: u8,
    pub velo: MidiValue,
    pub invert: ActiveState,
    pub key1: u8,
    pub key2: KeyModifier,
}

impl Default for Switch {
    fn default() -> Self {
        Self {
            kind: SwitchKind::default(),
            channel: MidiChannel::default(),
            midicc: 0.into(),
            mode: TriggerKind::default(),
            prog: 0.into(),
            msb: 0.into(),
            lsb: 0.into(),
            midi2din: ActiveState::default(),
            note: 0,
            velo: 100.into(),
            invert: ActiveState::default(),
            key1: 0,
            key2: KeyModifier::default(),
        }
    }
}

impl Switch {
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![
            self.kind as u8,
            self.channel as u8,
            self.midicc.into(),
            self.mode as u8,
            self.prog.into(),
            self.msb.into(),
            self.lsb.into(),
            self.midi2din as u8,
            self.note,
            self.velo.into(),
            self.invert as u8,
            self.key1,
            self.key2 as u8,
        ]
    }
}

impl TryFrom<RawSwitch> for Switch {
    type Error = super::error::SwitchParseError;

    fn try_from(raw: RawSwitch) -> Result<Self, Self::Error> {
        use super::error::SwitchParseError;
        Ok(Switch {
            kind: SwitchKind::try_from(raw.kind).map_err(SwitchParseError::Kind)?,
            channel: MidiChannel::try_from(raw.channel).map_err(SwitchParseError::Channel)?,
            midicc: raw.midicc.into(),
            mode: TriggerKind::try_from(raw.mode).map_err(SwitchParseError::Mode)?,
            prog: raw.prog.into(),
            msb: raw.msb.into(),
            lsb: raw.lsb.into(),
            midi2din: ActiveState::try_from(raw.midi2din).map_err(SwitchParseError::Midi2Din)?,
            note: raw.note,
            velo: raw.velo.into(),
            invert: ActiveState::try_from(raw.invert).map_err(SwitchParseError::Invert)?,
            key1: raw.key1,
            key2: KeyModifier::try_from(raw.key2).map_err(SwitchParseError::Key2)?,
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresetSettings {
    pub preset_slot: PresetSlot,
    pub preset_name: PresetName,
    pub tempo: Tempo,
    pub time_division_switch: TriggerKind,
    pub time_division: TimeDivision,
    pub note_repeat_switch: TriggerKind,
    pub gate: Gate,
    pub swing: SwingKind,
    pub transport: TransportKind,
}

impl Default for PresetSettings {
    fn default() -> Self {
        Self {
            preset_slot: PresetSlot::default(),
            preset_name: PresetName::default(),
            tempo: Tempo::default(),
            time_division_switch: TriggerKind::Toggle,
            time_division: TimeDivision::default(),
            note_repeat_switch: TriggerKind::Toggle,
            gate: Gate::from(50),
            swing: SwingKind::default(),
            transport: TransportKind::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manufacturer::akai::mpd226::raw::RawDial;
    use crate::manufacturer::akai::mpd226::raw::RawFader;
    use crate::manufacturer::akai::mpd226::raw::RawPad;
    use crate::manufacturer::akai::mpd226::raw::RawSwitch;

    mod pad {
        use super::*;

        #[test]
        fn test_pad_new() {
            let pad = Pad::new(5);
            assert_eq!(pad.id, 5);
            assert_eq!(pad.kind, PadKind::default());
            assert_eq!(pad.note, Note::default());
        }

        #[test]
        fn test_pad_as_bytes() {
            let pad = Pad {
                id: 0,
                kind: PadKind::Note,
                channel: MidiChannel::COMMON,
                note: Note::N60,
                midi2din: ActiveState::Off,
                trigger: TriggerKind::Momentary,
                aftertouch: AfterTouchKind::Channel,
                program: MidiValue::default(),
                msb: MidiValue::default(),
                lsb: MidiValue::default(),
                off_color: PadColor::Red,
                on_color: PadColor::Green,
            };

            let bytes = pad.as_bytes();
            assert_eq!(bytes.len(), 11);
            assert_eq!(bytes[0], PadKind::Note as u8);
            assert_eq!(bytes[1], MidiChannel::COMMON as u8);
            assert_eq!(bytes[2], Note::N60 as u8);
            assert_eq!(bytes[9], PadColor::Red as u8);
            assert_eq!(bytes[10], PadColor::Green as u8);
        }

        #[test]
        fn test_pad_try_from_raw() {
            let raw = RawPad {
                kind: 0,
                channel: 5,
                note: 72,
                midi2din: 0,
                trigger: 1,
                aftertouch: 2,
                program: 10,
                msb: 0,
                lsb: 0,
                off_color: 3,
                on_color: 5,
            };

            let pad = Pad::try_from((3, raw)).unwrap();
            assert_eq!(pad.id, 3);
            assert_eq!(pad.kind, PadKind::Note);
            assert_eq!(pad.channel, MidiChannel::A5);
            assert_eq!(pad.note, Note::N72);
            assert_eq!(pad.trigger, TriggerKind::Toggle);
            assert_eq!(pad.aftertouch, AfterTouchKind::Poly);
            assert_eq!(pad.off_color, PadColor::Amber);
            assert_eq!(pad.on_color, PadColor::Green);
        }

        #[test]
        fn test_pad_try_from_raw_invalid_kind() {
            let raw = RawPad {
                kind: 255,
                channel: 0,
                note: 60,
                midi2din: 0,
                trigger: 0,
                aftertouch: 0,
                program: 0,
                msb: 0,
                lsb: 0,
                off_color: 0,
                on_color: 0,
            };

            let result = Pad::try_from((0, raw));
            assert!(result.is_err());
        }

        #[test]
        fn test_pad_sysex_payload() {
            let pad = Pad::new(0);
            let payload = pad.sysex_payload();
            assert_eq!(payload.len(), 11);
        }
    }

    mod dial {
        use super::*;
        #[test]
        fn test_dial_default() {
            let dial = Dial::default();
            assert_eq!(dial.kind, DialKind::CC);
            assert_eq!(dial.channel, MidiChannel::COMMON);
        }

        #[test]
        fn test_dial_as_bytes() {
            let dial = Dial {
                kind: DialKind::CC,
                channel: MidiChannel::A1,
                midicc: 74.into(),
                min: 0.into(),
                max: 127.into(),
                midi2din: ActiveState::On,
                msb: 0.into(),
                lsb: 0.into(),
                value: 0.into(),
            };

            let bytes = dial.as_bytes();
            assert_eq!(bytes.len(), 9);
            assert_eq!(bytes[0], DialKind::CC as u8);
            assert_eq!(bytes[1], 1);
            assert_eq!(bytes[2], 74);
            assert_eq!(bytes[3], 0);
            assert_eq!(bytes[4], 127);
        }

        #[test]
        fn test_dial_try_from_raw() {
            let raw = RawDial {
                kind: 2,
                channel: 3,
                midicc: 50,
                min: 10,
                max: 100,
                midi2din: 1,
                msb: 5,
                lsb: 6,
                value: 64,
            };

            let dial = Dial::try_from(raw).unwrap();
            assert_eq!(dial.kind, DialKind::IncDec1);
            assert_eq!(dial.channel, MidiChannel::A3);
            assert_eq!(dial.midicc, 50.into());
            assert_eq!(dial.midi2din, ActiveState::On);
        }

        #[test]
        fn test_dial_try_from_raw_invalid() {
            let raw = RawDial {
                kind: 255,
                channel: 0,
                midicc: 0,
                min: 0,
                max: 127,
                midi2din: 0,
                msb: 0,
                lsb: 0,
                value: 0,
            };

            let result = Dial::try_from(raw);
            assert!(result.is_err());
        }
    }

    mod fader {
        use super::*;
        #[test]
        fn test_fader_default() {
            let fader = Fader::default();
            assert_eq!(fader.kind, FaderKind::CC);
        }

        #[test]
        fn test_fader_as_bytes() {
            let fader = Fader {
                kind: FaderKind::Aftertouch,
                channel: MidiChannel::A2,
                midicc: 7.into(),
                min: 0.into(),
                max: 127.into(),
                midi2din: ActiveState::Off,
            };

            let bytes = fader.as_bytes();
            assert_eq!(bytes.len(), 6);
            assert_eq!(bytes[0], FaderKind::Aftertouch as u8);
            assert_eq!(bytes[1], MidiChannel::A2 as u8);
            assert_eq!(bytes[2], 7);
        }

        #[test]
        fn test_fader_try_from_raw() {
            let raw = RawFader {
                kind: 1,
                channel: 5,
                midicc: 11,
                min: 20,
                max: 100,
                midi2din: 0,
            };

            let fader = Fader::try_from(raw).unwrap();
            assert_eq!(fader.kind, FaderKind::Aftertouch);
            assert_eq!(fader.channel, MidiChannel::A5);
            assert_eq!(fader.midicc, 11.into());
            assert_eq!(fader.min, 20.into());
            assert_eq!(fader.max, 100.into());
        }

        #[test]
        fn test_fader_try_from_raw_invalid() {
            let raw = RawFader {
                kind: 255,
                channel: 0,
                midicc: 0,
                min: 0,
                max: 127,
                midi2din: 0,
            };

            let result = Fader::try_from(raw);
            assert!(result.is_err());
        }
    }

    mod switch {
        use super::*;
        #[test]
        fn test_switch_default() {
            let switch = Switch::default();
            assert_eq!(switch.kind, SwitchKind::CC);
        }

        #[test]
        fn test_switch_as_bytes() {
            let switch = Switch {
                kind: SwitchKind::Program,
                channel: MidiChannel::A1,
                midicc: 64.into(),
                mode: TriggerKind::Toggle,
                prog: 5.into(),
                msb: 0.into(),
                lsb: 0.into(),
                midi2din: ActiveState::On,
                note: 60,
                velo: 100.into(),
                invert: ActiveState::Off,
                key1: 0,
                key2: KeyModifier::CTRL,
            };

            let bytes = switch.as_bytes();
            assert_eq!(bytes.len(), 13);
            assert_eq!(bytes[0], SwitchKind::Program as u8);
            assert_eq!(bytes[3], TriggerKind::Toggle as u8);
            assert_eq!(bytes[12], KeyModifier::CTRL as u8);
        }

        #[test]
        fn test_switch_try_from_raw() {
            let raw = RawSwitch {
                kind: 2,
                channel: 3,
                midicc: 65,
                mode: 1,
                prog: 10,
                msb: 0,
                lsb: 1,
                midi2din: 1,
                note: 72,
                velo: 127,
                invert: 0,
                key1: 65,
                key2: 5,
            };

            let switch = Switch::try_from(raw).unwrap();
            assert_eq!(switch.kind, SwitchKind::Program);
            assert_eq!(switch.mode, TriggerKind::Toggle);
            assert_eq!(switch.key2, KeyModifier::CTRL_SHIFT);
        }

        #[test]
        fn test_switch_try_from_raw_invalid() {
            let raw = RawSwitch {
                kind: 255,
                channel: 0,
                midicc: 0,
                mode: 0,
                prog: 0,
                msb: 0,
                lsb: 0,
                midi2din: 0,
                note: 0,
                velo: 0,
                invert: 0,
                key1: 0,
                key2: 0,
            };

            let result = Switch::try_from(raw);
            assert!(result.is_err());
        }
    }

    mod preset_settings {
        use super::*;

        #[test]
        fn test_preset_settings_default() {
            let preset_settings = PresetSettings::default();
            assert_eq!(preset_settings.preset_slot, PresetSlot::RAM);
            assert_eq!(preset_settings.tempo, Tempo(120));
            assert_eq!(preset_settings.time_division, TimeDivision::Div1_16);
            assert_eq!(preset_settings.gate, Gate::from(50));
        }

        #[test]
        fn test_preset_settings_direct_mutation() {
            let mut preset_settings = PresetSettings::default();
            assert_eq!(preset_settings.tempo, Tempo(120));

            preset_settings.tempo = Tempo(140);
            assert_eq!(preset_settings.tempo, Tempo(140));
        }
    }
}
