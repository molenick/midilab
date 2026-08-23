use std::time::Duration;

use midi_io::Client;
use midi_io::DestinationConnection;
use midi_io::SysEx;
use midilab::manufacturer::arturia::minilab_mk2::DeviceStatus;
use midilab::manufacturer::arturia::minilab_mk2::Global;
use midilab::manufacturer::arturia::minilab_mk2::ParamId;
use midilab::manufacturer::arturia::minilab_mk2::ParamStore;
use midilab::manufacturer::arturia::minilab_mk2::Preset;
use midilab::manufacturer::arturia::minilab_mk2::control::ControlId;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::MemorySlot;
use midilab::manufacturer::arturia::minilab_mk2::control::value_kind::PadColor;
use midilab::manufacturer::arturia::minilab_mk2::identity_request_message;
use midilab::manufacturer::arturia::minilab_mk2::read_param_message;
use midilab::manufacturer::arturia::minilab_mk2::recall_memory_message;
use midilab::manufacturer::arturia::minilab_mk2::set_pad_live_color_message;
use midilab::manufacturer::arturia::minilab_mk2::write_param_message;
use tokio::sync::mpsc;
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_secs(5);
const PROBE_TIMEOUT: Duration = Duration::from_millis(500);
const WRITE_PACING: Duration = Duration::from_millis(2);
const PORT_NAME_FRAGMENT: &str = "MiniLab";

async fn midi_setup() -> (DestinationConnection, mpsc::UnboundedReceiver<SysEx>) {
    let client = Client::new("minilab_mk2").await.unwrap();

    let destinations = client.destinations().await.unwrap();
    let out_port = destinations
        .iter()
        .find(|p| p.name().contains(PORT_NAME_FRAGMENT))
        .unwrap_or_else(|| {
            let names: Vec<&str> = destinations.iter().map(|p| p.name()).collect();
            panic!("no MiniLab destination found, available: {names:?}");
        });
    println!("matched destination port: {:?}", out_port.name());
    let conn_out = client.connect_destination(out_port).await.unwrap();

    let sources = client.sources().await.unwrap();
    let in_port = sources
        .iter()
        .find(|p| p.name().contains(PORT_NAME_FRAGMENT))
        .unwrap_or_else(|| {
            let names: Vec<&str> = sources.iter().map(|p| p.name()).collect();
            panic!("no MiniLab source found, available: {names:?}");
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

async fn recv_bytes(rx: &mut mpsc::UnboundedReceiver<SysEx>) -> SysEx {
    timeout(TIMEOUT, rx.recv()).await.unwrap().unwrap()
}

async fn try_recv_bytes(rx: &mut mpsc::UnboundedReceiver<SysEx>) -> Option<SysEx> {
    timeout(PROBE_TIMEOUT, rx.recv()).await.ok().flatten()
}

async fn read_param(
    conn: &DestinationConnection,
    rx: &mut mpsc::UnboundedReceiver<SysEx>,
    param: ParamId,
    control: ControlId,
) -> u8 {
    send(conn, &read_param_message(param, control)).await;
    let data = recv_bytes(rx).await;
    match DeviceStatus::try_from(data).unwrap() {
        DeviceStatus::ParamValue(pv) => {
            assert_eq!(pv.param, param);
            assert_eq!(pv.control, control);
            pv.value
        }
        other => panic!("expected param value, got {other:?}"),
    }
}

async fn read_full_preset(
    conn: &DestinationConnection,
    rx: &mut mpsc::UnboundedReceiver<SysEx>,
) -> Preset {
    let mut store = ParamStore::default();
    for message in Preset::read_messages() {
        send(conn, &message).await;
        let data = recv_bytes(rx).await;
        let status = DeviceStatus::try_from(data).unwrap();
        store.apply(&status);
    }
    store.try_into_preset().unwrap()
}

async fn write_full_preset(
    conn: &DestinationConnection,
    rx: &mut mpsc::UnboundedReceiver<SysEx>,
    preset: &Preset,
) {
    for message in preset.send_messages() {
        send(conn, &message).await;
        tokio::time::sleep(WRITE_PACING).await;
    }
    while rx.try_recv().is_ok() {}
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn probe_identity() {
    let (conn, mut rx) = midi_setup().await;

    send(&conn, &identity_request_message()).await;
    let data = recv_bytes(&mut rx).await;
    println!("identity reply: {data:02X?}");

    let status = DeviceStatus::try_from(data).unwrap();
    let DeviceStatus::IdentityReply(reply) = status else {
        panic!("expected identity reply, got {status:?}");
    };
    println!("firmware: {:?}", reply.firmware);
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn probe_write_ack_behavior() {
    let (conn, mut rx) = midi_setup().await;

    let original = read_param(&conn, &mut rx, ParamId::Data1, ControlId::Knob2).await;
    println!("knob2 cc: {original}");

    send(
        &conn,
        &write_param_message(ParamId::Data1, ControlId::Knob2, original),
    )
    .await;
    match try_recv_bytes(&mut rx).await {
        Some(reply) => println!("write produced a reply: {reply:02X?}"),
        None => println!("write produced no reply"),
    }
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn probe_shift_and_padbank_control_ids() {
    let (conn, mut rx) = midi_setup().await;

    for candidate in [0x2Eu8, 0x2F, 0x55, 0x56] {
        let message =
            SysEx::new(&[0x00, 0x20, 0x6B, 0x7F, 0x42, 0x01, 0x00, 0x01, candidate]).unwrap();
        send(&conn, &message).await;
        match try_recv_bytes(&mut rx).await {
            Some(reply) => println!("control {candidate:#04x} replied: {reply:02X?}"),
            None => println!("control {candidate:#04x} no reply"),
        }
    }
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn probe_pad_color_params() {
    let (conn, mut rx) = midi_setup().await;

    let stored = read_param(&conn, &mut rx, ParamId::PadColor, ControlId::Pad1).await;
    println!("pad1 stored color (0x11): {stored:#04x}");

    send(
        &conn,
        &set_pad_live_color_message(ControlId::Pad1, PadColor::Cyan),
    )
    .await;
    match try_recv_bytes(&mut rx).await {
        Some(reply) => println!("live color write (0x10) replied: {reply:02X?}"),
        None => println!("live color write (0x10) no reply (check pad 1 lights cyan)"),
    }

    let after = read_param(&conn, &mut rx, ParamId::PadColor, ControlId::Pad1).await;
    println!("pad1 stored color after live write: {after:#04x}");
    assert_eq!(stored, after);
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn param_round_trip() {
    let (conn, mut rx) = midi_setup().await;

    let original = read_param(&conn, &mut rx, ParamId::Data1, ControlId::Knob2).await;
    let mutated = if original == 0x7F { 0x00 } else { original + 1 };

    send(
        &conn,
        &write_param_message(ParamId::Data1, ControlId::Knob2, mutated),
    )
    .await;
    tokio::time::sleep(WRITE_PACING).await;
    while rx.try_recv().is_ok() {}

    let loaded = read_param(&conn, &mut rx, ParamId::Data1, ControlId::Knob2).await;
    assert_eq!(loaded, mutated);

    send(
        &conn,
        &write_param_message(ParamId::Data1, ControlId::Knob2, original),
    )
    .await;
    tokio::time::sleep(WRITE_PACING).await;
    while rx.try_recv().is_ok() {}

    let restored = read_param(&conn, &mut rx, ParamId::Data1, ControlId::Knob2).await;
    assert_eq!(restored, original);
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn preset_model_round_trip() {
    let (conn, mut rx) = midi_setup().await;

    let original = read_full_preset(&conn, &mut rx).await;

    write_full_preset(&conn, &mut rx, &original).await;

    let reloaded = read_full_preset(&conn, &mut rx).await;
    assert_eq!(original, reloaded);
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn global_round_trip() {
    let (conn, mut rx) = midi_setup().await;

    let mut store = ParamStore::default();
    for message in Global::read_messages() {
        send(&conn, &message).await;
        let data = recv_bytes(&mut rx).await;
        store.apply(&DeviceStatus::try_from(data).unwrap());
    }
    let original = store.try_into_global().unwrap();
    println!("global: {original:?}");

    for message in original.send_messages() {
        send(&conn, &message).await;
        tokio::time::sleep(WRITE_PACING).await;
    }
    while rx.try_recv().is_ok() {}

    let mut store = ParamStore::default();
    for message in Global::read_messages() {
        send(&conn, &message).await;
        let data = recv_bytes(&mut rx).await;
        store.apply(&DeviceStatus::try_from(data).unwrap());
    }
    let reloaded = store.try_into_global().unwrap();

    assert_eq!(original, reloaded);
}

#[ignore = "requires connected MiniLab mkII"]
#[tokio::test]
async fn probe_memory_recall() {
    let (conn, mut rx) = midi_setup().await;

    let working = read_full_preset(&conn, &mut rx).await;

    send(&conn, &recall_memory_message(MemorySlot::Slot2)).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    let recalled = read_full_preset(&conn, &mut rx).await;
    println!("recall changed working memory: {}", working != recalled);

    write_full_preset(&conn, &mut rx, &working).await;

    let restored = read_full_preset(&conn, &mut rx).await;
    assert_eq!(working, restored);
}
