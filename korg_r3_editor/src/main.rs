use std::time::Duration;

use eframe::egui::ViewportBuilder;
use korg_r3_editor::KorgR3Editor;
use korg_r3_editor::app::AppState;
use korg_r3_editor::config::AppConfig;
use korg_r3_editor::fs::load_app_config;
use korg_r3_editor::fs::load_formant_motion_from_file;
use korg_r3_editor::fs::load_global_from_file;
use korg_r3_editor::fs::load_program_from_file;
use korg_r3_editor::fs::persist_config;
use korg_r3_editor::fs::persist_user_settings;
use korg_r3_editor::fs::save_formant_motion;
use korg_r3_editor::fs::save_global;
use korg_r3_editor::fs::save_program;
use korg_r3_editor::message::AppEffect;
use korg_r3_editor::message::AppMsg;
use korg_r3_editor::message::DeviceMsg;
use korg_r3_editor::message::IoEffect;
use korg_r3_editor::message::IoMsg;
use korg_r3_editor::message::UserError;
use midilab::error::MidiError;
use midilab::manufacturer::korg::r3::KorgR3Message;
use midilab::manufacturer::korg::r3::PORT_KBD_KNOB;
use midilab::manufacturer::korg::r3::PORT_SOUND;
use midilab::manufacturer::korg::r3::current_formant_motion_dump_message;
use midilab::manufacturer::korg::r3::current_formant_motion_dump_request;
use midilab::manufacturer::korg::r3::current_program_dump_message;
use midilab::manufacturer::korg::r3::current_program_dump_request;
use midilab::manufacturer::korg::r3::formant_motion_dump_request;
use midilab::manufacturer::korg::r3::formant_motion_write_request;
use midilab::manufacturer::korg::r3::global_dump_request;
use midilab::manufacturer::korg::r3::live::LiveParam;
use midilab::manufacturer::korg::r3::live::ParamAddr;
use midilab::manufacturer::korg::r3::program_write_request;
use midilab::manufacturer::korg::r3::raw::RawProgram;
use midilab_io::midi::find_input_port;
use midilab_io::midi::find_output_port;
use midilab_io::midi::flush_coremidi_notifications;
use midilab_io::midi::recv_device_bytes;
use midir::MidiInput;
use midir::MidiOutput;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (app_tx, mut app_rx) = unbounded_channel();
    let (ui_tx, ui_rx) = unbounded_channel();
    let (midi_tx, mut midi_rx) = unbounded_channel();
    let (io_tx, mut io_rx) = unbounded_channel::<IoMsg>();
    let io_app_tx = app_tx.clone();
    let midi_app_tx = app_tx.clone();

    let _io = tokio::spawn(async move {
        while let Some(msg) = io_rx.recv().await {
            let effect = match msg {
                IoMsg::SaveProgram { program, path } => IoEffect::ProgramSaveResult(
                    save_program(*program, &path)
                        .await
                        .map_err(|e| e.to_string()),
                ),
                IoMsg::LoadProgram { path } => IoEffect::ProgramLoadResult(
                    load_program_from_file(&path)
                        .await
                        .map(Box::new)
                        .map_err(|e| e.to_string()),
                ),
                IoMsg::SaveGlobal { global, path } => IoEffect::GlobalSaveResult(
                    save_global(global, &path).await.map_err(|e| e.to_string()),
                ),
                IoMsg::LoadGlobal { path } => IoEffect::GlobalLoadResult(
                    load_global_from_file(&path)
                        .await
                        .map_err(|e| e.to_string()),
                ),
                IoMsg::SaveFormantMotion { motion, path } => IoEffect::FormantMotionSaveResult(
                    save_formant_motion(motion, &path)
                        .await
                        .map_err(|e| e.to_string()),
                ),
                IoMsg::LoadFormantMotion { path } => IoEffect::FormantMotionLoadResult(
                    load_formant_motion_from_file(&path)
                        .await
                        .map_err(|e| e.to_string()),
                ),
                IoMsg::PersistConfig { config, path } => IoEffect::PersistConfigResult(
                    persist_config(config, &path)
                        .await
                        .map_err(|e| e.to_string()),
                ),
                IoMsg::PersistUserSettings { config, path } => IoEffect::PersistUserSettingsResult(
                    persist_user_settings(config.clone(), &path)
                        .await
                        .map_err(|e| e.to_string()),
                ),
            };

            io_app_tx.send(AppMsg::Io(Box::new(effect))).unwrap();
        }
    });

    let _midi = tokio::spawn(async move {
        use std::collections::HashMap;
        use std::time::Instant;

        let (dev_tx, mut dev_rx) = unbounded_channel::<Vec<u8>>();
        let mut output: Option<midir::MidiOutputConnection> = connect_output().ok();
        let mut input = connect_input(dev_tx.clone()).ok();

        let mut pending: HashMap<(u16, u16), u16> = HashMap::new();
        let mut flush_at: Option<Instant> = None;
        const DEBOUNCE: Duration = Duration::from_millis(8);
        const IDLE_TICK: Duration = Duration::from_millis(50);

        loop {
            while let Ok(bytes) = dev_rx.try_recv() {
                if let Ok(kmsg) = KorgR3Message::try_from(bytes.as_slice()) {
                    let _ = midi_app_tx.send(AppMsg::Device(kmsg));
                }
            }
            if flush_at.is_some_and(|at| Instant::now() >= at) {
                if let Some(out) = output.as_mut() {
                    for ((id, sub), value) in pending.drain() {
                        let lp = LiveParam {
                            addr: ParamAddr { id, sub },
                            value,
                        };
                        let _ = out.send(&lp.to_sysex(0x00));
                    }
                } else {
                    pending.clear();
                }
                flush_at = None;
            }

            let wait = match flush_at {
                Some(at) => at.saturating_duration_since(Instant::now()).min(IDLE_TICK),
                None => IDLE_TICK,
            };
            let msg = match tokio::time::timeout(wait, midi_rx.recv()).await {
                Ok(Some(m)) => m,
                Ok(None) => break,
                Err(_) => continue,
            };

            if let DeviceMsg::LiveParams(params) = msg {
                for p in &params {
                    pending.insert((p.addr.id, p.addr.sub), p.value);
                }
                flush_at.get_or_insert_with(|| Instant::now() + DEBOUNCE);
                continue;
            }

            if output.is_none() {
                match connect_output() {
                    Ok(o) => output = Some(o),
                    Err(e) => {
                        let _ = midi_app_tx.send(AppMsg::UserError(UserError::Midi(
                            MidiError::OutputConnection(e),
                        )));
                        continue;
                    }
                }
            }
            if input.is_none() {
                input = connect_input(dev_tx.clone()).ok();
            }
            let out = output.as_mut().unwrap();

            let result: Result<Vec<u8>, MidiError> = handle_midi_msg(msg, out, &mut dev_rx).await;

            let msg = match result {
                Ok(bytes) => match KorgR3Message::try_from(bytes.as_slice()) {
                    Ok(kmsg) => AppMsg::Device(kmsg),
                    Err(_) => continue,
                },
                Err(e) => AppMsg::UserError(UserError::Midi(e)),
            };
            let _ = midi_app_tx.send(msg);
        }
    });

    let mut app_state = AppState::new(AppConfig::default());

    let app_ui_tx = ui_tx.clone();
    let _app = tokio::spawn(async move {
        while let Some(msg) = app_rx.recv().await {
            let effects = app_state.update(msg);

            for effect in effects {
                match effect {
                    AppEffect::Ui(ui_msg) => {
                        let _ = app_ui_tx.send(ui_msg);
                    }
                    AppEffect::Device(device_msg) => {
                        let _ = midi_tx.send(device_msg);
                    }
                    AppEffect::Io(io_msg) => {
                        let _ = io_tx.send(*io_msg);
                    }
                }
            }
        }
    });

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(eframe::egui::vec2(900., 700.))
            .with_min_inner_size(eframe::egui::vec2(400., 300.)),
        ..Default::default()
    };

    let config = load_app_config(&AppConfig::config_path().unwrap_or_default())
        .await
        .unwrap_or_default();

    eframe::run_native(
        "Korg R3 Editor",
        options,
        Box::new(move |_cc| Ok(Box::new(KorgR3Editor::new(app_tx, ui_rx, config.into())))),
    )?;

    Ok(())
}

fn connect_output() -> Result<midir::MidiOutputConnection, String> {
    flush_coremidi_notifications();
    let midi_out =
        MidiOutput::new("korg_r3").map_err(|e| format!("Failed to create MIDI output: {}", e))?;

    let port = find_output_port(&midi_out, PORT_SOUND)
        .ok_or_else(|| format!("R3 not found (no '{PORT_SOUND}' output) - is it connected?"))?;
    midi_out
        .connect(&port, PORT_SOUND)
        .map_err(|e| format!("Failed to connect to MIDI output: {}", e))
}

fn connect_input(tx: UnboundedSender<Vec<u8>>) -> Result<midir::MidiInputConnection<()>, String> {
    let midi_in = MidiInput::new("korg_r3-recv")
        .map_err(|e| format!("Failed to create MIDI input: {}", e))?;
    let port = find_input_port(&midi_in, PORT_KBD_KNOB)
        .ok_or_else(|| format!("R3 not found (no '{PORT_KBD_KNOB}' input) - is it connected?"))?;
    midi_in
        .connect(
            &port,
            PORT_KBD_KNOB,
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .map_err(|e| format!("Failed to connect to MIDI input: {}", e))
}

async fn handle_midi_msg(
    msg: DeviceMsg,
    output: &mut midir::MidiOutputConnection,
    rx: &mut UnboundedReceiver<Vec<u8>>,
) -> Result<Vec<u8>, MidiError> {
    match msg {
        DeviceMsg::DumpCurrentProgram => {
            let request = current_program_dump_request(0x00);
            output.send(&request).map_err(|_| MidiError::DumpPreset)?;

            let bytes = recv_device_bytes(rx, Duration::from_secs(3)).await?;
            Ok(bytes)
        }
        DeviceMsg::DumpProgram(slot) => {
            let request = midilab::manufacturer::korg::r3::program_dump_request(0x00, slot as u16);
            output.send(&request).map_err(|_| MidiError::DumpPreset)?;

            let bytes = recv_device_bytes(rx, Duration::from_secs(3)).await?;
            Ok(bytes)
        }
        DeviceMsg::DumpSlot(slot) => {
            let request =
                midilab::manufacturer::korg::r3::program_dump_request(0x00, slot.as_u16());
            output.send(&request).map_err(|_| MidiError::DumpPreset)?;

            let bytes = recv_device_bytes(rx, Duration::from_secs(3)).await?;
            Ok(bytes)
        }
        DeviceMsg::DumpGlobal => {
            let request = global_dump_request(0x00);
            output.send(&request).map_err(|_| MidiError::DumpPreset)?;

            let bytes = recv_device_bytes(rx, Duration::from_secs(3)).await?;
            Ok(bytes)
        }
        DeviceMsg::DumpCurrentFormantMotion => {
            let request = current_formant_motion_dump_request(0x00);
            output.send(&request).map_err(|_| MidiError::DumpPreset)?;

            let bytes = recv_device_bytes(rx, Duration::from_secs(3)).await?;
            Ok(bytes)
        }
        DeviceMsg::DumpFormantMotion(motion_no) => {
            let request = formant_motion_dump_request(0x00, motion_no);
            output.send(&request).map_err(|_| MidiError::DumpPreset)?;

            let bytes = recv_device_bytes(rx, Duration::from_secs(3)).await?;
            Ok(bytes)
        }
        DeviceMsg::WriteProgram { program, slot } => {
            let raw: RawProgram = (&*program).into();
            do_write_program(output, rx, &raw, slot as u16).await
        }
        DeviceMsg::WriteSelectedProgram { program, slot } => {
            let raw: RawProgram = (&*program).into();
            do_write_program(output, rx, &raw, slot.as_u16()).await
        }
        DeviceMsg::WriteFormantMotion { motion, motion_no } => {
            do_write_formant_motion(output, rx, &motion, motion_no).await
        }
        DeviceMsg::LiveParams(_) => Ok(vec![]),
    }
}

async fn do_write_program(
    output: &mut midir::MidiOutputConnection,
    rx: &mut UnboundedReceiver<Vec<u8>>,
    program: &RawProgram,
    slot: u16,
) -> Result<Vec<u8>, MidiError> {
    while let Ok(_v) = rx.try_recv() {}

    output
        .send(&current_program_dump_message(0x00, program))
        .map_err(|_| MidiError::WritePreset)?;
    wait_for_ack(rx).await?;

    output
        .send(&program_write_request(0x00, slot))
        .map_err(|_| MidiError::WritePreset)?;
    wait_for_ack(rx).await?;

    Ok(vec![])
}

async fn do_write_formant_motion(
    output: &mut midir::MidiOutputConnection,
    rx: &mut UnboundedReceiver<Vec<u8>>,
    motion: &midilab::manufacturer::korg::r3::wrappers::FormantMotion,
    motion_no: u8,
) -> Result<Vec<u8>, MidiError> {
    while let Ok(_v) = rx.try_recv() {}

    output
        .send(&current_formant_motion_dump_message(0x00, &motion.to_raw()))
        .map_err(|_| MidiError::WritePreset)?;
    wait_for_ack(rx).await?;

    output
        .send(&formant_motion_write_request(0x00, motion_no))
        .map_err(|_| MidiError::WritePreset)?;
    wait_for_ack(rx).await?;

    Ok(vec![])
}

async fn wait_for_ack(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
) -> Result<(), MidiError> {
    match recv_device_bytes(rx, Duration::from_secs(10)).await {
        Ok(bytes) => match KorgR3Message::try_from(bytes.as_slice()) {
            Ok(KorgR3Message::DataLoadCompleted) | Ok(KorgR3Message::WriteCompleted) => Ok(()),
            _ => Ok(()),
        },
        Err(MidiError::ResponseTimeout) => Ok(()),
        Err(e) => Err(e),
    }
}
