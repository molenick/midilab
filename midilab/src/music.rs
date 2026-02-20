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
    fn chord_offsets(self) -> [(usize, i8); 4] {
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

/// Creates a "chord row" - a 4 note sequence that voices a chord
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChordRowSequence {
    pub tonic: PitchClass,
    pub scale: ScaleKind,
    pub octave: Octave,
    pub voicing: ChordVoicing,
    pub direction: SequenceDirection,
    pub length: usize,
}

impl Default for ChordRowSequence {
    fn default() -> Self {
        Self {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Seventh,
            direction: SequenceDirection::Ascending,
            length: 64,
        }
    }
}

impl ChordRowSequence {
    pub fn as_midi_notes(&self) -> Vec<u8> {
        self.as_pitches().iter().map(|p| p.as_midi_note()).collect()
    }

    pub fn as_pitches(&self) -> Vec<Pitch> {
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
            let step = match self.direction {
                SequenceDirection::Ascending => intervals[i % intervals.len()],
                SequenceDirection::Descending => {
                    intervals[(intervals.len() - 1) - (i % intervals.len())]
                }
            };
            current_pitch = current_pitch.advance(self.tonic, self.direction, step);
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
                        octave: Octave::from(base.octave as i8 + shift / 12),
                    };
                    pitches.push(pitch);
                }
            }
            degree += 1;
        }

        pitches
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
    pub fn as_midi_notes(&self) -> Vec<u8> {
        let pitches: Vec<Pitch> = self.as_pitches();
        let mut notes = Vec::with_capacity(self.length);

        for pitch in pitches {
            notes.push(pitch.as_midi_note());
        }

        notes
    }

    pub fn as_pitches(&self) -> Vec<Pitch> {
        let intervals = self.scale.intervals();
        let mut pitches = Vec::with_capacity(self.length);
        let mut current_pitch = Pitch {
            class: self.tonic,
            octave: self.octave,
        };

        for i in 0..self.length {
            pitches.push(current_pitch);

            let step = match self.direction {
                SequenceDirection::Ascending => intervals[i % intervals.len()],
                SequenceDirection::Descending => {
                    intervals[(intervals.len() - 1) - (i % intervals.len())]
                }
            };
            current_pitch = current_pitch.advance(self.tonic, self.direction, step);
        }

        pitches
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
            i if i >= 9 => Octave::O9,
            _ => unreachable!("guarded arms are exhaustive"),
        }
    }
}

impl core::fmt::Display for Octave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = (*self as i8).to_string();

        write!(f, "{out}")
    }
}

#[derive(Clone, Copy, Debug, Display, EnumIter, PartialEq, Eq)]
pub enum SequenceDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pitch {
    pub class: PitchClass,
    pub octave: Octave,
}

impl Pitch {
    pub fn as_midi_note(&self) -> u8 {
        (12 * (self.octave as i16 + 1) + self.class as i16).clamp(0, 127) as u8
    }

    pub fn advance(&self, _tonic: PitchClass, direction: SequenceDirection, step: u8) -> Self {
        let current_midi = self.as_midi_note();
        let new_midi = match direction {
            SequenceDirection::Ascending => current_midi.saturating_add(step).min(127),
            SequenceDirection::Descending => current_midi.saturating_sub(step),
        };
        Pitch::from(new_midi)
    }
}

impl From<u8> for Pitch {
    fn from(value: u8) -> Self {
        let octave = (value / 12) as i8 - 1;
        let octave = Octave::from(octave);
        let class = PitchClass::from_midi_note(value);
        Self { class, octave }
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

impl From<Note> for PitchClass {
    fn from(note: Note) -> Self {
        {
            let semitone = u8::from(note) % 12;
            PitchClass::try_from_primitive(semitone).unwrap()
        }
    }
}

#[derive(Clone, Copy, Debug, Display, PartialEq)]
pub enum NotePattern {
    #[strum(to_string = "Scale")]
    Scale(ScaleSequence),
    #[strum(to_string = "Chord")]
    ChordRow(ChordRowSequence),
}

impl NotePattern {
    pub fn as_pitches(&self) -> Vec<Pitch> {
        match self {
            NotePattern::Scale(sequence) => sequence.as_pitches(),
            NotePattern::ChordRow(sequence) => sequence.as_pitches(),
        }
    }

    pub fn as_midi_notes(&self) -> Vec<u8> {
        match self {
            NotePattern::Scale(seq) => seq.as_midi_notes(),
            NotePattern::ChordRow(seq) => seq.as_midi_notes(),
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            NotePattern::Scale(sequence) => sequence.length,
            NotePattern::ChordRow(sequence) => sequence.length,
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
        let notes: Vec<u8> = ss.as_midi_notes();

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
            octave: Octave::O4,
            length: 16,
        };
        let notes = ss.as_midi_notes();

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
            octave: Octave::O5,
            length: 16,
        };
        let notes = ss.as_midi_notes();

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
            octave: Octave::O4,
            length: 16,
        };
        let notes = ss.as_midi_notes();

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
    fn test_chord_row_c_major_seventh() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Seventh,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        // Cmaj7: C4(60), E4(64), G4(67), B4(71)
        // Dm7:   D4(62), F4(65), A4(69), C5(72)
        assert_eq!(notes, vec![60, 64, 67, 71, 62, 65, 69, 72]);
    }

    #[test]
    fn test_chord_row_c_major_triad() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Triad,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        // C triad + root octave: C4(60), E4(64), G4(67), C5(72)
        // D triad + root octave: D4(62), F4(65), A4(69), D5(74)
        assert_eq!(notes, vec![60, 64, 67, 72, 62, 65, 69, 74]);
    }

    #[test]
    fn test_chord_row_saturates() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O9,
            voicing: ChordVoicing::Seventh,
            direction: SequenceDirection::Ascending,
            length: 16,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes.len(), 16);
        // Should not panic and all notes should be <= 127
        for &n in &notes {
            assert!(n <= 127);
        }
    }

    #[test]
    fn test_chord_row_inversion1() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Inversion1,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes, vec![64, 67, 71, 72, 65, 69, 72, 74]);
    }

    #[test]
    fn test_chord_row_inversion2() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Inversion2,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes, vec![67, 71, 72, 76, 69, 72, 74, 77]);
    }

    #[test]
    fn test_chord_row_inversion3() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Inversion3,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes, vec![71, 72, 76, 79, 72, 74, 77, 81]);
    }

    #[test]
    fn test_chord_row_drop2() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Drop2,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes, vec![55, 60, 64, 71, 57, 62, 65, 72]);
    }

    #[test]
    fn test_chord_row_quartal() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Quartal,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes, vec![60, 65, 71, 76, 62, 67, 72, 77]);
    }

    #[test]
    fn test_chord_row_shell() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Shell,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes, vec![60, 64, 71, 72, 62, 65, 72, 74]);
    }

    #[test]
    fn test_chord_row_power() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Power,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes, vec![60, 67, 72, 79, 62, 69, 74, 81]);
    }

    #[test]
    fn test_chord_row_add9() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::Add9,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes, vec![60, 64, 67, 74, 62, 65, 69, 76]);
    }

    #[test]
    fn test_chord_row_ninth_no5() {
        let seq = ChordRowSequence {
            tonic: PitchClass::C,
            scale: ScaleKind::Major,
            octave: Octave::O4,
            voicing: ChordVoicing::NinthNo5,
            direction: SequenceDirection::Ascending,
            length: 8,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes, vec![60, 64, 71, 74, 62, 65, 72, 76]);
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

    #[test]
    fn test_negative_octaves_no_overflow() {
        // Osub1: 12 * (-1 + 1) + 0 = 0
        assert_eq!(
            Pitch {
                class: PitchClass::C,
                octave: Octave::Osub1
            }
            .as_midi_note(),
            0
        );
        // Osub1: C# = 1
        assert_eq!(
            Pitch {
                class: PitchClass::Cs,
                octave: Octave::Osub1
            }
            .as_midi_note(),
            1
        );
        // Osub2: 12 * (-2 + 1) + 0 = -12 → clamps to 0
        assert_eq!(
            Pitch {
                class: PitchClass::C,
                octave: Octave::Osub2
            }
            .as_midi_note(),
            0
        );
        // Osub2: B = 11 → 12 * (-1) + 11 = -1 → clamps to 0
        assert_eq!(
            Pitch {
                class: PitchClass::B,
                octave: Octave::Osub2
            }
            .as_midi_note(),
            0
        );
    }
}
