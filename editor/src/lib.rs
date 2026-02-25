#![cfg(target_arch = "wasm32")]

mod midi;
use midi::handle_midi_msg;
use midilab::manufacturer::akai::mpd226::DeviceStatus;
use midilab::message::AppEffect;
use midilab::message::AppMsg;
use midilab::message::DeviceMsg;
use midilab::message::UiMsg;
use midilab::message::UserError;
use midilab::state::AppState;
use midilab_gui::AkaiMpd226Editor;
use tokio::sync::mpsc::unbounded_channel;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
}

#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            runner: eframe::WebRunner::new(),
        }
    }

    #[wasm_bindgen]
    pub async fn start(&self, canvas: web_sys::HtmlCanvasElement) -> Result<(), JsValue> {
        // Wait 2 seconds for WebMIDI permission prompt to complete
        futures_timer::Delay::new(std::time::Duration::from_millis(2000)).await;

        let _midi_out = match midir::MidiOutput::new("mpd226") {
            Ok(o) => o,
            Err(e) => {
                return Err(format!("MIDI output initialization failed: {}", e).into());
            }
        };

        let (app_tx, mut app_rx) = unbounded_channel::<AppMsg>();
        let (ui_tx, ui_rx) = unbounded_channel::<UiMsg>();
        let (midi_tx, mut midi_rx) = unbounded_channel::<DeviceMsg>();

        let midi_app_tx = app_tx.clone();

        spawn_local(async move {
            while let Some(msg) = midi_rx.recv().await {
                let result = handle_midi_msg(msg).await;

                let msg = match result {
                    Ok(bytes) => match DeviceStatus::try_from(bytes.as_slice()) {
                        Ok(msg) => AppMsg::Device(msg),
                        Err(e) => AppMsg::UserError(UserError::DeviceStatusParse(e)),
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("MIDI error: {}", e).into());
                        AppMsg::UserError(UserError::Midi(e))
                    }
                };

                let _ = midi_app_tx.send(msg);
            }
        });

        let app_ui_tx = ui_tx.clone();
        let mut app_state = AppState::new();

        spawn_local(async move {
            let _ = app_ui_tx.send(UiMsg::UpdatePreset(Box::new(app_state.preset)));

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
                        AppEffect::Io(_) => {
                            // noop
                        }
                    }
                }
            }
        });

        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|_cc| Ok(Box::new(AkaiMpd226Editor::new(app_tx, ui_rx)))),
            )
            .await?;

        web_sys::console::log_1(&"Eframe runner completed".into());
        Ok(())
    }
}
