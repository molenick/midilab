use std::time::Duration;

use eframe::egui::ViewportBuilder;
use midilab::error::MidiError;
use midilab::manufacturer::akai::mpd226::DeviceStatus;
use midilab::manufacturer::akai::mpd226::preset_dump_request;
use midilab::manufacturer::akai::mpd226::preset_send_message;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;
use midilab::message::AppEffect;
use midilab::message::AppMsg;
use midilab::message::AppState;
use midilab::message::DeviceMsg;
use midilab::message::UiMsg;
use midilab::sysex::Sysex;
use midilab_gui::AkaiMpd226Editor;
use midilab_gui::akai_mpd226_editor::APP_DIMENSIONS;
use midilab_io::find_input_port;
use midilab_io::find_output_port;
use midilab_io::flush_coremidi_notifications;
use midilab_io::recv_device_bytes;
use midir::MidiInput;
use midir::MidiOutput;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (app_tx, mut app_rx) = unbounded_channel();
    let (ui_tx, ui_rx) = unbounded_channel();
    let (midi_tx, mut midi_rx) = unbounded_channel::<DeviceMsg>();

    let midi_app_tx = app_tx.clone();
    let _midi = tokio::spawn(async move {
        while let Some(msg) = midi_rx.recv().await {
            let result = handle_midi_msg(msg).await;
            match result {
                Ok(Some(bytes)) => {
                    let sysex = match Sysex::try_from(bytes.as_slice()) {
                        Ok(sysex) => sysex,
                        Err(e) => {
                            midi_app_tx.send(AppMsg::SysexParseError(e)).unwrap();
                            continue;
                        }
                    };
                    let device_msg = match DeviceStatus::try_from(sysex) {
                        Ok(msg) => msg,
                        Err(e) => {
                            midi_app_tx.send(AppMsg::DeviceStatusParseError(e)).unwrap();
                            continue;
                        }
                    };
                    midi_app_tx.send(AppMsg::Device(device_msg)).unwrap();
                }
                Ok(None) => {}
                Err(e) => {
                    midi_app_tx.send(AppMsg::MidiError(e)).unwrap();
                }
            }
        }
    });

    let mut app_state = AppState::default();

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
        "Akai MPD226 Preset Editor",
        options,
        Box::new(|_cc| Ok(Box::new(AkaiMpd226Editor::new(app_tx, ui_rx)))),
    )?;

    Ok(())
}

const PORT_NAME: &str = "MPD226 Remote";

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

async fn handle_midi_msg(msg: DeviceMsg) -> Result<Option<Vec<u8>>, MidiError> {
    let mut output = connect_midi_output().map_err(MidiError::OutputConnection)?;

    let (tx, mut rx) = unbounded_channel::<Vec<u8>>();
    let _input = connect_midi_input(tx).map_err(MidiError::InputConnection)?;

    match msg {
        DeviceMsg::RequestPreset(slot) => {
            let request = preset_dump_request(slot as u8);
            output
                .send(&request)
                .map_err(|_| MidiError::RequestPreset)?;

            let bytes = recv_device_bytes(&mut rx, Duration::from_secs(2)).await?;
            Ok(Some(bytes))
        }
        DeviceMsg::SendPreset(preset) => {
            let raw_preset = RawPreset::from(preset.as_ref());
            let bytes = preset_send_message(&raw_preset);
            output.send(&bytes).map_err(|_| MidiError::SendPreset)?;

            let bytes = recv_device_bytes(&mut rx, Duration::from_secs(2)).await?;
            Ok(Some(bytes))
        }
    }
}
