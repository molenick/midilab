use midilab::manufacturer::akai::SYSEX_MANUFACTURER_ID;
use midilab::manufacturer::akai::mpd226::DEVICE_ID;
use midilab::manufacturer::akai::mpd226::DeviceCommandId;
use midilab::manufacturer::akai::mpd226::DeviceMessagePayload;
use midilab::manufacturer::akai::mpd226::DeviceStatusId;
use midilab::manufacturer::akai::mpd226::Global;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::control::value_kind::PresetSlot;
use midilab::manufacturer::akai::mpd226::raw::RawGlobal;
use midilab::manufacturer::akai::mpd226::raw::RawHeader;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;
use midilab::sysex::Sysex;
use midilab::sysex::pack_u14;
use midir::ConnectError;
use midir::MidiInput;
use midir::MidiOutput;
use midir::os::unix::VirtualInput;
use midir::os::unix::VirtualOutput;
use thiserror::Error;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::oneshot;

pub enum SimMsg {
    SysexReceived(Sysex),
}

pub enum SimEffect {
    SendSysex(Sysex),
    Noop,
}

#[derive(Default)]
pub struct Mpd226Sim {
    presets: [Preset; 21],
    global: Global,
}

impl Mpd226Sim {
    #[must_use]
    pub fn update(&mut self, msg: SimMsg) -> SimEffect {
        match msg {
            SimMsg::SysexReceived(sysex) => {
                let device_msg_payload: DeviceMessagePayload<DeviceCommandId> =
                    match DeviceMessagePayload::try_from(sysex.clone()) {
                        Ok(h) => h,
                        Err(e) => {
                            eprintln!("{}", e);
                            return SimEffect::Noop;
                        }
                    };

                let header = device_msg_payload.header;

                match header.cmd {
                    DeviceCommandId::DumpPreset => {
                        let slot = match PresetSlot::try_from(device_msg_payload.data[0]) {
                            Ok(s) => s,
                            Err(e) => {
                                eprintln!("{e}");
                                return SimEffect::Noop;
                            }
                        };

                        println!("decoded dump preset command for slot {slot}");

                        let preset = &self.presets[slot as usize];
                        let raw = RawPreset::from(preset);
                        SimEffect::SendSysex(Sysex::new(dump_preset(&raw)))
                    }
                    DeviceCommandId::WritePreset => {
                        let raw_preset: RawPreset =
                            *bytemuck::try_from_bytes(&device_msg_payload.data).unwrap();
                        let preset = match Preset::try_from(raw_preset) {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("{}", e);
                                return SimEffect::Noop;
                            }
                        };
                        let slot = preset.settings.preset_slot;
                        println!("decoded write preset command for slot {slot}");
                        self.presets[slot as usize] = preset;
                        SimEffect::SendSysex(Sysex::new(ack_preset(slot)))
                    }
                    DeviceCommandId::DumpGlobal => {
                        println!("decoded dump global command");
                        let raw = RawGlobal::from(&self.global);
                        SimEffect::SendSysex(Sysex::new(dump_global(&raw)))
                    }
                    DeviceCommandId::WriteGlobal => {
                        let addr = device_msg_payload.data[2];
                        let value = device_msg_payload.data[3];
                        let idx = (addr - 1) as usize;
                        let mut raw = RawGlobal::from(&self.global);
                        bytemuck::bytes_of_mut(&mut raw)[idx] = value;
                        match Global::try_from(raw) {
                            Ok(g) => {
                                println!(
                                    "decoded write global command: addr: {addr}, value: {value}, idx: {idx}"
                                );
                                self.global = g;
                                SimEffect::SendSysex(Sysex::new(ack_global(addr)))
                            }
                            Err(e) => {
                                eprintln!("{}", e);
                                SimEffect::Noop
                            }
                        }
                    }
                }
            }
        }
    }
}

fn ack_preset(slot: PresetSlot) -> Vec<u8> {
    let header = RawHeader {
        mfg_id: SYSEX_MANUFACTURER_ID,
        _unknown: 0,
        device_id: midilab::manufacturer::akai::mpd226::DEVICE_ID,
        cmd: DeviceStatusId::PresetAck as u8,
        length: pack_u14(2),
    };

    let mut sysex_payload = bytemuck::bytes_of(&header).to_vec();
    sysex_payload.push(slot as u8);
    sysex_payload.push(0x00);
    sysex_payload
}

fn dump_preset(raw: &RawPreset) -> Vec<u8> {
    let header = RawHeader {
        mfg_id: SYSEX_MANUFACTURER_ID,
        _unknown: 0,
        device_id: DEVICE_ID,
        cmd: DeviceCommandId::WritePreset as u8,
        length: 0x3308_u16.to_le_bytes(),
    };

    // todo: alt would be construct and serialize a DeviceMsg
    let mut sysex_payload = bytemuck::bytes_of(&header).to_vec();
    sysex_payload.extend_from_slice(bytemuck::bytes_of(raw));
    sysex_payload
}

fn dump_global(raw: &RawGlobal) -> Vec<u8> {
    let length = pack_u14(14);
    let header = RawHeader {
        mfg_id: SYSEX_MANUFACTURER_ID,
        _unknown: 0,
        device_id: midilab::manufacturer::akai::mpd226::DEVICE_ID,
        cmd: DeviceStatusId::WriteGlobal as u8,
        length,
    };

    let mut sysex_payload = bytemuck::bytes_of(&header).to_vec();
    sysex_payload.extend_from_slice(&[0x0B, 0x00, 0x01]);
    sysex_payload.extend_from_slice(bytemuck::bytes_of(raw));
    sysex_payload
}

fn ack_global(addr: u8) -> Vec<u8> {
    let header = RawHeader {
        mfg_id: SYSEX_MANUFACTURER_ID,
        _unknown: 0,
        device_id: midilab::manufacturer::akai::mpd226::DEVICE_ID,
        cmd: DeviceStatusId::GlobalAck as u8,
        length: pack_u14(4),
    };

    let mut sysex_payload = bytemuck::bytes_of(&header).to_vec();
    sysex_payload.extend_from_slice(&[0x01, 0x00, addr, 0x00]);
    sysex_payload
}

#[derive(Debug, Error)]
pub enum SimRunnerError {
    #[error("MIDI initialization error: {0}")]
    MidiInit(#[from] midir::InitError),
    #[error("Output port creation error: {0}")]
    OutputPortCreation(#[source] ConnectError<MidiOutput>),
    #[error("Input port creation error: {0}")]
    InputPortCreation(#[source] ConnectError<MidiInput>),
}

pub struct SimRunner {
    sim: Mpd226Sim,
    raw_midi_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    out_port: midir::MidiOutputConnection,
    _in_conn: midir::MidiInputConnection<()>,
    shutdown_rx: oneshot::Receiver<()>,
}

pub struct SimHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl SimRunner {
    pub fn start(port_name: &str) -> Result<(Self, SimHandle), SimRunnerError> {
        let (raw_midi_tx, raw_midi_rx) = unbounded_channel::<Vec<u8>>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let midi_out = MidiOutput::new("Mpd226 Remote")?;
        let out_port = midi_out
            .create_virtual(port_name)
            .map_err(SimRunnerError::OutputPortCreation)?;

        let midi_in = MidiInput::new("Mpd226 Remote")?;
        let _in_conn = midi_in
            .create_virtual(
                port_name,
                move |_ts, data, _| {
                    let _ = raw_midi_tx.send(data.to_vec());
                },
                (),
            )
            .map_err(SimRunnerError::InputPortCreation)?;

        let runner = SimRunner {
            sim: Mpd226Sim::default(),
            raw_midi_rx,
            out_port,
            _in_conn,
            shutdown_rx,
        };

        let handle = SimHandle {
            shutdown_tx: Some(shutdown_tx),
        };

        Ok((runner, handle))
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                _ = &mut self.shutdown_rx => {
                    break;
                }
                Some(raw) = self.raw_midi_rx.recv() => {
                    let sysex_in = match Sysex::try_from(raw.as_slice()) {
                        Ok(s) => {
                            println!("received sysex payload: {}", s.preview());
                            s
                        },
                        Err(e) => {
                            eprintln!("{e}");
                            continue;
                        }
                    };

                    let effect = self.sim.update(SimMsg::SysexReceived(sysex_in));

                    match effect {
                        SimEffect::SendSysex(sysex_out) => {
                            println!("sending sysex payload: {}", sysex_out.preview());
                            let bytes = sysex_out.as_bytes();

                            if let Err(e) = self.out_port.send(&bytes) {
                                eprintln!("MIDI send error: {e}");
                            }
                        }
                        SimEffect::Noop => {
                            println!("noop");
                        }
                    }
                }
            }
        }
    }
}

impl SimHandle {
    pub fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
