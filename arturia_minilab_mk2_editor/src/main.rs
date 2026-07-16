use std::time::Duration;

use arturia_minilab_mk2_editor::MinilabMk2Editor;
use arturia_minilab_mk2_editor::app::AppState;
use arturia_minilab_mk2_editor::config::AppConfig;
use arturia_minilab_mk2_editor::fs::load_app_config;
use arturia_minilab_mk2_editor::fs::load_global_from_file;
use arturia_minilab_mk2_editor::fs::load_preset_from_file;
use arturia_minilab_mk2_editor::fs::persist_config;
use arturia_minilab_mk2_editor::fs::persist_user_settings;
use arturia_minilab_mk2_editor::fs::save_global;
use arturia_minilab_mk2_editor::fs::save_preset;
use arturia_minilab_mk2_editor::message::AppEffect;
use arturia_minilab_mk2_editor::message::AppMsg;
use arturia_minilab_mk2_editor::message::DeviceEvent;
use arturia_minilab_mk2_editor::message::DeviceMsg;
use arturia_minilab_mk2_editor::message::IoEffect;
use arturia_minilab_mk2_editor::message::IoMsg;
use arturia_minilab_mk2_editor::message::UserError;
use eframe::egui::ViewportBuilder;
use midi_io::Client;
use midi_io::DestinationConnection;
use midi_io::SysEx;
use midilab::error::MidiError;
use midilab::manufacturer::arturia::minilab_mk2::DeviceStatus;
use midilab::manufacturer::arturia::minilab_mk2::Global;
use midilab::manufacturer::arturia::minilab_mk2::PORT_NAME;
use midilab::manufacturer::arturia::minilab_mk2::ParamStore;
use midilab::manufacturer::arturia::minilab_mk2::Preset;
use midilab::manufacturer::arturia::minilab_mk2::recall_memory_message;
use midilab::manufacturer::arturia::minilab_mk2::set_pad_live_color_message;
use midilab::manufacturer::arturia::minilab_mk2::store_memory_message;
use midilab_io::midi::find_input_port;
use midilab_io::midi::find_output_port;
use midilab_io::midi::recv_device_bytes;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;

const READ_TIMEOUT: Duration = Duration::from_secs(2);
const WRITE_PACING: Duration = Duration::from_millis(2);
const MEMORY_OP_SETTLE: Duration = Duration::from_millis(50);

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
                IoMsg::SavePreset { preset, path } => IoEffect::PresetSaveResult(
                    save_preset(*preset, &path).await.map_err(|e| e.to_string()),
                ),
                IoMsg::LoadPreset { path } => IoEffect::PresetLoadResult(
                    load_preset_from_file(&path)
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
        let client = Client::new("minilab_mk2")
            .await
            .expect("failed to init MIDI client");

        let (dev_tx, mut dev_rx) = unbounded_channel::<Vec<u8>>();
        let mut output: Option<DestinationConnection> = connect_output(&client).await.ok();
        let mut input = connect_input(&client, dev_tx.clone()).await.ok();

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
            if input.is_none() {
                input = connect_input(&client, dev_tx.clone()).await.ok();
            }
            let out = output.as_ref().unwrap();

            let msg = match handle_midi_msg(msg, out, &mut dev_rx).await {
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
            .with_inner_size(eframe::egui::vec2(900., 700.))
            .with_min_inner_size(eframe::egui::vec2(400., 300.)),
        ..Default::default()
    };

    let config = load_app_config(&AppConfig::config_path().unwrap_or_default())
        .await
        .unwrap_or_default();

    eframe::run_native(
        "Arturia MiniLab mkII Editor",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(MinilabMk2Editor::new(
                app_tx,
                ui_rx,
                config.into(),
            )))
        }),
    )?;

    Ok(())
}

async fn connect_output(client: &Client) -> Result<DestinationConnection, String> {
    let port = find_output_port(client, PORT_NAME)
        .await
        .ok_or_else(|| format!("MiniLab not found (no '{PORT_NAME}' output) - is it connected?"))?;
    client
        .connect_destination(&port)
        .await
        .map_err(|e| format!("Failed to connect to MIDI output: {}", e))
}

async fn connect_input(client: &Client, tx: UnboundedSender<Vec<u8>>) -> Result<(), String> {
    let port = find_input_port(client, PORT_NAME)
        .await
        .ok_or_else(|| format!("MiniLab not found (no '{PORT_NAME}' input) - is it connected?"))?;
    let conn = client
        .connect_source(&port)
        .await
        .map_err(|e| format!("Failed to connect to MIDI input: {}", e))?;

    tokio::spawn(async move {
        let mut sysex = conn.into_sysex();
        while let Some(timed) = sysex.recv().await {
            let _ = tx.send(timed.payload.to_wire_bytes());
        }
    });

    Ok(())
}

async fn send_bytes(output: &DestinationConnection, bytes: &[u8]) -> Result<(), MidiError> {
    let sysex = SysEx::try_from(bytes).map_err(|e| MidiError::OutputConnection(e.to_string()))?;
    output
        .send_sysex(&sysex)
        .await
        .map_err(|e| MidiError::OutputConnection(e.to_string()))
}

fn drain(rx: &mut UnboundedReceiver<Vec<u8>>) {
    while rx.try_recv().is_ok() {}
}

async fn handle_midi_msg(
    msg: DeviceMsg,
    output: &DestinationConnection,
    rx: &mut UnboundedReceiver<Vec<u8>>,
) -> Result<DeviceEvent, UserError> {
    match msg {
        DeviceMsg::ReadPreset => {
            drain(rx);
            let mut store = ParamStore::default();
            for message in Preset::read_messages() {
                let status = request_status(output, rx, &message).await?;
                store.apply(&status);
            }
            let preset = store
                .try_into_preset()
                .map_err(|e| UserError::Parse(e.to_string()))?;
            Ok(DeviceEvent::PresetRead(Box::new(preset)))
        }
        DeviceMsg::WritePreset(preset) => {
            drain(rx);
            for message in preset.send_messages() {
                send_bytes(output, &message)
                    .await
                    .map_err(UserError::Midi)?;
                tokio::time::sleep(WRITE_PACING).await;
            }
            drain(rx);
            Ok(DeviceEvent::PresetWritten)
        }
        DeviceMsg::ReadGlobal => {
            drain(rx);
            let mut store = ParamStore::default();
            for message in Global::read_messages() {
                let status = request_status(output, rx, &message).await?;
                store.apply(&status);
            }
            let global = store
                .try_into_global()
                .map_err(|e| UserError::Parse(e.to_string()))?;
            Ok(DeviceEvent::GlobalRead(global))
        }
        DeviceMsg::WriteGlobal(global) => {
            drain(rx);
            for message in global.send_messages() {
                send_bytes(output, &message)
                    .await
                    .map_err(UserError::Midi)?;
                tokio::time::sleep(WRITE_PACING).await;
            }
            drain(rx);
            Ok(DeviceEvent::GlobalWritten)
        }
        DeviceMsg::RecallMemory(slot) => {
            send_bytes(output, &recall_memory_message(slot))
                .await
                .map_err(UserError::Midi)?;
            tokio::time::sleep(MEMORY_OP_SETTLE).await;
            drain(rx);
            Ok(DeviceEvent::MemoryRecalled(slot))
        }
        DeviceMsg::StoreMemory(slot) => {
            send_bytes(output, &store_memory_message(slot))
                .await
                .map_err(UserError::Midi)?;
            tokio::time::sleep(MEMORY_OP_SETTLE).await;
            drain(rx);
            Ok(DeviceEvent::MemoryStored(slot))
        }
        DeviceMsg::SetLivePadColor { pad, color } => {
            send_bytes(output, &set_pad_live_color_message(pad, color))
                .await
                .map_err(UserError::Midi)?;
            Ok(DeviceEvent::LiveColorSent)
        }
    }
}

async fn request_status(
    output: &DestinationConnection,
    rx: &mut UnboundedReceiver<Vec<u8>>,
    message: &[u8],
) -> Result<DeviceStatus, UserError> {
    send_bytes(output, message).await.map_err(UserError::Midi)?;
    let bytes = recv_device_bytes(rx, READ_TIMEOUT)
        .await
        .map_err(UserError::Midi)?;
    DeviceStatus::try_from(bytes.as_slice()).map_err(|e| UserError::Parse(e.to_string()))
}
