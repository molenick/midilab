use bytemuck::Pod;
use bytemuck::Zeroable;

use crate::manufacturer::arturia::minilab_mk2::TOTAL_KNOBS;
use crate::manufacturer::arturia::minilab_mk2::TOTAL_PADS;
use crate::manufacturer::arturia::minilab_mk2::TOTAL_SHIFT_KNOBS;

pub const TOTAL_BUTTONS: usize = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawControl {
    pub mode: u8,
    pub channel: u8,
    pub data1: u8,
    pub data2: u8,
    pub data3: u8,
    pub option: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawPad {
    pub mode: u8,
    pub channel: u8,
    pub data1: u8,
    pub data2: u8,
    pub data3: u8,
    pub option: u8,
    pub color: u8,
}

impl RawControl {
    pub fn as_bytes(&self) -> [u8; 6] {
        [
            self.mode,
            self.channel,
            self.data1,
            self.data2,
            self.data3,
            self.option,
        ]
    }
}

impl RawPad {
    pub fn as_bytes(&self) -> [u8; 7] {
        [
            self.mode,
            self.channel,
            self.data1,
            self.data2,
            self.data3,
            self.option,
            self.color,
        ]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawPreset {
    pub knobs: [RawControl; TOTAL_KNOBS],
    pub shift_knobs: [RawControl; TOTAL_SHIFT_KNOBS],
    pub buttons: [RawControl; TOTAL_BUTTONS],
    pub mod_wheel: RawControl,
    pub pitch_bend: RawControl,
    pub sustain_pedal: RawControl,
    pub pads: [RawPad; TOTAL_PADS],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RawGlobal {
    pub keyboard_channel: u8,
    pub key_velocity_curve: u8,
    pub pad_velocity_curve: u8,
    pub knob_acceleration: u8,
    pub octave_button_blink: u8,
    pub pad_off_backlight: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_control_size() {
        assert_eq!(std::mem::size_of::<RawControl>(), 6);
    }

    #[test]
    fn test_raw_pad_size() {
        assert_eq!(std::mem::size_of::<RawPad>(), 7);
    }

    #[test]
    fn test_raw_preset_size() {
        assert_eq!(std::mem::size_of::<RawPreset>(), 25 * 6 + 16 * 7);
    }

    #[test]
    fn test_raw_global_size() {
        assert_eq!(std::mem::size_of::<RawGlobal>(), 6);
    }
}
