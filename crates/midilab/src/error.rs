use crate::manufacturer::akai::mpd226::error::GlobalAckParseError;
use crate::manufacturer::akai::mpd226::error::GlobalParseError;
use crate::manufacturer::akai::mpd226::error::PresetAckParseError;
use crate::manufacturer::akai::mpd226::error::PresetParseError;

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
