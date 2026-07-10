#![doc = include_str!("../README.md")]

/// High-level crate error enums
pub mod error;
/// Device integrations organized by Manufacturer and Device
pub mod manufacturer;
/// Midi utilities
pub mod midi;
/// Music theory
pub mod music;
///  Sysex deserialization
pub mod sysex;

/// Convienence re-export so consuming crates of our library one can use derived EnumIter features for our types
pub use strum::IntoEnumIterator;
