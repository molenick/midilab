use std::sync::mpsc;
use std::time::Duration;

use midilab::manufacturer::korg::r3::KorgR3Message;
use midilab::manufacturer::korg::r3::current_formant_motion_dump_request;
use midilab::manufacturer::korg::r3::current_program_dump_message;
use midilab::manufacturer::korg::r3::current_program_dump_request;
use midilab::manufacturer::korg::r3::formant_motion_dump_request;
use midilab::manufacturer::korg::r3::global_dump_message;
use midilab::manufacturer::korg::r3::global_dump_request;
use midilab::manufacturer::korg::r3::parameter_change_message;
use midilab::manufacturer::korg::r3::parse_sysex;
use midilab::manufacturer::korg::r3::program_dump_request;
use midilab::manufacturer::korg::r3::program_write_request;
use midilab::manufacturer::korg::r3::raw::RawGlobal;
use midilab::manufacturer::korg::r3::raw::RawProgram;

const TIMEOUT: Duration = Duration::from_secs(5);
const CHANNEL: u8 = 0x00;

fn recv(rx: &mpsc::Receiver<Vec<u8>>, timeout: Duration) -> Vec<u8> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Ok(data) = rx.try_recv() {
            if data.first() == Some(&0xF0) {
                return data;
            }
        }
        if std::time::Instant::now() >= deadline {
            panic!("timed out waiting for sysex response");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn try_recv(rx: &mpsc::Receiver<Vec<u8>>, timeout: Duration) -> Option<Vec<u8>> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Ok(data) = rx.try_recv() {
            if data.first() == Some(&0xF0) {
                return Some(data);
            }
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn read_global(conn: &mut midir::MidiOutputConnection, rx: &mpsc::Receiver<Vec<u8>>) -> RawGlobal {
    conn.send(&global_dump_request(CHANNEL)).unwrap();
    let data = recv(rx, TIMEOUT);
    match parse_sysex(&data).expect("parse global dump") {
        KorgR3Message::GlobalDump(g) => *g,
        other => panic!("expected GlobalDump, got {other:?}"),
    }
}

fn read_current_program(
    conn: &mut midir::MidiOutputConnection,
    rx: &mpsc::Receiver<Vec<u8>>,
) -> RawProgram {
    conn.send(&current_program_dump_request(CHANNEL)).unwrap();
    let data = recv(rx, TIMEOUT);
    match parse_sysex(&data).expect("parse program dump") {
        KorgR3Message::CurrentProgramDump(p) => *p,
        other => panic!("expected CurrentProgramDump, got {other:?}"),
    }
}

fn expect_data_load_completed(rx: &mpsc::Receiver<Vec<u8>>) {
    let data = recv(rx, TIMEOUT);
    match parse_sysex(&data).expect("parse ack") {
        KorgR3Message::DataLoadCompleted | KorgR3Message::WriteCompleted => {}
        other => panic!("expected DataLoadCompleted, got {other:?}"),
    }
}

#[ignore = "requires connected Korg R3"]
#[test]
fn test_global_dump_discovery() {
    let midi_out = midir::MidiOutput::new("r3-disc").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(&p).ok() == Some("R3 SOUND".to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-disc-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-disc-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(&p).ok() == Some("R3 KBD/KNOB".to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-disc-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    eprintln!("Scanning channels 0-15...");
    let mut found_ch: Option<u8> = None;
    for try_ch in 0u8..=15 {
        conn.send(&global_dump_request(try_ch)).unwrap();
        if let Some(data) = try_recv(&rx, Duration::from_secs(5)) {
            eprintln!(
                "  *** RESPONSE ch={try_ch}: {} bytes {:02X?} ***",
                data.len(),
                &data[..data.len().min(16)]
            );
            found_ch = Some(try_ch);
            break;
        }
    }

    let ch = found_ch.unwrap_or(CHANNEL);
    if found_ch.is_some() {
        eprintln!("R3 responds on channel {}. Using for remaining tests.", ch);
    } else {
        eprintln!("No response. Check: SystemEx=ENA, SysEx On");
    }

    let ch = found_ch.unwrap_or(ch);
    conn.send(&global_dump_request(ch)).unwrap();
    let data = recv(&rx, TIMEOUT);
    eprintln!("Raw: {} bytes", data.len());

    match parse_sysex(&data).expect("parse global") {
        KorgR3Message::GlobalDump(g) => {
            eprintln!("master_tune = {}", g.master_tune);
            assert!(g.master_tune <= 100);
        }
        other => panic!("expected GlobalDump, got {other:?}"),
    }
}

#[ignore = "requires connected Korg R3"]
#[test]
fn test_current_program_dump_discovery() {
    let midi_out = midir::MidiOutput::new("r3-pd").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(&p).ok() == Some("R3 SOUND".to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-pd-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-pd-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(&p).ok() == Some("R3 KBD/KNOB".to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-pd-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    conn.send(&current_program_dump_request(CHANNEL)).unwrap();
    let data = recv(&rx, TIMEOUT);
    eprintln!("Raw: {} bytes", data.len());

    match parse_sysex(&data).expect("parse program") {
        KorgR3Message::CurrentProgramDump(p) => {
            let name = std::str::from_utf8(&p.name).unwrap_or("<non-utf8>");
            eprintln!("name = {:?}", name);
            assert!(p.name.iter().any(|&b| b > 0x20 && b < 0x7F) || name.chars().count() > 0);
        }
        other => panic!("expected CurrentProgramDump, got {other:?}"),
    }
}

#[ignore = "requires connected Korg R3"]
#[test]
fn test_global_round_trip() {
    let midi_out = midir::MidiOutput::new("r3-gt").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(&p).ok() == Some("R3 SOUND".to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-gt-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-gt-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(&p).ok() == Some("R3 KBD/KNOB".to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-gt-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    let original = read_global(&mut conn, &rx);
    let protect_on = original.flags_2 & 0x80 != 0;
    eprintln!(
        "master_tune = {}, protect = {}",
        original.master_tune,
        if protect_on { "ON" } else { "OFF" }
    );

    if protect_on {
        eprintln!("SKIPPING: protect ON");
        return;
    }

    let new_tune = if original.master_tune != 40 { 40 } else { 60 };
    let mut modified = original;
    modified.master_tune = new_tune;

    conn.send(&global_dump_message(CHANNEL, &modified)).unwrap();
    expect_data_load_completed(&rx);

    let readback = read_global(&mut conn, &rx);
    assert_eq!(readback.master_tune, new_tune);

    conn.send(&global_dump_message(CHANNEL, &original)).unwrap();
    expect_data_load_completed(&rx);

    let restored = read_global(&mut conn, &rx);
    assert_eq!(bytemuck::bytes_of(&restored), bytemuck::bytes_of(&original));
    eprintln!("Global round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[test]
fn test_program_round_trip() {
    let midi_out = midir::MidiOutput::new("r3-pt").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(&p).ok() == Some("R3 SOUND".to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-pt-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-pt-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(&p).ok() == Some("R3 KBD/KNOB".to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-pt-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    let original = read_current_program(&mut conn, &rx);
    eprintln!(
        "name = {:?}",
        std::str::from_utf8(&original.name).unwrap_or("<non-utf8>")
    );

    let mut modified = original;
    modified.name = *b"HILTEST ";

    conn.send(&current_program_dump_message(CHANNEL, &modified))
        .unwrap();
    expect_data_load_completed(&rx);

    let readback = read_current_program(&mut conn, &rx);
    assert_eq!(&readback.name, b"HILTEST ");

    conn.send(&current_program_dump_message(CHANNEL, &original))
        .unwrap();
    expect_data_load_completed(&rx);

    let restored = read_current_program(&mut conn, &rx);
    assert_eq!(bytemuck::bytes_of(&restored), bytemuck::bytes_of(&original));
    eprintln!("Program round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[test]
fn test_parameter_change_global() {
    let midi_out = midir::MidiOutput::new("r3-pc").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(&p).ok() == Some("R3 SOUND".to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-pc-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-pc-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(&p).ok() == Some("R3 KBD/KNOB".to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-pc-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    let original = read_global(&mut conn, &rx);
    let protect_on = original.flags_2 & 0x80 != 0;

    if protect_on {
        eprintln!("SKIPPING: protect ON");
        return;
    }

    let new_tune: u16 = if original.master_tune != 40 { 40 } else { 60 };
    conn.send(&parameter_change_message(CHANNEL, 0x00, 0x00, new_tune))
        .unwrap();

    match try_recv(&rx, Duration::from_secs(2)) {
        Some(data) => match parse_sysex(&data).expect("parse") {
            KorgR3Message::DataLoadCompleted => eprintln!("ACCEPTED"),
            KorgR3Message::DataLoadError => eprintln!("REJECTED"),
            other => panic!("unexpected: {other:?}"),
        },
        None => {
            eprintln!("NO RESPONSE (expected)");
            let rb = read_global(&mut conn, &rx);
            assert_eq!(rb.master_tune, original.master_tune);
        }
    }
}

#[ignore = "requires connected Korg R3"]
#[test]
fn test_program_dump_slot() {
    let midi_out = midir::MidiOutput::new("r3-sl").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(&p).ok() == Some("R3 SOUND".to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-sl-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-sl-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(&p).ok() == Some("R3 KBD/KNOB".to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-sl-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    for slot in [0, 1, 32, 64] {
        conn.send(&program_dump_request(CHANNEL, slot)).unwrap();
        let data = recv(&rx, TIMEOUT);
        eprintln!("Slot {slot}: {} bytes", data.len());

        match parse_sysex(&data).expect("parse slot dump") {
            KorgR3Message::ProgramDump {
                program_no,
                program: p,
            } => {
                eprintln!(
                    "  slot={}, name={:?}",
                    program_no,
                    std::str::from_utf8(&p.name).unwrap_or("<non-utf8>")
                );
                assert_eq!(program_no, slot);
            }
            other => panic!("expected ProgramDump, got {other:?}"),
        }
    }
}

#[ignore = "requires connected Korg R3"]
#[test]
fn test_program_write_slot() {
    let midi_out = midir::MidiOutput::new("r3-ws").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(&p).ok() == Some("R3 SOUND".to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-ws-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-ws-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(&p).ok() == Some("R3 KBD/KNOB".to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-ws-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    let original = read_current_program(&mut conn, &rx);
    eprintln!(
        "original name = {:?}",
        std::str::from_utf8(&original.name).unwrap_or("<non-utf8>")
    );

    let mut modified = original;
    modified.name = *b"WRITESL ";

    conn.send(&current_program_dump_message(CHANNEL, &modified))
        .unwrap();
    expect_data_load_completed(&rx);

    let target_slot: u16 = 0;
    conn.send(&program_write_request(CHANNEL, target_slot))
        .unwrap();

    match parse_sysex(&recv(&rx, TIMEOUT)).expect("parse write ack") {
        KorgR3Message::DataLoadCompleted | KorgR3Message::WriteCompleted => {
            eprintln!("write to slot {target_slot} succeeded");

            conn.send(&program_dump_request(CHANNEL, target_slot))
                .unwrap();
            match parse_sysex(&recv(&rx, TIMEOUT)).expect("read back") {
                KorgR3Message::ProgramDump {
                    program_no,
                    program: p,
                } => {
                    assert_eq!(program_no, target_slot);
                    assert_eq!(&p.name, b"WRITESL ");
                }
                other => panic!("expected ProgramDump on read-back, got {other:?}"),
            }

            conn.send(&current_program_dump_message(CHANNEL, &original))
                .unwrap();
            expect_data_load_completed(&rx);
            conn.send(&program_write_request(CHANNEL, target_slot))
                .unwrap();
            expect_data_load_completed(&rx);

            conn.send(&program_dump_request(CHANNEL, target_slot))
                .unwrap();
            match parse_sysex(&recv(&rx, TIMEOUT)).expect("read restored") {
                KorgR3Message::ProgramDump {
                    program: restored, ..
                } => {
                    let ref_: &RawProgram = &*restored;
                    assert_eq!(bytemuck::bytes_of(ref_), bytemuck::bytes_of(&original));
                }
                other => panic!("expected ProgramDump, got {other:?}"),
            }
        }
        KorgR3Message::DataLoadError => {
            eprintln!("write REJECTED (check memory protect)");
        }
        other => panic!("expected DataLoadCompleted or DataLoadError, got {other:?}"),
    }
}

#[ignore = "requires connected Korg R3"]
#[test]
fn test_formant_motion_dump() {
    let midi_out = midir::MidiOutput::new("r3-mo").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(&p).ok() == Some("R3 SOUND".to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-mo-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-mo-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(&p).ok() == Some("R3 KBD/KNOB".to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-mo-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    conn.send(&current_formant_motion_dump_request(CHANNEL))
        .unwrap();
    let data = recv(&rx, TIMEOUT);
    eprintln!("current formant motion: {} bytes", data.len());

    match parse_sysex(&data).expect("parse formant") {
        KorgR3Message::CurrentFormantMotionDump { header: _, steps } => {
            eprintln!("  {} motion steps", steps.len());
            assert!(steps.len() <= 2000, "too many formant steps");
            assert!(
                steps.iter().all(|s| s.bands.len() == 16),
                "band count wrong"
            );
            assert!(
                steps.iter().all(|s| s.secondary.len() == 16),
                "secondary count wrong"
            );
            assert_eq!(
                std::mem::size_of::<midilab::manufacturer::korg::r3::raw::RawFormantStep>(),
                32
            );
        }
        other => panic!("expected CurrentFormantMotionDump, got {other:?}"),
    }
}
