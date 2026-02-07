use std::sync::mpsc;
use std::time::Duration;

use midilab::manufacturer::akai::mpd226::DeviceStatus;
use midilab::manufacturer::akai::mpd226::Global;
use midilab::manufacturer::akai::mpd226::Preset;
use midilab::manufacturer::akai::mpd226::control::value_kind::PresetName;
use midilab::manufacturer::akai::mpd226::global_dump_request;
use midilab::manufacturer::akai::mpd226::preset_dump_request;
use midilab::manufacturer::akai::mpd226::preset_send_message;
use midilab::manufacturer::akai::mpd226::raw::RawGlobal;
use midilab::manufacturer::akai::mpd226::raw::RawPreset;
use midilab::midi::Note;

const PORT_NAME: &str = "MPD226 Remote";
const TIMEOUT: Duration = Duration::from_secs(5);

fn midi_setup() -> (
    midir::MidiOutputConnection,
    mpsc::Receiver<Vec<u8>>,
    midir::MidiInputConnection<()>,
) {
    let midi_out = midir::MidiOutput::new("mpd226").unwrap();
    let out_ports = midi_out.ports();
    let out_port = out_ports
        .iter()
        .find(|p| midi_out.port_name(p).unwrap() == PORT_NAME)
        .unwrap();
    let conn_out = midi_out.connect(out_port, "mpd226-send").unwrap();

    let midi_in = midir::MidiInput::new("mpd226-recv").unwrap();
    let in_ports = midi_in.ports();
    let in_port = in_ports
        .iter()
        .find(|p| midi_in.port_name(p).unwrap() == PORT_NAME)
        .unwrap();

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let conn_in = midi_in
        .connect(
            in_port,
            "mpd226-recv",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));
    while rx.try_recv().is_ok() {}

    (conn_out, rx, conn_in)
}

#[ignore = "requires connected MPD226"]
#[test]
fn preset_round_trip() {
    let (mut conn, rx, _in_conn) = midi_setup();

    let original = {
        let conn: &mut midir::MidiOutputConnection = &mut conn;
        let rx: &mpsc::Receiver<Vec<u8>> = &rx;
        conn.send(&preset_dump_request(0x00)).unwrap();
        let res = {
            let data = rx.recv_timeout(TIMEOUT).unwrap();
            DeviceStatus::try_from(data.as_slice()).unwrap()
        };
        match res {
            DeviceStatus::PresetData(p) => *p,
            _ => panic!("wrong variant"),
        }
    };

    let mut mutated = original;
    mutated.settings.preset_name = PresetName(*b"HILTEST ");
    mutated.pads.pads[0].note = Note::N72;
    mutated.pads.pads[1].note = Note::N84;
    mutated.dials.0[0].midicc = 50;
    mutated.dials.0[1].midicc = 51;
    mutated.faders.0[0].midicc = 60;
    mutated.faders.0[1].midicc = 61;
    mutated.switches.0[0].midicc = 70;
    mutated.switches.0[1].midicc = 71;

    {
        let conn: &mut midir::MidiOutputConnection = &mut conn;
        let rx: &mpsc::Receiver<Vec<u8>> = &rx;
        let preset: &Preset = &mutated;
        let raw = RawPreset::from(preset);
        conn.send(&preset_send_message(&raw)).unwrap();
        let res = {
            let data = rx.recv_timeout(TIMEOUT).unwrap();
            DeviceStatus::try_from(data.as_slice()).unwrap()
        };
        match res {
            DeviceStatus::ReceivedPresetAck(_) => {}
            _ => panic!("wrong variant"),
        }
    };
    let loaded = {
        let conn: &mut midir::MidiOutputConnection = &mut conn;
        let rx: &mpsc::Receiver<Vec<u8>> = &rx;
        conn.send(&preset_dump_request(0x00)).unwrap();
        let res = {
            let data = rx.recv_timeout(TIMEOUT).unwrap();
            DeviceStatus::try_from(data.as_slice()).unwrap()
        };
        match res {
            DeviceStatus::PresetData(p) => *p,
            _ => panic!("wrong variant"),
        }
    };

    assert_eq!(loaded.settings.preset_name.0, *b"HILTEST ");
    assert_eq!(loaded.pads.pads[0].note, Note::N72);
    assert_eq!(loaded.pads.pads[1].note, Note::N84);
    assert_eq!(loaded.dials.0[0].midicc, 50);
    assert_eq!(loaded.dials.0[1].midicc, 51);
    assert_eq!(loaded.faders.0[0].midicc, 60);
    assert_eq!(loaded.faders.0[1].midicc, 61);
    assert_eq!(loaded.switches.0[0].midicc, 70);
    assert_eq!(loaded.switches.0[1].midicc, 71);

    {
        let conn: &mut midir::MidiOutputConnection = &mut conn;
        let rx: &mpsc::Receiver<Vec<u8>> = &rx;
        let preset: &Preset = &original;
        let raw = RawPreset::from(preset);
        conn.send(&preset_send_message(&raw)).unwrap();
        let res = {
            let data = rx.recv_timeout(TIMEOUT).unwrap();
            DeviceStatus::try_from(data.as_slice()).unwrap()
        };
        match res {
            DeviceStatus::ReceivedPresetAck(_) => {}
            _ => panic!("wrong variant"),
        }
    };
    let restored = {
        let conn: &mut midir::MidiOutputConnection = &mut conn;
        let rx: &mpsc::Receiver<Vec<u8>> = &rx;
        conn.send(&preset_dump_request(0x00)).unwrap();
        let res = {
            let data = rx.recv_timeout(TIMEOUT).unwrap();
            DeviceStatus::try_from(data.as_slice()).unwrap()
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

fn send_global(
    conn: &mut midir::MidiOutputConnection,
    rx: &mpsc::Receiver<Vec<u8>>,
    global: &Global,
) {
    let raw = RawGlobal::from(global);
    for msg in raw.global_send_messages() {
        conn.send(&msg).unwrap();
        let data = rx.recv_timeout(TIMEOUT).unwrap();
        let res = DeviceStatus::try_from(data.as_slice()).unwrap();
        match res {
            DeviceStatus::GlobalParamAck(_) => {}
            _ => panic!("wrong variant"),
        }
    }
}

fn read_global(conn: &mut midir::MidiOutputConnection, rx: &mpsc::Receiver<Vec<u8>>) -> Global {
    conn.send(&global_dump_request()).unwrap();
    let data = rx.recv_timeout(TIMEOUT).unwrap();
    let res = DeviceStatus::try_from(data.as_slice()).unwrap();
    match res {
        DeviceStatus::GlobalData(g) => *g,
        _ => panic!("wrong variant"),
    }
}

#[ignore = "requires connected MPD226"]
#[test]
fn global_round_trip() {
    let (mut conn, rx, _in_conn) = midi_setup();
    let device_original = read_global(&mut conn, &rx);

    send_global(&mut conn, &rx, &Global::default());
    let loaded_default = read_global(&mut conn, &rx);
    assert_eq!(loaded_default, Global::default());

    send_global(&mut conn, &rx, &device_original);
    let restored = read_global(&mut conn, &rx);
    assert_eq!(restored, device_original);
}
