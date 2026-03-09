use strum_macros::Display;
use strum_macros::EnumIter;

use super::theory::ChordVoicing;
use super::theory::Pitch;
use super::theory::PitchClass;
use super::theory::ScaleKind;
use crate::music::theory;

/// Represents a sequence of pitches generated from a scale or chord pattern.
///
/// This is the core output type for music generation, containing an ordered
/// collection of pitches that can be converted to MIDI notes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PitchSequence(pub Vec<Pitch>);

impl PitchSequence {
    /// Returns the number of pitches in this sequence.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if this sequence contains no pitches.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the underlying vector of pitches.
    pub fn as_vec(&self) -> &[Pitch] {
        &self.0
    }
}

/// Direction in which a musical sequence proceeds.
#[derive(Clone, Copy, Debug, Display, EnumIter, PartialEq, Eq)]
pub enum SequenceDirection {
    /// Moves upward through the scale (ascending pitch)
    Ascending,
    /// Moves downward through the scale (descending pitch)
    Descending,
}

impl PitchClass {
    /// Returns the next pitch class in the specified direction.
    ///
    /// Wraps around at octave boundaries (B → C or C → B).
    ///
    /// # Arguments
    ///
    /// * `direction` - Whether to move up (ascending) or down (descending)
    ///
    /// # Examples
    ///
    /// ```
    /// use midilab::music::generation::SequenceDirection;
    /// use midilab::music::theory::PitchClass;
    ///
    /// let c = PitchClass::C;
    /// assert_eq!(c.next(SequenceDirection::Ascending), PitchClass::Cs);
    /// assert_eq!(c.next(SequenceDirection::Descending), PitchClass::B);
    /// ```
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

/// A sequence that generates chord voicings across scale degrees.
///
/// This produces a "chord row" - a melodic pattern where each scale degree
/// is voiced as a chord using the specified voicing (e.g., seventh, triad,
/// drop 2, quartal).
///
/// Each chord is 4 notes, so a sequence of length N produces 4×N individual
/// pitch events.
///
/// # Examples
///
/// ```
/// use midilab::music::generation::{ChordRowSequence, SequenceDirection};
/// use midilab::music::theory::{ScaleKind, Octave, PitchClass, ChordVoicing};
///
/// let seq = ChordRowSequence {
///     tonic: PitchClass::C,
///     scale: ScaleKind::Major,
///     octave: Octave(4),
///     voicing: ChordVoicing::Seventh,
///     direction: SequenceDirection::Ascending,
///     length: 8,
/// };
/// let pitches = seq.as_pitches();
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChordRowSequence {
    /// The tonic (root) pitch class of the scale
    pub tonic: PitchClass,
    /// The scale kind determining which intervals to use
    pub scale: ScaleKind,
    /// The starting octave for the sequence
    pub octave: theory::Octave,
    /// How to voice each chord (seventh, triad, drop2, etc.)
    pub voicing: ChordVoicing,
    /// Whether to move ascending or descending through the scale
    pub direction: SequenceDirection,
    /// Number of scale degrees to sequence (each produces 4 chord notes)
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
        const SAFE_SCALE_PADDING: usize = 12;
        const MAX_VOICING_OFFSET: usize = 11;
        let needed_degrees = (self.length / 4) + SAFE_SCALE_PADDING;
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
        let max_offset = MAX_VOICING_OFFSET;

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

/// A sequence that generates notes from a musical scale.
///
/// This produces a melodic pattern by traversing the intervals of a scale
/// in the specified direction (ascending or descending) starting from a
/// given tonic and octave.
///
/// # Examples
///
/// ```
/// use midilab::music::generation::{ScaleSequence, SequenceDirection};
/// use midilab::music::theory::{ScaleKind, Octave, PitchClass};
///
/// let seq = ScaleSequence {
///     tonic: PitchClass::C,
///     scale: ScaleKind::Major,
///     direction: SequenceDirection::Ascending,
///     octave: Octave(4),
///     length: 16,
/// };
/// let pitches = seq.as_pitches();
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleSequence {
    /// The tonic (root) pitch class of the scale
    pub tonic: PitchClass,
    /// The scale kind determining which intervals to use
    pub scale: ScaleKind,
    /// Whether to move ascending or descending through the scale
    pub direction: SequenceDirection,
    /// The starting octave for the sequence
    pub octave: theory::Octave,
    /// Number of notes to generate in the sequence
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
    /// Generates the pitch sequence according to this scale sequence configuration.
    ///
    /// Returns a `PitchSequence` containing the specified number of notes,
    /// following the scale intervals in the specified direction.
    ///
    /// # Arguments
    ///
    /// * `self` - The scale sequence configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use midilab::music::generation::{ScaleSequence, SequenceDirection};
    /// use midilab::music::theory::{ScaleKind, Octave, PitchClass};
    ///
    /// let seq = ScaleSequence {
    ///     tonic: PitchClass::C,
    ///     scale: ScaleKind::Major,
    ///     direction: SequenceDirection::Ascending,
    ///     octave: Octave(4),
    ///     length: 8,
    /// };
    /// let pitches = seq.as_pitches();
    /// assert_eq!(pitches.len(), 8);
    /// ```
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

/// A pattern that can generate pitch sequences.
///
/// This enum wraps both scale-based and chord-based generation patterns,
/// providing a uniform interface for creating musical sequences.
#[derive(Clone, Copy, Debug, Display, PartialEq)]
pub enum PitchPattern {
    /// A scale sequence generating notes from a musical scale
    Scale(ScaleSequence),
    /// A chord row sequence generating chord voicings across scale degrees
    ChordRow(ChordRowSequence),
}

impl PitchPattern {
    /// Generates the pitch sequence for this pattern.
    ///
    /// Delegates to the underlying scale or chord sequence to produce
    /// the actual collection of pitches.
    pub fn as_pitches(&self) -> PitchSequence {
        match self {
            PitchPattern::Scale(sequence) => sequence.as_pitches(),
            PitchPattern::ChordRow(sequence) => sequence.as_pitches(),
        }
    }

    /// Returns the total number of notes this pattern will generate.
    ///
    /// For scale sequences, this is the configured length.
    /// For chord row sequences, this is the number of scale degrees
    /// (each producing 4 chord notes).
    pub fn len(&self) -> usize {
        match self {
            PitchPattern::Scale(sequence) => sequence.length,
            PitchPattern::ChordRow(sequence) => sequence.length,
        }
    }

    /// Returns true if this pattern will generate zero notes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

    #[test]
    fn test_scale_sequence_length_one() {
        let ss = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(4),
            length: 1,
        };
        let pitches = ss.as_pitches();
        assert_eq!(pitches.len(), 1);
        assert_eq!(pitches.as_vec()[0].class, PitchClass::C);
    }

    #[test]
    fn test_scale_sequence_length_zero() {
        let ss = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(4),
            length: 0,
        };
        let pitches = ss.as_pitches();
        assert_eq!(pitches.len(), 0);
        assert!(pitches.is_empty());
    }

    #[test]
    fn test_chord_row_descending_c4_8_notes() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(4),
            voicing: ChordVoicing::Seventh,
            direction: SequenceDirection::Descending,
            length: 8,
        };
        let notes: Vec<u8> = MidiNoteSequence::from(seq.as_pitches())
            .0
            .into_iter()
            .map(u8::from)
            .collect();
        assert_eq!(notes.len(), 8);
    }

    #[test]
    fn test_pitch_pattern_is_empty() {
        let scale = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(4),
            length: 0,
        };
        let pattern = PitchPattern::Scale(scale);
        assert!(pattern.is_empty());
        assert_eq!(pattern.len(), 0);
    }

    #[test]
    fn test_pitch_sequence_len_and_is_empty() {
        let seq = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            direction: SequenceDirection::Ascending,
            octave: theory::Octave(4),
            length: 10,
        };
        let pitches = seq.as_pitches();
        assert_eq!(pitches.len(), 10);
        assert!(!pitches.is_empty());
        assert_eq!(pitches.as_vec().len(), 10);
    }

    #[test]
    fn test_chord_row_high_octave_all_notes_valid() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: theory::Octave(8),
            voicing: ChordVoicing::Seventh,
            direction: SequenceDirection::Ascending,
            length: 4,
        };
        let notes = MidiNoteSequence::from(seq.as_pitches()).0;
        assert_eq!(notes.len(), 4);
        for n in &notes {
            assert!(n.as_u8() <= 127);
        }
    }
}
