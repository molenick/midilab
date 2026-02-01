use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;

use crate::error::DeviceStatusDeserializationError;
use crate::manufacturer::akai::SYSEX_MANUFACTURER_ID;
use crate::manufacturer::akai::mpd226::control::Global;
use crate::manufacturer::akai::mpd226::control::value_kind::GateValue;
use crate::manufacturer::akai::mpd226::control::value_kind::PadColor;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetName;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetSlot;
use crate::manufacturer::akai::mpd226::control::value_kind::SwingKind;
use crate::manufacturer::akai::mpd226::control::value_kind::Tempo;
use crate::manufacturer::akai::mpd226::control::value_kind::TimeDivision;
use crate::manufacturer::akai::mpd226::control::value_kind::TransportKind;
use crate::manufacturer::akai::mpd226::control::value_kind::TriggerKind;
use crate::manufacturer::akai::mpd226::raw::RawDials;
use crate::manufacturer::akai::mpd226::raw::RawFaders;
use crate::manufacturer::akai::mpd226::raw::RawGlobal;
use crate::manufacturer::akai::mpd226::raw::RawHeader;
use crate::manufacturer::akai::mpd226::raw::RawPads;
use crate::manufacturer::akai::mpd226::raw::RawPreset;
use crate::manufacturer::akai::mpd226::raw::RawSwitches;
use crate::manufacturer::akai::mpd226::repository::DialRepository;
use crate::manufacturer::akai::mpd226::repository::FaderRepository;
use crate::manufacturer::akai::mpd226::repository::PadRepository;
use crate::manufacturer::akai::mpd226::repository::SwitchRepository;
use crate::scale::ScaleSequence;
use crate::sysex::Sysex;
use crate::sysex::unpack_u14;

/// Contains domain representations of device controls
pub mod control;
/// MPD226-specific deserialization errors
pub mod error;
/// Contains raw "wire-type" representations of device data
pub mod raw;
/// Contains repositories for managing access and mutation of domain control collections
pub mod repository;

pub const DEVICE_ID: u8 = 0x35;

const PRESET_LENGTH: u16 = 1075;
const TOTAL_PADS: usize = 64;

pub(crate) const FOOTER_MAGIC_BYTES: usize = 12;
pub(crate) const FOOTER_MAGIC: [u8; FOOTER_MAGIC_BYTES] = [4, 0, 0, 4, 0, 2, 4, 0, 4, 4, 0, 6];

#[repr(u8)]
#[derive(Debug, IntoPrimitive, TryFromPrimitive)]
pub enum DeviceCommand {
    SendPreset = 0x10,
    PresetAck = 0x11,
    DumpPreset = 0x12,
}

pub fn preset_dump_request(slot: u8) -> Vec<u8> {
    let length = u16::from_le_bytes([0x00, 0x01]).to_le_bytes();

    let header = RawHeader {
        mfg_id: SYSEX_MANUFACTURER_ID,
        _unknown: 0,
        device_id: DEVICE_ID,
        cmd: DeviceCommand::DumpPreset as u8,
        length,
    };

    let mut sysex_payload = bytemuck::bytes_of(&header).to_vec();
    sysex_payload.extend_from_slice(bytemuck::bytes_of(&slot));
    Sysex::new(sysex_payload).as_bytes()
}

pub fn preset_send_message(preset: &RawPreset) -> Vec<u8> {
    let header = RawHeader {
        mfg_id: SYSEX_MANUFACTURER_ID,
        _unknown: 0,
        device_id: DEVICE_ID,
        cmd: DeviceCommand::SendPreset as u8,
        length: 0x3308_u16.to_le_bytes(),
    };

    let mut sysex_payload = bytemuck::bytes_of(&header).to_vec();
    sysex_payload.extend_from_slice(bytemuck::bytes_of(preset));
    Sysex::new(sysex_payload).as_bytes()
}

pub struct Header {
    pub cmd: DeviceCommand,
    pub length: u16,
}
impl TryFrom<Sysex> for Header {
    type Error = DeviceStatusDeserializationError;

    fn try_from(value: Sysex) -> Result<Self, Self::Error> {
        let header_size = std::mem::size_of::<RawHeader>();
        if value.payload().len() < header_size {
            return Err(DeviceStatusDeserializationError::InvalidHeader);
        }

        let raw_header: &RawHeader = bytemuck::from_bytes(&value.payload()[..header_size]);

        if raw_header.mfg_id != SYSEX_MANUFACTURER_ID {
            return Err(DeviceStatusDeserializationError::InvalidHeader);
        }

        if raw_header.device_id != DEVICE_ID {
            return Err(DeviceStatusDeserializationError::InvalidHeader);
        }

        let cmd = DeviceCommand::try_from(raw_header.cmd)
            .map_err(|_| DeviceStatusDeserializationError::InvalidCommand(raw_header.cmd))?;
        let length = unpack_u14(raw_header.length);

        Ok(Header { cmd, length })
    }
}

pub enum DeviceStatus {
    ReceivedPresetAck(PresetSlot),
    PresetData(Box<Preset>),
}
impl TryFrom<Sysex> for DeviceStatus {
    type Error = DeviceStatusDeserializationError;

    fn try_from(value: Sysex) -> Result<Self, Self::Error> {
        let header = Header::try_from(value.clone())
            .map_err(|_e| DeviceStatusDeserializationError::InvalidHeader)?;

        match header.cmd {
            DeviceCommand::SendPreset if header.length == PRESET_LENGTH => {
                let raw_preset = RawPreset::try_from(value)
                    .map_err(|_e| DeviceStatusDeserializationError::InvalidMsg)?;

                let preset = Box::new(Preset::try_from(raw_preset)?);

                Ok(DeviceStatus::PresetData(preset))
            }
            DeviceCommand::PresetAck => {
                let header_size = std::mem::size_of::<RawHeader>();
                let payload = value.payload();
                if payload.len() < header_size + 1 {
                    return Err(DeviceStatusDeserializationError::InvalidMsg);
                }
                let slot = PresetSlot::try_from(payload[header_size])
                    .map_err(|_| DeviceStatusDeserializationError::InvalidMsg)?;
                Ok(DeviceStatus::ReceivedPresetAck(slot))
            }
            _ => Err(DeviceStatusDeserializationError::InvalidMsg),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub global: Global,
    pub pads: PadRepository,
    pub dials: DialRepository,
    pub faders: FaderRepository,
    pub switches: SwitchRepository,
}
impl Default for Preset {
    fn default() -> Self {
        // CC and Color values from default manufactuer'rs Generic/default Mpd226 Editor preset
        const GENERIC_DIAL_CC: [u8; 12] = [3, 9, 14, 15, 52, 53, 54, 55, 83, 85, 86, 87];
        const GENERIC_FADER_CC: [u8; 12] = [20, 21, 22, 23, 61, 62, 63, 70, 92, 93, 94, 95];
        const GENERIC_SWITCH_CC: [u8; 12] = [28, 29, 30, 31, 75, 76, 77, 78, 106, 107, 108, 109];
        let global = Global::default();

        let mut pads = PadRepository::default(); // todo: this should get some dressing
        let dials = DialRepository::with_cc_values(GENERIC_DIAL_CC);
        let faders = FaderRepository::with_cc_values(GENERIC_FADER_CC);
        let switches = SwitchRepository::with_cc_values(GENERIC_SWITCH_CC);

        pads.set_off_color_pattern(
            0,
            64,
            ColorPattern::Grouped(vec![
                ColorSequence {
                    len: 4,
                    color: PadColor::Red,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Green,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Blue,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::LightPurple,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Orange,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::GreenBlue,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Purple,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::LightGreen,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Amber,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Aqua,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Pink,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::LightPink,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Yellow,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::LightBlue,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::HotPink,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Grey,
                },
            ]),
        );

        pads.set_on_color_pattern(
            0,
            64,
            ColorPattern::Grouped(vec![
                ColorSequence {
                    len: 4,
                    color: PadColor::Green,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Blue,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::LightPurple,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Orange,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::GreenBlue,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Purple,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::LightGreen,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Amber,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Aqua,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Pink,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::LightPink,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Yellow,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::LightBlue,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::HotPink,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Grey,
                },
                ColorSequence {
                    len: 4,
                    color: PadColor::Yellow,
                },
            ]),
        );

        Preset {
            global,
            pads,
            dials,
            faders,
            switches,
        }
    }
}

impl TryFrom<RawPreset> for Preset {
    type Error = error::PresetDeserializationError;

    fn try_from(raw: RawPreset) -> Result<Self, Self::Error> {
        let global = Global::try_from(raw.global)?;
        let pads = PadRepository::try_from(RawPads(raw.pads))?;
        let dials = DialRepository::try_from(RawDials(raw.dials))?;
        let faders = FaderRepository::try_from(RawFaders(raw.faders))?;
        let switches = SwitchRepository::try_from(RawSwitches(raw.switches))?;

        Ok(Preset {
            global,
            pads,
            dials,
            faders,
            switches,
        })
    }
}

impl From<&Preset> for RawPreset {
    fn from(preset: &Preset) -> Self {
        let global = RawGlobal::from(&preset.global);

        let mut pads = [[0u8; 11]; TOTAL_PADS];
        for (i, pad) in preset.pads.pads.iter().enumerate() {
            let bytes = pad.sysex_payload();
            pads[i].copy_from_slice(&bytes);
        }

        let mut dials = [[0u8; 9]; 12];
        for (i, dial) in preset.dials.0.iter().enumerate() {
            let bytes = dial.as_bytes();
            dials[i].copy_from_slice(&bytes);
        }

        let mut faders = [[0u8; 6]; 12];
        for (i, fader) in preset.faders.0.iter().enumerate() {
            let bytes = fader.as_bytes();
            faders[i].copy_from_slice(&bytes);
        }

        let mut switches = [[0u8; 13]; 12];
        for (i, switch) in preset.switches.0.iter().enumerate() {
            let bytes = switch.as_bytes();
            switches[i].copy_from_slice(&bytes);
        }

        RawPreset {
            global,
            pads: bytemuck::cast(pads),
            dials: bytemuck::cast(dials),
            faders: bytemuck::cast(faders),
            switches: bytemuck::cast(switches),
            footer_magic: FOOTER_MAGIC,
        }
    }
}

impl TryFrom<RawGlobal> for Global {
    type Error = error::GlobalDeserializationError;

    fn try_from(raw: RawGlobal) -> Result<Self, Self::Error> {
        use error::GlobalDeserializationError;
        Ok(Global {
            preset_slot: PresetSlot::try_from(raw.preset)
                .map_err(GlobalDeserializationError::PresetSlot)?,
            preset_name: PresetName(raw.name),
            tempo: Tempo::from_packed_bytes(raw.tempo),
            time_division_switch: TriggerKind::try_from(raw.time_division_switch)
                .map_err(GlobalDeserializationError::TimeDivisionSwitch)?,
            time_division: TimeDivision::try_from(raw.division)
                .map_err(GlobalDeserializationError::TimeDivision)?,
            note_repeat_switch: TriggerKind::try_from(raw.note_repeat_switch)
                .map_err(GlobalDeserializationError::NoteRepeatSwitch)?,
            gate: GateValue::try_from(raw.gate).map_err(GlobalDeserializationError::Gate)?,
            swing: SwingKind::try_from(raw.swing).map_err(GlobalDeserializationError::Swing)?,
            transport: TransportKind::try_from(raw.transport)
                .map_err(GlobalDeserializationError::Transport)?,
        })
    }
}

impl From<&Global> for RawGlobal {
    fn from(global: &Global) -> Self {
        RawGlobal {
            preset: global.preset_slot as u8,
            name: global.preset_name.0,
            un1: 0,
            tempo: global.tempo.to_packed_bytes(),
            time_division_switch: global.time_division_switch as u8,
            division: global.time_division as u8,
            note_repeat_switch: global.note_repeat_switch as u8,
            gate: global.gate as u8,
            swing: global.swing as u8,
            un5: 0,
            un6: 0,
            un7: 0,
            un8: 0,
            un9: 0,
            transport: global.transport as u8,
        }
    }
}

pub enum NotePattern {
    Scale(ScaleSequence),
}

#[derive(Clone)]
pub enum ColorPattern {
    Contiguous(PadColor),
    Grouped(Vec<ColorSequence>),
}

impl ColorPattern {
    pub fn color_at_index(&self, index: usize) -> PadColor {
        match self {
            ColorPattern::Contiguous(color) => *color,
            ColorPattern::Grouped(sequences) => {
                if sequences.is_empty() {
                    return PadColor::default();
                }

                let total_cycle_len: usize = sequences.iter().map(|s| s.len).sum();
                if total_cycle_len == 0 {
                    return PadColor::default();
                }

                let position = index % total_cycle_len;
                let mut accumulated = 0;

                for seq in sequences {
                    if position < accumulated + seq.len {
                        return seq.color;
                    }
                    accumulated += seq.len;
                }

                sequences.last().map(|s| s.color).unwrap_or_default()
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct ColorSequence {
    pub len: usize,
    pub color: PadColor,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::Note;

    #[test]
    fn test_pad_repository_direct_mutation() {
        let mut repo = PadRepository::default();
        assert_eq!(repo.pads[0].note, Note::N60);

        repo.pads[0].note = Note::N105;
        assert_eq!(repo.pads[0].note, Note::N105);
    }

    #[test]
    fn test_note_mapping() {
        let seq = ScaleSequence {
            length: 1,
            ..Default::default()
        };

        assert_eq!(seq.as_midi_notes().len(), 1);
    }

    #[test]
    fn test_tempo_encoding_round_trip() {
        for bpm in [30, 60, 90, 120, 140, 180, 200, 250, 300] {
            let tempo = Tempo(bpm);
            let packed = tempo.to_packed_bytes();
            let decoded = Tempo::from_packed_bytes(packed);
            assert_eq!(tempo, decoded, "Tempo {bpm} BPM failed round-trip");
        }
    }
}
