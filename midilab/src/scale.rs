use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::Display;
use strum_macros::EnumIter;

use crate::midi::Note;

/// ScaleSequences allow for the creation of note-mapping patterns from
/// common musical scales
#[derive(Clone, Copy, Debug)]
pub struct ScaleSequence {
    /// the tonic of the scale
    pub tonic: PitchClass,
    /// the supported scale kind, which determines intervals used
    pub scale: ScaleKind,
    /// are the notes ascending or descending?
    pub direction: SequenceDirection,
    /// what is the octave of the tonic where the sequence begins?
    pub octave: Octave,
    /// how notes do we want the sequence to produce?
    pub length: usize,
}
impl Default for ScaleSequence {
    fn default() -> Self {
        Self {
            tonic: PitchClass::C,
            scale: ScaleKind::Chromatic,
            direction: SequenceDirection::Ascending,
            octave: Octave::O4,
            length: 64,
        }
    }
}

impl ScaleSequence {
    /// Uses the field config values from self to produce a Vec<u8> of midi notes
    pub fn as_midi_notes(&self) -> Vec<u8> {
        let start = 12 * (self.octave as i8 + 1) as u8 + (self.tonic as u8);
        let intervals = self.scale.intervals();

        let mut notes = Vec::with_capacity(self.length);
        let mut cur = start;

        for i in 0..self.length {
            notes.push(cur);

            let step = intervals[i % intervals.len()];
            match self.direction {
                SequenceDirection::Ascending => cur = cur.saturating_add(step),
                SequenceDirection::Descending => cur = cur.saturating_sub(step),
            }
        }

        notes
    }
}

/// The octave as represented by Scientific Pitch Notation: https://en.wikipedia.org/wiki/Scientific_pitch_notation
#[repr(i8)]
#[derive(Copy, Clone, Debug, EnumIter, PartialEq, Eq)]
pub enum Octave {
    Osub2 = -2,
    Osub1 = -1,
    O0 = 0,
    O1 = 1,
    O2 = 2,
    O3 = 3,
    O4 = 4,
    O5 = 5,
    O6 = 6,
    O7 = 7,
    O8 = 8,
    O9 = 9,
}
impl core::fmt::Display for Octave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = (*self as i8).to_string();

        write!(f, "{out}")
    }
}

/// Determines whether the notes generated from a ScaleSequence are ascending or descending
#[derive(Clone, Copy, Debug, Display, EnumIter, PartialEq, Eq)]
pub enum SequenceDirection {
    Ascending,
    Descending,
}

/// The PitchClass according to https://en.wikipedia.org/wiki/Equal_temperament
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, IntoPrimitive, TryFromPrimitive, EnumIter)]
pub enum PitchClass {
    #[default]
    C,
    Cs,
    D,
    Ds,
    E,
    F,
    Fs,
    G,
    Gs,
    A,
    As,
    B,
}

impl core::fmt::Display for PitchClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            PitchClass::C => "C",
            PitchClass::Cs => "C#",
            PitchClass::D => "D",
            PitchClass::Ds => "D#",
            PitchClass::E => "E",
            PitchClass::F => "F",
            PitchClass::Fs => "F#",
            PitchClass::G => "G",
            PitchClass::Gs => "G#",
            PitchClass::A => "A",
            PitchClass::As => "A#",
            PitchClass::B => "B",
        };

        write!(f, "{out}")
    }
}

impl PitchClass {
    pub fn add_semitones(self, semitones: u8) -> Self {
        let new_value = (u8::from(self) + semitones) % 12;
        PitchClass::try_from(new_value).expect("modulo 12 guarantees valid pitch class")
    }
}

impl From<Note> for PitchClass {
    fn from(note: Note) -> Self {
        {
            let semitone = u8::from(note) % 12;
            PitchClass::try_from_primitive(semitone).unwrap()
        }
    }
}

/// Common scale variants used for note generation
#[derive(Clone, Copy, Debug, Eq, PartialEq, Display, EnumIter)]
pub enum ScaleKind {
    Major,
    NaturalMinor,
    HarmonicMinor,
    MelodicMinor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    MajorPentatonic,
    MinorPentatonic,
    WholeTone,
    DiminishedHalfWhole,
    DiminishedWholeHalf,
    Chromatic,
}

impl ScaleKind {
    /// Represents the intervals of a scale without repeating the tonic. Since these
    /// are used for note pattern generation, we deviate from the norm of including
    /// the final interval that leads back to the tonic since it's not what we want
    /// when generating notes from a ScaleSequence.
    pub fn intervals(self) -> &'static [u8] {
        match self {
            ScaleKind::Major => &[2, 2, 1, 2, 2, 2],
            ScaleKind::NaturalMinor => &[2, 1, 2, 2, 1, 2],
            ScaleKind::HarmonicMinor => &[2, 1, 2, 2, 1, 3],
            ScaleKind::MelodicMinor => &[2, 1, 2, 2, 2, 2],
            ScaleKind::Dorian => &[2, 1, 2, 2, 2, 1],
            ScaleKind::Phrygian => &[1, 2, 2, 2, 1, 2],
            ScaleKind::Lydian => &[2, 2, 2, 1, 2, 2],
            ScaleKind::Mixolydian => &[2, 2, 1, 2, 2, 1],
            ScaleKind::Locrian => &[1, 2, 2, 1, 2, 2],
            ScaleKind::MajorPentatonic => &[2, 2, 3, 2],
            ScaleKind::MinorPentatonic => &[3, 2, 2, 3],
            ScaleKind::WholeTone => &[2, 2, 2, 2, 2],
            ScaleKind::DiminishedHalfWhole => &[1, 2, 1, 2, 1, 2, 1],
            ScaleKind::DiminishedWholeHalf => &[2, 1, 2, 1, 2, 1, 2],
            ScaleKind::Chromatic => &[1; 11],
        }
    }

    /// Returns all pitch classes in this scale starting from the given tonic.
    pub fn pitch_classes_from_tonic(self, tonic: &PitchClass) -> Vec<PitchClass> {
        let intervals = self.intervals();
        let mut pitch_classes = Vec::with_capacity(intervals.len() + 1);

        let mut current_pitch = *tonic;
        pitch_classes.push(current_pitch);

        for interval in intervals.iter() {
            current_pitch = current_pitch.add_semitones(*interval);
            pitch_classes.push(current_pitch);
        }

        pitch_classes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scale_sequence() {
        let ss = ScaleSequence::default();
        let notes = ss.as_midi_notes();

        assert_eq!(notes.len(), 64);
        assert_eq!(notes[0], 60);
        assert_eq!(notes[63], 123);
    }

    #[test]
    fn test_major_scale_c4_16_notes() {
        let ss = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            direction: SequenceDirection::Ascending,
            octave: Octave::O4,
            length: 16,
        };
        let notes = ss.as_midi_notes();

        assert_eq!(
            notes,
            vec![
                60, 62, 64, 65, 67, 69, 71, 73, 75, 76, 78, 80, 82, 84, 86, 87
            ]
        );
    }

    #[test]
    fn test_natural_minor_a4_16_notes() {
        let ss = ScaleSequence {
            tonic: PitchClass::A,
            scale: ScaleKind::NaturalMinor,
            direction: SequenceDirection::Ascending,
            octave: Octave::O4,
            length: 16,
        };
        let notes = ss.as_midi_notes();

        assert_eq!(
            notes,
            vec![
                69, 71, 72, 74, 76, 77, 79, 81, 82, 84, 86, 87, 89, 91, 92, 94
            ]
        );
    }

    #[test]
    fn test_descending_major_c5_16_notes() {
        let ss = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            direction: SequenceDirection::Descending,
            octave: Octave::O5,
            length: 16,
        };
        let notes = ss.as_midi_notes();

        assert_eq!(
            notes,
            vec![
                72, 70, 68, 67, 65, 63, 61, 59, 57, 56, 54, 52, 50, 48, 46, 45
            ]
        );
    }

    #[test]
    fn test_major_pentatonic_c4_16_notes() {
        let ss = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::MajorPentatonic,
            direction: SequenceDirection::Ascending,
            octave: Octave::O4,
            length: 16,
        };
        let notes = ss.as_midi_notes();

        assert_eq!(
            notes,
            vec![
                60, 62, 64, 67, 69, 71, 73, 76, 78, 80, 82, 85, 87, 89, 91, 94
            ]
        );
    }

    #[test]
    fn test_chromatic_c4_16_notes() {
        let ss = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Chromatic,
            direction: SequenceDirection::Ascending,
            octave: Octave::O4,
            length: 16,
        };
        let notes = ss.as_midi_notes();

        assert_eq!(
            notes,
            vec![
                60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75
            ]
        );
    }

    #[test]
    fn test_pitch_class_add_semitones() {
        assert_eq!(PitchClass::C.add_semitones(0), PitchClass::C);
        assert_eq!(PitchClass::C.add_semitones(1), PitchClass::Cs);
        assert_eq!(PitchClass::C.add_semitones(12), PitchClass::C);
        assert_eq!(PitchClass::B.add_semitones(1), PitchClass::C);
        assert_eq!(PitchClass::G.add_semitones(5), PitchClass::C);
    }

    #[test]
    fn test_pitch_classes_from_tonic() {
        let c_major = ScaleKind::Major.pitch_classes_from_tonic(&PitchClass::C);
        assert_eq!(
            c_major,
            vec![
                PitchClass::C,
                PitchClass::D,
                PitchClass::E,
                PitchClass::F,
                PitchClass::G,
                PitchClass::A,
                PitchClass::B,
            ]
        );
    }

    #[test]
    fn test_different_octaves() {
        let o2 = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Chromatic,
            direction: SequenceDirection::Ascending,
            octave: Octave::O2,
            length: 1,
        };
        let o6 = ScaleSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Chromatic,
            direction: SequenceDirection::Ascending,
            octave: Octave::O6,
            length: 1,
        };

        assert_eq!(o2.as_midi_notes()[0], 36);
        assert_eq!(o6.as_midi_notes()[0], 84);
    }
}
