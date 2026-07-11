pub mod live;
pub mod raw;
pub mod wrappers;

use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use raw::RawFormantStep;
use raw::RawGlobal;
use raw::RawParameterChange;
use raw::RawProgram;

use crate::manufacturer::korg::SYSEX_MANUFACTURER_ID;
use crate::sysex::Sysex;
use crate::sysex::pack_u7;
use crate::sysex::pack_u14;
use crate::sysex::unpack_u7;
use crate::sysex::unpack_u14;

pub const DEVICE_ID: u8 = 0x7D;
pub const PORT_MIDI_IN: &str = "R3 MIDI IN";
pub const PORT_MIDI_OUT: &str = "R3 MIDI OUT";
pub const PORT_SOUND: &str = "R3 SOUND";
pub const PORT_KBD_KNOB: &str = "R3 KBD/KNOB";

const RAW_PROGRAM_SIZE: usize = std::mem::size_of::<RawProgram>();
const RAW_GLOBAL_SIZE: usize = std::mem::size_of::<RawGlobal>();

#[repr(u8)]
#[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Clone, Copy)]
pub enum DeviceCommandId {
    CurrentProgramDumpRequest = 0x10,
    ProgramWriteRequest = 0x11,
    CurrentFormantMotionDumpRequest = 0x13,
    FormantMotionWriteRequest = 0x03,
    GlobalDumpRequest = 0x0E,
    FormantMotionDumpRequest = 0x18,
    ProgramDumpRequest = 0x1C,
}

#[repr(u8)]
#[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Clone, Copy)]
pub enum DeviceStatusId {
    WriteCompleted = 0x21,
    WriteError = 0x22,
    DataLoadCompleted = 0x23,
    DataLoadError = 0x24,
    DataFormatError = 0x26,
    CurrentProgramDump = 0x40,
    ParameterChange = 0x41,
    CurrentFormantMotionDump = 0x43,
    FormantMotionDump = 0x48,
    ProgramDump = 0x4C,
    GlobalDump = 0x51,
}

#[derive(Debug)]
pub enum KorgR3Message {
    CurrentProgramDump(Box<RawProgram>),
    ProgramDump {
        program_no: u16,
        program: Box<RawProgram>,
    },
    CurrentFormantMotionDump {
        size: u16,
        steps: Vec<RawFormantStep>,
    },
    FormantMotionDump {
        motion_no: u8,
        size: u16,
        steps: Vec<RawFormantStep>,
    },
    GlobalDump(Box<RawGlobal>),
    ParameterChange(RawParameterChange),
    DataLoadCompleted,
    DataLoadError,
    WriteCompleted,
    WriteError,
    DataFormatError,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid sysex wrapper: {0}")]
    Sysex(#[from] crate::error::SysexParseError),
    #[error("message too short: {0} bytes")]
    TooShort(usize),
    #[error("invalid manufacturer id: 0x{0:02X}")]
    InvalidManufacturer(u8),
    #[error("invalid device id: 0x{0:02X}")]
    InvalidDevice(u8),
    #[error("unknown function id: 0x{0:02X}")]
    UnknownFunction(u8),
    #[error("invalid payload size for {context}: expected {expected}, got {actual}")]
    InvalidPayloadSize {
        context: &'static str,
        expected: usize,
        actual: usize,
    },
}

const CHANNEL_PREFIX: u8 = 0x30;

fn sysex_header(channel: u8, func_id: u8) -> Vec<u8> {
    vec![
        SYSEX_MANUFACTURER_ID,
        CHANNEL_PREFIX | (channel & 0x0F),
        DEVICE_ID,
        func_id,
    ]
}

pub fn current_program_dump_request(channel: u8) -> Vec<u8> {
    let payload = sysex_header(channel, DeviceCommandId::CurrentProgramDumpRequest.into());
    Sysex::new(payload).as_bytes()
}

pub fn program_dump_request(channel: u8, program_no: u16) -> Vec<u8> {
    let mut payload = sysex_header(channel, DeviceCommandId::ProgramDumpRequest.into());
    let [msb, lsb] = pack_u14(program_no);
    payload.push(lsb);
    payload.push(msb);
    Sysex::new(payload).as_bytes()
}

pub fn current_formant_motion_dump_request(channel: u8) -> Vec<u8> {
    let payload = sysex_header(
        channel,
        DeviceCommandId::CurrentFormantMotionDumpRequest.into(),
    );
    Sysex::new(payload).as_bytes()
}

pub fn formant_motion_dump_request(channel: u8, motion_no: u8) -> Vec<u8> {
    let mut payload = sysex_header(channel, DeviceCommandId::FormantMotionDumpRequest.into());
    let [msb, lsb] = pack_u14(motion_no.into());
    payload.push(lsb);
    payload.push(msb);
    Sysex::new(payload).as_bytes()
}

pub fn global_dump_request(channel: u8) -> Vec<u8> {
    let payload = sysex_header(channel, DeviceCommandId::GlobalDumpRequest.into());
    Sysex::new(payload).as_bytes()
}

pub fn program_write_request(channel: u8, dest_program_no: u16) -> Vec<u8> {
    let mut payload = sysex_header(channel, DeviceCommandId::ProgramWriteRequest.into());
    let [msb, lsb] = pack_u14(dest_program_no);
    payload.push(lsb);
    payload.push(msb);
    Sysex::new(payload).as_bytes()
}

pub fn formant_motion_write_request(channel: u8, dest_motion_no: u8) -> Vec<u8> {
    let mut payload = sysex_header(channel, DeviceCommandId::FormantMotionWriteRequest.into());
    let [msb, lsb] = pack_u14(dest_motion_no.into());
    payload.push(lsb);
    payload.push(msb);
    Sysex::new(payload).as_bytes()
}

pub fn current_program_dump_message(channel: u8, program: &RawProgram) -> Vec<u8> {
    let mut payload = sysex_header(channel, DeviceStatusId::CurrentProgramDump.into());
    let packed = pack_u7(bytemuck::bytes_of(program));
    payload.extend_from_slice(&packed);
    Sysex::new(payload).as_bytes()
}

pub fn program_dump_message(channel: u8, program_no: u16, program: &RawProgram) -> Vec<u8> {
    let mut payload = sysex_header(channel, DeviceStatusId::ProgramDump.into());
    let [msb, lsb] = pack_u14(program_no);
    payload.push(lsb);
    payload.push(msb);
    let packed = pack_u7(bytemuck::bytes_of(program));
    payload.extend_from_slice(&packed);
    Sysex::new(payload).as_bytes()
}

pub fn global_dump_message(channel: u8, global: &RawGlobal) -> Vec<u8> {
    let mut payload = sysex_header(channel, DeviceStatusId::GlobalDump.into());
    let packed = pack_u7(bytemuck::bytes_of(global));
    payload.extend_from_slice(&packed);
    Sysex::new(payload).as_bytes()
}

pub fn current_formant_motion_dump_message(channel: u8, steps: &[RawFormantStep]) -> Vec<u8> {
    let mut payload = sysex_header(channel, DeviceStatusId::CurrentFormantMotionDump.into());
    let size = steps.len() as u16;
    let [size_hi, size_lo] = pack_u14(size);
    payload.push(0x00);
    payload.push(size_lo);
    payload.push(size_hi);
    payload.push(0x00);
    let packed = pack_u7(bytemuck::cast_slice(steps));
    payload.extend_from_slice(&packed);
    Sysex::new(payload).as_bytes()
}

pub fn formant_motion_dump_message(
    channel: u8,
    motion_no: u8,
    steps: &[RawFormantStep],
) -> Vec<u8> {
    let mut payload = sysex_header(channel, DeviceStatusId::FormantMotionDump.into());
    let size = steps.len() as u16;
    let [size_hi, size_lo] = pack_u14(size);
    payload.push(motion_no);
    payload.push(size_lo);
    payload.push(size_hi);
    payload.push(0x00);
    let packed = pack_u7(bytemuck::cast_slice(steps));
    payload.extend_from_slice(&packed);
    Sysex::new(payload).as_bytes()
}

pub fn parameter_change_message(channel: u8, param_id: u16, sub_id: u16, value: u16) -> Vec<u8> {
    let mut payload = sysex_header(channel, DeviceStatusId::ParameterChange.into());
    let [pp_msb, pp_lsb] = pack_u14(param_id);
    let [qq_msb, qq_lsb] = pack_u14(sub_id);
    let [vv_msb, vv_lsb] = pack_u14(value);
    payload.extend_from_slice(&[pp_lsb, pp_msb, qq_lsb, qq_msb, vv_lsb, vv_msb]);
    Sysex::new(payload).as_bytes()
}

impl TryFrom<&[u8]> for KorgR3Message {
    type Error = ParseError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        parse_sysex_impl(data)
    }
}

fn parse_sysex_impl(data: &[u8]) -> Result<KorgR3Message, ParseError> {
    let sysex = Sysex::try_from(data)?;
    let payload = sysex.payload();

    if payload.len() < 4 {
        return Err(ParseError::TooShort(payload.len()));
    }

    if payload[0] != SYSEX_MANUFACTURER_ID {
        return Err(ParseError::InvalidManufacturer(payload[0]));
    }

    if payload[2] != DEVICE_ID {
        return Err(ParseError::InvalidDevice(payload[2]));
    }

    let func_id = DeviceStatusId::try_from(payload[3])
        .map_err(|_| ParseError::UnknownFunction(payload[3]))?;

    let body = &payload[4..];

    match func_id {
        DeviceStatusId::CurrentProgramDump => {
            let unpacked = unpack_u7(body);
            if unpacked.len() < RAW_PROGRAM_SIZE {
                return Err(ParseError::InvalidPayloadSize {
                    context: "current program dump",
                    expected: RAW_PROGRAM_SIZE,
                    actual: unpacked.len(),
                });
            }
            let program: RawProgram = *bytemuck::from_bytes(&unpacked[..RAW_PROGRAM_SIZE]);
            Ok(KorgR3Message::CurrentProgramDump(Box::new(program)))
        }
        DeviceStatusId::ProgramDump => {
            if body.len() < 2 {
                return Err(ParseError::TooShort(body.len()));
            }
            let program_no = unpack_u14([body[1], body[0]]);
            let unpacked = unpack_u7(&body[2..]);
            if unpacked.len() < RAW_PROGRAM_SIZE {
                return Err(ParseError::InvalidPayloadSize {
                    context: "program dump",
                    expected: RAW_PROGRAM_SIZE,
                    actual: unpacked.len(),
                });
            }
            let program: RawProgram = *bytemuck::from_bytes(&unpacked[..RAW_PROGRAM_SIZE]);
            Ok(KorgR3Message::ProgramDump {
                program_no,
                program: Box::new(program),
            })
        }
        DeviceStatusId::CurrentFormantMotionDump => {
            let (size, steps) = parse_formant_payload(body.get(1..).unwrap_or(&[]));
            Ok(KorgR3Message::CurrentFormantMotionDump { size, steps })
        }
        DeviceStatusId::FormantMotionDump => {
            if body.is_empty() {
                return Err(ParseError::TooShort(0));
            }
            let motion_no = body[0];
            let (size, steps) = parse_formant_payload(body.get(1..).unwrap_or(&[]));
            Ok(KorgR3Message::FormantMotionDump {
                motion_no,
                size,
                steps,
            })
        }
        DeviceStatusId::GlobalDump => {
            let unpacked = unpack_u7(body);
            if unpacked.len() < RAW_GLOBAL_SIZE {
                return Err(ParseError::InvalidPayloadSize {
                    context: "global dump",
                    expected: RAW_GLOBAL_SIZE,
                    actual: unpacked.len(),
                });
            }
            let global: RawGlobal = *bytemuck::from_bytes(&unpacked[..RAW_GLOBAL_SIZE]);
            Ok(KorgR3Message::GlobalDump(Box::new(global)))
        }
        DeviceStatusId::ParameterChange => {
            if body.len() < std::mem::size_of::<RawParameterChange>() {
                return Err(ParseError::InvalidPayloadSize {
                    context: "parameter change",
                    expected: std::mem::size_of::<RawParameterChange>(),
                    actual: body.len(),
                });
            }
            let raw: RawParameterChange =
                *bytemuck::from_bytes(&body[..std::mem::size_of::<RawParameterChange>()]);
            Ok(KorgR3Message::ParameterChange(raw))
        }
        DeviceStatusId::DataLoadCompleted => Ok(KorgR3Message::DataLoadCompleted),
        DeviceStatusId::DataLoadError => Ok(KorgR3Message::DataLoadError),
        DeviceStatusId::WriteCompleted => Ok(KorgR3Message::WriteCompleted),
        DeviceStatusId::WriteError => Ok(KorgR3Message::WriteError),
        DeviceStatusId::DataFormatError => Ok(KorgR3Message::DataFormatError),
    }
}

pub fn parse_formant_payload(rest: &[u8]) -> (u16, Vec<RawFormantStep>) {
    if rest.len() < 3 {
        return (0, Vec::new());
    }
    let size = unpack_u14([rest[1], rest[0]]);
    let unpacked = unpack_u7(&rest[3..]);
    let step_size = std::mem::size_of::<RawFormantStep>();
    let steps: Vec<RawFormantStep> = unpacked
        .chunks_exact(step_size)
        .map(|chunk| *bytemuck::from_bytes(chunk))
        .collect();
    (size, steps)
}

#[cfg(test)]
mod tests {
    use bytemuck::Zeroable;

    use super::*;

    #[test]
    fn test_sysex_header_format() {
        let header = sysex_header(0x05, 0x10);
        assert_eq!(header, vec![0x42, 0x35, 0x7D, 0x10]);
    }

    #[test]
    fn test_sysex_header_channel_masking() {
        let header = sysex_header(0xFF, 0x10);
        assert_eq!(header[1], 0x3F);
    }

    #[test]
    fn test_current_program_dump_request_format() {
        let msg = current_program_dump_request(0x00);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], 0x42);
        assert_eq!(msg[2], 0x30);
        assert_eq!(msg[3], 0x7D);
        assert_eq!(msg[4], 0x10);
        assert_eq!(msg[5], 0xF7);
        assert_eq!(msg.len(), 6);
    }

    #[test]
    fn test_program_dump_request_format() {
        let msg = program_dump_request(0x00, 0);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], 0x42);
        assert_eq!(msg[2], 0x30);
        assert_eq!(msg[3], 0x7D);
        assert_eq!(msg[4], 0x1C);
        assert_eq!(msg[5], 0x00);
        assert_eq!(msg[6], 0x00);
        assert_eq!(msg[7], 0xF7);
    }

    #[test]
    fn test_program_dump_request_program_number() {
        let msg = program_dump_request(0x00, 128);
        assert_eq!(msg[5], 0x00);
        assert_eq!(msg[6], 0x01);
    }

    #[test]
    fn test_global_dump_request_format() {
        let msg = global_dump_request(0x00);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], 0x42);
        assert_eq!(msg[2], 0x30);
        assert_eq!(msg[3], 0x7D);
        assert_eq!(msg[4], 0x0E);
        assert_eq!(msg[5], 0xF7);
    }

    #[test]
    fn test_formant_motion_dump_request_format() {
        let msg = formant_motion_dump_request(0x00, 3);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], 0x42);
        assert_eq!(msg[2], 0x30);
        assert_eq!(msg[3], 0x7D);
        assert_eq!(msg[4], 0x18);
        assert_eq!(msg[5], 3);
        assert_eq!(msg[6], 0x00);
        assert_eq!(msg[7], 0xF7);
    }

    #[test]
    fn test_program_write_request_format() {
        let msg = program_write_request(0x00, 5);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], 0x42);
        assert_eq!(msg[2], 0x30);
        assert_eq!(msg[3], 0x7D);
        assert_eq!(msg[4], 0x11);
        assert_eq!(msg[5], 0x05);
        assert_eq!(msg[6], 0x00);
        assert_eq!(msg[7], 0xF7);
    }

    #[test]
    fn test_parameter_change_message_format() {
        let msg = parameter_change_message(0x00, 0x01, 0x02, 0x03);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], 0x42);
        assert_eq!(msg[2], 0x30);
        assert_eq!(msg[3], 0x7D);
        assert_eq!(msg[4], 0x41);
        assert_eq!(msg[5], 0x01);
        assert_eq!(msg[6], 0x00);
        assert_eq!(msg[7], 0x02);
        assert_eq!(msg[8], 0x00);
        assert_eq!(msg[9], 0x03);
        assert_eq!(msg[10], 0x00);
        assert_eq!(msg[11], 0xF7);
    }

    #[test]
    fn test_current_program_dump_message_starts_correctly() {
        let program = RawProgram::zeroed();
        let msg = current_program_dump_message(0x00, &program);

        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], 0x42);
        assert_eq!(msg[2], 0x30);
        assert_eq!(msg[3], 0x7D);
        assert_eq!(msg[4], 0x40);
        assert_eq!(*msg.last().unwrap(), 0xF7);
    }

    #[test]
    fn test_program_dump_packed_size() {
        let program = RawProgram::zeroed();
        let packed = pack_u7(bytemuck::bytes_of(&program));
        assert_eq!(packed.len(), 522);
    }

    #[test]
    fn test_global_dump_packed_size() {
        let global = RawGlobal::zeroed();
        let packed = pack_u7(bytemuck::bytes_of(&global));
        assert_eq!(packed.len(), 92);
    }

    #[test]
    fn test_program_round_trip() {
        let mut program = RawProgram::zeroed();
        program.name = *b"TestProg";
        program.voice_arp = 0x42;
        program.timbre1.program.osc1_wave_mod = 3;
        program.timbre2.program.filter1_cutoff = 100;
        program.vocoder.threshold = 50;
        program.master_fx.fx_type = 2;
        program.arpeggio.gate_time = 80;

        let packed = pack_u7(bytemuck::bytes_of(&program));
        let unpacked = unpack_u7(&packed);

        assert!(unpacked.len() >= RAW_PROGRAM_SIZE);
        let restored: RawProgram = *bytemuck::from_bytes(&unpacked[..RAW_PROGRAM_SIZE]);

        assert_eq!(restored.name, *b"TestProg");
        assert_eq!(restored.voice_arp, 0x42);
        assert_eq!(restored.timbre1.program.osc1_wave_mod, 3);
        assert_eq!(restored.timbre2.program.filter1_cutoff, 100);
        assert_eq!(restored.vocoder.threshold, 50);
        assert_eq!(restored.master_fx.fx_type, 2);
        assert_eq!(restored.arpeggio.gate_time, 80);
    }

    #[test]
    fn test_global_round_trip() {
        let mut global = RawGlobal::zeroed();
        global.master_tune = 64;
        global.transpose = 12;
        global.midi_channel = 5;
        global.vel_curve = 2;
        global.cc_map_lo[0] = 74;

        let packed = pack_u7(bytemuck::bytes_of(&global));
        let unpacked = unpack_u7(&packed);

        assert!(unpacked.len() >= RAW_GLOBAL_SIZE);
        let restored: RawGlobal = *bytemuck::from_bytes(&unpacked[..RAW_GLOBAL_SIZE]);

        assert_eq!(restored.master_tune, 64);
        assert_eq!(restored.transpose, 12);
        assert_eq!(restored.midi_channel, 5);
        assert_eq!(restored.vel_curve, 2);
        assert_eq!(restored.cc_map_lo[0], 74);
    }

    #[test]
    fn test_pack_u7_known_vector() {
        let input = [0x80, 0x00, 0x81, 0x00, 0x00, 0x00, 0x00];
        let packed = pack_u7(&input);

        assert_eq!(packed.len(), 8);
        assert_eq!(packed[0], 0x05);
        assert_eq!(packed[1], 0x00);
        assert_eq!(packed[2], 0x00);
        assert_eq!(packed[3], 0x01);
        assert_eq!(packed[4], 0x00);
        assert_eq!(packed[5], 0x00);
        assert_eq!(packed[6], 0x00);
        assert_eq!(packed[7], 0x00);

        let unpacked = unpack_u7(&packed);
        assert_eq!(unpacked, input);
    }

    #[test]
    fn test_parse_current_program_dump() {
        let mut program = RawProgram::zeroed();
        program.name = *b"ParseTst";

        let msg = current_program_dump_message(0x00, &program);
        let parsed = KorgR3Message::try_from(msg.as_slice()).unwrap();

        match parsed {
            KorgR3Message::CurrentProgramDump(p) => {
                assert_eq!(p.name, *b"ParseTst");
            }
            _ => panic!("expected CurrentProgramDump"),
        }
    }

    #[test]
    fn test_parse_program_dump() {
        let mut program = RawProgram::zeroed();
        program.name = *b"SlotTest";

        let msg = program_dump_message(0x00, 42, &program);
        let parsed = KorgR3Message::try_from(msg.as_slice()).unwrap();

        match parsed {
            KorgR3Message::ProgramDump {
                program_no,
                program: p,
            } => {
                assert_eq!(program_no, 42);
                assert_eq!(p.name, *b"SlotTest");
            }
            _ => panic!("expected ProgramDump"),
        }
    }

    #[test]
    fn test_parse_global_dump() {
        let mut global = RawGlobal::zeroed();
        global.master_tune = 64;
        global.transpose = 12;

        let msg = global_dump_message(0x00, &global);
        let parsed = KorgR3Message::try_from(msg.as_slice()).unwrap();

        match parsed {
            KorgR3Message::GlobalDump(g) => {
                assert_eq!(g.master_tune, 64);
                assert_eq!(g.transpose, 12);
            }
            _ => panic!("expected GlobalDump"),
        }
    }

    #[test]
    fn test_parse_parameter_change() {
        let msg = parameter_change_message(0x00, 0x100, 0x02, 0x7F);
        let parsed = KorgR3Message::try_from(msg.as_slice()).unwrap();

        match parsed {
            KorgR3Message::ParameterChange(raw) => {
                let param = unpack_u14([raw.param_id[1], raw.param_id[0]]);
                let sub = unpack_u14([raw.sub_id[1], raw.sub_id[0]]);
                let val = unpack_u14([raw.value[1], raw.value[0]]);
                assert_eq!(param, 0x100);
                assert_eq!(sub, 0x02);
                assert_eq!(val, 0x7F);
            }
            _ => panic!("expected ParameterChange"),
        }
    }

    #[test]
    fn test_parse_status_messages() {
        for (func_id, expected_name) in [
            (DeviceStatusId::DataLoadCompleted, "DataLoadCompleted"),
            (DeviceStatusId::DataLoadError, "DataLoadError"),
            (DeviceStatusId::WriteCompleted, "WriteCompleted"),
            (DeviceStatusId::WriteError, "WriteError"),
            (DeviceStatusId::DataFormatError, "DataFormatError"),
        ] {
            let payload = sysex_header(0x00, func_id.into());
            let msg = Sysex::new(payload).as_bytes();
            let parsed = KorgR3Message::try_from(msg.as_slice()).unwrap();

            let name = format!("{parsed:?}");
            assert!(
                name.contains(expected_name),
                "expected {expected_name} in {name}"
            );
        }
    }

    #[test]
    fn test_parse_invalid_manufacturer() {
        let data = vec![0xF0, 0x43, 0x30, 0x7D, 0x40, 0xF7];
        let err = KorgR3Message::try_from(data.as_slice()).unwrap_err();
        assert!(matches!(err, ParseError::InvalidManufacturer(0x43)));
    }

    #[test]
    fn test_parse_invalid_device() {
        let data = vec![0xF0, 0x42, 0x30, 0x7E, 0x40, 0xF7];
        let err = KorgR3Message::try_from(data.as_slice()).unwrap_err();
        assert!(matches!(err, ParseError::InvalidDevice(0x7E)));
    }

    #[test]
    fn test_parse_unknown_function() {
        let data = vec![0xF0, 0x42, 0x30, 0x7D, 0x7F, 0xF7];
        let err = KorgR3Message::try_from(data.as_slice()).unwrap_err();
        assert!(matches!(err, ParseError::UnknownFunction(0x7F)));
    }

    #[test]
    fn test_parse_formant_payload_short_no_panic() {
        let (size, steps) = parse_formant_payload(&[0x00, 0x01]);
        assert_eq!(steps.len(), 0);
        assert_eq!(size, 0);
    }

    #[test]
    fn test_parse_empty_formant_dump_no_panic() {
        let payload = sysex_header(0x00, DeviceStatusId::CurrentFormantMotionDump.into());
        let msg = Sysex::new(payload).as_bytes();
        match KorgR3Message::try_from(msg.as_slice()).unwrap() {
            KorgR3Message::CurrentFormantMotionDump { steps, .. } => assert!(steps.is_empty()),
            other => panic!("expected CurrentFormantMotionDump, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_too_short() {
        let data = vec![0xF0, 0x42, 0x30, 0xF7];
        let err = KorgR3Message::try_from(data.as_slice()).unwrap_err();
        assert!(matches!(err, ParseError::TooShort(_)));
    }

    #[test]
    fn test_program_full_round_trip_via_sysex() {
        let mut program = RawProgram::zeroed();
        program.name = *b"FullTrip";
        program.timbre1.program.filter1_cutoff = 127;
        program.timbre2.insert_fx.fx1_params[0] = 64;
        program.vocoder.bands[15] = 100;
        program.tempo = [120, 0];
        program.arp_flags = 0x80;

        let msg = current_program_dump_message(0x05, &program);

        assert_eq!(msg[1], 0x42);
        assert_eq!(msg[2], 0x35);
        let parsed = KorgR3Message::try_from(msg.as_slice()).unwrap();
        match parsed {
            KorgR3Message::CurrentProgramDump(p) => {
                assert_eq!(p.name, *b"FullTrip");
                assert_eq!(p.timbre1.program.filter1_cutoff, 127);
                assert_eq!(p.timbre2.insert_fx.fx1_params[0], 64);
                assert_eq!(p.vocoder.bands[15], 100);
                assert_eq!(p.tempo, [120, 0]);
                assert_eq!(p.arp_flags, 0x80);
            }
            _ => panic!("expected CurrentProgramDump"),
        }
    }

    #[test]
    fn test_global_full_round_trip_via_sysex() {
        let mut global = RawGlobal::zeroed();
        global.master_tune = 64;
        global.transpose = 12;
        global.flags_2 = 0x88;
        global.vel_curve = 3;
        global.midi_channel = 9;
        global.filters = 0x39;
        global.cc_map_lo[0] = 74;
        global.cc_map_hi[2] = 127;

        let msg = global_dump_message(0x0A, &global);

        assert_eq!(msg[1], 0x42);
        assert_eq!(msg[2], 0x3A);
        let parsed = KorgR3Message::try_from(msg.as_slice()).unwrap();
        match parsed {
            KorgR3Message::GlobalDump(g) => {
                assert_eq!(g.master_tune, 64);
                assert_eq!(g.transpose, 12);
                assert_eq!(g.flags_2, 0x88);
                assert_eq!(g.vel_curve, 3);
                assert_eq!(g.midi_channel, 9);
                assert_eq!(g.filters, 0x39);
                assert_eq!(g.cc_map_lo[0], 74);
                assert_eq!(g.cc_map_hi[2], 127);
            }
            _ => panic!("expected GlobalDump"),
        }
    }

    #[test]
    fn test_current_formant_motion_dump_message_roundtrip() {
        let mut steps = vec![RawFormantStep::zeroed(); 3];
        steps[0].bands[0] = 0xFF;
        steps[1].bands[5] = 0x80;
        steps[2].bands[15] = 0x7F;

        let msg = current_formant_motion_dump_message(0x00, &steps);
        let parsed = KorgR3Message::try_from(msg.as_slice()).unwrap();

        match parsed {
            KorgR3Message::CurrentFormantMotionDump { size, steps: out } => {
                assert_eq!(size, 3);
                assert_eq!(out.len(), 3);
                assert_eq!(out[0].bands[0], 0xFF);
                assert_eq!(out[1].bands[5], 0x80);
                assert_eq!(out[2].bands[15], 0x7F);
            }
            _ => panic!("expected CurrentFormantMotionDump"),
        }
    }

    #[test]
    fn test_formant_motion_dump_message_roundtrip() {
        let mut steps = vec![RawFormantStep::zeroed(); 3];
        steps[0].bands[0] = 0xFF;
        steps[1].bands[5] = 0x80;
        steps[2].bands[15] = 0x7F;

        let msg = formant_motion_dump_message(0x00, 5, &steps);
        let parsed = KorgR3Message::try_from(msg.as_slice()).unwrap();

        match parsed {
            KorgR3Message::FormantMotionDump {
                motion_no,
                size,
                steps: out,
            } => {
                assert_eq!(motion_no, 5);
                assert_eq!(size, 3);
                assert_eq!(out.len(), 3);
                assert_eq!(out[0].bands[0], 0xFF);
                assert_eq!(out[1].bands[5], 0x80);
                assert_eq!(out[2].bands[15], 0x7F);
            }
            _ => panic!("expected FormantMotionDump"),
        }
    }
}
