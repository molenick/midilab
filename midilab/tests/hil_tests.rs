use std::sync::mpsc;
use std::time::Duration;

use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::control::PresetSettings;
use midilab::manufacturer::akai::mpd226::control::value_kind::PresetName;
use midilab::manufacturer::akai::mpd226::preset_dump_request;
use midilab::manufacturer::akai::mpd226::preset_send_message;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;
use midilab::sysex::Sysex;

#[ignore = "this flashes the device, we only want it to run sometimes"]
#[test]
fn test_construction_and_transmission_to_device() {
    const PORT_NAME: &str = "MPD226 Remote";

    let midi_out = midir::MidiOutput::new("mpd226").unwrap();
    let out_ports = midi_out.ports();
    let out_port = out_ports
        .iter()
        .find(|p| midi_out.port_name(p).unwrap() == PORT_NAME)
        .expect("MPD226 Remote output port not found");
    let mut conn = midi_out.connect(out_port, "mpd226-send").unwrap();

    let midi_in = midir::MidiInput::new("mpd226-recv").unwrap();
    let in_ports = midi_in.ports();
    let in_port = in_ports
        .iter()
        .find(|p| midi_in.port_name(p).unwrap() == PORT_NAME)
        .expect("MPD226 Remote input port not found");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _in_conn = midi_in
        .connect(
            in_port,
            "mpd226-recv",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    const SYSEX_PAYLOAD_SIZE: usize = 6 + 1075;
    const ACK_PAYLOAD_SIZE: usize = 8;

    let receive_ack = |rx: &mpsc::Receiver<Vec<u8>>, _: &str| {
        let data = rx.recv_timeout(Duration::from_secs(5)).unwrap();

        let sysex = Sysex::try_from(data.as_slice()).expect("ACK should be valid sysex");
        let payload = sysex.payload();

        assert_eq!(
            payload.len(),
            ACK_PAYLOAD_SIZE,
            "ACK payload should be {} bytes, got {}",
            ACK_PAYLOAD_SIZE,
            payload.len()
        );

        assert_eq!(payload[0], 0x47, "ACK mfg_id should be 0x47");
        assert_eq!(payload[1], 0x00, "ACK unknown byte should be 0x00");
        assert_eq!(payload[2], 0x35, "ACK device_id should be 0x35");
    };

    let receive_preset = |rx: &mpsc::Receiver<Vec<u8>>, name: &str| -> RawPreset {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                panic!("Timeout waiting for {} preset response", name);
            }

            match rx.recv_timeout(remaining) {
                Ok(data) => {
                    if let Ok(sysex) = Sysex::try_from(data.as_slice())
                        && sysex.payload().len() == SYSEX_PAYLOAD_SIZE
                        && let Ok(raw_preset) = RawPreset::try_from(sysex)
                    {
                        return raw_preset;
                    }
                }
                Err(_) => panic!("Timeout waiting for {} preset response", name),
            }
        }
    };

    let send_and_verify = |conn: &mut midir::MidiOutputConnection,
                           rx: &mpsc::Receiver<Vec<u8>>,
                           preset: &Preset,
                           name: &str| {
        let sent_raw = RawPreset::from(preset);

        conn.send(&{
            let raw = RawPreset::from(preset);
            preset_send_message(&raw)
        })
        .unwrap();
        receive_ack(rx, name);
        conn.send(&preset_dump_request(0x00)).unwrap();
        let received_raw = receive_preset(rx, name);

        let sent_bytes = bytemuck::bytes_of(&sent_raw);
        let received_bytes = bytemuck::bytes_of(&received_raw);

        let sent_data = &sent_bytes[..sent_bytes.len()];
        let received_data = &received_bytes[..received_bytes.len()];

        assert_eq!(
            sent_data, received_data,
            "Preset '{}' round-trip failed: sent != received",
            name
        );
    };

    let default = Preset::default();
    send_and_verify(&mut conn, &rx, &default, "default");

    let hello = Preset {
        settings: PresetSettings {
            preset_name: PresetName(*b"hello   "),
            ..Default::default()
        },
        ..Default::default()
    };
    send_and_verify(&mut conn, &rx, &hello, "hello");

    let world = Preset {
        settings: PresetSettings {
            preset_name: PresetName(*b"world   "),
            ..Default::default()
        },
        ..Default::default()
    };
    send_and_verify(&mut conn, &rx, &world, "world");
}
