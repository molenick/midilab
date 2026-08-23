use bytemuck::Pod;
use bytemuck::Zeroable;

use crate::manufacturer::nektar::impact_lx_plus::TOTAL_FADER_BUTTONS;
use crate::manufacturer::nektar::impact_lx_plus::TOTAL_FADERS;
use crate::manufacturer::nektar::impact_lx_plus::TOTAL_PAD_MAPS;
use crate::manufacturer::nektar::impact_lx_plus::TOTAL_PADS;
use crate::manufacturer::nektar::impact_lx_plus::TOTAL_POTS;
use crate::manufacturer::nektar::impact_lx_plus::TOTAL_PRESETS;
use crate::manufacturer::nektar::impact_lx_plus::TOTAL_SETTINGS;
use crate::manufacturer::nektar::impact_lx_plus::TOTAL_TRANSPORT_BUTTONS;

/// TLV parameter values `0x01`–`0x05` of a 30-byte control message,
/// in wire order.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawControl {
    pub channel: u8,
    pub kind: u8,
    pub data1: u8,
    pub min: u8,
    pub max: u8,
}

impl RawControl {
    pub fn as_bytes(&self) -> [u8; 5] {
        [self.channel, self.kind, self.data1, self.min, self.max]
    }
}

/// TLV parameter values `0x01`–`0x06` of a 34-byte pad message, in wire order.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawPad {
    pub channel: u8,
    pub kind: u8,
    pub data1: u8,
    pub min: u8,
    pub max: u8,
    pub note: u8,
}

impl RawPad {
    pub fn as_bytes(&self) -> [u8; 6] {
        [
            self.channel,
            self.kind,
            self.data1,
            self.min,
            self.max,
            self.note,
        ]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawPreset {
    pub faders: [RawControl; TOTAL_FADERS],
    pub pots: [RawControl; TOTAL_POTS],
    pub fader_buttons: [RawControl; TOTAL_FADER_BUTTONS],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawPadMap {
    pub pads: [RawPad; TOTAL_PADS],
}

/// Global setting values in dump order
/// (`01, 04, 05, 06, 07, 08, 09, 10, 11, 12, 0F`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawGlobalSettings {
    pub values: [u8; TOTAL_SETTINGS],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawGlobalControls {
    pub transport: [RawControl; TOTAL_TRANSPORT_BUTTONS],
    pub pitch_wheel: RawControl,
    pub mod_wheel: RawControl,
    pub foot_switch: RawControl,
}

/// The full device memory as captured by a 182-message dump.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawDump {
    pub presets: [RawPreset; TOTAL_PRESETS],
    pub pad_maps: [RawPadMap; TOTAL_PAD_MAPS],
    pub settings: RawGlobalSettings,
    pub controls: RawGlobalControls,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_control_size() {
        assert_eq!(std::mem::size_of::<RawControl>(), 5);
    }

    #[test]
    fn test_raw_pad_size() {
        assert_eq!(std::mem::size_of::<RawPad>(), 6);
    }

    #[test]
    fn test_raw_preset_size() {
        assert_eq!(std::mem::size_of::<RawPreset>(), 26 * 5);
    }

    #[test]
    fn test_raw_dump_size() {
        assert_eq!(
            std::mem::size_of::<RawDump>(),
            5 * 26 * 5 + 4 * 8 * 6 + 11 + 9 * 5
        );
    }
}
