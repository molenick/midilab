use crate::manufacturer::akai::mpd226::error::PresetDeserializationError;
use crate::sysex::END_BYTE;
use crate::sysex::START_BYTE;

/// Enumerates error states of Midi communication
#[derive(Debug, thiserror::Error)]
pub enum MidiError {
    #[error("send preset failed")]
    SendPreset,
    #[error("request preset failed")]
    RequestPreset,
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
pub enum SysexDeserializationError {
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
pub enum DeviceStatusDeserializationError {
    #[error("invalid msg")]
    InvalidMsg,
    #[error("invalid header")]
    InvalidHeader,
    #[error("invalid command: {0}")]
    InvalidCommand(u8),
    #[error("preset deserialization failed: {0}")]
    PresetDeserialization(#[from] PresetDeserializationError),
}
