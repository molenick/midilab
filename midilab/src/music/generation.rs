use strum_macros::Display;
use strum_macros::EnumIter;

use super::theory::ChordVoicing;
use super::theory::Pitch;
use super::theory::PitchClass;
use super::theory::ScaleKind;
use crate::music::theory;

pub struct PitchSequence(pub Vec<Pitch>);

#[derive(Clone, Copy, Debug, Display, EnumIter, PartialEq, Eq)]
pub enum SequenceDirection {
    Ascending,
    Descending,
}

impl PitchClass {
    pub fn next(&self, direction: SequenceDirection) -> Self {
        match direction {
            SequenceDirection::Ascending => match self {
                PitchClass::C => PitchClass::Cs,
                PitchClass::Cs => PitchClass::D,
                PitchClass::D => PitchClass::Ds,
                PitchClass::Ds => PitchClass::E,
                PitchClass::E => PitchClass::F,
                PitchClass::F => PitchClass::Fs,
                PitchClass::Fs => PitchClass::G,
                PitchClass::G => PitchClass::Gs,
                PitchClass::Gs => PitchClass::A,
                PitchClass::A => PitchClass::As,
                PitchClass::As => PitchClass::B,
                PitchClass::B => PitchClass::C,
            },
            SequenceDirection::Descending => match self {
                PitchClass::C => PitchClass::B,
                PitchClass::Cs => PitchClass::C,
                PitchClass::D => PitchClass::Cs,
                PitchClass::Ds => PitchClass::D,
                PitchClass::E => PitchClass::Ds,
                PitchClass::F => PitchClass::E,
                PitchClass::Fs => PitchClass::F,
                PitchClass::G => PitchClass::Fs,
                PitchClass::Gs => PitchClass::G,
                PitchClass::A => PitchClass::Gs,
                PitchClass::As => PitchClass::A,
                PitchClass::B => PitchClass::As,
            },
        }
    }
}

/// Creates a "chord row" - a 4 note sequence that voices a chord
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChordRowSequence {
    pub tonic: PitchClass,
    pub scale: ScaleKind,
    pub octave: theory::Octave,
    pub voicing: ChordVoicing,
    pub direction: SequenceDirection,
    pub length: usize,
}

impl Default for ChordRowSequence {
    fn default() -> Self {
        Self {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Seventh,
            direction: SequenceDirection::Ascending,
            length: 64,
        }
    }
}

impl ChordRowSequence {
    pub fn as_pitches(&self) -> PitchSequence {
        let intervals = self.scale.intervals();
        // +12 ensures enough scale pitches for the largest voicing offset (11, from Inversion3).
        let needed_degrees = (self.length / 4) + 12;
        let mut scale_pitches: Vec<Pitch> = Vec::with_capacity(needed_degrees);

        let mut current_pitch = Pitch {
            class: self.tonic,
            octave: self.octave,
        };

        scale_pitches.push(current_pitch);
        for i in 0.. {
            if scale_pitches.len() >= needed_degrees {
                break;
            }
            let signed_step = match self.direction {
                SequenceDirection::Ascending => intervals[i % intervals.len()] as i8,
                SequenceDirection::Descending => {
                    -(intervals[(intervals.len() - 1) - (i % intervals.len())] as i8)
                }
            };
            current_pitch = current_pitch.add_semitones(signed_step);
            scale_pitches.push(current_pitch);
        }

        let mut pitches = Vec::with_capacity(self.length);
        let mut degree = 0usize;

        let offsets = self.voicing.chord_offsets();
        let max_offset = offsets.iter().map(|(o, _)| *o).max().unwrap_or(0);

        while pitches.len() < self.length && degree < scale_pitches.len() {
            if degree + max_offset >= scale_pitches.len() {
                break;
            }
            for &(off, shift) in &offsets {
                if pitches.len() < self.length {
                    let base = scale_pitches[degree + off];
                    let pitch = Pitch {
                        class: base.class,
                        octave: theory::Octave(base.octave.0 + shift / 12),
                    };
                    pitches.push(pitch);
                }
            }
            degree += 1;
        }

        PitchSequence(pitches)
    }
}

/// ScaleSequences allow for the creation of note-mapping patterns from
/// common musical scales
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleSequence {
    /// the tonic of the scale
    pub tonic: PitchClass,
    /// the supported scale kind, which determines intervals used
    pub scale: ScaleKind,
    /// are the notes ascending or descending?
    pub direction: SequenceDirection,
    /// what is the octave of the tonic where the sequence begins?
    pub octave: theory::Octave,
    /// how notes do we want the sequence to produce?
    pub length: usize,
}

impl Default for ScaleSequence {
    fn default() -> Self {
        Self {
            tonic: PitchClass::C,
            scale: ScaleKind::Chromatic,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(4),
            length: 64,
        }
    }
}

impl ScaleSequence {
    pub fn as_pitches(&self) -> PitchSequence {
        let intervals = self.scale.intervals();
        let mut pitches = Vec::with_capacity(self.length);
        let mut current_pitch = Pitch {
            class: self.tonic,
            octave: self.octave,
        };

        for i in 0..self.length {
            pitches.push(current_pitch);

            let signed_step = match self.direction {
                SequenceDirection::Ascending => intervals[i % intervals.len()] as i8,
                SequenceDirection::Descending => {
                    -(intervals[(intervals.len() - 1) - (i % intervals.len())] as i8)
                }
            };
            current_pitch = current_pitch.add_semitones(signed_step);
        }

        PitchSequence(pitches)
    }
}

#[derive(Clone, Copy, Debug, Display, PartialEq)]
pub enum PitchPattern {
    #[strum(to_string = "Scale")]
    Scale(ScaleSequence),
    #[strum(to_string = "Chord")]
    ChordRow(ChordRowSequence),
}

impl PitchPattern {
    pub fn as_pitches(&self) -> PitchSequence {
        match self {
            PitchPattern::Scale(sequence) => sequence.as_pitches(),
            PitchPattern::ChordRow(sequence) => sequence.as_pitches(),
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            PitchPattern::Scale(sequence) => sequence.length,
            PitchPattern::ChordRow(sequence) => sequence.length,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::generation::MidiNoteSequence;

    #[test]
    fn test_default_scale_sequence() {
        let ss = ScaleSequence::default();
        let notes = MidiNoteSequence::from(ss.as_pitches()).0;

        assert_eq!(notes.len(), 64);
        assert_eq!(notes[0].as_u8(), 60);
        assert_eq!(notes[63].as_u8(), 123);
    }

    #[test]
    fn test_major_scale_c4_16_notes() {
        let ss = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(4),
            length: 16,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(ss.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();

        assert_eq!(
            notes,
            vec![
                60, 62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81, 83, 84, 86
            ]
        );
    }

    #[test]
    fn test_natural_minor_a4_16_notes() {
        let ss = ScaleSequence {
            tonic: PitchClass::A,
            scale: ScaleKind::NaturalMinor,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(4),
            length: 16,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(ss.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();

        assert_eq!(
            notes,
            vec![
                69, 71, 72, 74, 76, 77, 79, 81, 83, 84, 86, 88, 89, 91, 93, 95
            ]
        );
    }

    #[test]
    fn test_descending_major_c5_16_notes() {
        let ss = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            direction: SequenceDirection::Descending,
            octave: theory::Octave(5),
            length: 16,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(ss.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();

        assert_eq!(
            notes,
            vec![
                72, 71, 69, 67, 65, 64, 62, 60, 59, 57, 55, 53, 52, 50, 48, 47
            ]
        );
    }

    #[test]
    fn test_major_pentatonic_c4_16_notes() {
        let ss = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::MajorPentatonic,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(4),
            length: 16,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(ss.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();

        assert_eq!(
            notes,
            vec![
                60, 62, 64, 67, 69, 72, 74, 76, 79, 81, 84, 86, 88, 91, 93, 96
            ]
        );
    }

    #[test]
    fn test_chromatic_c4_16_notes() {
        let ss = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Chromatic,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(4),
            length: 16,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(ss.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();

        assert_eq!(
            notes,
            vec![
                60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75
            ]
        );
    }

    #[test]
    fn test_chord_row_c_major_seventh() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Seventh,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        // Cmaj7: C4(60), E4(64), G4(67), B4(71)
        // Dm7:   D4(62), F4(65), A4(69), C5(72)
        assert_eq!(notes, vec![60, 64, 67, 71, 62, 65, 69, 72]);
    }

    #[test]
    fn test_chord_row_c_major_triad() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Triad,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        // C triad + root octave: C4(60), E4(64), G4(67), C5(72)
        // D triad + root octave: D4(62), F4(65), A4(69), D5(74)
        assert_eq!(notes, vec![60, 64, 67, 72, 62, 65, 69, 74]);
    }

    #[test]
    fn test_chord_row_saturates() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(9),
            voicing: ChordVoicing::Seventh,
            direction: SequenceDirection::Ascending,
            length: 16,
        };
        let notes = MidiNoteSequence::from(seq.as_pitches()).0;
        assert_eq!(notes.len(), 16);
        // Should not panic and all notes should be <= 127
        for n in &notes {
            assert!(n.as_u8() <= 127);
        }
    }

    #[test]
    fn test_chord_row_inversion1() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Inversion1,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        assert_eq!(notes, vec![64, 67, 71, 72, 65, 69, 72, 74]);
    }

    #[test]
    fn test_chord_row_inversion2() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Inversion2,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        assert_eq!(notes, vec![67, 71, 72, 76, 69, 72, 74, 77]);
    }

    #[test]
    fn test_chord_row_inversion3() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Inversion3,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        assert_eq!(notes, vec![71, 72, 76, 79, 72, 74, 77, 81]);
    }

    #[test]
    fn test_chord_row_drop2() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Drop2,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        assert_eq!(notes, vec![55, 60, 64, 71, 57, 62, 65, 72]);
    }

    #[test]
    fn test_chord_row_quartal() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Quartal,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        assert_eq!(notes, vec![60, 65, 71, 76, 62, 67, 72, 77]);
    }

    #[test]
    fn test_chord_row_shell() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Shell,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        assert_eq!(notes, vec![60, 64, 71, 72, 62, 65, 72, 74]);
    }

    #[test]
    fn test_chord_row_power() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Power,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        assert_eq!(notes, vec![60, 67, 72, 79, 62, 69, 74, 81]);
    }

    #[test]
    fn test_chord_row_add9() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Add9,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        assert_eq!(notes, vec![60, 64, 67, 74, 62, 65, 69, 76]);
    }

    #[test]
    fn test_chord_row_ninth_no5() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::NinthNo5,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        assert_eq!(notes, vec![60, 64, 71, 74, 62, 65, 72, 76]);
    }

    #[test]
    fn test_different_octaves() {
        let o2 = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Chromatic,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(2),
            length: 1,
        };
        let o6 = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Chromatic,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(6),
            length: 1,
        };

        assert_eq!(MidiNoteSequence::from(o2.as_pitches()).0[0].as_u8(), 36);
        assert_eq!(MidiNoteSequence::from(o6.as_pitches()).0[0].as_u8(), 84);
    }
}
