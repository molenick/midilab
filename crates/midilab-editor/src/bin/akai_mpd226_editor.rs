use std::time::Duration;

use eframe::egui::ViewportBuilder;
use midi_io::Client;
use midi_io::SysEx;
use midilab::manufacturer::akai::mpd226::dump_global_from_device;
use midilab::manufacturer::akai::mpd226::dump_preset_from_device;
use midilab::manufacturer::akai::mpd226::raw::RawGlobal;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;
use midilab::manufacturer::akai::mpd226::reply_to;
use midilab::manufacturer::akai::mpd226::write_preset_to_device;
use midilab_editor::akai_mpd226::APP_DIMENSIONS;
use midilab_editor::akai_mpd226::AkaiMpd226Editor;
use midilab_editor::akai_mpd226::app::AppState;
use midilab_editor::akai_mpd226::config::AppConfig;
use midilab_editor::akai_mpd226::fs::load_app_config;
use midilab_editor::akai_mpd226::fs::load_global_from_file;
use midilab_editor::akai_mpd226::fs::load_preset_from_file;
use midilab_editor::akai_mpd226::fs::persist_config;
use midilab_editor::akai_mpd226::fs::persist_user_settings;
use midilab_editor::akai_mpd226::fs::save_global;
use midilab_editor::akai_mpd226::fs::save_preset;
use midilab_editor::akai_mpd226::message::AppEffect;
use midilab_editor::akai_mpd226::message::AppMsg;
use midilab_editor::akai_mpd226::message::DeviceMsg;
use midilab_editor::akai_mpd226::message::IoEffect;
use midilab_editor::akai_mpd226::message::IoMsg;
use midilab_editor::akai_mpd226::message::UiEffect;
use midilab_editor::akai_mpd226::message::UiMsg;
use midilab_editor::akai_mpd226::message::UserError;
use midilab_io::midi::Link;
use tokio::sync::mpsc::unbounded_channel;

/// Pace between messages of a multi-message write. Devices consume sysex
/// serially; small gaps keep a burst from outrunning the device's parser.
const WRITE_PACING: Duration = Duration::from_millis(2);

/// How long to wait for a response to a request (dump) after sending it.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);

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
                    save_global(*global, &path).await.map_err(|e| e.to_string()),
                ),
                IoMsg::LoadGlobal { path } => IoEffect::GlobalLoadResult(
                    load_global_from_file(&path)
                        .await
                        .map(Box::new)
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
        let client = Client::new("mpd226")
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
        app_ui_tx
            .send(UiMsg::UpdatePreset(Box::new(app_state.preset)))
            .unwrap();

        while let Some(msg) = app_rx.recv().await {
            let effects = app_state.update(msg);

            for effect in effects {
                match effect {
                    AppEffect::Ui(ui_msg) => app_ui_tx.send(ui_msg).unwrap(),
                    AppEffect::Device(device_msg) => midi_tx.send(device_msg).unwrap(),
                    AppEffect::Io(io_msg) => io_tx.send(*io_msg).unwrap(),
                }
            }
        }
    });

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(APP_DIMENSIONS)
            .with_min_inner_size(APP_DIMENSIONS),
        ..Default::default()
    };

    let config = load_app_config(&AppConfig::config_path().unwrap_or_default())
        .await
        .unwrap_or_default();

    if config.user.auto_sync_enabled {
        let _ = app_tx.send(AppMsg::Ui(UiEffect::AutoSync));
    }

    eframe::run_native(
        "Akai MPD226 Editor",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(AkaiMpd226Editor::new(
                app_tx,
                ui_rx,
                config.into(),
            )))
        }),
    )?;

    Ok(())
}

/// Sends a dump request and waits for the reply that answers it.
///
/// Silence is a status, not an error.
async fn request_device_data(link: &mut Link, request: SysEx) -> AppMsg {
    if let Err(e) = link.send(&request).await {
        return AppMsg::UserError(UserError::Midi(e));
    }

    match link.recv(RESPONSE_TIMEOUT, |s| reply_to(&request, s)).await {
        Some(status) => AppMsg::Device(status),
        None => AppMsg::MidiStatus("no response from device".to_string()),
    }
}

/// Handles a device message over a [`Link`] opened for it, returning the
/// app message to report.
///
/// Writes report success when delivered to the outputs, and dumps report a
/// status when the device does not answer.
async fn handle_midi_msg(msg: DeviceMsg, client: &Client) -> AppMsg {
    let mut link = Link::open(client).await;
    if link.output_count() == 0 {
        return AppMsg::MidiStatus("no MIDI output ports - not sent".to_string());
    }

    match msg {
        DeviceMsg::DumpPreset(slot) => {
            request_device_data(&mut link, dump_preset_from_device(slot as u8)).await
        }
        DeviceMsg::DumpGlobal => request_device_data(&mut link, dump_global_from_device()).await,
        DeviceMsg::WritePreset(preset) => {
            let slot = preset.settings.slot;
            let raw_preset = RawPreset::from(preset.as_ref());
            match link
                .send_paced([write_preset_to_device(&raw_preset)], WRITE_PACING)
                .await
            {
                Ok(()) => AppMsg::MidiStatus(format!("Sent to device preset slot {slot}")),
                Err(e) => AppMsg::UserError(UserError::Midi(e)),
            }
        }
        DeviceMsg::WriteGlobal(global) => {
            let raw_global = RawGlobal::from(global.as_ref());
            let messages = raw_global.global_send_messages();
            match link.send_paced(messages, WRITE_PACING).await {
                Ok(()) => AppMsg::MidiStatus("Wrote global settings to device".to_string()),
                Err(e) => AppMsg::UserError(UserError::Midi(e)),
            }
        }
    }
}
