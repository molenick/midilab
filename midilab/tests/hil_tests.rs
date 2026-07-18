use std::time::Duration;

use midi_io::Client;
use midi_io::DestinationConnection;
use midi_io::SysEx;
use midilab::manufacturer::akai::mpd226::DeviceStatus;
use midilab::manufacturer::akai::mpd226::Global;
use midilab::manufacturer::akai::mpd226::PORT_NAME;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::control::value_kind::PresetName;
use midilab::manufacturer::akai::mpd226::dump_global_from_device;
use midilab::manufacturer::akai::mpd226::dump_preset_from_device;
use midilab::manufacturer::akai::mpd226::raw::RawGlobal;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;
use midilab::manufacturer::akai::mpd226::write_preset_to_device;
use midilab::midi::Note;
use tokio::sync::mpsc;
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_secs(5);

async fn midi_setup() -> (DestinationConnection, mpsc::UnboundedReceiver<SysEx>) {
    let client = Client::new("mpd226").await.unwrap();

    let out_port = client
        .destinations()
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.name() == PORT_NAME)
        .unwrap();
    let conn_out = client.connect_destination(&out_port).await.unwrap();

    let in_port = client
        .sources()
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.name() == PORT_NAME)
        .unwrap();
    let conn_in = client.connect_source(&in_port).await.unwrap();

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

#[ignore = "requires connected MPD226"]
#[tokio::test]
async fn preset_round_trip() {
    let (conn, mut rx) = midi_setup().await;

    let original = {
        send(&conn, &dump_preset_from_device(0x00)).await;
        let res = {
            let data = recv_bytes(&mut rx).await;
            DeviceStatus::try_from(data).unwrap()
        };
        match res {
            DeviceStatus::PresetData(p) => *p,
            _ => panic!("wrong variant"),
        }
    };

    let mut mutated = original;
    mutated.settings.name = PresetName(*b"HILTEST ");
    mutated.pads.pads[0].note = Note::from(72);
    mutated.pads.pads[1].note = Note::from(84);
    mutated.dials.0[0].midicc = 50.into();
    mutated.dials.0[1].midicc = 51.into();
    mutated.faders.0[0].midicc = 60.into();
    mutated.faders.0[1].midicc = 61.into();
    mutated.switches.0[0].midicc = 70.into();
    mutated.switches.0[1].midicc = 71.into();

    {
        let preset: &Preset = &mutated;
        let raw = RawPreset::from(preset);
        send(&conn, &write_preset_to_device(&raw)).await;
        let res = {
            let data = recv_bytes(&mut rx).await;
            DeviceStatus::try_from(data).unwrap()
        };
        match res {
            DeviceStatus::ReceivedPresetAck(_) => {}
            _ => panic!("wrong variant"),
        }
    };
    let loaded = {
        send(&conn, &dump_preset_from_device(0x00)).await;
        let res = {
            let data = recv_bytes(&mut rx).await;
            DeviceStatus::try_from(data).unwrap()
        };
        match res {
            DeviceStatus::PresetData(p) => *p,
            _ => panic!("wrong variant"),
        }
    };

    assert_eq!(loaded.settings.name.0, *b"HILTEST ");
    assert_eq!(loaded.pads.pads[0].note, Note::from(72));
    assert_eq!(loaded.pads.pads[1].note, Note::from(84));
    assert_eq!(loaded.dials.0[0].midicc, 50.into());
    assert_eq!(loaded.dials.0[1].midicc, 51.into());
    assert_eq!(loaded.faders.0[0].midicc, 60.into());
    assert_eq!(loaded.faders.0[1].midicc, 61.into());
    assert_eq!(loaded.switches.0[0].midicc, 70.into());
    assert_eq!(loaded.switches.0[1].midicc, 71.into());

    {
        let preset: &Preset = &original;
        let raw = RawPreset::from(preset);
        send(&conn, &write_preset_to_device(&raw)).await;
        let res = {
            let data = recv_bytes(&mut rx).await;
            DeviceStatus::try_from(data).unwrap()
        };
        match res {
            DeviceStatus::ReceivedPresetAck(_) => {}
            _ => panic!("wrong variant"),
        }
    };
    let restored = {
        send(&conn, &dump_preset_from_device(0x00)).await;
        let res = {
            let data = recv_bytes(&mut rx).await;
            DeviceStatus::try_from(data).unwrap()
        };
        match res {
            DeviceStatus::PresetData(p) => *p,
            _ => panic!("wrong variant"),
        }
    };
    let raw_original = RawPreset::from(&original);
    let raw_restored = RawPreset::from(&restored);
    assert_eq!(
        bytemuck::bytes_of(&raw_original),
        bytemuck::bytes_of(&raw_restored)
    );
}

async fn send_global(
    conn: &DestinationConnection,
    rx: &mut mpsc::UnboundedReceiver<SysEx>,
    global: &Global,
) {
    let raw = RawGlobal::from(global);
    for msg in raw.global_send_messages() {
        send(conn, &msg).await;
        let data = recv_bytes(rx).await;
        let res = DeviceStatus::try_from(data).unwrap();
        match res {
            DeviceStatus::GlobalParamAck(_) => {}
            _ => panic!("wrong variant"),
        }
    }
}

async fn read_global(
    conn: &DestinationConnection,
    rx: &mut mpsc::UnboundedReceiver<SysEx>,
) -> Global {
    send(conn, &dump_global_from_device()).await;
    let data = recv_bytes(rx).await;
    let res = DeviceStatus::try_from(data).unwrap();
    match res {
        DeviceStatus::GlobalData(g) => *g,
        _ => panic!("wrong variant"),
    }
}

#[ignore = "requires connected MPD226"]
#[tokio::test]
async fn global_round_trip() {
    let (conn, mut rx) = midi_setup().await;
    let device_original = read_global(&conn, &mut rx).await;

    send_global(&conn, &mut rx, &Global::default()).await;
    let loaded_default = read_global(&conn, &mut rx).await;
    assert_eq!(loaded_default, Global::default());

    send_global(&conn, &mut rx, &device_original).await;
    let restored = read_global(&conn, &mut rx).await;
    assert_eq!(restored, device_original);
}
