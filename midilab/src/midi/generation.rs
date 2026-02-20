use crate::midi::MidiNote;
use crate::music::generation::PitchSequence;

pub struct MidiNoteSequence(pub Vec<MidiNote>);

impl From<PitchSequence> for MidiNoteSequence {
    fn from(pitches: PitchSequence) -> Self {
        // Saturates out-of-range pitches to MIDI bounds [0, 127].
        // TODO: consider notifying the user when saturation occurs.
        Self(pitches.0.iter().map(MidiNote::from).collect())
    }
}
