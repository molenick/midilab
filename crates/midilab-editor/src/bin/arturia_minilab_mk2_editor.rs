use std::time::Duration;

use eframe::egui::ViewportBuilder;
use midi_io::Client;
use midi_io::SysEx;
use midilab::error::MidiError;
use midilab::manufacturer::arturia::minilab_mk2::Global;
use midilab::manufacturer::arturia::minilab_mk2::ParamStore;
use midilab::manufacturer::arturia::minilab_mk2::Preset;
use midilab::manufacturer::arturia::minilab_mk2::recall_memory_message;
use midilab::manufacturer::arturia::minilab_mk2::reply_to;
use midilab::manufacturer::arturia::minilab_mk2::set_pad_live_color_message;
use midilab::manufacturer::arturia::minilab_mk2::store_memory_message;
use midilab_editor::arturia_minilab_mk2::MinilabMk2Editor;
use midilab_editor::arturia_minilab_mk2::app::AppState;
use midilab_editor::arturia_minilab_mk2::config::AppConfig;
use midilab_editor::arturia_minilab_mk2::fs::load_app_config;
use midilab_editor::arturia_minilab_mk2::fs::load_global_from_file;
use midilab_editor::arturia_minilab_mk2::fs::load_preset_from_file;
use midilab_editor::arturia_minilab_mk2::fs::persist_config;
use midilab_editor::arturia_minilab_mk2::fs::persist_user_settings;
use midilab_editor::arturia_minilab_mk2::fs::save_global;
use midilab_editor::arturia_minilab_mk2::fs::save_preset;
use midilab_editor::arturia_minilab_mk2::message::AppEffect;
use midilab_editor::arturia_minilab_mk2::message::AppMsg;
use midilab_editor::arturia_minilab_mk2::message::DeviceEvent;
use midilab_editor::arturia_minilab_mk2::message::DeviceMsg;
use midilab_editor::arturia_minilab_mk2::message::IoEffect;
use midilab_editor::arturia_minilab_mk2::message::IoMsg;
use midilab_editor::arturia_minilab_mk2::message::UserError;
use midilab_io::midi::Link;
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

        while let Some(msg) = midi_rx.recv().await {
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

/// Sends each read request and waits for the reply that answers it,
/// collecting the values. Returns `None` as soon as the device is silent:
/// absence of a response is a state to report, not an error.
async fn read_store(
    link: &mut Link,
    requests: Vec<SysEx>,
) -> Result<Option<ParamStore>, MidiError> {
    let mut store = ParamStore::default();
    for request in requests {
        link.send(&request).await?;
        let Some(status) = link.recv(READ_TIMEOUT, |s| reply_to(&request, s)).await else {
            return Ok(None);
        };
        store.apply(&status);
    }
    Ok(Some(store))
}

/// Sends `message`, then waits for the device to settle.
async fn send_settled(link: &Link, message: &SysEx) -> Result<(), MidiError> {
    link.send(message).await?;
    tokio::time::sleep(MEMORY_OP_SETTLE).await;
    Ok(())
}

/// Handles a device message over a [`Link`] opened for it, returning the
/// app message to report.
///
/// Writes report success when delivered to the outputs, and reads report a
/// status when the device does not answer.
async fn handle_midi_msg(msg: DeviceMsg, client: &Client) -> AppMsg {
    let mut link = Link::open(client).await;
    if link.output_count() == 0 {
        return AppMsg::MidiStatus("no MIDI output ports - not sent".to_string());
    }

    match msg {
        DeviceMsg::ReadPreset => match read_store(&mut link, Preset::read_messages()).await {
            Ok(Some(store)) => match store.try_into_preset() {
                Ok(preset) => AppMsg::Device(DeviceEvent::PresetRead(Box::new(preset))),
                Err(e) => AppMsg::UserError(UserError::Parse(e.to_string())),
            },
            Ok(None) => AppMsg::MidiStatus("no response from device".to_string()),
            Err(e) => AppMsg::UserError(UserError::Midi(e)),
        },
        DeviceMsg::WritePreset(preset) => {
            match link.send_paced(preset.send_messages(), WRITE_PACING).await {
                Ok(()) => AppMsg::Device(DeviceEvent::PresetWritten),
                Err(e) => AppMsg::UserError(UserError::Midi(e)),
            }
        }
        DeviceMsg::ReadGlobal => match read_store(&mut link, Global::read_messages()).await {
            Ok(Some(store)) => match store.try_into_global() {
                Ok(global) => AppMsg::Device(DeviceEvent::GlobalRead(global)),
                Err(e) => AppMsg::UserError(UserError::Parse(e.to_string())),
            },
            Ok(None) => AppMsg::MidiStatus("no response from device".to_string()),
            Err(e) => AppMsg::UserError(UserError::Midi(e)),
        },
        DeviceMsg::WriteGlobal(global) => {
            match link.send_paced(global.send_messages(), WRITE_PACING).await {
                Ok(()) => AppMsg::Device(DeviceEvent::GlobalWritten),
                Err(e) => AppMsg::UserError(UserError::Midi(e)),
            }
        }
        DeviceMsg::RecallMemory(slot) => {
            match send_settled(&link, &recall_memory_message(slot)).await {
                Ok(()) => AppMsg::Device(DeviceEvent::MemoryRecalled(slot)),
                Err(e) => AppMsg::UserError(UserError::Midi(e)),
            }
        }
        DeviceMsg::StoreMemory(slot) => {
            match send_settled(&link, &store_memory_message(slot)).await {
                Ok(()) => AppMsg::Device(DeviceEvent::MemoryStored(slot)),
                Err(e) => AppMsg::UserError(UserError::Midi(e)),
            }
        }
        DeviceMsg::SetLivePadColor { pad, color } => {
            match link.send(&set_pad_live_color_message(pad, color)).await {
                Ok(()) => AppMsg::Device(DeviceEvent::LiveColorSent),
                Err(e) => AppMsg::UserError(UserError::Midi(e)),
            }
        }
    }
}
