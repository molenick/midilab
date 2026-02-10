#![doc = include_str!("../README.md")]

/// Application configuration persistence
pub mod config;
/// High-level crate error enums
pub mod error;
/// Device integrations organized by Manufacturer and Device
pub mod manufacturer;
/// Messages and effects for finite state machines
pub mod message;
/// Midi utilities
pub mod midi;
/// Music theory
pub mod music;
///  Sysex deserialization
pub mod sysex;

/// Convienence re-export so consuming crates of our library one can use derived EnumIter features for our types
pub use strum::IntoEnumIterator;
