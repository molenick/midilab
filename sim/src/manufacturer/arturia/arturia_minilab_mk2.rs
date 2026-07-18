use midi_io::Client;
use midi_io::SysEx;
use midilab::manufacturer::arturia::minilab_mk2::DeviceStatus;
use midilab::manufacturer::arturia::minilab_mk2::Global;
use midilab::manufacturer::arturia::minilab_mk2::OpCode;
use midilab::manufacturer::arturia::minilab_mk2::ParamStore;
use midilab::manufacturer::arturia::minilab_mk2::Preset;
use midilab::manufacturer::arturia::minilab_mk2::SYSEX_COMMAND_HEADER;
use midilab::manufacturer::arturia::minilab_mk2::identity_reply_message;
use midilab::manufacturer::arturia::minilab_mk2::write_value_message;
use midilab::sysex::SysExPreview;
use thiserror::Error;
use tokio::sync::oneshot;

const TOTAL_MEMORIES: usize = 8;
const SIM_FIRMWARE: [u8; 4] = [0x01, 0x00, 0x02, 0x05];
const IDENTITY_REQUEST_PAYLOAD: [u8; 4] = [0x7E, 0x7F, 0x06, 0x01];

pub enum SimMsg {
    SysexReceived(SysEx),
}

pub enum SimEffect {
    SendSysex(SysEx),
    Noop,
}

pub struct MinilabMk2Sim {
    working: ParamStore,
    memories: [ParamStore; TOTAL_MEMORIES],
}

impl Default for MinilabMk2Sim {
    fn default() -> Self {
        let working = seeded_store();
        let memories = std::array::from_fn(|_| working.clone());

        Self { working, memories }
    }
}

fn seeded_store() -> ParamStore {
    let mut store = ParamStore::default();

    let messages = Preset::default()
        .send_messages()
        .into_iter()
        .chain(Global::default().send_messages());

    for message in messages {
        let status =
            DeviceStatus::try_from(message).expect("default preset/global messages must parse");
        store.apply(&status);
    }

    store
}

impl MinilabMk2Sim {
    #[must_use]
    pub fn update(&mut self, msg: SimMsg) -> SimEffect {
        match msg {
            SimMsg::SysexReceived(sysex) => self.handle_sysex(&sysex),
        }
    }

    fn handle_sysex(&mut self, sysex: &SysEx) -> SimEffect {
        let payload = sysex.bytes();

        if payload == IDENTITY_REQUEST_PAYLOAD {
            println!("decoded identity request");
            return SimEffect::SendSysex(identity_reply_message(SIM_FIRMWARE));
        }

        if !payload.starts_with(&SYSEX_COMMAND_HEADER) {
            eprintln!("unknown sysex header: {}", sysex.preview());
            return SimEffect::Noop;
        }

        let body = &payload[SYSEX_COMMAND_HEADER.len()..];
        let Some(op) = body.first().copied().and_then(|b| OpCode::try_from(b).ok()) else {
            eprintln!("unknown op in sysex: {}", sysex.preview());
            return SimEffect::Noop;
        };

        match op {
            OpCode::ReadParam => {
                let [_, _, param, control] = body else {
                    eprintln!("malformed read: {}", sysex.preview());
                    return SimEffect::Noop;
                };

                match self.working.get(*param, *control) {
                    Some(value) => {
                        println!("read param {param:#04x} control {control:#04x} -> {value}");
                        SimEffect::SendSysex(write_value_message(*param, *control, value))
                    }
                    None => {
                        eprintln!("read of unknown param {param:#04x} control {control:#04x}");
                        SimEffect::Noop
                    }
                }
            }
            OpCode::WriteParam => {
                let [_, _, param, control, value] = body else {
                    eprintln!("malformed write: {}", sysex.preview());
                    return SimEffect::Noop;
                };

                println!("write param {param:#04x} control {control:#04x} <- {value}");
                self.working.set(*param, *control, *value);
                SimEffect::Noop
            }
            OpCode::RecallMemory => {
                let Some(index) = memory_index(body) else {
                    eprintln!("malformed recall: {}", sysex.preview());
                    return SimEffect::Noop;
                };

                println!("recall memory slot {}", index + 1);
                self.working = self.memories[index].clone();
                SimEffect::Noop
            }
            OpCode::StoreMemory => {
                let Some(index) = memory_index(body) else {
                    eprintln!("malformed store: {}", sysex.preview());
                    return SimEffect::Noop;
                };

                println!("store memory slot {}", index + 1);
                self.memories[index] = self.working.clone();
                SimEffect::Noop
            }
        }
    }
}

fn memory_index(body: &[u8]) -> Option<usize> {
    let [_, slot] = body else {
        return None;
    };

    let index = (*slot as usize).checked_sub(1)?;
    (index < TOTAL_MEMORIES).then_some(index)
}

#[derive(Debug, Error)]
pub enum SimRunnerError {
    #[error("MIDI initialization error: {0}")]
    MidiInit(#[from] midi_io::Error),
}

pub struct SimRunner {
    sim: MinilabMk2Sim,
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

        let client = Client::new("MiniLab mkII Sim").await?;
        let out_port = client.create_virtual_source(port_name).await?;
        let in_port = client.create_virtual_destination(port_name).await?;
        let sysex_in = in_port.into_sysex();

        let runner = SimRunner {
            sim: MinilabMk2Sim::default(),
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
                    let sysex_in = timed.payload;
                    println!("received sysex payload: {}", sysex_in.preview());

                    let effect = self.sim.update(SimMsg::SysexReceived(sysex_in));

                    match effect {
                        SimEffect::SendSysex(sysex_out) => {
                            println!("sending sysex payload: {}", sysex_out.preview());

                            if let Err(e) = self.out_port.send_sysex(&sysex_out).await {
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

#[cfg(test)]
mod tests {
    use midilab::manufacturer::arturia::minilab_mk2::GlobalParamId;
    use midilab::manufacturer::arturia::minilab_mk2::ParamId;
    use midilab::manufacturer::arturia::minilab_mk2::control::ControlId;
    use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::KnobAcceleration;
    use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::MemorySlot;
    use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::PadColor;
    use midilab::manufacturer::arturia::minilab_mk2::identity_request_message;
    use midilab::manufacturer::arturia::minilab_mk2::read_global_message;
    use midilab::manufacturer::arturia::minilab_mk2::read_param_message;
    use midilab::manufacturer::arturia::minilab_mk2::recall_memory_message;
    use midilab::manufacturer::arturia::minilab_mk2::store_memory_message;
    use midilab::manufacturer::arturia::minilab_mk2::write_global_message;
    use midilab::manufacturer::arturia::minilab_mk2::write_param_message;

    use super::*;

    fn sim_request(sim: &mut MinilabMk2Sim, sysex: SysEx) -> SimEffect {
        sim.update(SimMsg::SysexReceived(sysex))
    }

    fn expect_status(effect: SimEffect) -> DeviceStatus {
        match effect {
            SimEffect::SendSysex(s) => DeviceStatus::try_from(s).unwrap(),
            SimEffect::Noop => panic!("expected SendSysex"),
        }
    }

    fn read_full_preset(sim: &mut MinilabMk2Sim) -> Preset {
        let mut store = ParamStore::default();
        for message in Preset::read_messages() {
            let status = expect_status(sim_request(sim, message));
            store.apply(&status);
        }
        store.try_into_preset().unwrap()
    }

    #[test]
    fn test_sim_identity_request() {
        let mut sim = MinilabMk2Sim::default();

        let status = expect_status(sim_request(&mut sim, identity_request_message()));

        match status {
            DeviceStatus::IdentityReply(reply) => assert_eq!(reply.firmware, SIM_FIRMWARE),
            _ => panic!("expected IdentityReply"),
        }
    }

    #[test]
    fn test_sim_read_default_param() {
        let mut sim = MinilabMk2Sim::default();

        let status = expect_status(sim_request(
            &mut sim,
            read_param_message(ParamId::Data1, ControlId::Knob1),
        ));

        match status {
            DeviceStatus::ParamValue(pv) => {
                assert_eq!(pv.param, ParamId::Data1);
                assert_eq!(pv.control, ControlId::Knob1);
                assert_eq!(pv.value, 102);
            }
            _ => panic!("expected ParamValue"),
        }
    }

    #[test]
    fn test_sim_write_then_read_param() {
        let mut sim = MinilabMk2Sim::default();

        let effect = sim_request(
            &mut sim,
            write_param_message(ParamId::PadColor, ControlId::Pad3, PadColor::Purple.into()),
        );
        assert!(matches!(effect, SimEffect::Noop));

        let status = expect_status(sim_request(
            &mut sim,
            read_param_message(ParamId::PadColor, ControlId::Pad3),
        ));

        match status {
            DeviceStatus::ParamValue(pv) => assert_eq!(pv.value, PadColor::Purple.into()),
            _ => panic!("expected ParamValue"),
        }
    }

    #[test]
    fn test_sim_full_preset_round_trip() {
        let mut sim = MinilabMk2Sim::default();

        assert_eq!(read_full_preset(&mut sim), Preset::default());

        let mut mutated = Preset::default();
        mutated.knobs.knobs[4].cc = 30.into();
        mutated.pads.pads[9].color = PadColor::Cyan;

        for message in mutated.send_messages() {
            let effect = sim_request(&mut sim, message);
            assert!(matches!(effect, SimEffect::Noop));
        }

        assert_eq!(read_full_preset(&mut sim), mutated);
    }

    #[test]
    fn test_sim_global_round_trip() {
        let mut sim = MinilabMk2Sim::default();

        let effect = sim_request(
            &mut sim,
            write_global_message(
                GlobalParamId::KnobAcceleration,
                KnobAcceleration::Fast.into(),
            ),
        );
        assert!(matches!(effect, SimEffect::Noop));

        let mut store = ParamStore::default();
        for message in Global::read_messages() {
            let status = expect_status(sim_request(&mut sim, message));
            store.apply(&status);
        }

        let global = store.try_into_global().unwrap();
        assert_eq!(global.knob_acceleration, KnobAcceleration::Fast);
    }

    #[test]
    fn test_sim_store_and_recall_memory() {
        let mut sim = MinilabMk2Sim::default();

        let mut first = Preset::default();
        first.knobs.knobs[0].cc = 11.into();
        for message in first.send_messages() {
            let _ = sim_request(&mut sim, message);
        }

        let _ = sim_request(&mut sim, store_memory_message(MemorySlot::Slot2));

        let mut second = Preset::default();
        second.knobs.knobs[0].cc = 22.into();
        for message in second.send_messages() {
            let _ = sim_request(&mut sim, message);
        }
        assert_eq!(read_full_preset(&mut sim), second);

        let _ = sim_request(&mut sim, recall_memory_message(MemorySlot::Slot2));
        assert_eq!(read_full_preset(&mut sim), first);
    }

    #[test]
    fn test_sim_read_of_global_marker_routes_to_global() {
        let mut sim = MinilabMk2Sim::default();

        let status = expect_status(sim_request(
            &mut sim,
            read_global_message(GlobalParamId::KeyboardChannel),
        ));

        assert!(matches!(status, DeviceStatus::GlobalValue(_)));
    }
}
