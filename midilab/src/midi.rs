use enumeric::range_enum;
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::EnumIter;

use crate::scale::PitchClass;

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

/// Finite enum representation of a midi note value
#[range_enum]
#[repr(u8)]
#[derive(IntoPrimitive, TryFromPrimitive, PartialEq, Clone, Copy, Debug, EnumIter)]
pub enum Note {
    #[range(0..128)]
    N,
}
#[expect(
    clippy::derivable_impls,
    reason = "can't derive default in combination with range_enum"
)]
impl Default for Note {
    fn default() -> Self {
        Self::N60
    }
}
impl Note {
    pub const MAX: u8 = Self::N127 as u8;
}

impl std::fmt::Display for Note {
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
    pub fn new(note: Note) -> Self {
        let level = note as i8 / 12 - 1;

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
        let note = Note::N0;
        assert_eq!(note.to_string(), "C-1");

        let note = Note::N60;
        assert_eq!(note.to_string(), "C4");

        let note = Note::N127;
        assert_eq!(note.to_string(), "G9");
    }
}
