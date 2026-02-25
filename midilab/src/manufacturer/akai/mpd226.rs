use std::collections::HashMap;

use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;

use crate::error::DeviceStatusParseError;
use crate::manufacturer::akai::SYSEX_MANUFACTURER_ID;
use crate::manufacturer::akai::mpd226::control::PresetSettings;
use crate::manufacturer::akai::mpd226::control::value_kind::ActiveState;
use crate::manufacturer::akai::mpd226::control::value_kind::Gate;
use crate::manufacturer::akai::mpd226::control::value_kind::MidiClock;
use crate::manufacturer::akai::mpd226::control::value_kind::NoteDisplay;
use crate::manufacturer::akai::mpd226::control::value_kind::PadColor;
use crate::manufacturer::akai::mpd226::control::value_kind::PadCurve;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetName;
use crate::manufacturer::akai::mpd226::control::value_kind::PresetSlot;
use crate::manufacturer::akai::mpd226::control::value_kind::SwingKind;
use crate::manufacturer::akai::mpd226::control::value_kind::TapAverage;
use crate::manufacturer::akai::mpd226::control::value_kind::Tempo;
use crate::manufacturer::akai::mpd226::control::value_kind::TimeDivision;
use crate::manufacturer::akai::mpd226::control::value_kind::TransportKind;
use crate::manufacturer::akai::mpd226::control::value_kind::TriggerKind;
use crate::manufacturer::akai::mpd226::control::value_kind::UsbChannel;
use crate::manufacturer::akai::mpd226::error::GlobalAckParseError;
use crate::manufacturer::akai::mpd226::error::GlobalParseError;
use crate::manufacturer::akai::mpd226::error::PresetAckParseError;
use crate::manufacturer::akai::mpd226::error::PresetParseError;
use crate::manufacturer::akai::mpd226::raw::RawDials;
use crate::manufacturer::akai::mpd226::raw::RawFaders;
use crate::manufacturer::akai::mpd226::raw::RawGlobal;
use crate::manufacturer::akai::mpd226::raw::RawGlobalParamAck;
use crate::manufacturer::akai::mpd226::raw::RawHeader;
use crate::manufacturer::akai::mpd226::raw::RawPads;
use crate::manufacturer::akai::mpd226::raw::RawPreset;
use crate::manufacturer::akai::mpd226::raw::RawPresetAck;
use crate::manufacturer::akai::mpd226::raw::RawPresetSettings;
use crate::manufacturer::akai::mpd226::raw::RawSwitches;
use crate::manufacturer::akai::mpd226::repository::DialRepository;
use crate::manufacturer::akai::mpd226::repository::FaderRepository;
use crate::manufacturer::akai::mpd226::repository::PadRepository;
use crate::manufacturer::akai::mpd226::repository::SwitchRepository;
use crate::music::generation::PitchPattern;
use crate::music::theory::PitchClass;
use crate::sysex::Sysex;
use crate::sysex::unpack_u14;

pub mod control;
pub mod error;
pub mod raw;
pub mod repository;

pub const DEVICE_ID: u8 = 0x35;
const TOTAL_PADS: usize = 64;

pub(crate) const PRESET_FOOTER_MAGIC_LEN: usize = 12;
pub(crate) const PRESET_FOOTER_MAGIC_BYTES: [u8; PRESET_FOOTER_MAGIC_LEN] =
    [4, 0, 0, 4, 0, 2, 4, 0, 4, 4, 0, 6];
const GLOBAL_VALUE_FOOTER_BYTES: usize = 2;
const GLOBAL_VALUE_FOOTER_MAGIC: [u8; GLOBAL_VALUE_FOOTER_BYTES] = [0x01, 0x00];

const SEND_GLOBAL_PADDING_LEN: usize = 3;
const SEND_GLOBAL_PADDING_BYTES: [u8; SEND_GLOBAL_PADDING_LEN] = [0x0B, 0x00, 0x01];

#[repr(u8)]
#[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Clone, Copy)]
pub enum GlobalParamCmdId {
    CommonChannel = 0x01,
    LcdContrast = 0x02,
    TapAverage = 0x03,
    TempoLed = 0x04,
    MidiClock = 0x05,
    TransportToDIN = 0x06,
    PadThreshold = 0x07,
    Unknown08 = 0x08,
    PadCurve = 0x09,
    PadGain = 0x0A,
    NoteDisplay = 0x0B,
}

#[repr(u8)]
#[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Clone, Copy)]
pub enum DeviceStatusId {
    WritePreset = 0x10,
    PresetAck = 0x11,
    WriteGlobal = 0x34,
    GlobalAck = 0x3C,
}

#[repr(u8)]
#[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Clone, Copy)]
pub enum DeviceCommandId {
    WritePreset = 0x10,
    DumpPreset = 0x12,
    DumpGlobal = 0x24,
    WriteGlobal = 0x34,
}

pub fn dump_preset_from_device(slot: u8) -> Vec<u8> {
    let mut sysex_payload = bytemuck::bytes_of(&RawHeader::dump_preset()).to_vec();
    sysex_payload.extend_from_slice(bytemuck::bytes_of(&slot));
    Sysex::new(sysex_payload).as_bytes()
}

pub fn write_preset_to_device(preset: &RawPreset) -> Vec<u8> {
    let mut sysex_payload = bytemuck::bytes_of(&RawHeader::write_preset()).to_vec();
    sysex_payload.extend_from_slice(bytemuck::bytes_of(preset));
    Sysex::new(sysex_payload).as_bytes()
}

pub fn dump_global_from_device() -> Vec<u8> {
    let length = u16::from_le_bytes([0x00, 0x03]).to_le_bytes();

    let header = RawHeader {
        mfg_id: SYSEX_MANUFACTURER_ID,
        _unknown: 0,
        device_id: DEVICE_ID,
        cmd: DeviceCommandId::DumpGlobal as u8,
        length,
    };

    let mut sysex_payload = bytemuck::bytes_of(&header).to_vec();

    sysex_payload.extend_from_slice(&SEND_GLOBAL_PADDING_BYTES);
    Sysex::new(sysex_payload).as_bytes()
}

pub fn write_global_param_to_device(addr: u8, value: u8) -> Vec<u8> {
    let length = u16::from_le_bytes([0x00, 0x04]).to_le_bytes();
    let header = RawHeader {
        mfg_id: SYSEX_MANUFACTURER_ID,
        _unknown: 0,
        device_id: DEVICE_ID,
        cmd: DeviceCommandId::WriteGlobal as u8,
        length,
    };

    let mut sysex_payload = bytemuck::bytes_of(&header).to_vec();
    sysex_payload.extend_from_slice(&GLOBAL_VALUE_FOOTER_MAGIC);
    sysex_payload.push(addr);
    sysex_payload.push(value);
    Sysex::new(sysex_payload).as_bytes()
}

pub struct DeviceMessagePayload<C> {
    pub header: DeviceHeader<C>,
    pub data: Vec<u8>,
}
impl<C: TryFrom<u8>> TryFrom<&[u8]> for DeviceMessagePayload<C> {
    type Error = DeviceStatusParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let s = Sysex::try_from(value)?;
        DeviceMessagePayload::try_from(s)
    }
}

#[derive(Debug, PartialEq)]
pub struct DeviceHeader<C> {
    pub cmd: C,
    pub message_length: u16,
}
impl<C: TryFrom<u8>> TryFrom<Sysex> for DeviceMessagePayload<C> {
    type Error = DeviceStatusParseError;

    fn try_from(value: Sysex) -> Result<DeviceMessagePayload<C>, Self::Error> {
        let header_size = std::mem::size_of::<RawHeader>();
        if value.payload().len() < header_size {
            return Err(DeviceStatusParseError::InvalidHeader);
        }

        let (hb, pb) = &value.payload().split_at(header_size);

        let raw_header: RawHeader =
            *bytemuck::try_from_bytes(hb).map_err(|_| DeviceStatusParseError::InvalidHeader)?;

        if raw_header.mfg_id != SYSEX_MANUFACTURER_ID {
            return Err(DeviceStatusParseError::InvalidHeader);
        }

        if raw_header.device_id != DEVICE_ID {
            return Err(DeviceStatusParseError::InvalidHeader);
        }

        let cmd = C::try_from(raw_header.cmd)
            .map_err(|_| DeviceStatusParseError::InvalidCommand(raw_header.cmd))?;

        let length = unpack_u14(raw_header.length);

        Ok(DeviceMessagePayload {
            header: DeviceHeader {
                cmd,
                message_length: length,
            },
            data: pb.to_vec(),
        })
    }
}

pub struct GlobalParamAck {
    pub addr: GlobalParamCmdId,
    pub status: u8,
}
impl TryFrom<&[u8]> for GlobalParamAck {
    type Error = GlobalAckParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let raw: RawGlobalParamAck = *bytemuck::try_from_bytes(value)
            .map_err(|_| GlobalAckParseError::InvalidLength(value.len()))?;

        GlobalParamAck::try_from(raw)
    }
}
impl TryFrom<RawGlobalParamAck> for GlobalParamAck {
    type Error = GlobalAckParseError;

    fn try_from(value: RawGlobalParamAck) -> Result<Self, Self::Error> {
        Ok(Self {
            addr: GlobalParamCmdId::try_from(value.addr)
                .map_err(|_| GlobalAckParseError::InvalidAddr(value.addr))?,
            status: value.status,
        })
    }
}
impl TryFrom<RawPresetAck> for PresetAck {
    type Error = PresetAckParseError;

    fn try_from(value: RawPresetAck) -> Result<Self, Self::Error> {
        let slot = PresetSlot::try_from(value.slot)
            .map_err(|_| PresetAckParseError::InvalidSlot(value.slot))?;

        Ok(Self { slot })
    }
}

pub struct PresetAck {
    pub slot: PresetSlot,
}
impl TryFrom<&[u8]> for PresetAck {
    type Error = PresetAckParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let raw: RawPresetAck = *bytemuck::try_from_bytes(value)
            .map_err(|_| PresetAckParseError::InvalidLength(value.len()))?;

        Self::try_from(raw)
    }
}

pub enum DeviceStatus {
    ReceivedPresetAck(PresetAck),
    PresetData(Box<Preset>),
    GlobalData(Box<Global>),
    GlobalParamAck(GlobalParamAck),
}

impl TryFrom<&[u8]> for DeviceStatus {
    type Error = DeviceStatusParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let payload: DeviceMessagePayload<DeviceStatusId> = DeviceMessagePayload::try_from(value)?;

        DeviceStatus::try_from(payload)
    }
}
impl TryFrom<DeviceMessagePayload<DeviceStatusId>> for DeviceStatus {
    type Error = DeviceStatusParseError;

    fn try_from(value: DeviceMessagePayload<DeviceStatusId>) -> Result<Self, Self::Error> {
        match value.header.cmd {
            DeviceStatusId::WritePreset => {
                let preset = Box::new(Preset::try_from(value.data.as_ref())?);
                Ok(DeviceStatus::PresetData(preset))
            }
            DeviceStatusId::PresetAck => {
                let ack = PresetAck::try_from(value.data.as_ref())?;
                Ok(DeviceStatus::ReceivedPresetAck(ack))
            }
            DeviceStatusId::WriteGlobal => {
                let global = Box::new(Global::try_from(value.data.as_ref())?);
                Ok(DeviceStatus::GlobalData(global))
            }
            DeviceStatusId::GlobalAck => {
                let ack = GlobalParamAck::try_from(value.data.as_ref())?;
                Ok(DeviceStatus::GlobalParamAck(ack))
            }
        }
    }
}
impl TryFrom<Sysex> for DeviceStatus {
    type Error = DeviceStatusParseError;

    fn try_from(value: Sysex) -> Result<Self, Self::Error> {
        let payload: DeviceMessagePayload<DeviceStatusId> =
            DeviceMessagePayload::try_from(value.clone())
                .map_err(|_e| DeviceStatusParseError::InvalidHeader)?;

        DeviceStatus::try_from(payload)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub settings: PresetSettings,
    pub pads: PadRepository,
    pub dials: DialRepository,
    pub faders: FaderRepository,
    pub switches: SwitchRepository,
}

impl Preset {
    const GENERIC_DIAL_CC: [u8; 12] = [3, 9, 14, 15, 52, 53, 54, 55, 83, 85, 86, 87];
    const GENERIC_FADER_CC: [u8; 12] = [20, 21, 22, 23, 61, 62, 63, 70, 92, 93, 94, 95];
    const GENERIC_SWITCH_CC: [u8; 12] = [28, 29, 30, 31, 75, 76, 77, 78, 106, 107, 108, 109];

    pub fn blank() -> Self {
        let settings = PresetSettings::default();

        let pads = PadRepository::default();
        let dials = DialRepository::with_cc_values(Self::GENERIC_DIAL_CC);
        let faders = FaderRepository::with_cc_values(Self::GENERIC_FADER_CC);
        let switches = SwitchRepository::with_cc_values(Self::GENERIC_SWITCH_CC);

        Self {
            settings,
            pads,
            dials,
            faders,
            switches,
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let raw = RawPreset::from(self);
        bytemuck::bytes_of(&raw).to_vec()
    }

    pub fn as_sysex_write(&self) -> Sysex {
        let preset = self.as_bytes();
        let header = RawHeader::write_preset();
        let header = bytemuck::bytes_of(&header);
        let bytes = header.iter().chain(&preset).cloned().collect::<Vec<u8>>();

        Sysex::new(bytes)
    }
}

impl TryFrom<&[u8]> for Preset {
    type Error = PresetParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let raw: RawPreset = *bytemuck::from_bytes(value);
        Preset::try_from(raw)
    }
}

impl Default for Preset {
    fn default() -> Self {
        let settings = PresetSettings::default();

        let mut pads = PadRepository::default();
        let dials = DialRepository::with_cc_values(Self::GENERIC_DIAL_CC);
        let faders = FaderRepository::with_cc_values(Self::GENERIC_FADER_CC);
        let switches = SwitchRepository::with_cc_values(Self::GENERIC_SWITCH_CC);

        pads.set_off_color_pattern(
            0,
            64,
            ColorPattern::Repeating(vec![
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
            ColorPattern::Repeating(vec![
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
            settings,
            pads,
            dials,
            faders,
            switches,
        }
    }
}

impl TryFrom<RawPreset> for Preset {
    type Error = error::PresetParseError;

    fn try_from(raw: RawPreset) -> Result<Self, Self::Error> {
        let settings = PresetSettings::try_from(raw.settings)?;
        let pads = PadRepository::try_from(RawPads(raw.pads))?;
        let dials = DialRepository::try_from(RawDials(raw.dials))?;
        let faders = FaderRepository::try_from(RawFaders(raw.faders))?;
        let switches = SwitchRepository::try_from(RawSwitches(raw.switches))?;

        Ok(Preset {
            settings,
            pads,
            dials,
            faders,
            switches,
        })
    }
}

impl From<&Preset> for RawPreset {
    fn from(preset: &Preset) -> Self {
        let settings = RawPresetSettings::from(&preset.settings);

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
            settings,
            pads: bytemuck::cast(pads),
            dials: bytemuck::cast(dials),
            faders: bytemuck::cast(faders),
            switches: bytemuck::cast(switches),
            footer_magic: PRESET_FOOTER_MAGIC_BYTES,
        }
    }
}

impl TryFrom<RawPresetSettings> for PresetSettings {
    type Error = error::PresetSettingsParseError;

    fn try_from(raw: RawPresetSettings) -> Result<Self, Self::Error> {
        use error::PresetSettingsParseError;
        Ok(PresetSettings {
            preset_slot: PresetSlot::try_from(raw.preset)
                .map_err(PresetSettingsParseError::PresetSlot)?,
            preset_name: PresetName(raw.name),
            tempo: Tempo::from_packed_bytes(raw.tempo),
            time_division_switch: TriggerKind::try_from(raw.time_division_switch)
                .map_err(PresetSettingsParseError::TimeDivisionSwitch)?,
            time_division: TimeDivision::try_from(raw.division)
                .map_err(PresetSettingsParseError::TimeDivision)?,
            note_repeat_switch: TriggerKind::try_from(raw.note_repeat_switch)
                .map_err(PresetSettingsParseError::NoteRepeatSwitch)?,
            gate: Gate::from(raw.gate),
            swing: SwingKind::try_from(raw.swing).map_err(PresetSettingsParseError::Swing)?,
            transport: TransportKind::try_from(raw.transport)
                .map_err(PresetSettingsParseError::Transport)?,
        })
    }
}

impl From<&PresetSettings> for RawPresetSettings {
    fn from(settings: &PresetSettings) -> Self {
        RawPresetSettings {
            preset: settings.preset_slot as u8,
            name: settings.preset_name.0,
            un1: 0,
            tempo: settings.tempo.to_packed_bytes(),
            time_division_switch: settings.time_division_switch as u8,
            division: settings.time_division as u8,
            note_repeat_switch: settings.note_repeat_switch as u8,
            gate: settings.gate.into(),
            swing: settings.swing as u8,
            un5: 0,
            un6: 0,
            un7: 0,
            un8: 0,
            un9: 0,
            transport: settings.transport as u8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Global {
    pub common_channel: UsbChannel,
    pub lcd_contrast: u8,
    pub tap_average: TapAverage,
    pub tempo_led: ActiveState,
    pub note_display: NoteDisplay,
    pub pad_threshold: u8,
    pub pad_curve: PadCurve,
    pub pad_gain: u8,
    pub midi_clock: MidiClock,
}

impl Default for Global {
    fn default() -> Self {
        Self {
            common_channel: UsbChannel::default(),
            lcd_contrast: 50,
            tap_average: TapAverage::default(),
            tempo_led: ActiveState::On,
            note_display: NoteDisplay::default(),
            pad_gain: 0,      // clamped: 0..=20
            pad_threshold: 1, // clamped: 1..=10
            pad_curve: PadCurve::default(),
            midi_clock: MidiClock::default(),
        }
    }
}

impl TryFrom<&[u8]> for Global {
    type Error = GlobalParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let global_params = &value[SEND_GLOBAL_PADDING_LEN..];
        let raw: RawGlobal = *bytemuck::try_from_bytes(global_params)
            .map_err(|_| GlobalParseError::InvalidLength(value.len()))?;

        Global::try_from(raw)
    }
}

impl TryFrom<RawGlobal> for Global {
    type Error = error::GlobalParseError;

    fn try_from(raw: RawGlobal) -> Result<Self, Self::Error> {
        use error::GlobalParseError;
        Ok(Global {
            common_channel: UsbChannel::try_from(raw.common_channel)
                .map_err(GlobalParseError::CommonChannel)?,
            lcd_contrast: raw.lcd_contrast,
            tap_average: TapAverage::try_from(raw.tap_average)
                .map_err(GlobalParseError::TapAverage)?,
            tempo_led: ActiveState::try_from(raw.tempo_led).map_err(GlobalParseError::TempoLed)?,
            note_display: NoteDisplay::try_from(raw.note_display)
                .map_err(GlobalParseError::NoteDisplay)?,
            pad_gain: raw.pad_gain,
            pad_threshold: raw.pad_threshold,
            pad_curve: PadCurve::try_from(raw.pad_curve).map_err(GlobalParseError::PadCurve)?,
            midi_clock: MidiClock::try_from(raw.midi_clock).map_err(GlobalParseError::MidiClock)?,
        })
    }
}

impl From<&Global> for RawGlobal {
    fn from(global: &Global) -> Self {
        RawGlobal {
            common_channel: global.common_channel as u8,
            lcd_contrast: global.lcd_contrast,
            tap_average: global.tap_average as u8,
            tempo_led: global.tempo_led as u8,
            note_display: global.note_display as u8,
            transport_to_din: 0,
            midi_clock: global.midi_clock as u8,
            _unknown_08: 0,
            pad_threshold: global.pad_threshold,
            pad_curve: global.pad_curve as u8,
            pad_gain: global.pad_gain,
        }
    }
}

#[derive(Clone)]
pub enum ColorPattern {
    Repeating(Vec<ColorSequence>),
}

impl ColorPattern {
    pub fn color_at_index(&self, index: usize) -> PadColor {
        match self {
            ColorPattern::Repeating(sequences) => {
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
impl From<(PitchPattern, NoteColorMap)> for ColorPattern {
    fn from((np, m): (PitchPattern, NoteColorMap)) -> Self {
        let mut pattern: Vec<ColorSequence> = Vec::with_capacity(np.len());
        let pcs = np.as_pitches();

        for p in pcs.0 {
            if let Some(c) = m.0.get(&p.class) {
                const PITCH_SEQ_LEN: usize = 1;
                let cs = ColorSequence {
                    len: PITCH_SEQ_LEN,
                    color: *c,
                };
                pattern.push(cs);
            }
        }

        ColorPattern::Repeating(pattern)
    }
}

#[derive(Clone)]
pub struct NoteColorMap(pub HashMap<PitchClass, PadColor>);
impl Default for NoteColorMap {
    fn default() -> Self {
        Self::default_chromatic_gradient()
    }
}

impl NoteColorMap {
    pub fn default_chromatic_gradient() -> Self {
        let mut hm = HashMap::with_capacity(12);

        hm.insert(PitchClass::C, PadColor::Red);
        hm.insert(PitchClass::Cs, PadColor::HotPink);
        hm.insert(PitchClass::D, PadColor::Pink);
        hm.insert(PitchClass::Ds, PadColor::LightPurple);
        hm.insert(PitchClass::E, PadColor::Purple);
        hm.insert(PitchClass::F, PadColor::Blue);
        hm.insert(PitchClass::Fs, PadColor::LightBlue);
        hm.insert(PitchClass::G, PadColor::Aqua);
        hm.insert(PitchClass::Gs, PadColor::GreenBlue);
        hm.insert(PitchClass::A, PadColor::Green);
        hm.insert(PitchClass::As, PadColor::Yellow);
        hm.insert(PitchClass::B, PadColor::Orange);

        Self(hm)
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
    use crate::manufacturer::akai::mpd226::control::ControlId;
    use crate::midi::MidiNote;
    use crate::music::generation::ScaleSequence;

    #[test]
    fn test_pad_repository_direct_mutation() {
        let mut repo = PadRepository::default();
        assert_eq!(repo.pads[0].note, MidiNote::from(60));

        repo.pads[0].note = MidiNote::from(105);
        assert_eq!(repo.pads[0].note, MidiNote::from(105));
    }

    #[test]
    fn test_note_mapping() {
        use crate::midi::generation::MidiNoteSequence;
        let seq = ScaleSequence {
            length: 1,
            ..Default::default()
        };

        assert_eq!(MidiNoteSequence::from(seq.as_pitches()).0.len(), 1);
    }

    #[test]
    fn test_tempo_encoding_round_trip() {
        for bpm in [30, 60, 90, 120, 140, 180, 200, 250, 300] {
            let tempo = Tempo(bpm);
            let packed = tempo.to_packed_bytes();
            let decoded = Tempo::from_packed_bytes(packed);
            assert_eq!(tempo, decoded);
        }
    }

    #[test]
    fn test_global_default() {
        use super::control::value_kind::ActiveState;
        use super::control::value_kind::MidiClock;
        use super::control::value_kind::NoteDisplay;
        use super::control::value_kind::PadCurve;
        use super::control::value_kind::TapAverage;

        let global = super::Global::default();

        assert_eq!(
            global.common_channel,
            super::control::value_kind::UsbChannel::A1
        );
        assert_eq!(global.lcd_contrast, 50);
        assert_eq!(global.tap_average, TapAverage::Tap3);
        assert_eq!(global.tempo_led, ActiveState::On);
        assert_eq!(global.note_display, NoteDisplay::Value);
        assert_eq!(global.pad_gain, 0);
        assert_eq!(global.pad_threshold, 1);
        assert_eq!(global.pad_curve, PadCurve::Linear);
        assert_eq!(global.midi_clock, MidiClock::Internal);
    }

    #[test]
    fn test_global_round_trip_conversion() {
        use super::Global;
        use super::control::value_kind::ActiveState;
        use super::control::value_kind::MidiClock;
        use super::control::value_kind::NoteDisplay;
        use super::control::value_kind::PadCurve;
        use super::control::value_kind::TapAverage;
        use super::raw::RawGlobal;

        let global = Global {
            common_channel: UsbChannel::A5,
            lcd_contrast: 42,
            tap_average: TapAverage::Tap4,
            tempo_led: ActiveState::Off,
            note_display: NoteDisplay::Number,
            pad_gain: 15,
            pad_threshold: 8,
            pad_curve: PadCurve::Exp2,
            midi_clock: MidiClock::Internal,
        };

        let raw = RawGlobal::from(&global);
        let restored = Global::try_from(raw).unwrap();

        assert_eq!(global, restored);
    }

    #[test]
    fn test_global_dump_request_format() {
        let request = super::dump_global_from_device();

        assert_eq!(request.len(), 11);
        assert_eq!(request[0], 0xF0);
        assert_eq!(request[1], 0x47);
        assert_eq!(request[2], 0x00);
        assert_eq!(request[3], 0x35);
        assert_eq!(request[4], 0x24);
        assert_eq!(request[5], 0x00);
        assert_eq!(request[6], 0x03);
        assert_eq!(request[7], 0x0B);
        assert_eq!(request[8], 0x00);
        assert_eq!(request[9], 0x01);
        assert_eq!(request[10], 0xF7);
    }

    #[test]
    fn test_global_write_param_format() {
        let msg = super::write_global_param_to_device(0x02, 50);

        assert_eq!(msg.len(), 12);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], 0x47);
        assert_eq!(msg[2], 0x00);
        assert_eq!(msg[3], 0x35);
        assert_eq!(msg[4], 0x34);
        assert_eq!(msg[5], 0x00);
        assert_eq!(msg[6], 0x04);
        assert_eq!(msg[7], 0x01);
        assert_eq!(msg[8], 0x00);
        assert_eq!(msg[9], 0x02);
        assert_eq!(msg[10], 50);
        assert_eq!(msg[11], 0xF7);
    }

    #[test]
    fn test_global_send_messages_count() {
        use super::raw::RawGlobal;

        let raw = RawGlobal::default();
        let messages = raw.global_send_messages();

        assert_eq!(messages.len(), 11);

        for msg in &messages {
            assert_eq!(msg.len(), 12);
        }
    }

    #[test]
    fn trace_preset_channel_bytes() {
        use super::control::Pad;
        use super::control::value_kind::MidiChannel;
        use super::raw::RawPreset;

        let a1_raw = MidiChannel::A1 as u8;
        assert_eq!(a1_raw, 1);

        let a12_raw = MidiChannel::A12 as u8;
        assert_eq!(a12_raw, 12);

        let mut pad = Pad::new(ControlId(0));
        pad.channel = MidiChannel::A1;
        let pad_bytes = pad.as_bytes();
        assert_eq!(pad_bytes[1], 1,);

        let mut preset = Preset::default();
        for pad in preset.pads.pads.iter_mut() {
            pad.channel = MidiChannel::A1;
        }
        for dial in preset.dials.0.iter_mut() {
            dial.channel = MidiChannel::A1;
        }
        for fader in preset.faders.0.iter_mut() {
            fader.channel = MidiChannel::A1;
        }
        for switch in preset.switches.0.iter_mut() {
            switch.channel = MidiChannel::A1;
        }

        let raw = RawPreset::from(&preset);
        let raw_bytes = bytemuck::bytes_of(&raw);

        let settings_size = std::mem::size_of::<super::raw::RawPresetSettings>();
        let pad_channel_offset = settings_size + 1;
        assert_eq!(raw_bytes[pad_channel_offset], 1,);

        let dials_start = settings_size + (64 * 11);
        let dial_channel_offset = dials_start + 1;
        assert_eq!(raw_bytes[dial_channel_offset], 1,);

        let raw2: RawPreset = *bytemuck::from_bytes(raw_bytes);
        let preset2 = Preset::try_from(raw2).unwrap();

        assert_eq!(preset2.pads.pads[0].channel, MidiChannel::A1,);
        assert_eq!(preset2.dials.0[0].channel, MidiChannel::A1,);
        assert_eq!(preset2.faders.0[0].channel, MidiChannel::A1,);
        assert_eq!(preset2.switches.0[0].channel, MidiChannel::A1,);

        let sysex_bytes = write_preset_to_device(&raw);

        let data_start = 1 + 6;
        let data_end = sysex_bytes.len() - 1;
        let data = &sysex_bytes[data_start..data_end];

        assert_eq!(data.len(), std::mem::size_of::<RawPreset>(),);

        let raw3: RawPreset = *bytemuck::from_bytes(data);
        let preset3 = Preset::try_from(raw3).unwrap();

        assert_eq!(preset3.pads.pads[0].channel, MidiChannel::A1,);
    }

    #[test]
    fn trace_global_dump_response_strips_prefix() {
        use super::DeviceStatus;

        let common_channel_a1: u8 = 0;

        let device_response = vec![
            0xF0,
            0x47,
            0x00,
            0x35,
            0x34,
            0x00,
            0x0E,
            0x0B,
            0x00,
            0x01,
            common_channel_a1,
            0x32,
            0x03,
            0x01,
            0x00,
            0x00,
            0x05,
            0x00,
            0x00,
            0x00,
            0x00,
            0xF7,
        ];

        let status = DeviceStatus::try_from(device_response.as_slice()).unwrap();

        match status {
            DeviceStatus::GlobalData(global) => {
                assert_eq!(global.common_channel, UsbChannel::A1,);
                assert_eq!(global.lcd_contrast, 50);
                assert_eq!(global.pad_threshold, 5);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_device_status_global_ack_parsing() {
        use super::DeviceStatus;
        use crate::sysex::Sysex;

        let bytes = vec![
            0xF0, 0x47, 0x00, 0x35, 0x3C, 0x04, 0x00, 0x01, 0x00, 0x02, 0x00, 0xF7,
        ];

        let sysex = Sysex::try_from(bytes.as_slice()).unwrap();
        let status = DeviceStatus::try_from(sysex).unwrap();

        let ack = match status {
            DeviceStatus::GlobalParamAck(ack) => ack,
            _ => panic!("wrong variant"),
        };

        assert_eq!(ack.addr, GlobalParamCmdId::try_from(0x02).unwrap());
        assert_eq!(ack.status, 0x00);
    }

    #[test]
    fn test_preset_ack_ram_slot_parsed_correctly() {
        let ack = PresetAck::try_from([0x00u8, 0x01u8].as_ref()).unwrap();
        assert_eq!(ack.slot, control::value_kind::PresetSlot::RAM);
    }

    #[test]
    fn test_note_pattern_scale_delegates() {
        use crate::midi::generation::MidiNoteSequence;
        use crate::music::generation::PitchPattern;
        let seq = ScaleSequence::default();
        let pattern = PitchPattern::Scale(seq);
        assert_eq!(
            MidiNoteSequence::from(pattern.as_pitches()).0,
            MidiNoteSequence::from(seq.as_pitches()).0
        );
    }

    #[test]
    fn test_preset_as_bytes_size() {
        let preset = Preset::default();
        assert_eq!(
            preset.as_bytes().len(),
            std::mem::size_of::<raw::RawPreset>()
        );
    }

    #[test]
    fn test_preset_as_sysex_write() {
        let preset = Preset::default();
        let sysex = preset.as_sysex_write();

        let payload: DeviceMessagePayload<DeviceCommandId> =
            DeviceMessagePayload::try_from(sysex).unwrap();

        assert_eq!(payload.header.cmd, DeviceCommandId::WritePreset);
        assert_eq!(
            payload.header.message_length as usize,
            std::mem::size_of::<raw::RawPreset>()
        );
    }
}
