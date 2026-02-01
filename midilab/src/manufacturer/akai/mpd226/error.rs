use num_enum::TryFromPrimitiveError;

use crate::manufacturer::akai::mpd226::control::value_kind::ActiveState;
use crate::manufacturer::akai::mpd226::control::value_kind::AfterTouchKind;
use crate::manufacturer::akai::mpd226::control::value_kind::DialKind;
use crate::manufacturer::akai::mpd226::control::value_kind::FaderKind;
use crate::manufacturer::akai::mpd226::control::value_kind::GateValue;
use crate::manufacturer::akai::mpd226::control::value_kind::MidiChannel;
use crate::manufacturer::akai::mpd226::control::value_kind::PadColor;
use crate::manufacturer::akai::mpd226::control::value_kind::PadKind;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetSlot;
use crate::manufacturer::akai::mpd226::control::value_kind::SwingKind;
use crate::manufacturer::akai::mpd226::control::value_kind::SwitchKey2;
use crate::manufacturer::akai::mpd226::control::value_kind::SwitchKind;
use crate::manufacturer::akai::mpd226::control::value_kind::TimeDivision;
use crate::manufacturer::akai::mpd226::control::value_kind::TransportKind;
use crate::manufacturer::akai::mpd226::control::value_kind::TriggerKind;
use crate::midi::Note;

#[derive(Debug, thiserror::Error)]
pub enum PresetDeserializationError {
    #[error("global: {0}")]
    Global(#[from] GlobalDeserializationError),
    #[error("pad {index}: {source}")]
    Pad {
        index: usize,
        source: PadDeserializationError,
    },
    #[error("dial {index}: {source}")]
    Dial {
        index: usize,
        source: DialDeserializationError,
    },
    #[error("fader {index}: {source}")]
    Fader {
        index: usize,
        source: FaderDeserializationError,
    },
    #[error("switch {index}: {source}")]
    Switch {
        index: usize,
        source: SwitchDeserializationError,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum GlobalDeserializationError {
    #[error("invalid preset_slot: {0}")]
    PresetSlot(TryFromPrimitiveError<PresetSlot>),
    #[error("invalid time_division_switch: {0}")]
    TimeDivisionSwitch(TryFromPrimitiveError<TriggerKind>),
    #[error("invalid time_division: {0}")]
    TimeDivision(TryFromPrimitiveError<TimeDivision>),
    #[error("invalid note_repeat_switch: {0}")]
    NoteRepeatSwitch(TryFromPrimitiveError<TriggerKind>),
    #[error("invalid gate: {0}")]
    Gate(TryFromPrimitiveError<GateValue>),
    #[error("invalid swing: {0}")]
    Swing(TryFromPrimitiveError<SwingKind>),
    #[error("invalid transport: {0}")]
    Transport(TryFromPrimitiveError<TransportKind>),
}

#[derive(Debug, thiserror::Error)]
pub enum PadDeserializationError {
    #[error("invalid kind: {0}")]
    Kind(TryFromPrimitiveError<PadKind>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<MidiChannel>),
    #[error("invalid note: {0}")]
    Note(TryFromPrimitiveError<Note>),
    #[error("invalid midi2din: {0}")]
    Midi2Din(TryFromPrimitiveError<ActiveState>),
    #[error("invalid trigger: {0}")]
    Trigger(TryFromPrimitiveError<TriggerKind>),
    #[error("invalid aftertouch: {0}")]
    Aftertouch(TryFromPrimitiveError<AfterTouchKind>),
    #[error("invalid off_color: {0}")]
    OffColor(TryFromPrimitiveError<PadColor>),
    #[error("invalid on_color: {0}")]
    OnColor(TryFromPrimitiveError<PadColor>),
}

#[derive(Debug, thiserror::Error)]
pub enum DialDeserializationError {
    #[error("invalid kind: {0}")]
    Kind(TryFromPrimitiveError<DialKind>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<MidiChannel>),
    #[error("invalid midi2din: {0}")]
    Midi2Din(TryFromPrimitiveError<ActiveState>),
}

#[derive(Debug, thiserror::Error)]
pub enum FaderDeserializationError {
    #[error("invalid kind: {0}")]
    Kind(TryFromPrimitiveError<FaderKind>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<MidiChannel>),
    #[error("invalid midi2din: {0}")]
    Midi2Din(TryFromPrimitiveError<ActiveState>),
}

#[derive(Debug, thiserror::Error)]
pub enum SwitchDeserializationError {
    #[error("invalid kind: {0}")]
    Kind(TryFromPrimitiveError<SwitchKind>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<MidiChannel>),
    #[error("invalid mode: {0}")]
    Mode(TryFromPrimitiveError<TriggerKind>),
    #[error("invalid midi2din: {0}")]
    Midi2Din(TryFromPrimitiveError<ActiveState>),
    #[error("invalid invert: {0}")]
    Invert(TryFromPrimitiveError<ActiveState>),
    #[error("invalid key2: {0}")]
    Key2(TryFromPrimitiveError<SwitchKey2>),
}
