use std::time::Duration;

use midi_io::Client;
use midilab::manufacturer::akai::mpd226::DeviceStatus;
use midilab::manufacturer::akai::mpd226::Global;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::control::value_kind::PresetName;
use midilab::manufacturer::akai::mpd226::dump_global_from_device;
use midilab::manufacturer::akai::mpd226::dump_preset_from_device;
use midilab::manufacturer::akai::mpd226::raw::RawGlobal;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;
use midilab::manufacturer::akai::mpd226::reply_to;
use midilab::manufacturer::akai::mpd226::write_preset_to_device;
use midilab::midi::Note;
use midilab_io::midi::Link;

const TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_PACING: Duration = Duration::from_millis(2);

async fn midi_setup() -> Client {
    Client::new("mpd226").await.unwrap()
}

async fn read_preset(client: &Client) -> Preset {
    let mut link = Link::open(client).await;
    let request = dump_preset_from_device(0x00);
    link.send(&request).await.unwrap();
    match link.recv(TIMEOUT, |s| reply_to(&request, s)).await {
        Some(DeviceStatus::PresetData(p)) => *p,
        Some(_) => panic!("wrong variant"),
        None => panic!("no preset dump from the MPD226"),
    }
}

async fn write_preset(client: &Client, preset: &Preset) {
    let mut link = Link::open(client).await;
    let raw = RawPreset::from(preset);
    link.send(&write_preset_to_device(&raw)).await.unwrap();
    let ack = link
        .recv(TIMEOUT, |s| {
            matches!(
                DeviceStatus::try_from(s),
                Ok(DeviceStatus::ReceivedPresetAck(_))
            )
            .then_some(())
        })
        .await;
    assert!(ack.is_some(), "no preset ack from the MPD226");
}

#[ignore = "requires connected MPD226"]
#[tokio::test]
async fn preset_round_trip() {
    let client = midi_setup().await;

    let original = read_preset(&client).await;

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

    write_preset(&client, &mutated).await;
    let loaded = read_preset(&client).await;

    assert_eq!(loaded.settings.name.0, *b"HILTEST ");
    assert_eq!(loaded.pads.pads[0].note, Note::from(72));
    assert_eq!(loaded.pads.pads[1].note, Note::from(84));
    assert_eq!(loaded.dials.0[0].midicc, 50.into());
    assert_eq!(loaded.dials.0[1].midicc, 51.into());
    assert_eq!(loaded.faders.0[0].midicc, 60.into());
    assert_eq!(loaded.faders.0[1].midicc, 61.into());
    assert_eq!(loaded.switches.0[0].midicc, 70.into());
    assert_eq!(loaded.switches.0[1].midicc, 71.into());

    write_preset(&client, &original).await;
    let restored = read_preset(&client).await;
    let raw_original = RawPreset::from(&original);
    let raw_restored = RawPreset::from(&restored);
    assert_eq!(
        bytemuck::bytes_of(&raw_original),
        bytemuck::bytes_of(&raw_restored)
    );
}

async fn send_global(client: &Client, global: &Global) {
    let link = Link::open(client).await;
    let raw = RawGlobal::from(global);
    link.send_paced(raw.global_send_messages(), WRITE_PACING)
        .await
        .unwrap();
}

async fn read_global(client: &Client) -> Global {
    let mut link = Link::open(client).await;
    let request = dump_global_from_device();
    link.send(&request).await.unwrap();
    match link.recv(TIMEOUT, |s| reply_to(&request, s)).await {
        Some(DeviceStatus::GlobalData(g)) => *g,
        Some(_) => panic!("wrong variant"),
        None => panic!("no global dump from the MPD226"),
    }
}

#[ignore = "requires connected MPD226"]
#[tokio::test]
async fn global_round_trip() {
    let client = midi_setup().await;
    let device_original = read_global(&client).await;

    send_global(&client, &Global::default()).await;
    let loaded_default = read_global(&client).await;
    assert_eq!(loaded_default, Global::default());

    send_global(&client, &device_original).await;
    let restored = read_global(&client).await;
    assert_eq!(restored, device_original);
}
