use std::time::Duration;

use midilab::error::MidiError;
use midir::MidiInput;
use midir::MidiInputPort;
use midir::MidiOutput;
use midir::MidiOutputPort;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::timeout;

#[cfg(target_vendor = "apple")]
pub fn flush_coremidi_notifications() {
    core_foundation::runloop::CFRunLoop::run_in_mode(
        unsafe { core_foundation::runloop::kCFRunLoopDefaultMode },
        Duration::from_millis(10),
        true,
    );
}

#[cfg(not(target_vendor = "apple"))]
pub fn flush_coremidi_notifications() {}

pub fn find_output_port(midi_out: &MidiOutput, name: &str) -> Option<MidiOutputPort> {
    midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok().as_deref() == Some(name))
}

pub fn find_input_port(midi_in: &MidiInput, name: &str) -> Option<MidiInputPort> {
    midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok().as_deref() == Some(name))
}

pub async fn recv_device_bytes(
    rx: &mut UnboundedReceiver<Vec<u8>>,
    timeout_duration: Duration,
) -> Result<Vec<u8>, MidiError> {
    match timeout(timeout_duration, rx.recv()).await {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(MidiError::ChannelClosed),
        Err(_) => Err(MidiError::ResponseTimeout),
    }
}

pub mod fs {
    use std::path::Path;

    use midilab::manufacturer::akai::mpd226::raw::RawPreset;
    use midilab::manufacturer::akai::mpd226::{self};
    use midilab::sysex::Sysex;

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error(transparent)]
        FileSys(#[from] std::io::Error),
    }

    pub async fn save_akai_mpd226_preset_as_sysex(
        preset: mpd226::Preset,
        path: &Path,
    ) -> Result<(), Error> {
        let raw = RawPreset::from(&preset);
        let payload = bytemuck::bytes_of(&raw).to_vec();
        let sysex = Sysex::new(payload);
        let bytes = sysex.as_bytes();

        Ok(tokio::fs::write(path, bytes).await?)
    }
}
