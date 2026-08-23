use std::time::Duration;

use midi_io::Client;
use midi_io::Destination;
use midi_io::Source;
use midilab::error::MidiError;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::timeout;

pub async fn find_output_port(client: &Client, name: &str) -> Option<Destination> {
    client
        .destinations()
        .await
        .ok()?
        .into_iter()
        .find(|p| p.name() == name)
}

pub async fn find_input_port(client: &Client, name: &str) -> Option<Source> {
    client
        .sources()
        .await
        .ok()?
        .into_iter()
        .find(|p| p.name() == name)
}

pub async fn recv_device<T>(
    rx: &mut UnboundedReceiver<T>,
    timeout_duration: Duration,
) -> Result<T, MidiError> {
    match timeout(timeout_duration, rx.recv()).await {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(MidiError::ChannelClosed),
        Err(_) => Err(MidiError::ResponseTimeout),
    }
}
