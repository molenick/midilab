use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use strum_macros::Display;
use strum_macros::EnumIter;

use crate::midi::Note;

#[derive(Clone, Copy, Debug)]
pub struct IntervalRowSequence {
    pub base_note: Note,
    /// semitones between rows (e.g. 5 = perfect 4th, 7 = perfect 5th, 12 = octave)
    pub interval: u8,
    pub direction: SequenceDirection,
    pub length: usize,
}

impl Default for IntervalRowSequence {
    fn default() -> Self {
        Self {
            base_note: Note::N36,
            interval: 5,
            direction: SequenceDirection::Ascending,
            length: 64,
        }
    }
}

impl IntervalRowSequence {
    pub fn as_midi_notes(&self) -> Vec<u8> {
        let base: u8 = self.base_note.into();
        let mut notes = Vec::new();
        let mut row_note = base as i16;
        let mut last_valid_note = row_note;

        while notes.len() < self.length {
            if (0..=127).contains(&row_note) {
                last_valid_note = row_note;
                for _ in 0..4 {
                    if notes.len() < self.length {
                        notes.push(row_note as u8);
                    }
                }
            } else {
                while notes.len() < self.length {
                    notes.push(last_valid_note as u8);
                }
                break;
            }
            row_note = match self.direction {
                SequenceDirection::Ascending => row_note + self.interval as i16,
                SequenceDirection::Descending => row_note - self.interval as i16,
            };
        }

        notes
    }
}

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
    /// Returns (degree_offset, semitone_adjustment) for each of the 4 chord voices.
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

#[derive(Clone, Copy, Debug)]
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
        // Build a long run of scale tones from (tonic, octave)
        let start = 12 * (self.octave as i8 + 1) as u8 + (self.tonic as u8);
        let partial_intervals = self.scale.intervals();

        // ScaleKind::intervals() omits the final interval back to the tonic.
        // We need the full octave cycle for correct chord spelling.
        let partial_sum: u8 = partial_intervals.iter().sum();
        let closing_interval = 12u8.saturating_sub(partial_sum);
        let mut full_intervals: Vec<u8> = partial_intervals.to_vec();
        if closing_interval > 0 {
            full_intervals.push(closing_interval);
        }

        // Generate enough scale tones to cover all needed chord tones
        let needed_degrees = (self.length / 4) + 12;
        let mut scale_notes: Vec<u8> = Vec::with_capacity(needed_degrees);

        match self.direction {
            SequenceDirection::Ascending => {
                let mut cur = start;
                scale_notes.push(cur);
                for i in 0.. {
                    if scale_notes.len() >= needed_degrees {
                        break;
                    }
                    let step = full_intervals[i % full_intervals.len()];
                    cur = cur.saturating_add(step).min(127);
                    scale_notes.push(cur);
                }
            }
            SequenceDirection::Descending => {
                let mut cur = start;
                scale_notes.push(cur);
                for i in 0.. {
                    if scale_notes.len() >= needed_degrees {
                        break;
                    }
                    let step = full_intervals[i % full_intervals.len()];
                    cur = cur.saturating_sub(step);
                    scale_notes.push(cur);
                }
            }
        }

        let mut notes = Vec::with_capacity(self.length);
        let mut degree = 0usize;

        let offsets = self.voicing.chord_offsets();
        let max_offset = offsets.iter().map(|(o, _)| *o).max().unwrap();

        while notes.len() < self.length {
            if degree + max_offset >= scale_notes.len() {
                break;
            }
            for &(off, shift) in &offsets {
                if notes.len() < self.length {
                    let note =
                        (scale_notes[degree + off] as i16 + shift as i16).clamp(0, 127) as u8;
                    notes.push(note);
                }
            }
            degree += 1;
        }

        // If we ran out of scale notes before reaching length, fill with last chord
        if notes.len() < self.length && !notes.is_empty() {
            let last_chord_start = (notes.len() / 4).saturating_sub(1) * 4;
            let last_chord: Vec<u8> = notes[last_chord_start..last_chord_start + 4].to_vec();
            while notes.len() < self.length {
                for &n in &last_chord {
                    if notes.len() < self.length {
                        notes.push(n);
                    }
                }
            }
        }

        notes
    }
}

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

#[derive(Clone, Copy, Debug)]
pub struct OctaveRowSequence {
    pub base_note: Note,
    pub direction: SequenceDirection,
    pub length: usize,
}

impl Default for OctaveRowSequence {
    fn default() -> Self {
        Self {
            base_note: Note::N36,
            direction: SequenceDirection::Ascending,
            length: 64,
        }
    }
}

impl OctaveRowSequence {
    pub fn as_midi_notes(&self) -> Vec<u8> {
        let base: u8 = self.base_note.into();
        let mut notes = Vec::new();
        let mut row_note = base as i16;
        let mut last_valid_note = row_note;

        while notes.len() < self.length {
            if (0..=127).contains(&row_note) {
                last_valid_note = row_note;
                for _ in 0..4 {
                    if notes.len() < self.length {
                        notes.push(row_note as u8);
                    }
                }
            } else {
                while notes.len() < self.length {
                    notes.push(last_valid_note as u8);
                }
                break;
            }
            row_note = match self.direction {
                SequenceDirection::Ascending => row_note + 12,
                SequenceDirection::Descending => row_note - 12,
            };
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
    fn test_interval_row_fourths() {
        let seq = IntervalRowSequence {
            base_note: Note::N60,
            interval: 5,
            direction: SequenceDirection::Ascending,
            length: 16,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(
            notes,
            vec![
                60, 60, 60, 60, 65, 65, 65, 65, 70, 70, 70, 70, 75, 75, 75, 75
            ]
        );
    }

    #[test]
    fn test_interval_row_saturates() {
        let seq = IntervalRowSequence {
            base_note: Note::N120,
            interval: 5,
            direction: SequenceDirection::Ascending,
            length: 16,
        };
        let notes = seq.as_midi_notes();
        assert_eq!(notes.len(), 16);
        assert_eq!(&notes[0..4], &[120, 120, 120, 120]);
        assert_eq!(&notes[4..8], &[125, 125, 125, 125]);
        // 130 > 127 so saturates at 125
        assert_eq!(&notes[8..12], &[125, 125, 125, 125]);
        assert_eq!(&notes[12..16], &[125, 125, 125, 125]);
    }

    #[test]
    fn test_interval_row_octave_matches_octave_row() {
        let interval_seq = IntervalRowSequence {
            base_note: Note::N36,
            interval: 12,
            direction: SequenceDirection::Ascending,
            length: 64,
        };
        let octave_seq = OctaveRowSequence {
            base_note: Note::N36,
            direction: SequenceDirection::Ascending,
            length: 64,
        };
        assert_eq!(interval_seq.as_midi_notes(), octave_seq.as_midi_notes());
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
}
