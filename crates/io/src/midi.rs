use std::time::Duration;

use midi_io::Client;
use midi_io::DestinationConnection;
use midi_io::SysEx;
use midilab::error::MidiError;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::time::timeout_at;

/// Every MIDI source, each feeding received sysex to its own handler.
///
/// Dropping the `Listener` disconnects every source.
pub struct Listener {
    tasks: Vec<JoinHandle<()>>,
}

impl Listener {
    /// Connects every source, calling `handler` once per source to build the
    /// function that receives that source's sysex (so per-port state stays
    /// per port). Sources that fail to list or connect are skipped.
    pub async fn open<F, H>(client: &Client, mut handler: F) -> Listener
    where
        F: FnMut() -> H,
        H: FnMut(SysEx) + Send + 'static,
    {
        let sources = client.sources().await.unwrap_or_else(|e| {
            eprintln!("failed to list midi input ports: {e}");
            Vec::new()
        });

        let mut tasks = Vec::with_capacity(sources.len());
        for port in &sources {
            let connection = match client.connect_source(port).await {
                Ok(connection) => connection,
                Err(e) => {
                    eprintln!("midi input connect failed: {} - {e}", port.name());
                    continue;
                }
            };
            let mut handle = handler();
            tasks.push(tokio::spawn(async move {
                let mut sysex = connection.into_sysex();
                while let Some(timed) = sysex.recv().await {
                    handle(timed.payload);
                }
            }));
        }

        Listener { tasks }
    }

    pub fn port_count(&self) -> usize {
        self.tasks.len()
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// A connection to every MIDI port, opened for one operation.
///
/// Sysex is self-addressed, so it is sent to every output and a device is
/// recognised by the replies it sends, not by port name. Inputs are
/// connected before anything is sent, so no reply is missed, and each
/// `Link` has its own channel, so replies to earlier operations never leak
/// in. Dropping the `Link` disconnects every port.
pub struct Link {
    outputs: Vec<DestinationConnection>,
    inputs: Listener,
    rx: UnboundedReceiver<SysEx>,
}

impl Link {
    /// Connects every source, then every destination. Ports that fail to
    /// list or connect are skipped: absence of a transport is reported
    /// through [`Link::output_count`] and [`Link::input_count`].
    pub async fn open(client: &Client) -> Link {
        let (tx, rx) = unbounded_channel();

        let inputs = Listener::open(client, || {
            let tx = tx.clone();
            move |sysex| {
                let _ = tx.send(sysex);
            }
        })
        .await;
        drop(tx);

        let destinations = client.destinations().await.unwrap_or_else(|e| {
            eprintln!("failed to list midi output ports: {e}");
            Vec::new()
        });
        let mut outputs = Vec::with_capacity(destinations.len());
        for port in &destinations {
            match client.connect_destination(port).await {
                Ok(connection) => outputs.push(connection),
                Err(e) => eprintln!("midi output connect failed: {} - {e}", port.name()),
            }
        }

        Link {
            outputs,
            inputs,
            rx,
        }
    }

    pub fn output_count(&self) -> usize {
        self.outputs.len()
    }

    pub fn input_count(&self) -> usize {
        self.inputs.port_count()
    }

    /// Sends `sysex` to every output. Sending to zero outputs is a no-op;
    /// individual failures are tolerated (stale ports), and this errors
    /// only if every output fails.
    pub async fn send(&self, sysex: &SysEx) -> Result<(), MidiError> {
        let mut failures = 0;

        for output in &self.outputs {
            if let Err(e) = output.send_sysex(sysex).await {
                failures += 1;
                eprintln!("midi send failed: {e}");
            }
        }

        if !self.outputs.is_empty() && failures == self.outputs.len() {
            return Err(MidiError::Send(format!(
                "all {failures} output connection(s) failed to send"
            )));
        }

        Ok(())
    }

    /// Sends each message with [`Link::send`], sleeping `pace` after each so
    /// a burst does not outrun the device's parser.
    pub async fn send_paced(
        &self,
        messages: impl IntoIterator<Item = SysEx>,
        pace: Duration,
    ) -> Result<(), MidiError> {
        for message in messages {
            self.send(&message).await?;
            tokio::time::sleep(pace).await;
        }
        Ok(())
    }

    /// Receives until `f` maps a message to `Some`, skipping every message
    /// it maps to `None`, for up to `timeout_duration` in total.
    ///
    /// `None` means no response: silence on a shared bus is a result, not
    /// an error.
    pub async fn recv<T>(
        &mut self,
        timeout_duration: Duration,
        mut f: impl FnMut(SysEx) -> Option<T>,
    ) -> Option<T> {
        let deadline = Instant::now() + timeout_duration;

        loop {
            match timeout_at(deadline, self.rx.recv()).await {
                Ok(Some(sysex)) => {
                    if let Some(value) = f(sysex) {
                        return Some(value);
                    }
                }
                Ok(None) | Err(_) => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::timeout;

    use super::*;

    #[tokio::test]
    async fn link_sends_and_receives_on_every_port() {
        let client = Client::new("link-ports").await.unwrap();
        let device_in = client
            .create_virtual_destination("Link Device In")
            .await
            .unwrap();
        let device_out = client
            .create_virtual_source("Link Device Out")
            .await
            .unwrap();
        let mut requests = device_in.into_sysex();

        let mut link = Link::open(&client).await;
        assert!(link.output_count() >= 1);
        assert!(link.input_count() >= 1);

        let request = SysEx::new(&[0x7D, 0x01, 0x02]).unwrap();
        link.send(&request).await.unwrap();
        let received = timeout(Duration::from_secs(5), requests.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(received.payload, request);

        let noise = SysEx::new(&[0x7D, 0x03, 0x04]).unwrap();
        let reply = SysEx::new(&[0x7D, 0x05, 0x06]).unwrap();
        device_out.send_sysex(&noise).await.unwrap();
        device_out.send_sysex(&reply).await.unwrap();

        let matched = link
            .recv(Duration::from_secs(5), |s| (s == reply).then_some(s))
            .await;
        assert_eq!(matched, Some(reply.clone()));

        let silence = link
            .recv(Duration::from_millis(50), |s| (s == reply).then_some(s))
            .await;
        assert_eq!(silence, None);
    }

    #[tokio::test]
    async fn link_send_to_no_outputs_is_a_noop() {
        let (_tx, rx) = unbounded_channel();
        let link = Link {
            outputs: Vec::new(),
            inputs: Listener { tasks: Vec::new() },
            rx,
        };

        link.send(&SysEx::new(&[0x7D, 0x01, 0x02]).unwrap())
            .await
            .unwrap();
    }
}
