//! Hardware-in-loop tests for the Nektar Impact LX61+.
//!
//! The LX+ has no dump-request sysex: reading device memory requires pressing
//! [Setup] followed by the *Memory Dump* key (G2) on the device. Each test
//! prints instructions and waits for the dump, so run them interactively:
//!
//! ```sh
//! cargo test -p midilab-io --test hil_nektar_impact_lx_plus -- --ignored --nocapture --test-threads=1
//! ```

use std::time::Duration;

use midi_io::Client;
use midi_io::SysEx;
use midilab::manufacturer::nektar::impact_lx_plus::DUMP_MESSAGE_COUNT;
use midilab::manufacturer::nektar::impact_lx_plus::DeviceStatus;
use midilab::manufacturer::nektar::impact_lx_plus::Dump;
use midilab::manufacturer::nektar::impact_lx_plus::DumpAssembler;
use midilab::manufacturer::nektar::impact_lx_plus::is_impact_lx_plus_sysex;
use midilab_io::midi::Link;

/// Long enough for the user to walk to the device and trigger the dump.
const DUMP_START_TIMEOUT: Duration = Duration::from_secs(120);
/// The device streams the remaining messages promptly once started.
const DUMP_MESSAGE_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_PACING: Duration = Duration::from_millis(2);

async fn midi_setup() -> Client {
    Client::new("impact_lx_plus").await.unwrap()
}

async fn next_lx_plus_sysex(link: &mut Link, wait: Duration) -> Option<SysEx> {
    link.recv(wait, |s| is_impact_lx_plus_sysex(&s).then_some(s))
        .await
}

async fn capture_dump(client: &Client) -> Vec<SysEx> {
    let mut link = Link::open(client).await;
    println!();
    println!(">>> On the keyboard: press [Setup], then the key labeled *Memory Dump* (G2).");
    println!(">>> The display reads SYS while the dump is sent.");
    println!();

    let mut messages = Vec::with_capacity(DUMP_MESSAGE_COUNT);
    let first = next_lx_plus_sysex(&mut link, DUMP_START_TIMEOUT)
        .await
        .expect("timed out waiting for the memory dump to start");
    messages.push(first);

    while messages.len() < DUMP_MESSAGE_COUNT {
        let message = next_lx_plus_sysex(&mut link, DUMP_MESSAGE_TIMEOUT)
            .await
            .unwrap_or_else(|| {
                panic!(
                    "dump stalled after {} of {DUMP_MESSAGE_COUNT} messages",
                    messages.len()
                )
            });
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
    let client = midi_setup().await;

    let captured = capture_dump(&client).await;

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
    let client = midi_setup().await;

    println!("first capture:");
    let original = capture_dump(&client).await;
    let dump = assemble(&original);

    println!(
        "replaying {} messages back to the device...",
        original.len()
    );
    Link::open(&client)
        .await
        .send_paced(dump.to_messages(), WRITE_PACING)
        .await
        .unwrap();

    println!("second capture (verifies the replay):");
    let restored = capture_dump(&client).await;
    assert_eq!(original, restored);
}
