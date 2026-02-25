#![cfg(not(target_arch = "wasm32"))]

use eframe::egui::ViewportBuilder;
use midi::handle_midi_msg;
use midilab::manufacturer::akai::mpd226::DeviceStatus;
use midilab::message::AppEffect;
use midilab::message::AppMsg;
use midilab::message::IoEffect;
use midilab::message::IoMsg;
use midilab::message::UiMsg;
use midilab::message::UserError;
use midilab::state::AppState;
use midilab_gui::AkaiMpd226Editor;
use midilab_gui::akai_mpd226_editor::APP_DIMENSIONS;
use midilab_io::fs::load_akai_mpd226_preset_from_sysex;
use midilab_io::fs::save_akai_mpd226_preset;
use tokio::sync::mpsc::unbounded_channel;

mod midi;

// todo: auto sync device state once at app start

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
                    save_akai_mpd226_preset(*preset, &path)
                        .await
                        .map_err(|e| e.to_string()),
                ),
                IoMsg::LoadPreset { path } => IoEffect::PresetLoadResult(
                    load_akai_mpd226_preset_from_sysex(&path)
                        .await
                        .map(Box::new)
                        .map_err(|e| e.to_string()),
                ),
            };

            io_app_tx.send(AppMsg::Io(Box::new(effect))).unwrap();
        }
    });

    let _midi = tokio::spawn(async move {
        while let Some(msg) = midi_rx.recv().await {
            let result = handle_midi_msg(msg).await;

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

    let mut app_state = AppState::new();

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
    eframe::run_native(
        "Akai MPD226 Editor",
        options,
        Box::new(|_cc| Ok(Box::new(AkaiMpd226Editor::new(app_tx, ui_rx)))),
    )?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
