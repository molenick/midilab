use std::time::Duration;

use eframe::egui::ViewportBuilder;
use midi_io::Client;
use midi_io::SysEx;
use midilab::manufacturer::korg::r3::KorgR3Message;
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
use midilab::manufacturer::korg::r3::reply_to;
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
use midilab_io::midi::Link;
use midilab_io::midi::Listener;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;

/// How long to wait for a response to a dump request after sending it.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(3);

/// Pace between messages of a multi-message write. Devices consume sysex
/// serially; small gaps keep a burst from outrunning the device's parser.
const WRITE_PACING: Duration = Duration::from_millis(2);

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

        let mut listener = listen_for_panel(&client, &midi_app_tx).await;

        let mut pending: HashMap<(u16, u16), u16> = HashMap::new();
        let mut flush_at: Option<Instant> = None;
        const DEBOUNCE: Duration = Duration::from_millis(8);
        const IDLE_TICK: Duration = Duration::from_millis(50);

        loop {
            if flush_at.is_some_and(|at| Instant::now() >= at) {
                let link = Link::open(&client).await;
                for ((id, sub), value) in pending.drain() {
                    let lp = LiveParam {
                        addr: ParamAddr { id, sub },
                        value,
                    };
                    let _ = link.send(&lp.to_sysex(0x00)).await;
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

            let mut link = Link::open(&client).await;
            // Pick up sources that appeared (device or hub plugged in)
            // since the panel listener was opened.
            if link.input_count() != listener.port_count() {
                drop(listener);
                listener = listen_for_panel(&client, &midi_app_tx).await;
            }

            let Some(msg) = handle_midi_msg(msg, &mut link).await else {
                continue;
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

/// Listens on every source for PARAMETER CHANGE sent when the R3's panel
/// is edited, forwarding it to the app. Replies to operations are received
/// by the operation's own [`Link`], so everything else is skipped here.
async fn listen_for_panel(client: &Client, app_tx: &UnboundedSender<AppMsg>) -> Listener {
    Listener::open(client, || {
        let app_tx = app_tx.clone();
        move |sysex: SysEx| {
            if let Ok(kmsg @ KorgR3Message::ParameterChange(_)) = KorgR3Message::try_from(&sysex) {
                let _ = app_tx.send(AppMsg::Device(kmsg));
            }
        }
    })
    .await
}

/// Sends a dump request and waits for the reply that answers it.
///
/// Silence is a status, not an error.
async fn request_dump(link: &mut Link, request: SysEx) -> AppMsg {
    if let Err(e) = link.send(&request).await {
        return AppMsg::UserError(UserError::Midi(e));
    }

    match link.recv(RESPONSE_TIMEOUT, |s| reply_to(&request, s)).await {
        Some(kmsg) => AppMsg::Device(kmsg),
        None => AppMsg::MidiStatus("no response from device".to_string()),
    }
}

/// Handles a device message over a [`Link`] opened for it, returning the
/// app message to report (or `None` when there is nothing to report).
///
/// Writes report success when delivered to the outputs, and dumps report a
/// status when the device does not answer.
async fn handle_midi_msg(msg: DeviceMsg, link: &mut Link) -> Option<AppMsg> {
    if link.output_count() == 0 {
        return Some(AppMsg::MidiStatus(
            "no MIDI output ports - not sent".to_string(),
        ));
    }

    match msg {
        DeviceMsg::DumpCurrentProgram => {
            Some(request_dump(link, current_program_dump_request(0x00)).await)
        }
        DeviceMsg::DumpProgram(slot) => {
            let request = midilab::manufacturer::korg::r3::program_dump_request(0x00, slot as u16);
            Some(request_dump(link, request).await)
        }
        DeviceMsg::DumpSlot(slot) => {
            let request =
                midilab::manufacturer::korg::r3::program_dump_request(0x00, slot.as_u16());
            Some(request_dump(link, request).await)
        }
        DeviceMsg::DumpGlobal => Some(request_dump(link, global_dump_request(0x00)).await),
        DeviceMsg::DumpCurrentFormantMotion => {
            Some(request_dump(link, current_formant_motion_dump_request(0x00)).await)
        }
        DeviceMsg::DumpFormantMotion(motion_no) => {
            let request = formant_motion_dump_request(0x00, motion_no);
            Some(request_dump(link, request).await)
        }
        DeviceMsg::WriteProgram { program, slot } => {
            let raw: RawProgram = (&*program).into();
            let messages = [
                current_program_dump_message(0x00, &raw),
                program_write_request(0x00, slot as u16),
            ];
            match link.send_paced(messages, WRITE_PACING).await {
                Ok(()) => Some(AppMsg::MidiStatus(format!("Program sent to slot {slot}"))),
                Err(e) => Some(AppMsg::UserError(UserError::Midi(e))),
            }
        }
        DeviceMsg::WriteSelectedProgram { program, slot } => {
            let raw: RawProgram = (&*program).into();
            let messages = [
                current_program_dump_message(0x00, &raw),
                program_write_request(0x00, slot.as_u16()),
            ];
            match link.send_paced(messages, WRITE_PACING).await {
                Ok(()) => Some(AppMsg::MidiStatus(format!("Program sent to slot {slot}"))),
                Err(e) => Some(AppMsg::UserError(UserError::Midi(e))),
            }
        }
        DeviceMsg::WriteFormantMotion { motion, motion_no } => {
            let messages = [
                current_formant_motion_dump_message(0x00, &motion.to_raw()),
                formant_motion_write_request(0x00, motion_no),
            ];
            match link.send_paced(messages, WRITE_PACING).await {
                Ok(()) => Some(AppMsg::MidiStatus(format!(
                    "Formant motion {motion_no} sent"
                ))),
                Err(e) => Some(AppMsg::UserError(UserError::Midi(e))),
            }
        }
        DeviceMsg::LiveParams(_) => None,
    }
}
