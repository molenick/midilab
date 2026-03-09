use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::Display;
use strum_macros::EnumIter;

use crate::midi::Note;

/// Chord voicings represent different ways to arrange the notes of a chord.
#[derive(Clone, Copy, Debug, Display, EnumIter, PartialEq, Eq)]
pub enum ChordVoicing {
    Triad,
    Seventh,
    Inversion1,
    Inversion2,
    Inversion3,
    Drop2,
    Quartal,
    Shell,
    Power,
    Add9,
    NinthNo5,
}

impl ChordVoicing {
    /// Returns (degree_offset, octave_adjustment) for each of the 4 chord voices.
    /// Each tuple represents: (scale degree offset, octave adjustment in semitones).
    /// The semitone value must be a multiple of 12 (i.e. whole octaves only).
    ///
    /// Notes on specific voicings:
    ///
    /// `Quartal` uses every 3rd scale degree (0, 3, 6, 9), producing *diatonic* quartal
    /// harmony. Interval sizes vary with the scale (some perfect 4ths, some tritones)
    /// rather than strictly stacked perfect 4ths.
    ///
    /// `Shell` uses root, 3rd, 7th, and root+octave. Traditional jazz shell voicings
    /// are 3-note (root, 3rd, 7th only), but this adds a doubled root to maintain the
    /// 4-voice model used throughout this API.
    pub(crate) fn chord_offsets(self) -> [(usize, i8); 4] {
        match self {
            Self::Triad => [(0, 0), (2, 0), (4, 0), (0, 12)],
            Self::Seventh => [(0, 0), (2, 0), (4, 0), (6, 0)],
            Self::Inversion1 => [(2, 0), (4, 0), (6, 0), (7, 0)],
            Self::Inversion2 => [(4, 0), (6, 0), (7, 0), (9, 0)],
            Self::Inversion3 => [(6, 0), (7, 0), (9, 0), (11, 0)],
            Self::Drop2 => [(4, -12), (0, 0), (2, 0), (6, 0)],
            Self::Quartal => [(0, 0), (3, 0), (6, 0), (9, 0)],
            Self::Shell => [(0, 0), (2, 0), (6, 0), (7, 0)],
            Self::Power => [(0, 0), (4, 0), (7, 0), (11, 0)],
            Self::Add9 => [(0, 0), (2, 0), (4, 0), (8, 0)],
            Self::NinthNo5 => [(0, 0), (2, 0), (6, 0), (8, 0)],
        }
    }
}

/// Octave in Scientific Pitch Notation. Unbounded — any i8 value is valid.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Octave(pub i8);

impl From<i8> for Octave {
    fn from(value: i8) -> Self {
        Self(value)
    }
}

impl core::fmt::Display for Octave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pitch {
    pub class: PitchClass,
    pub octave: Octave,
}

impl From<u8> for Pitch {
    fn from(value: u8) -> Self {
        let octave = Octave((value / 12) as i8 - 1);
        let class = PitchClass::from_midi_note(value);
        Self { class, octave }
    }
}

impl core::fmt::Display for Pitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let octave_superscript = self
            .octave
            .0
            .to_string()
            .replace('-', "⁻")
            .replace('0', "⁰")
            .replace('1', "¹")
            .replace('2', "²")
            .replace('3', "³")
            .replace('4', "⁴")
            .replace('5', "⁵")
            .replace('6', "⁶")
            .replace('7', "⁷")
            .replace('8', "⁸")
            .replace('9', "⁹");
        write!(f, "{}{}", self.class, octave_superscript)
    }
}

impl Pitch {
    pub fn add_semitones(self, semitones: i8) -> Self {
        let absolute = 12 * (self.octave.0 as i16 + 1) + u8::from(self.class) as i16;
        let new_absolute = (absolute + semitones as i16).clamp(0, 127) as u8;
        Pitch::from(new_absolute)
    }
}

/// The PitchClass according to https://en.wikipedia.org/wiki/Equal_temperament
#[repr(u8)]
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Default, IntoPrimitive, TryFromPrimitive, EnumIter, Hash,
)]
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
    pub fn from_midi_note(val: u8) -> Self {
        PitchClass::try_from(val % 12).unwrap()
    }

    pub fn add_semitones(self, semitones: u8) -> Self {
        let new_value = (u8::from(self) + semitones) % 12;
        PitchClass::try_from(new_value).expect("modulo 12 guarantees valid pitch class")
    }
}

impl From<Note> for PitchClass {
    fn from(note: Note) -> Self {
        let semitone = note.as_u8() % 12;
        PitchClass::try_from_primitive(semitone).unwrap()
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
    // When building a contiguous sequence of notes, we don't want the repeating tonic
    pub fn sequence_intervals(self) -> &'static [u8] {
        let intervals = self.intervals();
        &intervals[..intervals.len().saturating_sub(1)]
    }

    pub fn intervals(self) -> &'static [u8] {
        match self {
            ScaleKind::Major => &[2, 2, 1, 2, 2, 2, 1],
            ScaleKind::NaturalMinor => &[2, 1, 2, 2, 1, 2, 2],
            ScaleKind::HarmonicMinor => &[2, 1, 2, 2, 1, 3, 1],
            ScaleKind::MelodicMinor => &[2, 1, 2, 2, 2, 2, 1],
            ScaleKind::Dorian => &[2, 1, 2, 2, 2, 1, 2],
            ScaleKind::Phrygian => &[1, 2, 2, 2, 1, 2, 2],
            ScaleKind::Lydian => &[2, 2, 2, 1, 2, 2, 1],
            ScaleKind::Mixolydian => &[2, 2, 1, 2, 2, 1, 2],
            ScaleKind::Locrian => &[1, 2, 2, 1, 2, 2, 2],
            ScaleKind::MajorPentatonic => &[2, 2, 3, 2, 3],
            ScaleKind::MinorPentatonic => &[3, 2, 2, 3, 2],
            ScaleKind::WholeTone => &[2, 2, 2, 2, 2, 2],
            ScaleKind::DiminishedHalfWhole => &[1, 2, 1, 2, 1, 2, 1, 2],
            ScaleKind::DiminishedWholeHalf => &[2, 1, 2, 1, 2, 1, 2, 1],
            ScaleKind::Chromatic => &[1; 12],
        }
    }

    /// Returns all pitch classes in this scale starting from the given tonic.
    pub fn pitch_classes_from_tonic(self, tonic: &PitchClass) -> Vec<PitchClass> {
        let intervals = self.sequence_intervals();
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
    use crate::midi::Note;

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
    fn test_negative_octaves_no_overflow() {
        // Osub1: 12 * (-1 + 1) + 0 = 0
        assert_eq!(
            Note::from(&Pitch {
                class: PitchClass::C,
                octave: Octave(-1)
            })
            .as_u8(),
            0
        );
        // Osub1: C# = 1
        assert_eq!(
            Note::from(&Pitch {
                class: PitchClass::Cs,
                octave: Octave(-1)
            })
            .as_u8(),
            1
        );
        // Osub2: 12 * (-2 + 1) + 0 = -12 → clamps to 0
        assert_eq!(
            Note::from(&Pitch {
                class: PitchClass::C,
                octave: Octave(-2)
            })
            .as_u8(),
            0
        );
        // Osub2: B = 11 → 12 * (-1) + 11 = -1 → clamps to 0
        assert_eq!(
            Note::from(&Pitch {
                class: PitchClass::B,
                octave: Octave(-2)
            })
            .as_u8(),
            0
        );
    }
}
