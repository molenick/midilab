use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use midi_io::Client;
use midi_io::DestinationConnection;
use midi_io::SysEx;
use midilab::error::MidiError;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::time::timeout_at;

type Taps = Arc<Mutex<Vec<UnboundedSender<SysEx>>>>;

/// Every MIDI source, each feeding received sysex to its own handler and to
/// every [`Link`] opened from it.
///
/// A source can be connected only once per [`Client`], so a long-lived
/// `Listener` shares its sources with the links opened through
/// [`Listener::link`]. Dropping the `Listener` disconnects every source once
/// its tasks wind down; use [`Listener::close`] to wait for that.
pub struct Listener {
    tasks: Vec<JoinHandle<()>>,
    taps: Taps,
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

        let taps: Taps = Arc::default();
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
            let taps = taps.clone();
            tasks.push(tokio::spawn(async move {
                let mut sysex = connection.into_sysex();
                while let Some(timed) = sysex.recv().await {
                    taps.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .retain(|tap| tap.send(timed.payload.clone()).is_ok());
                    handle(timed.payload);
                }
            }));
        }

        Listener { tasks, taps }
    }

    pub fn port_count(&self) -> usize {
        self.tasks.len()
    }

    /// Opens a [`Link`] that receives from this listener's sources and sends
    /// to every destination.
    pub async fn link(&self, client: &Client) -> Link {
        let (tx, rx) = unbounded_channel();
        self.taps.lock().unwrap_or_else(|e| e.into_inner()).push(tx);

        Link {
            outputs: connect_destinations(client).await,
            input_count: self.port_count(),
            rx,
            _inputs: None,
        }
    }

    /// Disconnects every source and waits until they are released, so the
    /// same client can connect them again.
    pub async fn close(mut self) {
        let tasks = std::mem::take(&mut self.tasks);
        for task in &tasks {
            task.abort();
        }
        for task in tasks {
            let _ = task.await;
        }
    }
}

async fn connect_destinations(client: &Client) -> Vec<DestinationConnection> {
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
    outputs
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
/// in. Dropping the `Link` disconnects the ports it opened.
pub struct Link {
    outputs: Vec<DestinationConnection>,
    input_count: usize,
    rx: UnboundedReceiver<SysEx>,
    _inputs: Option<Listener>,
}

impl Link {
    /// Connects every source, then every destination. Ports that fail to
    /// list or connect are skipped: absence of a transport is reported
    /// through [`Link::output_count`] and [`Link::input_count`].
    ///
    /// A client that keeps a [`Listener`] open must use [`Listener::link`]
    /// instead, since its sources are already connected.
    pub async fn open(client: &Client) -> Link {
        let inputs = Listener::open(client, || |_: SysEx| {}).await;
        let mut link = inputs.link(client).await;
        link._inputs = Some(inputs);
        link
    }

    pub fn output_count(&self) -> usize {
        self.outputs.len()
    }

    pub fn input_count(&self) -> usize {
        self.input_count
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
            input_count: 0,
            rx,
            _inputs: None,
        };

        link.send(&SysEx::new(&[0x7D, 0x01, 0x02]).unwrap())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn listener_shares_sources_with_its_links() {
        let client = Client::new("listener-links").await.unwrap();
        let device_out = client
            .create_virtual_source("Listener Device Out")
            .await
            .unwrap();

        let (heard_tx, mut heard) = unbounded_channel();
        let listener = Listener::open(&client, || {
            let heard_tx = heard_tx.clone();
            move |sysex| {
                let _ = heard_tx.send(sysex);
            }
        })
        .await;
        assert!(listener.port_count() >= 1);

        let mut first = listener.link(&client).await;
        let mut second = listener.link(&client).await;
        assert_eq!(first.input_count(), listener.port_count());

        let reply = SysEx::new(&[0x7D, 0x0A, 0x0B]).unwrap();
        device_out.send_sysex(&reply).await.unwrap();

        let wait = Duration::from_secs(5);
        let matched = |s: SysEx| (s == reply).then_some(s);
        assert_eq!(first.recv(wait, matched).await, Some(reply.clone()));
        assert_eq!(second.recv(wait, matched).await, Some(reply.clone()));
        let heard_reply = timeout(wait, async {
            while let Some(s) = heard.recv().await {
                if s == reply {
                    return s;
                }
            }
            unreachable!()
        })
        .await
        .unwrap();
        assert_eq!(heard_reply, reply);

        listener.close().await;
        let reopened = Listener::open(&client, || |_: SysEx| {}).await;
        let mut after_reopen = reopened.link(&client).await;
        device_out.send_sysex(&reply).await.unwrap();
        assert_eq!(after_reopen.recv(wait, matched).await, Some(reply.clone()));
    }
}
