use crate::manufacturer::akai::mpd226::ColorPattern;
use crate::manufacturer::akai::mpd226::NoteColorMap;
use crate::manufacturer::akai::mpd226::PitchPattern;
use crate::manufacturer::akai::mpd226::TOTAL_PADS;
use crate::manufacturer::akai::mpd226::control::ControlId;
use crate::manufacturer::akai::mpd226::control::Dial;
use crate::manufacturer::akai::mpd226::control::Fader;
use crate::manufacturer::akai::mpd226::control::Pad;
use crate::manufacturer::akai::mpd226::control::Switch;
use crate::manufacturer::akai::mpd226::raw::RawDials;
use crate::manufacturer::akai::mpd226::raw::RawFaders;
use crate::manufacturer::akai::mpd226::raw::RawPads;
use crate::manufacturer::akai::mpd226::raw::RawSwitches;
use crate::midi::generation::MidiNoteSequence;
use crate::music::generation::ScaleSequence;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PadRepository {
    pub pads: [Pad; TOTAL_PADS],
}

impl PadRepository {
    pub fn set_note_pattern(&mut self, starting_position: usize, pattern: PitchPattern) {
        let clamped_starting_position = starting_position.min(TOTAL_PADS);
        let notes = MidiNoteSequence::from(pattern.as_pitches()).0;
        let changing_pads = &mut self.pads[clamped_starting_position..TOTAL_PADS];

        for (rel_idx, pad) in changing_pads.iter_mut().enumerate() {
            if let Some(&midi_note) = notes.get(rel_idx) {
                pad.note = midi_note;
            }
        }
    }

    pub fn set_note_pattern_with_off_colors(
        &mut self,
        starting_position: usize,
        note_pattern: PitchPattern,
        off_pad_color_map: NoteColorMap,
    ) {
        self.set_note_pattern(starting_position, note_pattern);

        let color_pattern = ColorPattern::from((note_pattern, off_pad_color_map));
        self.set_off_color_pattern(starting_position, note_pattern.len(), color_pattern);
    }

    pub fn set_off_color_pattern(
        &mut self,
        starting_position: usize,
        length: usize,
        pattern: ColorPattern,
    ) {
        if matches!(&pattern, ColorPattern::Repeating(seqs) if seqs.is_empty()) {
            return;
        }

        let clamped_starting_position = starting_position.min(TOTAL_PADS);
        let end = (clamped_starting_position + length).min(TOTAL_PADS);
        let changing_pads = &mut self.pads[clamped_starting_position..end];

        for (i, pad) in changing_pads.iter_mut().enumerate() {
            pad.off_color = pattern.color_at_index(i);
        }
    }

    pub fn set_on_color_pattern(
        &mut self,
        starting_position: usize,
        length: usize,
        pattern: ColorPattern,
    ) {
        if matches!(&pattern, ColorPattern::Repeating(seqs) if seqs.is_empty()) {
            return;
        }

        let clamped_starting_position = starting_position.min(TOTAL_PADS);
        let end = (clamped_starting_position + length).min(TOTAL_PADS);
        let changing_pads = &mut self.pads[clamped_starting_position..end];

        for (i, pad) in changing_pads.iter_mut().enumerate() {
            pad.on_color = pattern.color_at_index(i);
        }
    }
}

impl Default for PadRepository {
    fn default() -> Self {
        let pads = std::array::from_fn(|i| Pad {
            id: ControlId(i),
            ..Default::default()
        });

        let mut repo = PadRepository { pads };
        repo.set_note_pattern(0, PitchPattern::Scale(ScaleSequence::default()));

        repo
    }
}

impl TryFrom<RawPads> for PadRepository {
    type Error = super::error::PresetParseError;

    fn try_from(raw: RawPads) -> Result<Self, Self::Error> {
        let mut pad_repo = PadRepository::default();

        for (i, raw_pad) in raw.0.iter().enumerate() {
            pad_repo.pads[i] = Pad::try_from((ControlId(i), *raw_pad))
                .map_err(|source| super::error::PresetParseError::Pad { index: i, source })?;
        }

        Ok(pad_repo)
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct FaderRepository(pub [Fader; 12]);
impl FaderRepository {
    pub fn with_cc_values(values: [u8; 12]) -> Self {
        let mut repo = Self::default();

        for (i, fader) in repo.0.iter_mut().enumerate() {
            fader.id = ControlId(i);
            fader.midicc = values[i].into();
        }

        repo
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct SwitchRepository(pub [Switch; 12]);
impl SwitchRepository {
    pub fn with_cc_values(values: [u8; 12]) -> Self {
        let mut repo = Self::default();

        for (i, switch) in repo.0.iter_mut().enumerate() {
            switch.id = ControlId(i);
            switch.midicc = values[i].into();
        }

        repo
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct DialRepository(pub [Dial; 12]);
impl DialRepository {
    pub fn with_cc_values(values: [u8; 12]) -> Self {
        let mut repo = Self::default();

        for (i, dial) in repo.0.iter_mut().enumerate() {
            dial.id = ControlId(i);
            dial.midicc = values[i].into();
        }

        repo
    }
}

impl TryFrom<RawFaders> for FaderRepository {
    type Error = super::error::PresetParseError;

    fn try_from(raw: RawFaders) -> Result<Self, Self::Error> {
        let mut faders = [Fader::default(); 12];

        for (i, r) in raw.0.iter().enumerate() {
            let fader = Fader::try_from((ControlId(i), *r))
                .map_err(|source| super::error::PresetParseError::Fader { index: i, source })?;
            faders[i] = fader;
        }

        Ok(FaderRepository(faders))
    }
}

impl TryFrom<RawSwitches> for SwitchRepository {
    type Error = super::error::PresetParseError;

    fn try_from(raw: RawSwitches) -> Result<Self, Self::Error> {
        let mut switches = [Switch::default(); 12];

        for (i, r) in raw.0.iter().enumerate() {
            let switch = Switch::try_from((ControlId(i), *r))
                .map_err(|source| super::error::PresetParseError::Switch { index: i, source })?;
            switches[i] = switch;
        }

        Ok(SwitchRepository(switches))
    }
}

impl TryFrom<RawDials> for DialRepository {
    type Error = super::error::PresetParseError;

    fn try_from(raw: RawDials) -> Result<Self, Self::Error> {
        let mut dials = [Dial::default(); 12];

        for (i, r) in raw.0.iter().enumerate() {
            let dial = Dial::try_from((ControlId(i), *r))
                .map_err(|source| super::error::PresetParseError::Dial { index: i, source })?;
            dials[i] = dial;
        }

        Ok(DialRepository(dials))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manufacturer::akai::mpd226::ColorSequence;
    use crate::manufacturer::akai::mpd226::control::value_kind::ActiveState;
    use crate::manufacturer::akai::mpd226::control::value_kind::DialKind;
    use crate::manufacturer::akai::mpd226::control::value_kind::FaderKind;
    use crate::manufacturer::akai::mpd226::control::value_kind::MidiChannel;
    use crate::manufacturer::akai::mpd226::control::value_kind::PadColor;
    use crate::manufacturer::akai::mpd226::control::value_kind::SwitchKind;
    use crate::manufacturer::akai::mpd226::control::value_kind::TriggerKind;
    use crate::manufacturer::akai::mpd226::raw::RawDial;
    use crate::manufacturer::akai::mpd226::raw::RawFader;
    use crate::manufacturer::akai::mpd226::raw::RawPad;
    use crate::manufacturer::akai::mpd226::raw::RawSwitch;
    use crate::midi::MidiNote;
    use crate::music::generation::ChordRowSequence;
    use crate::music::generation::ScaleSequence;
    use crate::music::generation::SequenceDirection;
    use crate::music::theory::ChordVoicing;
    use crate::music::theory::Octave;
    use crate::music::theory::PitchClass;
    use crate::music::theory::ScaleKind;

    #[test]
    fn test_pad_repository_set_note_pattern_chromatic() {
        let mut repo = PadRepository::default();

        let scale_seq = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Chromatic,
            direction: SequenceDirection::Ascending,
            octave: Octave(4),
            length: 12,
        };

        repo.set_note_pattern(0, PitchPattern::Scale(scale_seq));

        assert_eq!(repo.pads[0].note, MidiNote::from(60));
        assert_eq!(repo.pads[1].note, MidiNote::from(61));
        assert_eq!(repo.pads[11].note, MidiNote::from(71));
    }

    #[test]
    fn test_pad_repository_set_note_pattern_major() {
        let mut repo = PadRepository::default();

        let scale_seq = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            direction: SequenceDirection::Ascending,
            octave: Octave(4),
            length: 8,
        };

        repo.set_note_pattern(0, PitchPattern::Scale(scale_seq));

        assert_eq!(repo.pads[0].note, MidiNote::from(60));
        assert_eq!(repo.pads[1].note, MidiNote::from(62));
        assert_eq!(repo.pads[2].note, MidiNote::from(64));
        assert_eq!(repo.pads[3].note, MidiNote::from(65));
        assert_eq!(repo.pads[4].note, MidiNote::from(67));
        assert_eq!(repo.pads[5].note, MidiNote::from(69));
        assert_eq!(repo.pads[6].note, MidiNote::from(71));
    }

    #[test]
    fn test_pad_repository_set_note_pattern_with_offset() {
        let mut repo = PadRepository::default();

        assert_eq!(repo.pads[0].note, MidiNote::from(60));
        assert_eq!(repo.pads[15].note, MidiNote::from(75));

        let scale_seq = ScaleSequence {
            tonic: PitchClass::D,
            scale: ScaleKind::Chromatic,
            direction: SequenceDirection::Ascending,
            octave: Octave(4),
            length: 8,
        };

        repo.set_note_pattern(16, PitchPattern::Scale(scale_seq));

        assert_eq!(repo.pads[0].note, MidiNote::from(60));
        assert_eq!(repo.pads[15].note, MidiNote::from(75));

        assert_eq!(repo.pads[16].note, MidiNote::from(62));
        assert_eq!(repo.pads[17].note, MidiNote::from(63));
    }

    #[test]
    fn test_pad_repository_set_off_color_pattern() {
        let mut repo = PadRepository::default();

        repo.set_off_color_pattern(
            0,
            16,
            ColorPattern::Repeating(vec![ColorSequence {
                len: 16,
                color: PadColor::Blue,
            }]),
        );

        for i in 0..16 {
            assert_eq!(repo.pads[i].off_color, PadColor::Blue);
        }

        assert_eq!(repo.pads[16].off_color, PadColor::default());
    }

    #[test]
    fn test_pad_repository_set_off_color_pattern_with_offset() {
        let mut repo = PadRepository::default();

        repo.set_off_color_pattern(
            8,
            8,
            ColorPattern::Repeating(vec![ColorSequence {
                len: 8,
                color: PadColor::Red,
            }]),
        );

        for i in 0..8 {
            assert_eq!(repo.pads[i].off_color, PadColor::default());
        }

        for i in 8..16 {
            assert_eq!(repo.pads[i].off_color, PadColor::Red);
        }
    }

    #[test]
    fn test_pad_repository_set_on_color_pattern() {
        let mut repo = PadRepository::default();

        repo.set_on_color_pattern(
            0,
            8,
            ColorPattern::Repeating(vec![ColorSequence {
                len: 8,
                color: PadColor::Green,
            }]),
        );

        for i in 0..8 {
            assert_eq!(repo.pads[i].on_color, PadColor::Green);
        }
        assert_eq!(repo.pads[8].on_color, PadColor::default());
    }

    #[test]
    fn test_pad_repository_apply_color_pattern_exceeds_total() {
        let mut repo = PadRepository::default();

        repo.set_off_color_pattern(
            60,
            10,
            ColorPattern::Repeating(vec![ColorSequence {
                len: 10,
                color: PadColor::Yellow,
            }]),
        );

        for i in 60..TOTAL_PADS {
            assert_eq!(repo.pads[i].off_color, PadColor::Yellow);
        }
    }

    #[test]
    fn test_pad_repository_try_from_raw_pads() {
        let mut raw_pads = [RawPad {
            kind: 0,
            channel: 0,
            note: 60,
            midi2din: 0,
            trigger: 0,
            aftertouch: 1,
            program: 0,
            msb: 0,
            lsb: 0,
            off_color: 0,
            on_color: 5,
        }; TOTAL_PADS];

        raw_pads[0].note = 36;
        raw_pads[1].note = 48;
        raw_pads[2].note = 60;

        let repo = PadRepository::try_from(RawPads(raw_pads)).unwrap();

        assert_eq!(repo.pads[0].note, MidiNote::from(36));
        assert_eq!(repo.pads[1].note, MidiNote::from(48));
        assert_eq!(repo.pads[2].note, MidiNote::from(60));
    }

    #[test]
    fn test_fader_repository_default() {
        let repo = FaderRepository::default();
        assert_eq!(repo.0.len(), 12);
    }

    #[test]
    fn test_fader_repository_try_from_raw() {
        let raw_faders = [RawFader {
            kind: 0,
            channel: 1,
            midicc: 7,
            min: 0,
            max: 127,
            midi2din: 0,
        }; 12];

        let repo = FaderRepository::try_from(RawFaders(raw_faders)).unwrap();
        assert_eq!(repo.0[0].kind, FaderKind::CC);
        assert_eq!(repo.0[0].channel, MidiChannel::A1);
        assert_eq!(repo.0[0].midicc, 7.into());
    }

    #[test]
    fn test_fader_repository_try_from_raw_invalid() {
        let mut raw_faders = [RawFader {
            kind: 0,
            channel: 0,
            midicc: 0,
            min: 0,
            max: 127,
            midi2din: 0,
        }; 12];

        raw_faders[5].kind = 255;

        let result = FaderRepository::try_from(RawFaders(raw_faders));
        assert!(result.is_err());
    }

    #[test]
    fn test_switch_repository_default() {
        let repo = SwitchRepository::default();
        assert_eq!(repo.0.len(), 12);
    }

    #[test]
    fn test_switch_repository_try_from_raw() {
        let raw_switches = [RawSwitch {
            kind: 0,
            channel: 1,
            midicc: 64,
            mode: 1,
            prog: 0,
            msb: 0,
            lsb: 0,
            midi2din: 0,
            note: 60,
            velo: 100,
            invert: 0,
            key1: 0,
            key2: 0,
        }; 12];

        let repo = SwitchRepository::try_from(RawSwitches(raw_switches)).unwrap();
        assert_eq!(repo.0[0].kind, SwitchKind::CC);
        assert_eq!(repo.0[0].mode, TriggerKind::Toggle);
    }

    #[test]
    fn test_switch_repository_try_from_raw_invalid() {
        let mut raw_switches = [RawSwitch {
            kind: 0,
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
        }; 12];

        raw_switches[3].kind = 255;

        let result = SwitchRepository::try_from(RawSwitches(raw_switches));
        assert!(result.is_err());
    }

    #[test]
    fn test_dial_repository_default() {
        let repo = DialRepository::default();
        assert_eq!(repo.0.len(), 12);
    }

    #[test]
    fn test_dial_repository_try_from_raw() {
        let raw_dials = [RawDial {
            kind: 2,
            channel: 1,
            midicc: 74,
            min: 0,
            max: 127,
            midi2din: 1,
            msb: 0,
            lsb: 0,
            value: 64,
        }; 12];

        let repo = DialRepository::try_from(RawDials(raw_dials)).unwrap();
        assert_eq!(repo.0[0].kind, DialKind::IncDec1);
        assert_eq!(repo.0[0].midi2din, ActiveState::On);
    }

    #[test]
    fn test_dial_repository_try_from_raw_invalid() {
        let mut raw_dials = [RawDial {
            kind: 0,
            channel: 0,
            midicc: 0,
            min: 0,
            max: 127,
            midi2din: 0,
            msb: 0,
            lsb: 0,
            value: 0,
        }; 12];

        raw_dials[7].kind = 255;

        let result = DialRepository::try_from(RawDials(raw_dials));
        assert!(result.is_err());
    }

    #[test]
    fn test_color_groups_basic() {
        use crate::manufacturer::akai::mpd226::ColorSequence;

        let mut repo = PadRepository::default();
        let pattern = ColorPattern::Repeating(vec![
            ColorSequence {
                len: 4,
                color: PadColor::Red,
            },
            ColorSequence {
                len: 4,
                color: PadColor::Green,
            },
        ]);

        repo.set_off_color_pattern(0, 16, pattern);

        for i in 0..4 {
            assert_eq!(repo.pads[i].off_color, PadColor::Red);
        }

        for i in 4..8 {
            assert_eq!(repo.pads[i].off_color, PadColor::Green);
        }

        for i in 8..12 {
            assert_eq!(repo.pads[i].off_color, PadColor::Red);
        }

        for i in 12..16 {
            assert_eq!(repo.pads[i].off_color, PadColor::Green);
        }
    }

    #[test]
    fn test_color_groups_with_offset() {
        use crate::manufacturer::akai::mpd226::ColorSequence;

        let mut repo = PadRepository::default();
        let pattern = ColorPattern::Repeating(vec![
            ColorSequence {
                len: 2,
                color: PadColor::Blue,
            },
            ColorSequence {
                len: 2,
                color: PadColor::Yellow,
            },
        ]);

        repo.set_off_color_pattern(4, 8, pattern);

        for i in 0..4 {
            assert_eq!(repo.pads[i].off_color, PadColor::default());
        }

        assert_eq!(repo.pads[4].off_color, PadColor::Blue);
        assert_eq!(repo.pads[5].off_color, PadColor::Blue);
        assert_eq!(repo.pads[6].off_color, PadColor::Yellow);
        assert_eq!(repo.pads[7].off_color, PadColor::Yellow);
        assert_eq!(repo.pads[8].off_color, PadColor::Blue);
        assert_eq!(repo.pads[9].off_color, PadColor::Blue);
        assert_eq!(repo.pads[10].off_color, PadColor::Yellow);
        assert_eq!(repo.pads[11].off_color, PadColor::Yellow);
    }

    #[test]
    fn test_color_groups_on_color() {
        use crate::manufacturer::akai::mpd226::ColorSequence;

        let mut repo = PadRepository::default();
        let pattern = ColorPattern::Repeating(vec![
            ColorSequence {
                len: 3,
                color: PadColor::Orange,
            },
            ColorSequence {
                len: 3,
                color: PadColor::Purple,
            },
        ]);

        repo.set_on_color_pattern(0, 6, pattern);

        for i in 0..3 {
            assert_eq!(repo.pads[i].on_color, PadColor::Orange);
        }

        for i in 3..6 {
            assert_eq!(repo.pads[i].on_color, PadColor::Purple);
        }
    }

    #[test]
    fn test_color_groups_uneven_lengths() {
        use crate::manufacturer::akai::mpd226::ColorSequence;

        let mut repo = PadRepository::default();
        let pattern = ColorPattern::Repeating(vec![
            ColorSequence {
                len: 3,
                color: PadColor::Red,
            },
            ColorSequence {
                len: 5,
                color: PadColor::Blue,
            },
        ]);

        repo.set_off_color_pattern(0, 16, pattern);

        assert_eq!(repo.pads[0].off_color, PadColor::Red);
        assert_eq!(repo.pads[1].off_color, PadColor::Red);
        assert_eq!(repo.pads[2].off_color, PadColor::Red);
        assert_eq!(repo.pads[3].off_color, PadColor::Blue);
        assert_eq!(repo.pads[4].off_color, PadColor::Blue);
        assert_eq!(repo.pads[5].off_color, PadColor::Blue);
        assert_eq!(repo.pads[6].off_color, PadColor::Blue);
        assert_eq!(repo.pads[7].off_color, PadColor::Blue);

        assert_eq!(repo.pads[8].off_color, PadColor::Red);
        assert_eq!(repo.pads[9].off_color, PadColor::Red);
        assert_eq!(repo.pads[10].off_color, PadColor::Red);
        assert_eq!(repo.pads[11].off_color, PadColor::Blue);
        assert_eq!(repo.pads[12].off_color, PadColor::Blue);
        assert_eq!(repo.pads[13].off_color, PadColor::Blue);
        assert_eq!(repo.pads[14].off_color, PadColor::Blue);
        assert_eq!(repo.pads[15].off_color, PadColor::Blue);
    }

    #[test]
    fn test_color_groups_empty_sequences() {
        let mut repo = PadRepository::default();
        let pattern = ColorPattern::Repeating(vec![]);

        repo.set_off_color_pattern(0, 4, pattern);

        for i in 0..4 {
            assert_eq!(repo.pads[i].off_color, PadColor::default());
        }
    }

    #[test]
    fn test_color_groups_single_sequence() {
        use crate::manufacturer::akai::mpd226::ColorSequence;

        let mut repo = PadRepository::default();
        let pattern = ColorPattern::Repeating(vec![ColorSequence {
            len: 4,
            color: PadColor::Aqua,
        }]);

        repo.set_off_color_pattern(0, 8, pattern);

        for i in 0..8 {
            assert_eq!(repo.pads[i].off_color, PadColor::Aqua);
        }
    }

    #[test]
    fn test_pad_repository_set_note_pattern_chord_row() {
        let mut repo = PadRepository::default();

        repo.set_note_pattern(
            0,
            PitchPattern::ChordRow(ChordRowSequence {
                tonic: PitchClass::C,
                scale: ScaleKind::Major,
                octave: Octave(4),
                voicing: ChordVoicing::Seventh,
                direction: SequenceDirection::Ascending,
                length: 8,
            }),
        );

        // Cmaj7: C4(60), E4(64), G4(67), B4(71)
        assert_eq!(repo.pads[0].note, MidiNote::from(60));
        assert_eq!(repo.pads[1].note, MidiNote::from(64));
        assert_eq!(repo.pads[2].note, MidiNote::from(67));
        assert_eq!(repo.pads[3].note, MidiNote::from(71));
        // Dm7: D4(62), F4(65), A4(69), C5(72)
        assert_eq!(repo.pads[4].note, MidiNote::from(62));
        assert_eq!(repo.pads[5].note, MidiNote::from(65));
        assert_eq!(repo.pads[6].note, MidiNote::from(69));
        assert_eq!(repo.pads[7].note, MidiNote::from(72));
    }
}
