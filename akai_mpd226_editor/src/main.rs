use std::time::Duration;

use akai_mpd226_editor::APP_DIMENSIONS;
use akai_mpd226_editor::AkaiMpd226Editor;
use akai_mpd226_editor::app::AppState;
use akai_mpd226_editor::config::AppConfig;
use akai_mpd226_editor::fs::load_app_config;
use akai_mpd226_editor::fs::load_global_from_file;
use akai_mpd226_editor::fs::load_preset_from_file;
use akai_mpd226_editor::fs::persist_config;
use akai_mpd226_editor::fs::persist_user_settings;
use akai_mpd226_editor::fs::save_global;
use akai_mpd226_editor::fs::save_preset;
use akai_mpd226_editor::message::AppEffect;
use akai_mpd226_editor::message::AppMsg;
use akai_mpd226_editor::message::DeviceMsg;
use akai_mpd226_editor::message::IoEffect;
use akai_mpd226_editor::message::IoMsg;
use akai_mpd226_editor::message::UiEffect;
use akai_mpd226_editor::message::UiMsg;
use akai_mpd226_editor::message::UserError;
use eframe::egui::ViewportBuilder;
use midilab::error::MidiError;
use midilab::manufacturer::akai::mpd226::DeviceStatus;
use midilab::manufacturer::akai::mpd226::PORT_NAME;
use midilab::manufacturer::akai::mpd226::dump_global_from_device;
use midilab::manufacturer::akai::mpd226::dump_preset_from_device;
use midilab::manufacturer::akai::mpd226::raw::RawGlobal;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;
use midilab::manufacturer::akai::mpd226::write_preset_to_device;
use midilab_io::midi::find_input_port;
use midilab_io::midi::find_output_port;
use midilab_io::midi::flush_coremidi_notifications;
use midilab_io::midi::recv_device_bytes;
use midir::MidiInput;
use midir::MidiOutput;
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
        while let Some(msg) = midi_rx.recv().await {
            let result: Result<Vec<u8>, MidiError> = handle_midi_msg(msg).await;

            let msg = match result {
                Ok(bytes) => match DeviceStatus::try_from(bytes.as_slice()) {
                    Ok(msg) => AppMsg::Device(msg),
                    Err(e) => AppMsg::UserError(UserError::DeviceStatusParse(e)),
                },

                Err(e) => AppMsg::UserError(UserError::Midi(e)),
            };

            midi_app_tx.send(msg).unwrap();
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

fn connect_midi_output() -> Result<midir::MidiOutputConnection, String> {
    flush_coremidi_notifications();
    let midi_out =
        MidiOutput::new("mpd226").map_err(|e| format!("Failed to create MIDI output: {}", e))?;

    let port = find_output_port(&midi_out, PORT_NAME)
        .ok_or_else(|| "MPD226 not found - make sure device is connected".to_string())?;
    midi_out
        .connect(&port, PORT_NAME)
        .map_err(|e| format!("Failed to connect to MIDI output: {}", e))
}

fn connect_midi_input(
    tx: UnboundedSender<Vec<u8>>,
) -> Result<midir::MidiInputConnection<()>, String> {
    let midi_in =
        MidiInput::new("mpd226-recv").map_err(|e| format!("Failed to create MIDI input: {}", e))?;
    let port = find_input_port(&midi_in, PORT_NAME)
        .ok_or_else(|| "MPD226 not found - make sure device is connected".to_string())?;
    midi_in
        .connect(
            &port,
            PORT_NAME,
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .map_err(|e| format!("Failed to connect to MIDI input: {}", e))
}

async fn handle_midi_msg(msg: DeviceMsg) -> Result<Vec<u8>, MidiError> {
    let mut output = connect_midi_output().map_err(MidiError::OutputConnection)?;

    let (tx, mut rx) = unbounded_channel::<Vec<u8>>();
    let _input = connect_midi_input(tx).map_err(MidiError::InputConnection)?;

    match msg {
        DeviceMsg::DumpPreset(slot) => {
            let request = dump_preset_from_device(slot as u8);
            output.send(&request).map_err(|_| MidiError::DumpPreset)?;

            let bytes = recv_device_bytes(&mut rx, Duration::from_secs(2)).await?;
            Ok(bytes)
        }
        DeviceMsg::WritePreset(preset) => {
            let raw_preset = RawPreset::from(preset.as_ref());
            let bytes = write_preset_to_device(&raw_preset);
            output.send(&bytes).map_err(|_| MidiError::WritePreset)?;

            let bytes = recv_device_bytes(&mut rx, Duration::from_secs(2)).await?;
            Ok(bytes)
        }
        DeviceMsg::DumpGlobal => {
            let request = dump_global_from_device();
            output.send(&request).map_err(|_| MidiError::DumpPreset)?;

            let bytes = recv_device_bytes(&mut rx, Duration::from_secs(2)).await?;
            Ok(bytes)
        }
        DeviceMsg::WriteGlobal(global) => {
            let raw_global = RawGlobal::from(global.as_ref());
            let messages = raw_global.global_send_messages();

            let mut bytes = vec![];

            for msg in messages {
                output.send(&msg).map_err(|_| MidiError::WritePreset)?;
                bytes = recv_device_bytes(&mut rx, Duration::from_millis(500))
                    .await
                    .unwrap();
            }

            Ok(bytes)
        }
    }
}
