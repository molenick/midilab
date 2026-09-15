//! End-to-end test for sysex on a shared bus (molenick/midilab#120).
//!
//! The MPD226 is "connected" through a MIDI hub: the host only sees ports
//! with generic names. The editor sends its request to every port and
//! recognises the MPD226 by its reply, skipping everything else on the bus:
//! the request looped back by the hub and traffic from other devices.

use std::time::Duration;

use midi_io::Client;
use midi_io::SysEx;
use midilab::manufacturer::akai::mpd226::DeviceStatus;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::dump_global_from_device;
use midilab::manufacturer::akai::mpd226::dump_preset_from_device;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;
use midilab::manufacturer::akai::mpd226::reply_to;
use midilab::manufacturer::akai::mpd226::write_preset_to_device;
use midilab_io::midi::Link;

#[tokio::test]
async fn mpd226_dump_through_hub() {
    let client = Client::new("mpd226-hub-test").await.unwrap();

    let hub_out = client
        .create_virtual_destination("Hub MIDI Out")
        .await
        .unwrap();
    let hub_in = client.create_virtual_source("Hub MIDI In").await.unwrap();
    let mut requests = hub_out.into_sysex();

    let device_task = tokio::spawn(async move {
        while let Some(timed) = requests.recv().await {
            let request = timed.payload;

            hub_in.send_sysex(&request).await.unwrap();

            let noise = SysEx::new(&[0x42, 0x00, 0x06, 0x07, 0x00, 0x00]).unwrap();
            hub_in.send_sysex(&noise).await.unwrap();

            let raw: RawPreset = (&Preset::default()).into();
            hub_in
                .send_sysex(&write_preset_to_device(&raw))
                .await
                .unwrap();
        }
    });

    let mut link = Link::open(&client).await;
    let request = dump_preset_from_device(0x00);
    link.send(&request).await.unwrap();

    let response = link
        .recv(Duration::from_secs(5), |s| reply_to(&request, s))
        .await
        .expect("MPD226 response through the hub");
    assert!(
        matches!(response, DeviceStatus::PresetData(_)),
        "expected a preset dump response from the MPD226"
    );

    device_task.abort();

    let unanswered = dump_global_from_device();
    let silence = link
        .recv(Duration::from_millis(200), |s| reply_to(&unanswered, s))
        .await;
    assert!(
        silence.is_none(),
        "device silence must be a None, not an error"
    );
}
