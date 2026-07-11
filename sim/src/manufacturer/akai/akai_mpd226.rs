use midi_io::Client;
use midi_io::SysEx;
use midilab::manufacturer::akai::SYSEX_MANUFACTURER_ID;
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
use thiserror::Error;
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
                    match DeviceMessagePayload::try_from(sysex) {
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
                        let slot = preset.settings.slot;
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

    let sysex = midilab::sysex::Sysex::from_header_and_body(&header, [slot as u8, 0x00]);
    sysex.payload().to_vec()
}

fn dump_preset(raw: &RawPreset) -> Vec<u8> {
    let sysex = midilab::sysex::Sysex::from_header_and_body(
        &RawHeader::write_preset(),
        bytemuck::bytes_of(raw),
    );
    sysex.payload().to_vec()
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
    let sysex = midilab::sysex::Sysex::from_header_and_body(&header, [0x0B, 0x00, 0x01]);
    let mut payload = sysex.payload().to_vec();
    payload.extend_from_slice(bytemuck::bytes_of(raw));
    payload
}

fn ack_global(addr: u8) -> Vec<u8> {
    let header = RawHeader {
        mfg_id: SYSEX_MANUFACTURER_ID,
        _unknown: 0,
        device_id: midilab::manufacturer::akai::mpd226::DEVICE_ID,
        cmd: DeviceStatusId::GlobalAck as u8,
        length: pack_u14(4),
    };
    let sysex = midilab::sysex::Sysex::from_header_and_body(&header, [0x01, 0x00, addr, 0x00]);
    sysex.payload().to_vec()
}

#[derive(Debug, Error)]
pub enum SimRunnerError {
    #[error("MIDI initialization error: {0}")]
    MidiInit(#[from] midi_io::Error),
}

pub struct SimRunner {
    sim: Mpd226Sim,
    sysex_in: midi_io::SysexStream,
    out_port: midi_io::VirtualSource,
    shutdown_rx: oneshot::Receiver<()>,
}

pub struct SimHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl SimRunner {
    pub async fn start(port_name: &str) -> Result<(Self, SimHandle), SimRunnerError> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let client = Client::new("Mpd226 Remote").await?;
        let out_port = client.create_virtual_source(port_name).await?;
        let in_port = client.create_virtual_destination(port_name).await?;
        let sysex_in = in_port.into_sysex();

        let runner = SimRunner {
            sim: Mpd226Sim::default(),
            sysex_in,
            out_port,
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
                Some(timed) = self.sysex_in.recv() => {
                    let sysex_in = Sysex::new(timed.payload.bytes().to_vec());
                    println!("received sysex payload: {}", sysex_in.preview());

                    let effect = self.sim.update(SimMsg::SysexReceived(sysex_in));

                    match effect {
                        SimEffect::SendSysex(sysex_out) => {
                            println!("sending sysex payload: {}", sysex_out.preview());

                            match SysEx::new(sysex_out.payload()) {
                                Ok(sysex) => {
                                    if let Err(e) = self.out_port.send_sysex(&sysex).await {
                                        eprintln!("MIDI send error: {e}");
                                    }
                                }
                                Err(e) => eprintln!("MIDI send error: {e}"),
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

#[cfg(test)]
mod tests {
    use midilab::manufacturer::akai::mpd226::DeviceHeader;
    use midilab::manufacturer::akai::mpd226::DeviceStatus;
    use midilab::manufacturer::akai::mpd226::dump_global_from_device;
    use midilab::manufacturer::akai::mpd226::dump_preset_from_device;
    use midilab::manufacturer::akai::mpd226::write_global_param_to_device;

    use super::*;

    #[test]
    fn test_sim_update_write_preset() {
        let mut sim = Mpd226Sim::default();
        let preset = Preset::default();
        let cmd = DeviceCommandId::WritePreset;
        let sysex = preset.as_sysex_write();

        // Verify as_sysex_write produces the expected header before feeding to sim
        let payload: DeviceMessagePayload<DeviceCommandId> =
            DeviceMessagePayload::try_from(sysex.clone()).unwrap();
        assert_eq!(
            payload.header,
            DeviceHeader {
                cmd,
                message_length: 1075
            }
        );

        let effect = sim.update(SimMsg::SysexReceived(sysex));

        let response = match effect {
            SimEffect::SendSysex(s) => s,
            SimEffect::Noop => panic!("expected SendSysex"),
        };

        let status = DeviceStatus::try_from(response.as_bytes().as_slice()).unwrap();
        match status {
            DeviceStatus::ReceivedPresetAck(ack) => {
                assert_eq!(ack.slot, preset.settings.slot)
            }
            _ => panic!("expected ReceivedPresetAck"),
        }

        // Verify the preset was stored in sim state
        let slot = preset.settings.slot;
        assert_eq!(sim.presets[slot as usize].as_bytes(), preset.as_bytes());
    }

    #[test]
    fn test_sim_update_dump_preset() {
        let mut sim = Mpd226Sim::default();
        let request_bytes = dump_preset_from_device(PresetSlot::RAM as u8);
        let sysex = Sysex::try_from(request_bytes.as_slice()).unwrap();

        let effect = sim.update(SimMsg::SysexReceived(sysex));

        let response = match effect {
            SimEffect::SendSysex(s) => s,
            _ => panic!("expected SendSysex"),
        };

        let status = DeviceStatus::try_from(response.as_bytes().as_slice()).unwrap();
        assert!(matches!(status, DeviceStatus::PresetData(_)));
    }

    #[test]
    fn test_sim_update_dump_global() {
        let mut sim = Mpd226Sim::default();
        let request_bytes = dump_global_from_device();
        let sysex = Sysex::try_from(request_bytes.as_slice()).unwrap();

        let effect = sim.update(SimMsg::SysexReceived(sysex));

        let response = match effect {
            SimEffect::SendSysex(s) => s,
            _ => panic!("expected SendSysex"),
        };

        let status = DeviceStatus::try_from(response.as_bytes().as_slice()).unwrap();
        assert!(matches!(status, DeviceStatus::GlobalData(_)));
    }

    #[test]
    fn test_sim_update_write_global() {
        use midilab::manufacturer::akai::mpd226::GlobalParamCmdId;

        let mut sim = Mpd226Sim::default();
        let addr = GlobalParamCmdId::LcdContrast as u8;
        let value = 75u8;
        let request_bytes = write_global_param_to_device(addr, value);
        let sysex = Sysex::try_from(request_bytes.as_slice()).unwrap();

        let effect = sim.update(SimMsg::SysexReceived(sysex));

        let response = match effect {
            SimEffect::SendSysex(s) => s,
            _ => panic!("expected SendSysex"),
        };

        let status = DeviceStatus::try_from(response.as_bytes().as_slice()).unwrap();
        match status {
            DeviceStatus::GlobalParamAck(ack) => assert_eq!(ack.addr as u8, addr),
            _ => panic!("expected GlobalParamAck"),
        }

        // Verify the global param was stored in sim state
        assert_eq!(sim.global.lcd_contrast, value);
    }
}
