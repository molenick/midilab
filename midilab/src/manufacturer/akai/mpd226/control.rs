use crate::manufacturer::akai::mpd226::control::value_kind::AfterTouchKind;
use crate::manufacturer::akai::mpd226::control::value_kind::DialKind;
use crate::manufacturer::akai::mpd226::control::value_kind::FaderKind;
use crate::manufacturer::akai::mpd226::control::value_kind::GateValue;
use crate::manufacturer::akai::mpd226::control::value_kind::Midi2Din;
use crate::manufacturer::akai::mpd226::control::value_kind::MidiChannel;
use crate::manufacturer::akai::mpd226::control::value_kind::PadColor;
use crate::manufacturer::akai::mpd226::control::value_kind::PadKind;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetName;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetSlot;
use crate::manufacturer::akai::mpd226::control::value_kind::SwingKind;
use crate::manufacturer::akai::mpd226::control::value_kind::SwitchKey2;
use crate::manufacturer::akai::mpd226::control::value_kind::SwitchKind;
use crate::manufacturer::akai::mpd226::control::value_kind::Tempo;
use crate::manufacturer::akai::mpd226::control::value_kind::TimeDivision;
use crate::manufacturer::akai::mpd226::control::value_kind::TransportKind;
use crate::manufacturer::akai::mpd226::control::value_kind::TriggerKind;
use crate::manufacturer::akai::mpd226::raw::RawDial;
use crate::manufacturer::akai::mpd226::raw::RawFader;
use crate::manufacturer::akai::mpd226::raw::RawPad;
use crate::manufacturer::akai::mpd226::raw::RawSwitch;
use crate::midi::Note;

/// Contains common value types used the controls
pub mod value_kind;

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Pad {
    pub id: usize,
    pub kind: PadKind,
    pub channel: MidiChannel,
    pub note: Note,
    pub midi2din: Midi2Din,
    pub trigger: TriggerKind,
    pub aftertouch: AfterTouchKind,
    pub program: u8,
    pub msb: u8,
    pub lsb: u8,
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
            self.program,
            self.msb,
            self.lsb,
            self.off_color as u8,
            self.on_color as u8,
        ]
    }
}

impl TryFrom<(usize, RawPad)> for Pad {
    type Error = super::error::PadDeserializationError;

    fn try_from(value: (usize, RawPad)) -> Result<Self, Self::Error> {
        use super::error::PadDeserializationError;
        let (index, raw) = value;
        Ok(Pad {
            id: index,
            kind: PadKind::try_from(raw.kind).map_err(PadDeserializationError::Kind)?,
            channel: MidiChannel::try_from(raw.channel)
                .map_err(PadDeserializationError::Channel)?,
            note: Note::try_from(raw.note).map_err(PadDeserializationError::Note)?,
            midi2din: Midi2Din::try_from(raw.midi2din)
                .map_err(PadDeserializationError::Midi2Din)?,
            trigger: TriggerKind::try_from(raw.trigger)
                .map_err(PadDeserializationError::Trigger)?,
            aftertouch: AfterTouchKind::try_from(raw.aftertouch)
                .map_err(PadDeserializationError::Aftertouch)?,
            program: raw.program,
            msb: raw.msb,
            lsb: raw.lsb,
            off_color: PadColor::try_from(raw.off_color)
                .map_err(PadDeserializationError::OffColor)?,
            on_color: PadColor::try_from(raw.on_color).map_err(PadDeserializationError::OnColor)?,
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dial {
    pub kind: DialKind,
    pub channel: MidiChannel,
    pub midicc: u8, // CC and ID2 only
    pub min: u8,    // CC and AT only
    pub max: u8,    // CC and AT only
    pub midi2din: Midi2Din,
    pub msb: u8,   // ID1 only
    pub lsb: u8,   // ID1 only
    pub value: u8, // ID1 only
}

impl Default for Dial {
    fn default() -> Self {
        Self {
            kind: DialKind::default(),
            channel: MidiChannel::default(),
            midicc: 0,
            min: 0,
            max: 127,
            midi2din: Midi2Din::default(),
            msb: 0,
            lsb: 0,
            value: 64,
        }
    }
}

impl Dial {
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![
            self.kind as u8,
            self.channel as u8,
            self.midicc,
            self.min,
            self.max,
            self.midi2din as u8,
            self.msb,
            self.lsb,
            self.value,
        ]
    }
}

impl TryFrom<RawDial> for Dial {
    type Error = super::error::DialDeserializationError;

    fn try_from(raw: RawDial) -> Result<Self, Self::Error> {
        use super::error::DialDeserializationError;
        Ok(Dial {
            kind: DialKind::try_from(raw.kind).map_err(DialDeserializationError::Kind)?,
            channel: MidiChannel::try_from(raw.channel)
                .map_err(DialDeserializationError::Channel)?,
            midicc: raw.midicc,
            min: raw.min,
            max: raw.max,
            midi2din: Midi2Din::try_from(raw.midi2din)
                .map_err(DialDeserializationError::Midi2Din)?,
            msb: raw.msb,
            lsb: raw.lsb,
            value: raw.value,
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fader {
    pub kind: FaderKind,
    pub channel: u8,
    pub midicc: u8, // cc only
    pub min: u8,    // both cc and aftertouch
    pub max: u8,    // both cc and aftertouch
    pub midi2din: Midi2Din,
}

impl Default for Fader {
    fn default() -> Self {
        Self {
            kind: FaderKind::default(),
            channel: 0,
            midicc: 0,
            min: 0,
            max: 127,
            midi2din: Midi2Din::default(),
        }
    }
}

impl Fader {
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![
            self.kind as u8,
            self.channel,
            self.midicc,
            self.min,
            self.max,
            self.midi2din as u8,
        ]
    }
}

impl TryFrom<RawFader> for Fader {
    type Error = super::error::FaderDeserializationError;

    fn try_from(raw: RawFader) -> Result<Self, Self::Error> {
        use super::error::FaderDeserializationError;
        Ok(Fader {
            kind: FaderKind::try_from(raw.kind).map_err(FaderDeserializationError::Kind)?,
            channel: raw.channel,
            midicc: raw.midicc,
            min: raw.min,
            max: raw.max,
            midi2din: Midi2Din::try_from(raw.midi2din)
                .map_err(FaderDeserializationError::Midi2Din)?,
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Switch {
    pub kind: SwitchKind,
    pub channel: u8, // all but keystroke
    pub midicc: u8,  // cc only
    pub mode: TriggerKind,
    pub prog: u8,
    pub msb: u8,
    pub lsb: u8,
    pub midi2din: Midi2Din,
    pub note: u8,
    pub velo: u8,
    pub invert: Midi2Din,
    pub key1: u8,
    pub key2: SwitchKey2,
}

impl Default for Switch {
    fn default() -> Self {
        Self {
            kind: SwitchKind::default(),
            channel: 0,
            midicc: 0,
            mode: TriggerKind::default(),
            prog: 0,
            msb: 0,
            lsb: 0,
            midi2din: Midi2Din::default(),
            note: 0,
            velo: 100,
            invert: Midi2Din::default(),
            key1: 0,
            key2: SwitchKey2::default(),
        }
    }
}

impl Switch {
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![
            self.kind as u8,
            self.channel,
            self.midicc,
            self.mode as u8,
            self.prog,
            self.msb,
            self.lsb,
            self.midi2din as u8,
            self.note,
            self.velo,
            self.invert as u8,
            self.key1,
            self.key2 as u8,
        ]
    }
}

impl TryFrom<RawSwitch> for Switch {
    type Error = super::error::SwitchDeserializationError;

    fn try_from(raw: RawSwitch) -> Result<Self, Self::Error> {
        use super::error::SwitchDeserializationError;
        Ok(Switch {
            kind: SwitchKind::try_from(raw.kind).map_err(SwitchDeserializationError::Kind)?,
            channel: raw.channel,
            midicc: raw.midicc,
            mode: TriggerKind::try_from(raw.mode).map_err(SwitchDeserializationError::Mode)?,
            prog: raw.prog,
            msb: raw.msb,
            lsb: raw.lsb,
            midi2din: Midi2Din::try_from(raw.midi2din)
                .map_err(SwitchDeserializationError::Midi2Din)?,
            note: raw.note,
            velo: raw.velo,
            invert: Midi2Din::try_from(raw.invert).map_err(SwitchDeserializationError::Invert)?,
            key1: raw.key1,
            key2: SwitchKey2::try_from(raw.key2).map_err(SwitchDeserializationError::Key2)?,
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Global {
    pub preset_slot: PresetSlot,
    pub preset_name: PresetName,
    pub tempo: Tempo,
    pub time_division_switch: TriggerKind,
    pub time_division: TimeDivision,
    pub note_repeat_switch: TriggerKind,
    pub gate: GateValue,
    pub swing: SwingKind,
    pub transport: TransportKind,
}

impl Default for Global {
    fn default() -> Self {
        Self {
            preset_slot: PresetSlot::default(),
            preset_name: PresetName::default(),
            tempo: Tempo::default(),
            time_division_switch: TriggerKind::Toggle,
            time_division: TimeDivision::default(),
            note_repeat_switch: TriggerKind::Toggle,
            gate: GateValue::try_from(50).unwrap(),
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
                midi2din: Midi2Din::Off,
                trigger: TriggerKind::Momentary,
                aftertouch: AfterTouchKind::Channel,
                program: 0,
                msb: 0,
                lsb: 0,
                off_color: PadColor::Red,
                on_color: PadColor::Green,
            };

            let bytes = pad.as_bytes();
            assert_eq!(bytes.len(), 11);
            assert_eq!(bytes[0], PadKind::Note as u8);
            assert_eq!(bytes[1], MidiChannel::COMMON as u8); // channel
            assert_eq!(bytes[2], Note::N60.into()); // note
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
                midicc: 74,
                min: 0,
                max: 127,
                midi2din: Midi2Din::On,
                msb: 0,
                lsb: 0,
                value: 64,
            };

            let bytes = dial.as_bytes();
            assert_eq!(bytes.len(), 9);
            assert_eq!(bytes[0], DialKind::CC as u8);
            assert_eq!(bytes[1], 1); // channel
            assert_eq!(bytes[2], 74); // midicc
            assert_eq!(bytes[3], 0); // min
            assert_eq!(bytes[4], 127); // max
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
            assert_eq!(dial.midicc, 50);
            assert_eq!(dial.midi2din, Midi2Din::On);
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
                channel: 2,
                midicc: 7,
                min: 0,
                max: 127,
                midi2din: Midi2Din::Off,
            };

            let bytes = fader.as_bytes();
            assert_eq!(bytes.len(), 6);
            assert_eq!(bytes[0], FaderKind::Aftertouch as u8);
            assert_eq!(bytes[1], 2); // channel
            assert_eq!(bytes[2], 7); // midicc
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
            assert_eq!(fader.channel, 5);
            assert_eq!(fader.midicc, 11);
            assert_eq!(fader.min, 20);
            assert_eq!(fader.max, 100);
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
                channel: 1,
                midicc: 64,
                mode: TriggerKind::Toggle,
                prog: 5,
                msb: 0,
                lsb: 0,
                midi2din: Midi2Din::On,
                note: 60,
                velo: 100,
                invert: Midi2Din::Off,
                key1: 0,
                key2: SwitchKey2::CTRL,
            };

            let bytes = switch.as_bytes();
            assert_eq!(bytes.len(), 13);
            assert_eq!(bytes[0], SwitchKind::Program as u8);
            assert_eq!(bytes[3], TriggerKind::Toggle as u8);
            assert_eq!(bytes[12], SwitchKey2::CTRL as u8);
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
            assert_eq!(switch.key2, SwitchKey2::CTRL_SHIFT);
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

    mod global {
        use super::*;

        #[test]
        fn test_global_default() {
            let global = Global::default();
            assert_eq!(global.preset_slot, PresetSlot::RAM);
            assert_eq!(global.tempo, Tempo(120));
            assert_eq!(global.time_division, TimeDivision::Div1_16);
            assert_eq!(global.gate, GateValue::try_from(50).unwrap());
        }

        #[test]
        fn test_global_direct_mutation() {
            let mut global = Global::default();
            assert_eq!(global.tempo, Tempo(120));

            global.tempo = Tempo(140);
            assert_eq!(global.tempo, Tempo(140));
        }
    }
}
