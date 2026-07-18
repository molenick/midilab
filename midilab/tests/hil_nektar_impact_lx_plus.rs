//! Hardware-in-loop tests for the Nektar Impact LX61+.
//!
//! The LX+ has no dump-request sysex: reading device memory requires pressing
//! [Setup] followed by the *Memory Dump* key (G2) on the device. Each test
//! prints instructions and waits for the dump, so run them interactively:
//!
//! ```sh
//! cargo test -p midilab --test hil_nektar_impact_lx_plus -- --ignored --nocapture --test-threads=1
//! ```

use std::time::Duration;

use midi_io::Client;
use midi_io::DestinationConnection;
use midi_io::SysEx;
use midilab::manufacturer::nektar::impact_lx_plus::DUMP_MESSAGE_COUNT;
use midilab::manufacturer::nektar::impact_lx_plus::DeviceStatus;
use midilab::manufacturer::nektar::impact_lx_plus::Dump;
use midilab::manufacturer::nektar::impact_lx_plus::DumpAssembler;
use midilab::manufacturer::nektar::impact_lx_plus::is_sysex_port;
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Long enough for the user to walk to the device and trigger the dump.
const DUMP_START_TIMEOUT: Duration = Duration::from_secs(120);
/// The device streams the remaining messages promptly once started.
const DUMP_MESSAGE_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_PACING: Duration = Duration::from_millis(2);

async fn midi_setup() -> (DestinationConnection, mpsc::UnboundedReceiver<SysEx>) {
    let client = Client::new("impact_lx_plus").await.unwrap();

    let destinations = client.destinations().await.unwrap();
    let out_port = destinations
        .iter()
        .find(|p| is_sysex_port(p.name()))
        .unwrap_or_else(|| {
            let names: Vec<&str> = destinations.iter().map(|p| p.name()).collect();
            panic!("no Impact LX+ sysex destination found, available: {names:?}");
        });
    println!("matched destination port: {:?}", out_port.name());
    let conn_out = client.connect_destination(out_port).await.unwrap();

    let sources = client.sources().await.unwrap();
    let in_port = sources
        .iter()
        .find(|p| is_sysex_port(p.name()))
        .unwrap_or_else(|| {
            let names: Vec<&str> = sources.iter().map(|p| p.name()).collect();
            panic!("no Impact LX+ sysex source found, available: {names:?}");
        });
    println!("matched source port: {:?}", in_port.name());
    let conn_in = client.connect_source(in_port).await.unwrap();

    let (tx, mut rx) = mpsc::unbounded_channel::<SysEx>();
    tokio::spawn(async move {
        let mut sysex = conn_in.into_sysex();
        while let Some(timed) = sysex.recv().await {
            let _ = tx.send(timed.payload);
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    (conn_out, rx)
}

async fn send(conn: &DestinationConnection, sysex: &SysEx) {
    conn.send_sysex(sysex).await.unwrap();
}

async fn capture_dump(rx: &mut mpsc::UnboundedReceiver<SysEx>) -> Vec<SysEx> {
    println!();
    println!(">>> On the keyboard: press [Setup], then the key labeled *Memory Dump* (G2).");
    println!(">>> The display reads SYS while the dump is sent.");
    println!();

    let mut messages = Vec::with_capacity(DUMP_MESSAGE_COUNT);
    let first = timeout(DUMP_START_TIMEOUT, rx.recv())
        .await
        .expect("timed out waiting for the memory dump to start")
        .unwrap();
    messages.push(first);

    while messages.len() < DUMP_MESSAGE_COUNT {
        let message = timeout(DUMP_MESSAGE_TIMEOUT, rx.recv())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "dump stalled after {} of {DUMP_MESSAGE_COUNT} messages",
                    messages.len()
                )
            })
            .unwrap();
        messages.push(message);
    }
    println!("captured {} messages", messages.len());
    messages
}

fn assemble(messages: &[SysEx]) -> Dump {
    let mut assembler = DumpAssembler::default();
    for message in messages {
        let status = DeviceStatus::try_from(message.clone()).unwrap();
        assembler.apply(&status);
    }
    assert!(assembler.is_complete());
    assembler.try_into_dump().unwrap()
}

/// Every dump message must decode into the typed model and re-encode to the
/// exact captured bytes, and the assembled dump must re-emit the capture in
/// canonical order.
#[ignore = "requires connected Impact LX+ and a panel-triggered memory dump"]
#[tokio::test]
async fn dump_model_round_trip() {
    let (_conn, mut rx) = midi_setup().await;

    let captured = capture_dump(&mut rx).await;

    for (index, message) in captured.iter().enumerate() {
        let status = DeviceStatus::try_from(message.clone()).unwrap();
        assert_eq!(
            &status.message(),
            message,
            "message {index} re-encode differs"
        );
    }

    let dump = assemble(&captured);
    let encoded = dump.to_messages();
    assert_eq!(encoded, captured, "canonical order re-encode differs");
}

/// Replays the captured dump back to the device, then verifies with a second
/// panel-triggered dump that stored memory is byte-identical.
#[ignore = "requires connected Impact LX+ and two panel-triggered memory dumps"]
#[tokio::test]
async fn dump_restore_round_trip() {
    let (conn, mut rx) = midi_setup().await;

    println!("first capture:");
    let original = capture_dump(&mut rx).await;
    let dump = assemble(&original);

    println!(
        "replaying {} messages back to the device...",
        original.len()
    );
    for message in dump.to_messages() {
        send(&conn, &message).await;
        tokio::time::sleep(WRITE_PACING).await;
    }
    while rx.try_recv().is_ok() {}

    println!("second capture (verifies the replay):");
    let restored = capture_dump(&mut rx).await;
    assert_eq!(original, restored);
}
