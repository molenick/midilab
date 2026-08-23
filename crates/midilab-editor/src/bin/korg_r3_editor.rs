use std::time::Duration;

use eframe::egui::ViewportBuilder;
use midi_io::Client;
use midi_io::DestinationConnection;
use midi_io::SysEx;
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
use midilab_editor::korg_r3::KorgR3Editor;
use midilab_editor::korg_r3::app::AppState;
use midilab_editor::korg_r3::config::AppConfig;
use midilab_editor::korg_r3::fs::load_app_config;
use midilab_editor::korg_r3::fs::load_formant_motion_from_file;
use midilab_editor::korg_r3::fs::load_global_from_file;
use midilab_editor::korg_r3::fs::load_program_from_file;
use midilab_editor::korg_r3::fs::persist_config;
use midilab_editor::korg_r3::fs::persist_user_settings;
use midilab_editor::korg_r3::fs::save_formant_motion;
use midilab_editor::korg_r3::fs::save_global;
use midilab_editor::korg_r3::fs::save_program;
use midilab_editor::korg_r3::message::AppEffect;
use midilab_editor::korg_r3::message::AppMsg;
use midilab_editor::korg_r3::message::DeviceMsg;
use midilab_editor::korg_r3::message::IoEffect;
use midilab_editor::korg_r3::message::IoMsg;
use midilab_editor::korg_r3::message::UserError;
use midilab_io::midi::find_input_port;
use midilab_io::midi::find_output_port;
use midilab_io::midi::recv_device;
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

        let client = Client::new("korg_r3")
            .await
            .expect("failed to init MIDI client");

        let (dev_tx, mut dev_rx) = unbounded_channel::<SysEx>();
        let mut output: Option<DestinationConnection> = connect_output(&client).await.ok();
        let mut input = connect_input(&client, dev_tx.clone()).await.ok();

        let mut pending: HashMap<(u16, u16), u16> = HashMap::new();
        let mut flush_at: Option<Instant> = None;
        const DEBOUNCE: Duration = Duration::from_millis(8);
        const IDLE_TICK: Duration = Duration::from_millis(50);

        loop {
            while let Ok(sysex) = dev_rx.try_recv() {
                if let Ok(kmsg) = KorgR3Message::try_from(&sysex) {
                    let _ = midi_app_tx.send(AppMsg::Device(kmsg));
                }
            }
            if flush_at.is_some_and(|at| Instant::now() >= at) {
                if let Some(out) = output.as_ref() {
                    for ((id, sub), value) in pending.drain() {
                        let lp = LiveParam {
                            addr: ParamAddr { id, sub },
                            value,
                        };
                        let _ = send(out, &lp.to_sysex(0x00)).await;
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
                match connect_output(&client).await {
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
                input = connect_input(&client, dev_tx.clone()).await.ok();
            }
            let out = output.as_ref().unwrap();

            let result = handle_midi_msg(msg, out, &mut dev_rx).await;

            let msg = match result {
                Ok(Some(sysex)) => match KorgR3Message::try_from(&sysex) {
                    Ok(kmsg) => AppMsg::Device(kmsg),
                    Err(_) => continue,
                },
                Ok(None) => continue,
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

async fn connect_output(client: &Client) -> Result<DestinationConnection, String> {
    let port = find_output_port(client, PORT_SOUND)
        .await
        .ok_or_else(|| format!("R3 not found (no '{PORT_SOUND}' output) - is it connected?"))?;
    client
        .connect_destination(&port)
        .await
        .map_err(|e| format!("Failed to connect to MIDI output: {}", e))
}

async fn connect_input(client: &Client, tx: UnboundedSender<SysEx>) -> Result<(), String> {
    let port = find_input_port(client, PORT_KBD_KNOB)
        .await
        .ok_or_else(|| format!("R3 not found (no '{PORT_KBD_KNOB}' input) - is it connected?"))?;
    let conn = client
        .connect_source(&port)
        .await
        .map_err(|e| format!("Failed to connect to MIDI input: {}", e))?;

    tokio::spawn(async move {
        let mut sysex = conn.into_sysex();
        while let Some(timed) = sysex.recv().await {
            let _ = tx.send(timed.payload);
        }
    });

    Ok(())
}

async fn send(output: &DestinationConnection, sysex: &SysEx) -> Result<(), ()> {
    output.send_sysex(sysex).await.map_err(|_| ())
}

async fn handle_midi_msg(
    msg: DeviceMsg,
    output: &DestinationConnection,
    rx: &mut UnboundedReceiver<SysEx>,
) -> Result<Option<SysEx>, MidiError> {
    match msg {
        DeviceMsg::DumpCurrentProgram => {
            let request = current_program_dump_request(0x00);
            send(output, &request)
                .await
                .map_err(|_| MidiError::DumpPreset)?;

            let sysex = recv_device(rx, Duration::from_secs(3)).await?;
            Ok(Some(sysex))
        }
        DeviceMsg::DumpProgram(slot) => {
            let request = midilab::manufacturer::korg::r3::program_dump_request(0x00, slot as u16);
            send(output, &request)
                .await
                .map_err(|_| MidiError::DumpPreset)?;

            let sysex = recv_device(rx, Duration::from_secs(3)).await?;
            Ok(Some(sysex))
        }
        DeviceMsg::DumpSlot(slot) => {
            let request =
                midilab::manufacturer::korg::r3::program_dump_request(0x00, slot.as_u16());
            send(output, &request)
                .await
                .map_err(|_| MidiError::DumpPreset)?;

            let sysex = recv_device(rx, Duration::from_secs(3)).await?;
            Ok(Some(sysex))
        }
        DeviceMsg::DumpGlobal => {
            let request = global_dump_request(0x00);
            send(output, &request)
                .await
                .map_err(|_| MidiError::DumpPreset)?;

            let sysex = recv_device(rx, Duration::from_secs(3)).await?;
            Ok(Some(sysex))
        }
        DeviceMsg::DumpCurrentFormantMotion => {
            let request = current_formant_motion_dump_request(0x00);
            send(output, &request)
                .await
                .map_err(|_| MidiError::DumpPreset)?;

            let sysex = recv_device(rx, Duration::from_secs(3)).await?;
            Ok(Some(sysex))
        }
        DeviceMsg::DumpFormantMotion(motion_no) => {
            let request = formant_motion_dump_request(0x00, motion_no);
            send(output, &request)
                .await
                .map_err(|_| MidiError::DumpPreset)?;

            let sysex = recv_device(rx, Duration::from_secs(3)).await?;
            Ok(Some(sysex))
        }
        DeviceMsg::WriteProgram { program, slot } => {
            let raw: RawProgram = (&*program).into();
            do_write_program(output, rx, &raw, slot as u16).await?;
            Ok(None)
        }
        DeviceMsg::WriteSelectedProgram { program, slot } => {
            let raw: RawProgram = (&*program).into();
            do_write_program(output, rx, &raw, slot.as_u16()).await?;
            Ok(None)
        }
        DeviceMsg::WriteFormantMotion { motion, motion_no } => {
            do_write_formant_motion(output, rx, &motion, motion_no).await?;
            Ok(None)
        }
        DeviceMsg::LiveParams(_) => Ok(None),
    }
}

async fn do_write_program(
    output: &DestinationConnection,
    rx: &mut UnboundedReceiver<SysEx>,
    program: &RawProgram,
    slot: u16,
) -> Result<(), MidiError> {
    while let Ok(_v) = rx.try_recv() {}

    send(output, &current_program_dump_message(0x00, program))
        .await
        .map_err(|_| MidiError::WritePreset)?;
    wait_for_ack(rx).await?;

    send(output, &program_write_request(0x00, slot))
        .await
        .map_err(|_| MidiError::WritePreset)?;
    wait_for_ack(rx).await?;

    Ok(())
}

async fn do_write_formant_motion(
    output: &DestinationConnection,
    rx: &mut UnboundedReceiver<SysEx>,
    motion: &midilab::manufacturer::korg::r3::wrappers::FormantMotion,
    motion_no: u8,
) -> Result<(), MidiError> {
    while let Ok(_v) = rx.try_recv() {}

    send(
        output,
        &current_formant_motion_dump_message(0x00, &motion.to_raw()),
    )
    .await
    .map_err(|_| MidiError::WritePreset)?;
    wait_for_ack(rx).await?;

    send(output, &formant_motion_write_request(0x00, motion_no))
        .await
        .map_err(|_| MidiError::WritePreset)?;
    wait_for_ack(rx).await?;

    Ok(())
}

async fn wait_for_ack(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<SysEx>,
) -> Result<(), MidiError> {
    match recv_device(rx, Duration::from_secs(10)).await {
        Ok(sysex) => match KorgR3Message::try_from(&sysex) {
            Ok(KorgR3Message::DataLoadCompleted) | Ok(KorgR3Message::WriteCompleted) => Ok(()),
            _ => Ok(()),
        },
        Err(MidiError::ResponseTimeout) => Ok(()),
        Err(e) => Err(e),
    }
}
