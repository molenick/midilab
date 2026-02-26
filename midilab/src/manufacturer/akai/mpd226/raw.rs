use bytemuck::Pod;
use bytemuck::Zeroable;

use crate::manufacturer::akai::SYSEX_MANUFACTURER_ID;
use crate::manufacturer::akai::mpd226::DEVICE_ID;
use crate::manufacturer::akai::mpd226::DeviceCommandId;
use crate::manufacturer::akai::mpd226::DeviceHeader;
use crate::manufacturer::akai::mpd226::GLOBAL_VALUE_MAGIC;
use crate::manufacturer::akai::mpd226::TOTAL_PADS;
use crate::sysex::Sysex;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawGlobalParamAck {
    _unknown1: u8,
    _unknown2: u8,
    pub addr: u8,
    pub status: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawPresetAck {
    pub slot: u8,
    pub _unknown1: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawPreset {
    pub settings: RawPresetSettings,
    pub pads: [RawPad; TOTAL_PADS],
    pub dials: [RawDial; 12],
    pub faders: [RawFader; 12],
    pub switches: [RawSwitch; 12],
    pub footer_magic: [u8; 12],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct RawHeader {
    pub mfg_id: u8,
    pub _unknown: u8,
    pub device_id: u8,
    pub cmd: u8,
    pub length: [u8; 2],
}

impl RawHeader {
    pub fn write_preset() -> Self {
        Self {
            mfg_id: SYSEX_MANUFACTURER_ID,
            _unknown: 0,
            device_id: DEVICE_ID,
            cmd: DeviceCommandId::WritePreset.into(),
            length: 0x3308_u16.to_le_bytes(),
        }
    }

    pub fn dump_preset() -> Self {
        RawHeader {
            mfg_id: SYSEX_MANUFACTURER_ID,
            _unknown: 0,
            device_id: DEVICE_ID,
            cmd: DeviceCommandId::DumpPreset.into(),
            length: [0x00, 0x01],
        }
    }
}

impl<C: Into<u8> + Copy> From<&DeviceHeader<C>> for RawHeader {
    fn from(value: &DeviceHeader<C>) -> Self {
        RawHeader {
            mfg_id: SYSEX_MANUFACTURER_ID,
            _unknown: 0,
            device_id: DEVICE_ID,
            cmd: value.cmd.into(),
            length: value.message_length.to_le_bytes(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawPresetSettings {
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

/// Wire representation of MPD226 global parameters (11 bytes).
/// Field order matches device dump byte order (after the 3-byte prefix is
/// stripped). All bytes are in sequential addr order (0x01..=0x0B).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Default, Debug)]
pub struct RawGlobal {
    pub common_channel: u8,   // byte 0, addr 0x01
    pub lcd_contrast: u8,     // byte 1, addr 0x02
    pub tap_average: u8,      // byte 2, addr 0x03
    pub tempo_led: u8,        // byte 3, addr 0x04
    pub midi_clock: u8,       // byte 4, addr 0x05
    pub transport_to_din: u8, // byte 5, addr 0x06
    pub pad_threshold: u8,    // byte 6, addr 0x07
    pub _unknown_08: u8,      // byte 7, addr 0x08
    pub pad_curve: u8,        // byte 8, addr 0x09
    pub pad_gain: u8,         // byte 9, addr 0x0A
    pub note_display: u8,     // byte 10, addr 0x0B
}

impl RawGlobal {
    fn param_pairs(&self) -> [(u8, u8); 11] {
        [
            (0x01, self.common_channel),
            (0x02, self.lcd_contrast),
            (0x03, self.tap_average),
            (0x04, self.tempo_led),
            (0x05, self.midi_clock),
            (0x06, self.transport_to_din),
            (0x07, self.pad_threshold),
            (0x08, self._unknown_08),
            (0x09, self.pad_curve),
            (0x0A, self.pad_gain),
            (0x0B, self.note_display),
        ]
    }

    /// Send all global parameters to device (as individual writes)
    /// Returns Vec of sysex messages, one per parameter
    pub fn global_send_messages(&self) -> Vec<Vec<u8>> {
        self.param_pairs()
            .into_iter()
            .map(|(addr, value)| Self::global_write_param(addr, value))
            .collect()
    }

    fn global_write_param(addr: u8, value: u8) -> Vec<u8> {
        let length = u16::from_le_bytes([0x00, 0x04]).to_le_bytes();
        let header = RawHeader {
            mfg_id: SYSEX_MANUFACTURER_ID,
            _unknown: 0,
            device_id: DEVICE_ID,
            cmd: DeviceCommandId::WriteGlobal as u8,
            length,
        };
        Sysex::from_header_and_body_as_bytes(
            &header,
            [GLOBAL_VALUE_MAGIC[0], GLOBAL_VALUE_MAGIC[1], addr, value],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manufacturer::akai::SYSEX_MANUFACTURER_ID;
    use crate::manufacturer::akai::mpd226::DEVICE_ID;
    use crate::manufacturer::akai::mpd226::DeviceCommandId;
    use crate::manufacturer::akai::mpd226::dump_preset_from_device;

    #[test]
    fn test_raw_header_size() {
        assert_eq!(std::mem::size_of::<RawHeader>(), 6);
    }

    #[test]
    fn test_raw_preset_settings_data_size() {
        assert_eq!(std::mem::size_of::<RawPresetSettings>(), 23);
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
    fn test_raw_preset_settings_data_fields() {
        let preset_settings = RawPresetSettings {
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

        assert_eq!(preset_settings.preset, 5);
        assert_eq!(&preset_settings.name, b"MyPreset");
        assert_eq!(preset_settings.gate, 75);
        assert_eq!(preset_settings.swing, 56);
    }

    #[test]
    fn test_preset_dump_request_ram() {
        let request = dump_preset_from_device(0x00);

        assert_eq!(request.len(), 9);
        assert_eq!(request[0], 0xF0);
        assert_eq!(request[1], 0x47);
        assert_eq!(request[2], 0x00);
        assert_eq!(request[3], 0x35);
        assert_eq!(request[4], DeviceCommandId::DumpPreset as u8);
        assert_eq!(request[5], 0x00);
        assert_eq!(request[6], 0x01);
        assert_eq!(request[7], 0x00);
        assert_eq!(request[8], 0xF7);
    }

    #[test]
    fn test_preset_dump_request_slot_0() {
        let request = dump_preset_from_device(0x00);

        assert_eq!(request.len(), 9);
        assert_eq!(request[7], 0x00);
    }

    #[test]
    fn test_raw_global_size() {
        assert_eq!(std::mem::size_of::<super::RawGlobal>(), 11);
    }

    #[test]
    fn test_raw_preset_ack_size() {
        assert_eq!(std::mem::size_of::<RawPresetAck>(), 2);
    }

    #[test]
    fn test_raw_preset_ack_slot_is_byte_0() {
        let bytes = [0x00u8, 0x01u8];
        let ack: &RawPresetAck = bytemuck::from_bytes(&bytes);
        assert_eq!(ack.slot, 0);
        assert_eq!(ack._unknown1, 1);
    }

    #[test]
    fn test_raw_header_write_preset() {
        let header = RawHeader::write_preset();
        assert_eq!(header.mfg_id, SYSEX_MANUFACTURER_ID);
        assert_eq!(header._unknown, 0);
        assert_eq!(header.device_id, DEVICE_ID);
        assert_eq!(header.cmd, u8::from(DeviceCommandId::WritePreset));
        // 1075 (RawPreset size) encoded as MIDI 14-bit: high 7 bits = 0x08, low 7 bits = 0x33
        assert_eq!(header.length, 0x3308_u16.to_le_bytes());
    }

    #[test]
    fn test_raw_header_dump_preset() {
        let header = RawHeader::dump_preset();
        assert_eq!(header.mfg_id, SYSEX_MANUFACTURER_ID);
        assert_eq!(header._unknown, 0);
        assert_eq!(header.device_id, DEVICE_ID);
        assert_eq!(header.cmd, u8::from(DeviceCommandId::DumpPreset));
        assert_eq!(header.length, 0x0100_u16.to_le_bytes());
    }
}
