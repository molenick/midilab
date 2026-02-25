use std::future::Future;
use std::time::Duration;

use midilab::error::MidiError;
use midir::MidiInput;
use midir::MidiInputPort;
use midir::MidiOutput;
use midir::MidiOutputPort;
use tokio::sync::mpsc::UnboundedReceiver;

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

async fn async_timeout<F: Future>(duration: Duration, future: F) -> Option<F::Output> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::timeout(duration, future).await.ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        use futures_timer::Delay;
        tokio::select! {
            result = future => Some(result),
            _ = Delay::new(duration) => None,
        }
    }
}

pub async fn recv_device_bytes(
    rx: &mut UnboundedReceiver<Vec<u8>>,
    timeout_duration: Duration,
) -> Result<Vec<u8>, MidiError> {
    match async_timeout(timeout_duration, rx.recv()).await {
        Some(Some(bytes)) => Ok(bytes),
        Some(None) => Err(MidiError::ChannelClosed),
        None => Err(MidiError::ResponseTimeout),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod fs {
    use std::path::Path;

    use bytemuck::PodCastError;
    use midilab::manufacturer::akai;
    use midilab::manufacturer::akai::mpd226::Preset;
    use midilab::manufacturer::akai::mpd226::error::PresetParseError;
    use midilab::manufacturer::akai::mpd226::raw::RawPreset;

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error(transparent)]
        FileSys(#[from] std::io::Error),
        #[error(transparent)]
        RawPresetDeserialization(#[from] PodCastError),
        #[error(transparent)]
        PresetDeserialization(#[from] PresetParseError),
    }

    pub async fn save_akai_mpd226_preset(
        preset: akai::mpd226::Preset,
        path: &Path,
    ) -> Result<(), Error> {
        let raw = RawPreset::from(&preset);
        let payload = bytemuck::bytes_of(&raw).to_vec();

        Ok(tokio::fs::write(path, payload).await?)
    }

    pub async fn load_akai_mpd226_preset_from_sysex(
        path: &Path,
    ) -> Result<akai::mpd226::Preset, Error> {
        let bytes = tokio::fs::read(path).await?;
        let raw: RawPreset = *bytemuck::try_from_bytes(&bytes)?;
        let preset = Preset::try_from(raw)?;

        Ok(preset)
    }
}
