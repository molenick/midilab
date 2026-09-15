use std::time::Duration;

use midi_io::Client;
use midi_io::SysEx;
use midilab::manufacturer::arturia::minilab_mk2::DeviceStatus;
use midilab::manufacturer::arturia::minilab_mk2::Global;
use midilab::manufacturer::arturia::minilab_mk2::ParamId;
use midilab::manufacturer::arturia::minilab_mk2::ParamStore;
use midilab::manufacturer::arturia::minilab_mk2::Preset;
use midilab::manufacturer::arturia::minilab_mk2::SYSEX_COMMAND_HEADER;
use midilab::manufacturer::arturia::minilab_mk2::control::ControlId;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::MemorySlot;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::PadColor;
use midilab::manufacturer::arturia::minilab_mk2::identity_request_message;
use midilab::manufacturer::arturia::minilab_mk2::read_param_message;
use midilab::manufacturer::arturia::minilab_mk2::recall_memory_message;
use midilab::manufacturer::arturia::minilab_mk2::reply_to;
use midilab::manufacturer::arturia::minilab_mk2::set_pad_live_color_message;
use midilab::manufacturer::arturia::minilab_mk2::write_param_message;
use midilab_io::midi::Link;

const TIMEOUT: Duration = Duration::from_secs(5);
const PROBE_TIMEOUT: Duration = Duration::from_millis(500);
const WRITE_PACING: Duration = Duration::from_millis(2);

async fn midi_setup() -> Client {
    Client::new("minilab_mk2").await.unwrap()
}

async fn request(client: &Client, message: SysEx) -> DeviceStatus {
    let mut link = Link::open(client).await;
    link.send(&message).await.unwrap();
    link.recv(TIMEOUT, |s| reply_to(&message, s))
        .await
        .expect("no reply from the MiniLab")
}

async fn send_and_probe(client: &Client, sent: &SysEx) -> Option<SysEx> {
    let mut link = Link::open(client).await;
    link.send(sent).await.unwrap();
    link.recv(PROBE_TIMEOUT, |s| {
        (s != *sent && s.bytes().starts_with(&SYSEX_COMMAND_HEADER)).then_some(s)
    })
    .await
}

async fn read_param(client: &Client, param: ParamId, control: ControlId) -> u8 {
    match request(client, read_param_message(param, control)).await {
        DeviceStatus::ParamValue(pv) => {
            assert_eq!(pv.param, param);
            assert_eq!(pv.control, control);
            pv.value
        }
        other => panic!("expected param value, got {other:?}"),
    }
}

async fn read_full_preset(client: &Client) -> Preset {
    let mut store = ParamStore::default();
    for message in Preset::read_messages() {
        store.apply(&request(client, message).await);
    }
    store.try_into_preset().unwrap()
}

async fn read_full_global(client: &Client) -> Global {
    let mut store = ParamStore::default();
    for message in Global::read_messages() {
        store.apply(&request(client, message).await);
    }
    store.try_into_global().unwrap()
}

async fn send_paced(client: &Client, messages: impl IntoIterator<Item = SysEx>) {
    Link::open(client)
        .await
        .send_paced(messages, WRITE_PACING)
        .await
        .unwrap();
}

async fn write_full_preset(client: &Client, preset: &Preset) {
    send_paced(client, preset.send_messages()).await;
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn probe_identity() {
    let client = midi_setup().await;

    let mut link = Link::open(&client).await;
    link.send(&identity_request_message()).await.unwrap();
    let reply = link
        .recv(TIMEOUT, |s| match DeviceStatus::try_from(s) {
            Ok(DeviceStatus::IdentityReply(reply)) => Some(reply),
            _ => None,
        })
        .await
        .expect("no identity reply from the MiniLab");
    println!("firmware: {:?}", reply.firmware);
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn probe_write_ack_behavior() {
    let client = midi_setup().await;

    let original = read_param(&client, ParamId::Data1, ControlId::Knob2).await;
    println!("knob2 cc: {original}");

    let write = write_param_message(ParamId::Data1, ControlId::Knob2, original);
    match send_and_probe(&client, &write).await {
        Some(reply) => println!("write produced a reply: {reply:02X?}"),
        None => println!("write produced no reply"),
    }
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn probe_shift_and_padbank_control_ids() {
    let client = midi_setup().await;

    for candidate in [0x2Eu8, 0x2F, 0x55, 0x56] {
        let message =
            SysEx::new(&[0x00, 0x20, 0x6B, 0x7F, 0x42, 0x01, 0x00, 0x01, candidate]).unwrap();
        match send_and_probe(&client, &message).await {
            Some(reply) => println!("control {candidate:#04x} replied: {reply:02X?}"),
            None => println!("control {candidate:#04x} no reply"),
        }
    }
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn probe_pad_color_params() {
    let client = midi_setup().await;

    let stored = read_param(&client, ParamId::PadColor, ControlId::Pad1).await;
    println!("pad1 stored color (0x11): {stored:#04x}");

    let live = set_pad_live_color_message(ControlId::Pad1, PadColor::Cyan);
    match send_and_probe(&client, &live).await {
        Some(reply) => println!("live color write (0x10) replied: {reply:02X?}"),
        None => println!("live color write (0x10) no reply (check pad 1 lights cyan)"),
    }

    let after = read_param(&client, ParamId::PadColor, ControlId::Pad1).await;
    println!("pad1 stored color after live write: {after:#04x}");
    assert_eq!(stored, after);
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn param_round_trip() {
    let client = midi_setup().await;

    let original = read_param(&client, ParamId::Data1, ControlId::Knob2).await;
    let mutated = if original == 0x7F { 0x00 } else { original + 1 };

    send_paced(
        &client,
        [write_param_message(
            ParamId::Data1,
            ControlId::Knob2,
            mutated,
        )],
    )
    .await;

    let loaded = read_param(&client, ParamId::Data1, ControlId::Knob2).await;
    assert_eq!(loaded, mutated);

    send_paced(
        &client,
        [write_param_message(
            ParamId::Data1,
            ControlId::Knob2,
            original,
        )],
    )
    .await;

    let restored = read_param(&client, ParamId::Data1, ControlId::Knob2).await;
    assert_eq!(restored, original);
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn preset_model_round_trip() {
    let client = midi_setup().await;

    let original = read_full_preset(&client).await;

    write_full_preset(&client, &original).await;

    let reloaded = read_full_preset(&client).await;
    assert_eq!(original, reloaded);
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn global_round_trip() {
    let client = midi_setup().await;

    let original = read_full_global(&client).await;
    println!("global: {original:?}");

    send_paced(&client, original.send_messages()).await;

    let reloaded = read_full_global(&client).await;

    assert_eq!(original, reloaded);
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn probe_memory_recall() {
    let client = midi_setup().await;

    let working = read_full_preset(&client).await;

    Link::open(&client)
        .await
        .send(&recall_memory_message(MemorySlot::Slot2))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let recalled = read_full_preset(&client).await;
    println!("recall changed working memory: {}", working != recalled);

    write_full_preset(&client, &working).await;

    let restored = read_full_preset(&client).await;
    assert_eq!(working, restored);
}
