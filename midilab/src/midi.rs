use strum_macros::EnumIter;

use crate::music::theory;
use crate::music::theory::Pitch;

pub mod generation;

/// The octave as represented by Scientific Pitch Notation, bounded to the
/// range representable as MIDI notes (0–127). Use `music::Octave` for
/// unbounded pitch-space arithmetic.
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

/// Saturates on the low end at Osub2 and on the high end at O9.
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

/// Many midi values are u4 with a MIN value of 0 and
/// MAX of 127. A clamped u8 is used instead of a u4 so
/// that we don't have to convert back to u8 before sending
/// over the wire.
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
        let value = value.clamp(Self::MIN, Self::MAX);
        Self(value)
    }
}

impl From<Value> for u8 {
    fn from(val: Value) -> Self {
        val.0
    }
}

/// A MIDI note number, constrained to 0–127.
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
        {
            let val = (12 * (pitch.octave.0 as i16 + 1) + pitch.class as i16).clamp(0, 127) as u8;
            Note::from(val)
        }
    }
}

impl From<&Note> for Pitch {
    fn from(note: &Note) -> Self {
        let value: u8 = note.as_u8();
        Pitch::from(value)
    }
}

/// Converts a MIDI-bounded `midi::Octave` into an unbounded `music::Octave`.
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
}
