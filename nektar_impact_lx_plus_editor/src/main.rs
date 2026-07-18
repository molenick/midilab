use std::time::Duration;

use eframe::egui::ViewportBuilder;
use midi_io::Client;
use midi_io::DestinationConnection;
use midi_io::SysEx;
use midilab::error::MidiError;
use midilab::manufacturer::nektar::impact_lx_plus::DeviceStatus;
use midilab::manufacturer::nektar::impact_lx_plus::DumpAssembler;
use midilab::manufacturer::nektar::impact_lx_plus::is_sysex_port;
use nektar_impact_lx_plus_editor::ImpactLxPlusEditor;
use nektar_impact_lx_plus_editor::app::AppState;
use nektar_impact_lx_plus_editor::config::AppConfig;
use nektar_impact_lx_plus_editor::fs::load_app_config;
use nektar_impact_lx_plus_editor::fs::load_dump_from_file;
use nektar_impact_lx_plus_editor::fs::persist_config;
use nektar_impact_lx_plus_editor::fs::save_dump;
use nektar_impact_lx_plus_editor::message::AppEffect;
use nektar_impact_lx_plus_editor::message::AppMsg;
use nektar_impact_lx_plus_editor::message::DeviceEvent;
use nektar_impact_lx_plus_editor::message::DeviceMsg;
use nektar_impact_lx_plus_editor::message::IoEffect;
use nektar_impact_lx_plus_editor::message::IoMsg;
use nektar_impact_lx_plus_editor::message::UserError;
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

        let mut output: Option<DestinationConnection> = connect_output(&client).await.ok();
        let mut input_connected = connect_input(&client, midi_app_tx.clone()).await.is_ok();

        while let Some(msg) = midi_rx.recv().await {
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
            if !input_connected {
                input_connected = connect_input(&client, midi_app_tx.clone()).await.is_ok();
            }
            let out = output.as_ref().unwrap();

            let msg = match handle_midi_msg(msg, out).await {
                Ok(event) => AppMsg::Device(event),
                Err(e) => AppMsg::UserError(e),
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

async fn connect_output(client: &Client) -> Result<DestinationConnection, String> {
    let destinations = client
        .destinations()
        .await
        .map_err(|e| format!("Failed to list MIDI outputs: {e}"))?;
    let port = destinations
        .into_iter()
        .find(|p| is_sysex_port(p.name()))
        .ok_or_else(|| {
            "Impact LX+ not found (no 'Impact LX... MIDI1' output) - is it connected?".to_string()
        })?;
    client
        .connect_destination(&port)
        .await
        .map_err(|e| format!("Failed to connect to MIDI output: {e}"))
}

/// Connects the sysex input and spawns a listener that assembles
/// panel-triggered memory dumps. The LX+ has no dump-request sysex, so every
/// incoming message is folded into a [`DumpAssembler`]; when all 182 messages
/// have arrived, the assembled dump is delivered to the app.
async fn connect_input(client: &Client, app_tx: UnboundedSender<AppMsg>) -> Result<(), String> {
    let sources = client
        .sources()
        .await
        .map_err(|e| format!("Failed to list MIDI inputs: {e}"))?;
    let port = sources
        .into_iter()
        .find(|p| is_sysex_port(p.name()))
        .ok_or_else(|| {
            "Impact LX+ not found (no 'Impact LX... MIDI1' input) - is it connected?".to_string()
        })?;
    let conn = client
        .connect_source(&port)
        .await
        .map_err(|e| format!("Failed to connect to MIDI input: {e}"))?;

    tokio::spawn(async move {
        let mut sysex = conn.into_sysex();
        let mut assembler = DumpAssembler::default();
        while let Some(timed) = sysex.recv().await {
            let status = match DeviceStatus::try_from(timed.payload) {
                Ok(status) => status,
                Err(e) => {
                    let _ = app_tx.send(AppMsg::UserError(UserError::Parse(e.to_string())));
                    continue;
                }
            };

            if assembler.is_empty() {
                let _ = app_tx.send(AppMsg::Device(DeviceEvent::DumpStarted));
            }
            assembler.apply(&status);

            if assembler.is_complete() {
                match assembler.try_into_dump() {
                    Ok(dump) => {
                        let _ =
                            app_tx.send(AppMsg::Device(DeviceEvent::DumpReceived(Box::new(dump))));
                    }
                    Err(e) => {
                        let _ = app_tx.send(AppMsg::UserError(UserError::Parse(e.to_string())));
                    }
                }
                assembler = DumpAssembler::default();
            }
        }
    });

    Ok(())
}

async fn send(output: &DestinationConnection, sysex: &SysEx) -> Result<(), MidiError> {
    output
        .send_sysex(sysex)
        .await
        .map_err(|e| MidiError::OutputConnection(e.to_string()))
}

async fn send_all(output: &DestinationConnection, messages: Vec<SysEx>) -> Result<(), UserError> {
    for message in messages {
        send(output, &message).await.map_err(UserError::Midi)?;
        tokio::time::sleep(WRITE_PACING).await;
    }
    Ok(())
}

async fn handle_midi_msg(
    msg: DeviceMsg,
    output: &DestinationConnection,
) -> Result<DeviceEvent, UserError> {
    match msg {
        DeviceMsg::WriteDump(dump) => {
            send_all(output, dump.to_messages()).await?;
            Ok(DeviceEvent::DumpWritten)
        }
        DeviceMsg::WritePreset { id, preset } => {
            send_all(output, preset.send_messages(id)).await?;
            Ok(DeviceEvent::PresetWritten(id))
        }
        DeviceMsg::WritePadMap { id, map } => {
            send_all(output, map.send_messages(id)).await?;
            Ok(DeviceEvent::PadMapWritten(id))
        }
        DeviceMsg::WriteGlobalSettings(settings) => {
            send_all(output, settings.send_messages()).await?;
            Ok(DeviceEvent::GlobalSettingsWritten)
        }
        DeviceMsg::WriteGlobalControls(controls) => {
            send_all(output, controls.send_messages()).await?;
            Ok(DeviceEvent::GlobalControlsWritten)
        }
        DeviceMsg::Reconnect => Ok(DeviceEvent::Reconnected),
    }
}
