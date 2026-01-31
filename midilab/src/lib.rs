/// High-level crate error enums
pub mod error;
/// Device integrations organized by Manufacturer and Device
pub mod manufacturer;
/// Messages and effects for finite state machines
pub mod message;
/// Midi utilities
pub mod midi;
/// Note-mapping utilities
pub mod scale;
///  Sysex deserialization
pub mod sysex;

/// Convienence re-export so consuming crates of our library one can use derived EnumIter features for our types
pub use strum::IntoEnumIterator;
