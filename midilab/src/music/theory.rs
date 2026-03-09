/// Music theory abstractions
///
/// This module provides types and utilities for working with musical scales,
/// chord voicings, and pitch calculations.
///
/// ## Modules
///
/// - `ScaleKind`: Common musical scales with interval definitions
/// - `PitchClass`: Musical pitch classes (C, C#, D, etc.)
/// - `Octave`: Unbounded octave representation
/// - `Pitch`: Combined pitch class and octave
/// - `ChordVoicing`: Different ways to arrange chord notes
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::Display;
use strum_macros::EnumIter;

use crate::midi::Note;

/// Chord voicings represent different ways to arrange the notes of a chord.
///
/// Each voicing is represented as a 4-voice texture with offsets from a root
/// scale degree, and optional octave shifts. The offsets are expressed as
/// (degree_offset, octave_adjustment) tuples.
#[derive(Clone, Copy, Debug, Display, EnumIter, PartialEq, Eq)]
pub enum ChordVoicing {
    /// Basic 3-note chord (root, 3rd, 5th) extended to 4 voices by adding an octave root.
    Triad,
    /// 4-note seventh chord (root, 3rd, 5th, 7th).
    Seventh,
    /// First inversion: 3rd, 5th, 7th, 9th (root moved up an octave).
    Inversion1,
    /// Second inversion: 5th, 7th, 9th, 11th (root and 3rd moved up).
    Inversion2,
    /// Third inversion: 7th, 9th, 11th, 13th (root, 3rd, 5th moved up).
    Inversion3,
    /// Drop 2 voicing: 5th voice dropped down an octave (5th, root, 3rd, 7th).
    Drop2,
    /// Quartal harmony: built in 4th intervals (root, 3rd up, 6th up, 9th up).
    Quartal,
    /// Jazz shell voicing: root, 3rd, 7th, octave root (omits 5th for compactness).
    Shell,
    /// Power chord: root, 5th, octave root, octave 5th (root and 5th doubled across octaves).
    Power,
    /// Added 9th chord: root, 3rd, 5th, 9th (adds 9th without 7th).
    Add9,
    /// 9th chord without 5th: root, 3rd, 7th, 9th (omits 5th for cleaner voicing).
    NinthNo5,
}

impl ChordVoicing {
    /// Returns voice offsets for a 4-voice chord arrangement.
    ///
    /// Each tuple contains:
    /// - `degree_offset`: Scale degree offset from the root (0 = root, 2 = 3rd, 4 = 5th, etc.)
    /// - `octave_adjustment`: Octave shift in semitones (must be multiple of 12)
    ///
    /// # Examples
    ///
    /// ```
    /// use midilab::music::theory::ChordVoicing;
    ///
    /// // Seventh voicing: root, 3rd, 5th, 7th (all in same octave)
    /// // Each tuple is (scale_degree_offset, octave_shift)
    /// // offsets = [(0,0), (2,0), (4,0), (6,0)]
    /// ```
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

/// Octave in Scientific Pitch Notation.
///
/// Unlike `midi::Octave` which is bounded to MIDI range, this octave
/// can represent any octave from -∞ to +∞, useful for theoretical calculations.
///
/// # Examples
///
/// ```
/// use midilab::music::theory::Octave;
///
/// let octave_4 = Octave(4);  // C4 is middle C
/// let octave_minus_1 = Octave(-1);  // Below O0
/// let octave_9 = Octave(9);  // Very high
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Octave(pub i8);

impl From<i8> for Octave {
    /// Constructs an octave from an i8 value.
    fn from(value: i8) -> Self {
        Self(value)
    }
}

impl core::fmt::Display for Octave {
    /// Formats the octave number in standard Scientific Pitch Notation.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A musical pitch combining pitch class and octave.
///
/// This represents a complete pitch in Scientific Pitch Notation (e.g., C⁴, A♭⁵).
/// Use `PitchClass` alone for pitch class only, `Octave` alone for octave number.
///
/// # Examples
///
/// ```
/// use midilab::music::theory::{Pitch, PitchClass, Octave};
///
/// let middle_c = Pitch {
///     class: PitchClass::C,
///     octave: Octave(4),
/// };
/// assert_eq!(middle_c.to_string(), "C⁴");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pitch {
    /// The pitch class (note name without octave)
    pub class: PitchClass,
    /// The octave number in Scientific Pitch Notation
    pub octave: Octave,
}

impl From<u8> for Pitch {
    /// Converts a MIDI note number (0-127) to a Pitch.
    ///
    /// MIDI note 0 = C⁻¹ (octave -1, pitch class C)
    /// MIDI note 60 = C⁴ (middle C)
    /// MIDI note 127 = G⁹
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

/// Pitch class according to [Equal temperament](https://en.wikipedia.org/wiki/Equal_temperament).
///
/// Represents the 12 pitch classes of the chromatic scale:
/// C, C#, D, D#, E, F, F#, G, G#, A, A#, B
///
/// This type is representation-optimized with `#[repr(u8)]` and supports
/// conversions to/from MIDI notes (mod 12).
#[repr(u8)]
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Default, IntoPrimitive, TryFromPrimitive, EnumIter, Hash,
)]
pub enum PitchClass {
    #[default]
    /// C pitch class
    C,
    /// C sharp (D flat) pitch class
    Cs,
    /// D pitch class
    D,
    /// D sharp (E flat) pitch class
    Ds,
    /// E pitch class
    E,
    /// F pitch class
    F,
    /// F sharp (G flat) pitch class
    Fs,
    /// G pitch class
    G,
    /// G sharp (A flat) pitch class
    Gs,
    /// A pitch class
    A,
    /// A sharp (B flat) pitch class
    As,
    /// B pitch class
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
    /// Constructs a pitch class from a MIDI note number.
    ///
    /// The note number is reduced modulo 12 to get the pitch class.
    pub fn from_midi_note(val: u8) -> Self {
        PitchClass::try_from(val % 12).unwrap()
    }

    /// Adds semitones to this pitch class, wrapping at the octave.
    ///
    /// # Examples
    ///
    /// ```
    /// use midilab::music::theory::PitchClass;
    ///
    /// assert_eq!(PitchClass::C.add_semitones(0), PitchClass::C);
    /// assert_eq!(PitchClass::C.add_semitones(1), PitchClass::Cs);
    /// assert_eq!(PitchClass::C.add_semitones(12), PitchClass::C);
    /// assert_eq!(PitchClass::B.add_semitones(1), PitchClass::C);
    /// ```
    pub fn add_semitones(self, semitones: u8) -> Self {
        let new_value = (u8::from(self) + semitones) % 12;
        PitchClass::try_from(new_value).expect("modulo 12 guarantees valid pitch class")
    }
}

/// Converts a MIDI `Note` to a pitch class.
impl From<Note> for PitchClass {
    fn from(note: Note) -> Self {
        let semitone = note.as_u8() % 12;
        PitchClass::try_from_primitive(semitone).unwrap()
    }
}

/// Common scale variants used for note generation and scale synthesis.
///
/// This enum defines 15 different scale types spanning major, minor, modal,
/// pentatonic, whole tone, diminished, and chromatic scales.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Display, EnumIter)]
pub enum ScaleKind {
    /// Major scale: W-W-H-W-W-W-H (whole-whole-half)
    Major,
    /// Natural minor scale: W-H-W-W-H-W-W
    NaturalMinor,
    /// Harmonic minor scale: W-H-W-W-H-½-H (augmented 2nd between 6th and 7th)
    HarmonicMinor,
    /// Melodic minor scale: W-H-W-W-W-W-H (ascending), different descending
    MelodicMinor,
    /// Dorian mode: W-H-W-W-W-H-W (minor with raised 6th)
    Dorian,
    /// Phrygian mode: H-W-W-W-H-W-W (minor with lowered 2nd)
    Phrygian,
    /// Lydian mode: W-W-W-H-W-W-H (major with raised 4th)
    Lydian,
    /// Mixolydian mode: W-W-H-W-W-H-W (major with lowered 7th)
    Mixolydian,
    /// Locrian mode: H-W-W-H-W-W-W (diminished with lowered 2nd and 5th)
    Locrian,
    /// Major pentatonic scale: W-W-½-W-½ (C-D-E-G-A)
    MajorPentatonic,
    /// Minor pentatonic scale: ½-W-W-½-W-W (C-E♭-F-G-B♭)
    MinorPentatonic,
    /// Whole tone scale: W-W-W-W-W-W (six equal major 2nds)
    WholeTone,
    /// Diminished half-whole: H-W-H-W-H-W-H-W (alternating half/whole steps)
    DiminishedHalfWhole,
    /// Diminished whole-half: W-H-W-H-W-H-W-H (alternating whole/half steps)
    DiminishedWholeHalf,
    /// Chromatic scale: 12 half steps (H×12)
    Chromatic,
}

impl ScaleKind {
    /// Returns scale intervals excluding the final repeating tonic.
    ///
    /// When building a contiguous sequence of notes, we don't want the
    /// repeating tonic at the end, so this returns all intervals except the last.
    ///
    /// # Examples
    ///
    /// ```
    /// use midilab::music::theory::ScaleKind;
    ///
    /// // Major scale has 7 intervals, but 8 notes (including repeat of tonic)
    /// let intervals = ScaleKind::Major.sequence_intervals();
    /// assert_eq!(intervals, &[2, 2, 1, 2, 2, 2]);  // excludes final 1
    /// ```
    pub fn sequence_intervals(self) -> &'static [u8] {
        let intervals = self.intervals();
        &intervals[..intervals.len().saturating_sub(1)]
    }

    /// Returns the sequence of semitone intervals that define this scale.
    ///
    /// The intervals are measured in semitones from the tonic.
    /// For most scales, this includes the interval from the last scale degree
    /// back to the tonic (completing the octave).
    ///
    /// # Examples
    ///
    /// ```
    /// use midilab::music::theory::ScaleKind;
    ///
    /// // Major scale: whole-whole-half-whole-whole-whole-half
    /// let intervals = ScaleKind::Major.intervals();
    /// assert_eq!(intervals, &[2, 2, 1, 2, 2, 2, 1]);
    ///
    /// // Chromatic scale: 12 half steps
    /// let chromatic = ScaleKind::Chromatic.intervals();
    /// assert_eq!(chromatic, &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
    /// ```
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

    /// Generates all pitch classes in this scale starting from the given tonic.
    ///
    /// Returns a vector of pitch classes including the starting tonic and
    /// all notes up to (but not including) the repeated tonic at the octave.
    ///
    /// # Examples
    ///
    /// ```
    /// use midilab::music::theory::{ScaleKind, PitchClass};
    ///
    /// // C major scale: C D E F G A B
    /// let c_major = ScaleKind::Major.pitch_classes_from_tonic(&PitchClass::C);
    /// assert_eq!(c_major.len(), 7);
    ///
    /// // A natural minor: A B C D E F G
    /// let a_minor = ScaleKind::NaturalMinor.pitch_classes_from_tonic(&PitchClass::A);
    /// assert_eq!(a_minor, vec![
    ///     PitchClass::A, PitchClass::B, PitchClass::C,
    ///     PitchClass::D, PitchClass::E, PitchClass::F, PitchClass::G
    /// ]);
    /// ```
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
    use strum::IntoEnumIterator;

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
    fn test_interval_array_count() {
        for scale in ScaleKind::iter() {
            let intervals = scale.intervals();
            let sequence = scale.sequence_intervals();

            match scale {
                ScaleKind::Chromatic => {
                    assert_eq!(intervals.len(), 12);
                    assert_eq!(sequence.len(), 11);
                }
                ScaleKind::WholeTone => {
                    assert_eq!(intervals.len(), 6);
                    assert_eq!(sequence.len(), 5);
                }
                ScaleKind::MajorPentatonic | ScaleKind::MinorPentatonic => {
                    assert_eq!(intervals.len(), 5);
                    assert_eq!(sequence.len(), 4);
                }
                ScaleKind::DiminishedHalfWhole | ScaleKind::DiminishedWholeHalf => {
                    assert_eq!(intervals.len(), 8);
                    assert_eq!(sequence.len(), 7);
                }
                _ => {
                    assert_eq!(intervals.len(), 7);
                    assert_eq!(sequence.len(), 6);
                }
            }
        }
    }

    #[test]
    fn test_octave_display() {
        assert_eq!(Octave(-2).to_string(), "-2");
        assert_eq!(Octave(-1).to_string(), "-1");
        assert_eq!(Octave(0).to_string(), "0");
        assert_eq!(Octave(4).to_string(), "4");
        assert_eq!(Octave(9).to_string(), "9");
    }

    #[test]
    fn test_pitch_display_with_superscripts() {
        assert_eq!(Pitch::from(60).to_string(), "C⁴");
        assert_eq!(Pitch::from(0).to_string(), "C⁻¹");
        assert_eq!(Pitch::from(127).to_string(), "G⁹");
        assert_eq!(Pitch::from(61).to_string(), "C#⁴");
        assert_eq!(Pitch::from(71).to_string(), "B⁴");
    }

    #[test]
    fn test_chord_offsets_triad() {
        let offsets = ChordVoicing::Triad.chord_offsets();
        assert_eq!(offsets, [(0, 0), (2, 0), (4, 0), (0, 12)]);
    }

    #[test]
    fn test_chord_offsets_seventh() {
        let offsets = ChordVoicing::Seventh.chord_offsets();
        assert_eq!(offsets, [(0, 0), (2, 0), (4, 0), (6, 0)]);
    }

    #[test]
    fn test_chord_offsets_inversion1() {
        let offsets = ChordVoicing::Inversion1.chord_offsets();
        assert_eq!(offsets, [(2, 0), (4, 0), (6, 0), (7, 0)]);
    }

    #[test]
    fn test_chord_offsets_inversion2() {
        let offsets = ChordVoicing::Inversion2.chord_offsets();
        assert_eq!(offsets, [(4, 0), (6, 0), (7, 0), (9, 0)]);
    }

    #[test]
    fn test_chord_offsets_inversion3() {
        let offsets = ChordVoicing::Inversion3.chord_offsets();
        assert_eq!(offsets, [(6, 0), (7, 0), (9, 0), (11, 0)]);
    }

    #[test]
    fn test_chord_offsets_drop2() {
        let offsets = ChordVoicing::Drop2.chord_offsets();
        assert_eq!(offsets, [(4, -12), (0, 0), (2, 0), (6, 0)]);
    }

    #[test]
    fn test_chord_offsets_quartal() {
        let offsets = ChordVoicing::Quartal.chord_offsets();
        assert_eq!(offsets, [(0, 0), (3, 0), (6, 0), (9, 0)]);
    }

    #[test]
    fn test_chord_offsets_shell() {
        let offsets = ChordVoicing::Shell.chord_offsets();
        assert_eq!(offsets, [(0, 0), (2, 0), (6, 0), (7, 0)]);
    }

    #[test]
    fn test_chord_offsets_power() {
        let offsets = ChordVoicing::Power.chord_offsets();
        assert_eq!(offsets, [(0, 0), (4, 0), (7, 0), (11, 0)]);
    }

    #[test]
    fn test_chord_offsets_add9() {
        let offsets = ChordVoicing::Add9.chord_offsets();
        assert_eq!(offsets, [(0, 0), (2, 0), (4, 0), (8, 0)]);
    }

    #[test]
    fn test_chord_offsets_ninth_no5() {
        let offsets = ChordVoicing::NinthNo5.chord_offsets();
        assert_eq!(offsets, [(0, 0), (2, 0), (6, 0), (8, 0)]);
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
