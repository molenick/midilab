use strum_macros::EnumIter;

use crate::music::generation::PitchSequence;
use crate::music::theory::Pitch;
use crate::music::theory::{self};

/// A sequence of MIDI notes.
pub struct NoteSequence(pub Vec<Note>);

impl From<PitchSequence> for NoteSequence {
    fn from(pitches: PitchSequence) -> Self {
        Self(pitches.0.iter().map(Note::from).collect())
    }
}

/// The octave as represented by Scientific Pitch Notation, bounded to the
/// range representable as MIDI notes (0–127).
///
/// Use `music::theory::Octave` for unbounded pitch-space arithmetic.
///
/// # MIDI Note Range
///
/// - `Osub2` (MIDI 0) = C⁻¹ (lowest)
/// - `O9` (MIDI 127) = G⁹ (highest)
///
/// # Examples
///
/// ```
/// use midilab::midi::Octave;
///
/// let oct = Octave::O4;
/// assert_eq!(oct.to_string(), "4");
/// ```
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

/// Constructs an `Octave` from an integer, saturating at the MIDI range bounds.
///
/// Values ≤ -2 become `Osub2`. Values ≥ 9 become `O9`.
///
/// # Examples
///
/// ```
/// use midilab::midi::Octave;
///
/// let o: Octave = Octave::from(-5);
/// assert_eq!(o, Octave::Osub2);
///
/// let o: Octave = Octave::from(10);
/// assert_eq!(o, Octave::O9);
///
/// let o: Octave = Octave::from(4);
/// assert_eq!(o, Octave::O4);
/// ```
impl From<i8> for Octave {
    fn from(value: i8) -> Self {
        match value {
            i if i <= -2 => Octave::Osub2,
            -1 => Octave::Osub1,
            0 => Octave::O0,
            1 => Octave::O1,
            2 => Octave::O2,
            3 => Octave::O3,
            4 => Octave::O4,
            5 => Octave::O5,
            6 => Octave::O6,
            7 => Octave::O7,
            8 => Octave::O8,
            _ => Octave::O9,
        }
    }
}

impl core::fmt::Display for Octave {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", *self as i8)
    }
}

/// A 7-bit MIDI value clamped to the range 0–127.
///
/// Many MIDI protocols use 4-bit or 7-bit values. This type ensures
/// values stay within the valid MIDI range by clamping on conversion.
/// Use `u8` directly for values outside 0–127, they will be clamped.
///
/// # Examples
///
/// ```
/// use midilab::midi::Value;
///
/// let v = Value::from(64);
/// assert_eq!(v.as_u8(), 64);
///
/// let v = Value::from(200);
/// assert_eq!(v.as_u8(), 127);
/// ```
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Value(u8);

impl Value {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 127;

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }
}

impl From<Value> for u8 {
    fn from(val: Value) -> Self {
        val.0
    }
}

/// A MIDI note number, constrained to 0–127.
///
/// This type wraps a `Value` to represent a specific note in the MIDI range.
/// It provides conversions to/from `u8`, `Value`, `Pitch`, and supports
/// display formatting in Scientific Pitch Notation.
///
/// # Examples
///
/// ```
/// use midilab::midi::Note;
///
/// let middle_c = Note::from(60);
/// assert_eq!(middle_c.to_string(), "C⁴");
/// assert_eq!(middle_c.as_u8(), 60);
/// ```
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Note(Value);

impl core::fmt::Display for Note {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let pitch = Pitch::from(self);
        write!(f, "{}", pitch)
    }
}

impl Note {
    pub fn as_u8(&self) -> u8 {
        self.0.as_u8()
    }
}

impl From<u8> for Note {
    fn from(value: u8) -> Self {
        Self(Value::from(value))
    }
}

impl From<Note> for u8 {
    fn from(note: Note) -> Self {
        note.0.into()
    }
}

impl From<Value> for Note {
    fn from(value: Value) -> Self {
        Self(value)
    }
}

impl From<Note> for Value {
    fn from(note: Note) -> Self {
        note.0
    }
}

impl From<&Pitch> for Note {
    fn from(pitch: &Pitch) -> Self {
        let val = (12 * (pitch.octave.0 as i16 + 1) + pitch.class as i16).clamp(0, 127) as u8;
        Note::from(val)
    }
}

impl From<&Note> for Pitch {
    fn from(note: &Note) -> Self {
        let value: u8 = note.as_u8();
        Pitch::from(value)
    }
}

/// Converts a MIDI-bounded `midi::Octave` into an unbounded `music::Octave`.
///
/// # Examples
///
/// ```
/// use midilab::midi::Octave;
/// use midilab::music::theory::Octave as TheoryOctave;
///
/// let midi_octave = Octave::O4;
/// let theory_octave = TheoryOctave::from(midi_octave);
/// assert_eq!(theory_octave.0, 4);
/// ```
impl From<Octave> for theory::Octave {
    fn from(o: Octave) -> Self {
        theory::Octave(o as i8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_notes() {
        let note = Note::from(0);
        assert_eq!(note.to_string(), "C⁻¹");

        let note = Note::from(60);
        assert_eq!(note.to_string(), "C⁴");

        let note = Note::from(127);
        assert_eq!(note.to_string(), "G⁹");
    }

    #[test]
    fn test_value_clamping() {
        assert_eq!(Value::from(0).as_u8(), 0);
        assert_eq!(Value::from(127).as_u8(), 127);
        assert_eq!(Value::from(128).as_u8(), 127);
        assert_eq!(Value::from(255).as_u8(), 127);
    }

    #[test]
    fn test_octave_from_i8() {
        assert_eq!(Octave::from(-5), Octave::Osub2);
        assert_eq!(Octave::from(-2), Octave::Osub2);
        assert_eq!(Octave::from(-1), Octave::Osub1);
        assert_eq!(Octave::from(0), Octave::O0);
        assert_eq!(Octave::from(4), Octave::O4);
        assert_eq!(Octave::from(8), Octave::O8);
        assert_eq!(Octave::from(9), Octave::O9);
        assert_eq!(Octave::from(10), Octave::O9);
    }

    #[test]
    fn test_octave_display() {
        assert_eq!(Octave::O0.to_string(), "0");
        assert_eq!(Octave::O4.to_string(), "4");
        assert_eq!(Octave::O9.to_string(), "9");
    }

    #[test]
    fn test_note_roundtrip() {
        let note = Note::from(60);
        let u8_val: u8 = note.into();
        assert_eq!(u8_val, 60);

        let note: Note = u8_val.into();
        assert_eq!(note.as_u8(), 60);
    }

    #[test]
    fn test_note_sequence_conversion() {
        use crate::music::generation::PitchSequence;

        let pitches = PitchSequence(vec![
            crate::music::theory::Pitch {
                class: crate::music::theory::PitchClass::C,
                octave: crate::music::theory::Octave(4),
            },
            crate::music::theory::Pitch {
                class: crate::music::theory::PitchClass::E,
                octave: crate::music::theory::Octave(4),
            },
            crate::music::theory::Pitch {
                class: crate::music::theory::PitchClass::G,
                octave: crate::music::theory::Octave(4),
            },
        ]);

        let seq = NoteSequence::from(pitches);
        assert_eq!(seq.0.len(), 3);
        assert_eq!(seq.0[0].as_u8(), 60);
    }
}
