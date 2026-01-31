use bytemuck::Pod;
use bytemuck::PodCastError;
use bytemuck::Zeroable;

use crate::manufacturer::akai::mpd226::TOTAL_PADS;
use crate::sysex::Sysex;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawPreset {
    pub global: RawGlobal,
    pub pads: [RawPad; TOTAL_PADS],
    pub dials: [RawDial; 12],
    pub faders: [RawFader; 12],
    pub switches: [RawSwitch; 12],
    pub footer_magic: [u8; 12],
}

impl TryFrom<Sysex> for RawPreset {
    type Error = PodCastError;

    fn try_from(sysex: Sysex) -> Result<Self, PodCastError> {
        let payload = sysex.payload();
        let mid = std::mem::size_of::<RawHeader>();
        let (_header, preset_bytes) = payload.split_at(mid);

        let preset = bytemuck::try_from_bytes::<RawPreset>(preset_bytes)?;
        Ok(*preset)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawHeader {
    pub mfg_id: u8,
    pub _unknown: u8,
    pub device_id: u8,
    pub cmd: u8,
    pub length: [u8; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawGlobal {
    pub preset: u8,
    pub name: [u8; 8],
    pub un1: u8,
    pub tempo: [u8; 2],
    pub time_division_switch: u8,
    pub division: u8,
    pub note_repeat_switch: u8,
    pub gate: u8,
    pub swing: u8,
    pub un5: u8,
    pub un6: u8,
    pub un7: u8,
    pub un8: u8,
    pub un9: u8,
    pub transport: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawDial {
    pub kind: u8,
    pub channel: u8,
    pub midicc: u8,
    pub min: u8,
    pub max: u8,
    pub midi2din: u8,
    pub msb: u8,
    pub lsb: u8,
    pub value: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawFader {
    pub kind: u8,
    pub channel: u8,
    pub midicc: u8,
    pub min: u8,
    pub max: u8,
    pub midi2din: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawSwitch {
    pub kind: u8,
    pub channel: u8,
    pub midicc: u8,
    pub mode: u8,
    pub prog: u8,
    pub msb: u8,
    pub lsb: u8,
    pub midi2din: u8,
    pub note: u8,
    pub velo: u8,
    pub invert: u8,
    pub key1: u8,
    pub key2: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawPad {
    pub kind: u8,
    pub channel: u8,
    pub note: u8,
    pub midi2din: u8,
    pub trigger: u8,
    pub aftertouch: u8,
    pub program: u8,
    pub msb: u8,
    pub lsb: u8,
    pub off_color: u8,
    pub on_color: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawPads(pub [RawPad; TOTAL_PADS]);

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawDials(pub [RawDial; 12]);

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawFaders(pub [RawFader; 12]);

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawSwitches(pub [RawSwitch; 12]);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manufacturer::akai::mpd226::DeviceCommand;
    use crate::manufacturer::akai::mpd226::preset_dump_request;

    #[test]
    fn test_raw_header_size() {
        assert_eq!(std::mem::size_of::<RawHeader>(), 6);
    }

    #[test]
    fn test_raw_global_data_size() {
        assert_eq!(std::mem::size_of::<RawGlobal>(), 23);
    }

    #[test]
    fn test_raw_pad_size() {
        assert_eq!(std::mem::size_of::<RawPad>(), 11);
    }

    #[test]
    fn test_raw_dial_size() {
        assert_eq!(std::mem::size_of::<RawDial>(), 9);
    }

    #[test]
    fn test_raw_fader_size() {
        assert_eq!(std::mem::size_of::<RawFader>(), 6);
    }

    #[test]
    fn test_raw_switch_size() {
        assert_eq!(std::mem::size_of::<RawSwitch>(), 13);
    }

    #[test]
    fn test_raw_preset_size() {
        assert_eq!(std::mem::size_of::<RawPreset>(), 1075);
    }

    #[test]
    fn test_raw_pads_array_size() {
        assert_eq!(std::mem::size_of::<RawPads>(), 11 * TOTAL_PADS);
    }

    #[test]
    fn test_raw_dials_array_size() {
        assert_eq!(std::mem::size_of::<RawDials>(), 9 * 12);
    }

    #[test]
    fn test_raw_faders_array_size() {
        assert_eq!(std::mem::size_of::<RawFaders>(), 6 * 12);
    }

    #[test]
    fn test_raw_switches_array_size() {
        assert_eq!(std::mem::size_of::<RawSwitches>(), 13 * 12);
    }

    #[test]
    fn test_raw_header_fields() {
        let header = RawHeader {
            mfg_id: 0x47,
            _unknown: 0x00,
            device_id: 0x35,
            cmd: 0x10,
            length: [0x08, 0x33],
        };

        assert_eq!(header.mfg_id, 0x47);
        assert_eq!(header._unknown, 0x00);
        assert_eq!(header.device_id, 0x35);
        assert_eq!(header.cmd, 0x10);
    }

    #[test]
    fn test_raw_global_data_fields() {
        let global = RawGlobal {
            preset: 5,
            name: *b"MyPreset",
            un1: 0,
            tempo: [1, 64],
            time_division_switch: 1,
            division: 2,
            note_repeat_switch: 0,
            gate: 75,
            swing: 56,
            un5: 0,
            un6: 0,
            un7: 0,
            un8: 0,
            un9: 0,
            transport: 2,
        };

        assert_eq!(global.preset, 5);
        assert_eq!(&global.name, b"MyPreset");
        assert_eq!(global.gate, 75);
        assert_eq!(global.swing, 56);
    }

    #[test]
    fn test_preset_dump_request_ram() {
        let request = preset_dump_request(0x00);

        assert_eq!(request.len(), 9);
        assert_eq!(request[0], 0xF0);
        assert_eq!(request[1], 0x47);
        assert_eq!(request[2], 0x00);
        assert_eq!(request[3], 0x35);
        assert_eq!(request[4], DeviceCommand::DumpPreset as u8);
        assert_eq!(request[5], 0x00);
        assert_eq!(request[6], 0x01);
        assert_eq!(request[7], 0x00);
        assert_eq!(request[8], 0xF7);
    }

    #[test]
    fn test_preset_dump_request_slot_0() {
        let request = preset_dump_request(0x00);

        assert_eq!(request.len(), 9);
        assert_eq!(request[7], 0x00);
    }
}
