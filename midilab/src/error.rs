use crate::manufacturer::akai::mpd226::error::GlobalAckParseError;
use crate::manufacturer::akai::mpd226::error::GlobalParseError;
use crate::manufacturer::akai::mpd226::error::PresetAckParseError;
use crate::manufacturer::akai::mpd226::error::PresetParseError;
use crate::sysex::END_BYTE;
use crate::sysex::START_BYTE;

/// Enumerates error states of Midi communication
#[derive(Debug, thiserror::Error)]
pub enum MidiError {
    #[error("send preset failed")]
    WritePreset,
    #[error("request preset failed")]
    DumpPreset,
    #[error("midi output connection failed: {0}")]
    OutputConnection(String),
    #[error("midi input connection failed: {0}")]
    InputConnection(String),
    #[error("timeout waiting for response")]
    ResponseTimeout,
    #[error("channel closed")]
    ChannelClosed,
}

/// Enumerates error states of Sysex deserialization
#[derive(thiserror::Error, Debug)]
pub enum SysexParseError {
    #[error("starting byte was {0:02X} but it should be #{START_BYTE:02X}")]
    InvalidStart(u8),
    #[error("ending byte was {0:02X} but it should be #{END_BYTE:02X}")]
    InvalidEnding(u8),
    #[error("missing ending")]
    MissingEnding,
    #[error("was empty but it should have start #{START_BYTE:02X} and end #{END_BYTE:02X}")]
    Empty,
}

/// Enumerates error states of DeviceStatus deserialization
#[derive(Debug, thiserror::Error)]
pub enum DeviceStatusParseError {
    #[error("invalid sysex: {0}")]
    InvalidSysex(#[from] SysexParseError),
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
