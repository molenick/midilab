use strum_macros::EnumIter;

use crate::music::Pitch;
use crate::music::PitchClass;

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as i8)
    }
}

/// Many midi values are u4 with a MIN value of 0 and
/// MAX of 127. A clamped u8 is used instead of a u4 so
/// that we don't have to convert back to u8 before sending
/// over the wire.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MidiValue(u8);
impl MidiValue {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 127;

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl From<u8> for MidiValue {
    fn from(value: u8) -> Self {
        let value = value.clamp(Self::MIN, Self::MAX);
        Self(value)
    }
}

impl From<MidiValue> for u8 {
    fn from(val: MidiValue) -> Self {
        val.0
    }
}

/// A MIDI note number, constrained to 0–127.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MidiNote(MidiValue);

impl MidiNote {
    pub fn as_u8(&self) -> u8 {
        self.0.as_u8()
    }
}

impl From<u8> for MidiNote {
    fn from(value: u8) -> Self {
        Self(MidiValue::from(value))
    }
}

impl From<MidiNote> for u8 {
    fn from(note: MidiNote) -> Self {
        note.0.into()
    }
}

impl From<MidiValue> for MidiNote {
    fn from(value: MidiValue) -> Self {
        Self(value)
    }
}

impl From<MidiNote> for MidiValue {
    fn from(note: MidiNote) -> Self {
        note.0
    }
}

impl From<&Pitch> for MidiNote {
    fn from(pitch: &Pitch) -> Self {
        {
            let val = (12 * (pitch.octave.0 as i16 + 1) + pitch.class as i16).clamp(0, 127) as u8;
            MidiNote::from(val)
        }
    }
}

impl std::fmt::Display for MidiNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = RolandMidiOctave::new(*self);
        let pitch_class = PitchClass::from(*self);

        write!(f, "{}{}", pitch_class, level)
    }
}

/// A representation of octave level in Scientific Pitch Notation. Named colloquially
/// so we don't have to shorthand w/ acronym or use the monstrous ScientificPitchNotation.
/// It's Roland, where middle C is C4 as opposed Yamaha where it is C3.
#[derive(Debug)]
pub struct RolandMidiOctave(i8);
impl RolandMidiOctave {
    pub fn new(note: MidiNote) -> Self {
        let level = note.as_u8() as i8 / 12 - 1;

        let clamped = level.clamp(-1, 9);
        Self(clamped)
    }
}
impl std::fmt::Display for RolandMidiOctave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_notes() {
        let note = MidiNote::from(0);
        assert_eq!(note.to_string(), "C-1");

        let note = MidiNote::from(60);
        assert_eq!(note.to_string(), "C4");

        let note = MidiNote::from(127);
        assert_eq!(note.to_string(), "G9");
    }
}
