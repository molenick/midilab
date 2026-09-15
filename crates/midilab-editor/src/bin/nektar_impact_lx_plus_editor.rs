use std::time::Duration;

use eframe::egui::ViewportBuilder;
use midi_io::Client;
use midi_io::SysEx;
use midilab::manufacturer::nektar::impact_lx_plus::DeviceStatus;
use midilab::manufacturer::nektar::impact_lx_plus::DumpAssembler;
use midilab::manufacturer::nektar::impact_lx_plus::is_impact_lx_plus_sysex;
use midilab_editor::nektar_impact_lx_plus::ImpactLxPlusEditor;
use midilab_editor::nektar_impact_lx_plus::app::AppState;
use midilab_editor::nektar_impact_lx_plus::config::AppConfig;
use midilab_editor::nektar_impact_lx_plus::fs::load_app_config;
use midilab_editor::nektar_impact_lx_plus::fs::load_dump_from_file;
use midilab_editor::nektar_impact_lx_plus::fs::persist_config;
use midilab_editor::nektar_impact_lx_plus::fs::save_dump;
use midilab_editor::nektar_impact_lx_plus::message::AppEffect;
use midilab_editor::nektar_impact_lx_plus::message::AppMsg;
use midilab_editor::nektar_impact_lx_plus::message::DeviceEvent;
use midilab_editor::nektar_impact_lx_plus::message::DeviceMsg;
use midilab_editor::nektar_impact_lx_plus::message::IoEffect;
use midilab_editor::nektar_impact_lx_plus::message::IoMsg;
use midilab_editor::nektar_impact_lx_plus::message::UserError;
use midilab_io::midi::Link;
use midilab_io::midi::Listener;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;

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
                IoMsg::SaveDump { dump, path } => IoEffect::DumpSaveResult(
                    save_dump(*dump, &path).await.map_err(|e| e.to_string()),
                ),
                IoMsg::LoadDump { path } => IoEffect::DumpLoadResult(
                    load_dump_from_file(&path)
                        .await
                        .map(Box::new)
                        .map_err(|e| e.to_string()),
                ),
                IoMsg::PersistConfig { config, path } => IoEffect::PersistConfigResult(
                    persist_config(config, &path)
                        .await
                        .map_err(|e| e.to_string()),
                ),
            };

            io_app_tx.send(AppMsg::Io(Box::new(effect))).unwrap();
        }
    });

    let _midi = tokio::spawn(async move {
        let client = Client::new("impact_lx_plus")
            .await
            .expect("failed to init MIDI client");

        let mut listener = listen_for_dumps(&client, &midi_app_tx).await;

        while let Some(msg) = midi_rx.recv().await {
            if matches!(msg, DeviceMsg::Reconnect) {
                drop(listener);
                listener = listen_for_dumps(&client, &midi_app_tx).await;
            }

            let msg = handle_midi_msg(msg, &client).await;
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
            .with_inner_size(eframe::egui::vec2(980., 720.))
            .with_min_inner_size(eframe::egui::vec2(400., 300.)),
        ..Default::default()
    };

    let config = load_app_config(&AppConfig::config_path().unwrap_or_default())
        .await
        .unwrap_or_default();

    eframe::run_native(
        "Nektar Impact LX+ Editor",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(ImpactLxPlusEditor::new(
                app_tx,
                ui_rx,
                config.into(),
            )))
        }),
    )?;

    Ok(())
}

/// Listens on every source for panel-triggered memory dumps.
///
/// The LX+ has no dump-request sysex, so every incoming message is folded
/// into a [`DumpAssembler`] per port; when all 182 messages have arrived, the
/// assembled dump is delivered to the app. Sysex from other devices on the
/// bus is skipped.
async fn listen_for_dumps(client: &Client, app_tx: &UnboundedSender<AppMsg>) -> Listener {
    Listener::open(client, || {
        let app_tx = app_tx.clone();
        let mut assembler = DumpAssembler::default();
        move |payload: SysEx| {
            if !is_impact_lx_plus_sysex(&payload) {
                return;
            }
            let Ok(status) = DeviceStatus::try_from(payload) else {
                return;
            };

            if assembler.is_empty() {
                let _ = app_tx.send(AppMsg::Device(DeviceEvent::DumpStarted));
            }
            assembler.apply(&status);

            if assembler.is_complete() {
                match std::mem::take(&mut assembler).try_into_dump() {
                    Ok(dump) => {
                        let _ =
                            app_tx.send(AppMsg::Device(DeviceEvent::DumpReceived(Box::new(dump))));
                    }
                    Err(e) => {
                        let _ = app_tx.send(AppMsg::UserError(UserError::Parse(e.to_string())));
                    }
                }
            }
        }
    })
    .await
}

/// Handles a device message, sending writes over a [`Link`] opened for it
/// and returning the app message to report.
///
/// Writes report success when delivered to the outputs.
async fn handle_midi_msg(msg: DeviceMsg, client: &Client) -> AppMsg {
    let (messages, written) = match msg {
        DeviceMsg::WriteDump(dump) => (dump.to_messages(), DeviceEvent::DumpWritten),
        DeviceMsg::WritePreset { id, preset } => {
            (preset.send_messages(id), DeviceEvent::PresetWritten(id))
        }
        DeviceMsg::WritePadMap { id, map } => {
            (map.send_messages(id), DeviceEvent::PadMapWritten(id))
        }
        DeviceMsg::WriteGlobalSettings(settings) => {
            (settings.send_messages(), DeviceEvent::GlobalSettingsWritten)
        }
        DeviceMsg::WriteGlobalControls(controls) => {
            (controls.send_messages(), DeviceEvent::GlobalControlsWritten)
        }
        DeviceMsg::Reconnect => return AppMsg::Device(DeviceEvent::Reconnected),
    };

    let link = Link::open(client).await;
    if link.output_count() == 0 {
        return AppMsg::MidiStatus("no MIDI output ports - not sent".to_string());
    }

    match link.send_paced(messages, WRITE_PACING).await {
        Ok(()) => AppMsg::Device(written),
        Err(e) => AppMsg::UserError(UserError::Midi(e)),
    }
}
