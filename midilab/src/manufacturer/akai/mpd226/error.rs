use num_enum::TryFromPrimitiveError;

use crate::manufacturer::akai::mpd226::control::value_kind::ActiveState;
use crate::manufacturer::akai::mpd226::control::value_kind::AfterTouchKind;
use crate::manufacturer::akai::mpd226::control::value_kind::DialKind;
use crate::manufacturer::akai::mpd226::control::value_kind::FaderKind;
use crate::manufacturer::akai::mpd226::control::value_kind::KeyModifier;
use crate::manufacturer::akai::mpd226::control::value_kind::MidiChannel;
use crate::manufacturer::akai::mpd226::control::value_kind::MidiClock;
use crate::manufacturer::akai::mpd226::control::value_kind::NoteDisplay;
use crate::manufacturer::akai::mpd226::control::value_kind::PadColor;
use crate::manufacturer::akai::mpd226::control::value_kind::PadCurve;
use crate::manufacturer::akai::mpd226::control::value_kind::PadKind;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetSlot;
use crate::manufacturer::akai::mpd226::control::value_kind::SwingKind;
use crate::manufacturer::akai::mpd226::control::value_kind::SwitchKind;
use crate::manufacturer::akai::mpd226::control::value_kind::TapAverage;
use crate::manufacturer::akai::mpd226::control::value_kind::TimeDivision;
use crate::manufacturer::akai::mpd226::control::value_kind::TransportKind;
use crate::manufacturer::akai::mpd226::control::value_kind::TriggerKind;
use crate::manufacturer::akai::mpd226::control::value_kind::UsbChannel;
use crate::midi::Note;

#[derive(Debug, thiserror::Error)]

pub enum PresetParseError {
    #[error("preset settings: {0}")]
    PresetSettings(#[from] PresetSettingsParseError),
    #[error("pad {index}: {source}")]
    Pad { index: usize, source: PadParseError },
    #[error("dial {index}: {source}")]
    Dial {
        index: usize,
        source: DialParseError,
    },
    #[error("fader {index}: {source}")]
    Fader {
        index: usize,
        source: FaderParseError,
    },
    #[error("switch {index}: {source}")]
    Switch {
        index: usize,
        source: SwitchParseError,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum PresetSettingsParseError {
    #[error("invalid preset_slot data: {0:?}")]
    PresetSlotData(Vec<u8>),
    #[error("invalid preset_slot: {0}")]
    PresetSlot(TryFromPrimitiveError<PresetSlot>),
    #[error("invalid time_division_switch: {0}")]
    TimeDivisionSwitch(TryFromPrimitiveError<TriggerKind>),
    #[error("invalid time_division: {0}")]
    TimeDivision(TryFromPrimitiveError<TimeDivision>),
    #[error("invalid note_repeat_switch: {0}")]
    NoteRepeatSwitch(TryFromPrimitiveError<TriggerKind>),
    #[error("invalid swing: {0}")]
    Swing(TryFromPrimitiveError<SwingKind>),
    #[error("invalid transport: {0}")]
    Transport(TryFromPrimitiveError<TransportKind>),
}

#[derive(Debug, thiserror::Error)]
pub enum PadParseError {
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
pub enum DialParseError {
    #[error("invalid kind: {0}")]
    Kind(TryFromPrimitiveError<DialKind>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<MidiChannel>),
    #[error("invalid midi2din: {0}")]
    Midi2Din(TryFromPrimitiveError<ActiveState>),
}

#[derive(Debug, thiserror::Error)]
pub enum FaderParseError {
    #[error("invalid kind: {0}")]
    Kind(TryFromPrimitiveError<FaderKind>),
    #[error("invalid channel: {0}")]
    Channel(TryFromPrimitiveError<MidiChannel>),
    #[error("invalid midi2din: {0}")]
    Midi2Din(TryFromPrimitiveError<ActiveState>),
}

#[derive(Debug, thiserror::Error)]
pub enum SwitchParseError {
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
    Key2(TryFromPrimitiveError<KeyModifier>),
}

#[derive(Debug, thiserror::Error)]
pub enum GlobalParseError {
    #[error("invalid common_channel: {0}")]
    CommonChannel(TryFromPrimitiveError<UsbChannel>),
    #[error("invalid tap_average: {0}")]
    TapAverage(TryFromPrimitiveError<TapAverage>),
    #[error("invalid tempo_led: {0}")]
    TempoLed(TryFromPrimitiveError<ActiveState>),
    #[error("invalid note_display: {0}")]
    NoteDisplay(TryFromPrimitiveError<NoteDisplay>),
    #[error("invalid transport_to_din: {0}")]
    TransportToDin(TryFromPrimitiveError<ActiveState>),
    #[error("invalid pad_curve: {0}")]
    PadCurve(TryFromPrimitiveError<PadCurve>),
    #[error("invalid midi_clock: {0}")]
    MidiClock(TryFromPrimitiveError<MidiClock>),
    #[error("invalid length, expected x but got {0}")]
    InvalidLength(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum GlobalAckParseError {
    #[error("invalid addr: {0}")]
    InvalidAddr(u8),
    #[error("invalid length, was {0} but expect 4")]
    InvalidLength(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum PresetAckParseError {
    #[error("invalid slot: {0}")]
    InvalidSlot(u8),
    #[error("invalid length, was {0} but expect 4")]
    InvalidLength(usize),
}
