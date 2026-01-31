use midilab::manufacturer::akai::SYSEX_MANUFACTURER_ID;
use midilab::manufacturer::akai::mpd226::DeviceCommand;
use midilab::manufacturer::akai::mpd226::Header;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::control::value_kind::PresetSlot;
use midilab::manufacturer::akai::mpd226::preset_send_message;
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
    SendSysex(Vec<u8>),
    Noop,
}

#[derive(Default)]
pub struct Mpd226Sim {
    presets: [Preset; 21],
}

impl Mpd226Sim {
    #[must_use]
    pub fn update(&mut self, msg: SimMsg) -> SimEffect {
        match msg {
            SimMsg::SysexReceived(sysex) => {
                let header = match Header::try_from(sysex.clone()) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("{}", e);
                        return SimEffect::Noop;
                    }
                };

                match header.cmd {
                    DeviceCommand::DumpPreset => {
                        let header_size = std::mem::size_of::<RawHeader>();
                        let payload = sysex.payload();

                        let slot_byte = payload[header_size];
                        let slot = match PresetSlot::try_from(slot_byte) {
                            Ok(s) => s,
                            Err(e) => {
                                eprintln!("{}", e);
                                return SimEffect::Noop;
                            }
                        };

                        let preset = &self.presets[slot as usize];

                        let raw = RawPreset::from(preset);
                        SimEffect::SendSysex(preset_send_message(&raw))
                    }
                    DeviceCommand::SendPreset => {
                        let raw_preset = match RawPreset::try_from(sysex) {
                            Ok(r) => r,
                            Err(e) => {
                                eprintln!("{}", e);
                                return SimEffect::Noop;
                            }
                        };
                        let preset = match Preset::try_from(raw_preset) {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("{}", e);
                                return SimEffect::Noop;
                            }
                        };
                        let slot = preset.global.preset_slot;
                        self.presets[slot as usize] = preset;
                        SimEffect::SendSysex(preset_ack_message(slot))
                    }
                    DeviceCommand::PresetAck => SimEffect::Noop,
                }
            }
        }
    }
}

fn preset_ack_message(slot: PresetSlot) -> Vec<u8> {
    let header = RawHeader {
        mfg_id: SYSEX_MANUFACTURER_ID,
        _unknown: 0,
        device_id: midilab::manufacturer::akai::mpd226::DEVICE_ID,
        cmd: DeviceCommand::PresetAck as u8,
        length: pack_u14(2),
    };

    let mut sysex_payload = bytemuck::bytes_of(&header).to_vec();
    sysex_payload.push(slot as u8);
    sysex_payload.push(0x00);
    Sysex::new(sysex_payload).as_bytes()
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
                    let sysex = match Sysex::try_from(raw.as_slice()) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("{e}");
                            continue;
                        }
                    };

                    let effect = self.sim.update(SimMsg::SysexReceived(sysex));

                    match effect {
                        SimEffect::SendSysex(bytes) => {
                            if let Err(e) = self.out_port.send(&bytes) {
                                eprintln!("MIDI send error: {e}");
                            }
                        }
                        SimEffect::Noop => {}
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
