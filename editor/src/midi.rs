use std::time::Duration;

use midilab::error::MidiError;
use midilab::manufacturer::akai::mpd226::dump_global_from_device;
use midilab::manufacturer::akai::mpd226::dump_preset_from_device;
use midilab::manufacturer::akai::mpd226::raw::RawGlobal;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;
use midilab::manufacturer::akai::mpd226::write_preset_to_device;
use midilab::message::DeviceMsg;
use midilab_io::flush_coremidi_notifications;
use midilab_io::recv_device_bytes;
use midir::MidiInput;
use midir::MidiOutput;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;

const PORT_NAME: &str = "MPD226 Remote";

pub fn connect_midi_output() -> Result<midir::MidiOutputConnection, String> {
    flush_coremidi_notifications();

    let midi_out =
        MidiOutput::new("mpd226").map_err(|e| format!("Failed to create MIDI output: {}", e))?;

    let port = midilab_io::find_output_port(&midi_out, PORT_NAME)
        .ok_or_else(|| "MPD226 not found - make sure device is connected".to_string())?;

    midi_out
        .connect(&port, PORT_NAME)
        .map_err(|e| format!("Failed to connect to MIDI output: {}", e))
}

pub fn connect_midi_input(
    tx: UnboundedSender<Vec<u8>>,
) -> Result<midir::MidiInputConnection<()>, String> {
    let midi_in =
        MidiInput::new("mpd226-recv").map_err(|e| format!("Failed to create MIDI input: {}", e))?;

    let port = midilab_io::find_input_port(&midi_in, PORT_NAME)
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

pub async fn handle_midi_msg(msg: DeviceMsg) -> Result<Vec<u8>, MidiError> {
    let mut output = connect_midi_output().map_err(MidiError::OutputConnection)?;

    let (tx, mut rx) = unbounded_channel::<Vec<u8>>();
    let _input = connect_midi_input(tx).map_err(MidiError::InputConnection)?;

    #[cfg(target_arch = "wasm32")]
    futures_timer::Delay::new(std::time::Duration::from_millis(50)).await;

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

            for msg in messages.iter() {
                output.send(msg).map_err(|_| MidiError::WritePreset)?;
                bytes = match recv_device_bytes(&mut rx, Duration::from_millis(500)).await {
                    Ok(b) => b,
                    Err(e) => {
                        return Err(e);
                    }
                };
            }

            Ok(bytes)
        }
    }
}
