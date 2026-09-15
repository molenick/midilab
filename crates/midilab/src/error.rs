use crate::manufacturer::akai::mpd226::error::GlobalAckParseError;
use crate::manufacturer::akai::mpd226::error::GlobalParseError;
use crate::manufacturer::akai::mpd226::error::PresetAckParseError;
use crate::manufacturer::akai::mpd226::error::PresetParseError;

/// Enumerates error states of Midi communication.
///
/// Errors are reserved for things that are definitely wrong: a live
/// connection that rejects a send. Absence of ports or of a device
/// response is a state to report (status), not an error.
#[derive(Debug, thiserror::Error)]
pub enum MidiError {
    #[error("midi send failed: {0}")]
    Send(String),
}

/// Enumerates error states of DeviceStatus deserialization
#[derive(Debug, thiserror::Error)]
pub enum DeviceStatusParseError {
    #[error("invalid sysex: {0}")]
    InvalidSysex(#[from] midi_io::SysExError),
    #[error("invalid msg")]
    InvalidMsg,
    #[error("invalid header")]
    InvalidHeader,
    #[error("invalid command: {0}")]
    InvalidCommand(u8),
    #[error("preset deserialization failed: {0}")]
    PresetDeserialization(#[from] PresetParseError),
    #[error("global deserialization failed: {0}")]
    GlobalDeserialization(#[from] GlobalParseError),
    #[error("invalid global param ack: {0}")]
    GlobalAck(#[from] GlobalAckParseError),
    #[error("invalid preset  ack: {0}")]
    PresetAck(#[from] PresetAckParseError),
}
