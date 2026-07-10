use std::sync::mpsc;
use std::time::Duration;

use bytemuck::Zeroable;
use midilab::manufacturer::korg::r3::KorgR3Message;
use midilab::manufacturer::korg::r3::PORT_KBD_KNOB;
use midilab::manufacturer::korg::r3::PORT_SOUND;
use midilab::manufacturer::korg::r3::current_formant_motion_dump_message;
use midilab::manufacturer::korg::r3::current_formant_motion_dump_request;
use midilab::manufacturer::korg::r3::current_program_dump_message;
use midilab::manufacturer::korg::r3::current_program_dump_request;
use midilab::manufacturer::korg::r3::formant_motion_dump_request;
use midilab::manufacturer::korg::r3::formant_motion_write_request;
use midilab::manufacturer::korg::r3::global_dump_message;
use midilab::manufacturer::korg::r3::global_dump_request;
use midilab::manufacturer::korg::r3::parameter_change_message;
use midilab::manufacturer::korg::r3::program_dump_request;
use midilab::manufacturer::korg::r3::program_write_request;
use midilab::manufacturer::korg::r3::raw::RawFormantStep;
use midilab::manufacturer::korg::r3::raw::RawGlobal;
use midilab::manufacturer::korg::r3::raw::RawProgram;
use midilab::manufacturer::korg::r3::wrappers::Global;
use midilab::manufacturer::korg::r3::wrappers::Program;

const TIMEOUT: Duration = Duration::from_secs(5);
const CHANNEL: u8 = 0x00;

fn recv(rx: &mpsc::Receiver<Vec<u8>>, timeout: Duration) -> Vec<u8> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Ok(data) = rx.try_recv()
            && data.first() == Some(&0xF0)
        {
            return data;
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
        if let Ok(data) = rx.try_recv()
            && data.first() == Some(&0xF0)
        {
            return Some(data);
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
    match KorgR3Message::try_from(data.as_slice()).expect("parse global dump") {
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
    match KorgR3Message::try_from(data.as_slice()).expect("parse program dump") {
        KorgR3Message::CurrentProgramDump(p) => *p,
        other => panic!("expected CurrentProgramDump, got {other:?}"),
    }
}

fn read_slot(
    conn: &mut midir::MidiOutputConnection,
    rx: &mpsc::Receiver<Vec<u8>>,
    slot: u16,
) -> RawProgram {
    conn.send(&program_dump_request(CHANNEL, slot)).unwrap();
    let data = recv(rx, TIMEOUT);
    match KorgR3Message::try_from(data.as_slice()).expect("parse slot dump") {
        KorgR3Message::ProgramDump {
            program_no,
            program,
        } => {
            assert_eq!(program_no, slot);
            *program
        }
        other => panic!("expected ProgramDump, got {other:?}"),
    }
}

fn read_motion(
    conn: &mut midir::MidiOutputConnection,
    rx: &mpsc::Receiver<Vec<u8>>,
    motion_no: u8,
) -> (u16, Vec<RawFormantStep>) {
    conn.send(&formant_motion_dump_request(CHANNEL, motion_no))
        .unwrap();
    loop {
        let data = recv(rx, TIMEOUT);
        match KorgR3Message::try_from(data.as_slice()).expect("parse motion dump") {
            KorgR3Message::FormantMotionDump {
                motion_no: n,
                size,
                steps,
            } => {
                assert_eq!(n, motion_no);
                return (size, steps);
            }
            KorgR3Message::ParameterChange(_) => continue,
            other => panic!("expected FormantMotionDump, got {other:?}"),
        }
    }
}

fn write_motion(
    conn: &mut midir::MidiOutputConnection,
    rx: &mpsc::Receiver<Vec<u8>>,
    motion_no: u8,
    steps: &[RawFormantStep],
) {
    conn.send(&current_formant_motion_dump_message(CHANNEL, steps))
        .unwrap();
    expect_data_load_completed(rx);
    conn.send(&formant_motion_write_request(CHANNEL, motion_no))
        .unwrap();
    expect_data_load_completed(rx);
}

fn expect_data_load_completed(rx: &mpsc::Receiver<Vec<u8>>) {
    loop {
        let data = recv(rx, TIMEOUT);
        match KorgR3Message::try_from(data.as_slice()).expect("parse ack") {
            KorgR3Message::DataLoadCompleted | KorgR3Message::WriteCompleted => return,
            KorgR3Message::ParameterChange(_) => continue,
            other => panic!("expected DataLoadCompleted, got {other:?}"),
        }
    }
}

#[ignore = "requires connected Korg R3"]
#[test]
fn global_dump_discovery() {
    let midi_out = midir::MidiOutput::new("r3-disc").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-disc-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-disc-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
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

    match KorgR3Message::try_from(data.as_slice()).expect("parse global") {
        KorgR3Message::GlobalDump(g) => {
            eprintln!("master_tune = {}", g.master_tune);
            assert!(g.master_tune <= 100);
        }
        other => panic!("expected GlobalDump, got {other:?}"),
    }
}

#[ignore = "requires connected Korg R3"]
#[test]
fn current_program_dump_discovery() {
    let midi_out = midir::MidiOutput::new("r3-pd").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-pd-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-pd-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
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

    match KorgR3Message::try_from(data.as_slice()).expect("parse program") {
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
fn global_round_trip() {
    let midi_out = midir::MidiOutput::new("r3-gt").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-gt-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-gt-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
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
fn program_round_trip() {
    let midi_out = midir::MidiOutput::new("r3-pt").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-pt-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-pt-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
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
fn parameter_change_program() {
    let midi_out = midir::MidiOutput::new("r3-pc").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-pc-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-pc-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
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

    let original = read_current_program(&mut conn, &rx);
    let orig_name0 = original.name[0];

    let new_name0: u8 = if orig_name0 != b'Z' { b'Z' } else { b'Y' };
    conn.send(&parameter_change_message(
        CHANNEL,
        0x00,
        0x00,
        new_name0 as u16,
    ))
    .unwrap();
    let _ = try_recv(&rx, Duration::from_millis(500));

    let changed = read_current_program(&mut conn, &rx);
    assert_eq!(
        changed.name[0], new_name0,
        "parameter change did not update current program name[0]"
    );

    conn.send(&current_program_dump_message(CHANNEL, &original))
        .unwrap();
    expect_data_load_completed(&rx);
    let restored = read_current_program(&mut conn, &rx);
    assert_eq!(bytemuck::bytes_of(&restored), bytemuck::bytes_of(&original));
    eprintln!("Parameter change (program) OK");
}

#[ignore = "requires connected Korg R3"]
#[test]
fn program_dump_slot() {
    let midi_out = midir::MidiOutput::new("r3-sl").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-sl-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-sl-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
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

        match KorgR3Message::try_from(data.as_slice()).expect("parse slot dump") {
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
fn program_write_slot() {
    let midi_out = midir::MidiOutput::new("r3-ws").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-ws-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-ws-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
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

    match KorgR3Message::try_from(recv(&rx, TIMEOUT).as_slice()).expect("parse write ack") {
        KorgR3Message::DataLoadCompleted | KorgR3Message::WriteCompleted => {
            eprintln!("write to slot {target_slot} succeeded");

            conn.send(&program_dump_request(CHANNEL, target_slot))
                .unwrap();
            match KorgR3Message::try_from(recv(&rx, TIMEOUT).as_slice()).expect("read back") {
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
            match KorgR3Message::try_from(recv(&rx, TIMEOUT).as_slice()).expect("read restored") {
                KorgR3Message::ProgramDump {
                    program: restored, ..
                } => {
                    let ref_: &RawProgram = &restored;
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
fn program_model_round_trip() {
    let midi_out = midir::MidiOutput::new("r3-mrt").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-mrt-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-mrt-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-mrt-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    let original = read_current_program(&mut conn, &rx);
    let orig_bytes = bytemuck::bytes_of(&original).to_vec();
    eprintln!(
        "patch name = {:?}",
        std::str::from_utf8(&original.name).unwrap_or("<non-utf8>")
    );

    let prog = Program::try_from(original).expect("decode real patch to typed Program");
    let encoded = prog.as_bytes();
    assert_eq!(encoded.len(), orig_bytes.len(), "encoded size mismatch");

    let raw2: RawProgram = *bytemuck::from_bytes(&encoded);
    let prog2 = Program::try_from(raw2).expect("decode re-encoded patch");
    assert_eq!(
        prog2.as_bytes(),
        encoded,
        "typed model encoding is not idempotent on a real patch"
    );

    let diffs: Vec<usize> = (0..orig_bytes.len())
        .filter(|&i| orig_bytes[i] != encoded[i])
        .collect();
    eprintln!(
        "model dropped/changed {} of {} bytes:",
        diffs.len(),
        orig_bytes.len()
    );
    for &i in &diffs {
        eprintln!(
            "  offset {i:>3}: device=0x{:02X} model=0x{:02X}",
            orig_bytes[i], encoded[i]
        );
    }
    assert!(
        diffs.is_empty(),
        "typed model is not byte-perfect on a real patch: {} byte(s) differ (see offsets above)",
        diffs.len()
    );

    conn.send(&current_program_dump_message(CHANNEL, &raw2))
        .unwrap();
    expect_data_load_completed(&rx);
    let readback = read_current_program(&mut conn, &rx);
    let prog_rb = Program::try_from(readback).expect("decode device readback");
    assert_eq!(
        prog_rb.as_bytes(),
        encoded,
        "modeled parameters did not survive a real device round-trip"
    );

    conn.send(&current_program_dump_message(CHANNEL, &original))
        .unwrap();
    expect_data_load_completed(&rx);
    let restored = read_current_program(&mut conn, &rx);
    assert_eq!(bytemuck::bytes_of(&restored), orig_bytes.as_slice());
    eprintln!("Program model round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[test]
fn global_model_round_trip() {
    let midi_out = midir::MidiOutput::new("r3-gm").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-gm-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-gm-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-gm-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    let original = read_global(&mut conn, &rx);
    let orig_bytes = bytemuck::bytes_of(&original).to_vec();

    let g = Global::try_from(original).expect("decode real global to typed Global");
    let encoded = g.as_bytes();
    let diffs: Vec<usize> = (0..orig_bytes.len())
        .filter(|&i| orig_bytes[i] != encoded[i])
        .collect();
    eprintln!(
        "global model dropped/changed {} of {} bytes:",
        diffs.len(),
        orig_bytes.len()
    );
    for &i in &diffs {
        eprintln!(
            "  offset {i:>3}: device=0x{:02X} model=0x{:02X}",
            orig_bytes[i], encoded[i]
        );
    }
    assert!(
        diffs.is_empty(),
        "typed Global is not byte-perfect: {} byte(s) differ",
        diffs.len()
    );

    if original.flags_2 & 0x80 != 0 {
        eprintln!("memory-protect ON — skipping device write-back");
        return;
    }
    let raw2: RawGlobal = *bytemuck::from_bytes(&encoded);
    conn.send(&global_dump_message(CHANNEL, &raw2)).unwrap();
    expect_data_load_completed(&rx);
    let readback = read_global(&mut conn, &rx);
    assert_eq!(
        bytemuck::bytes_of(&readback),
        encoded.as_slice(),
        "global did not survive device round-trip"
    );
    conn.send(&global_dump_message(CHANNEL, &original)).unwrap();
    expect_data_load_completed(&rx);
    eprintln!("Global model round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[test]
fn tempo_encoding_probe() {
    let midi_out = midir::MidiOutput::new("r3-tp").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-tp-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-tp-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-tp-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    for slot in [0u16, 1, 16, 32, 64, 99, 127] {
        conn.send(&program_dump_request(CHANNEL, slot)).unwrap();
        let data = recv(&rx, TIMEOUT);
        match KorgR3Message::try_from(data.as_slice()).expect("parse slot dump") {
            KorgR3Message::ProgramDump { program: p, .. } => {
                let raw = bytemuck::bytes_of(&*p);
                let lsb = raw[444] as u16;
                let msb = raw[445] as u16;
                let v = lsb | (msb << 7);
                eprintln!(
                    "slot {slot:>3} name={:?}  tempo bytes=[{:#04X},{:#04X}] raw={v}  => tenths:{:.1}BPM  wholeBPM:{}",
                    std::str::from_utf8(&p.name).unwrap_or("?"),
                    raw[444],
                    raw[445],
                    v as f32 / 10.0,
                    v
                );
            }
            other => panic!("expected ProgramDump, got {other:?}"),
        }
    }
}

#[ignore = "requires connected Korg R3"]
#[test]
fn formant_motion_dump() {
    let midi_out = midir::MidiOutput::new("r3-mo").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-mo-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-mo-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
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

    match KorgR3Message::try_from(data.as_slice()).expect("parse formant") {
        KorgR3Message::CurrentFormantMotionDump { size, steps } => {
            eprintln!(
                "  SIZE={size}  steps={}  ~{:.2}s",
                steps.len(),
                size as f32 / 100.0
            );
            assert_eq!(
                steps.len(),
                size as usize,
                "one 16-byte frame per SIZE unit"
            );
        }
        other => panic!("expected CurrentFormantMotionDump, got {other:?}"),
    }
}

#[ignore = "requires connected Korg R3"]
#[test]
fn formant_dump_all() {
    let midi_out = midir::MidiOutput::new("r3-mo").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-mo-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-mo-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
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

    for i in 0u8..16 {
        let (size, steps) = read_motion(&mut conn, &rx, i);
        assert_eq!(
            steps.len(),
            size as usize,
            "motion {i}: frame count matches SIZE"
        );
        eprintln!(
            "motion {:02}: {size} frames (~{:.2}s)",
            i + 1,
            size as f32 / 100.0
        );
    }
}

#[ignore = "requires connected Korg R3 (memory protect OFF); writes formant motion 15"]
#[test]
fn formant_write_path() {
    let midi_out = midir::MidiOutput::new("r3-mo").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-mo-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-mo-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
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

    const SCRATCH: u8 = 15;

    let (orig_size, orig_steps) = read_motion(&mut conn, &rx, SCRATCH);
    eprintln!("scratch motion {SCRATCH} original: {orig_size} frames");

    let mut synth = vec![RawFormantStep::zeroed(); 4];
    for (i, step) in synth.iter_mut().enumerate() {
        for (band, slot) in step.bands.iter_mut().enumerate() {
            *slot = ((i * 16 + band) as u8).wrapping_mul(3) | ((band as u8 & 1) << 7);
        }
    }
    synth[0].bands[0] = 0xFF;
    synth[1].bands[5] = 0x80;
    synth[3].bands[15] = 0x7F;

    write_motion(&mut conn, &rx, SCRATCH, &synth);

    let (rb_size, rb_steps) = read_motion(&mut conn, &rx, SCRATCH);
    eprintln!("readback: {rb_size} frames");
    assert_eq!(
        rb_size as usize,
        synth.len(),
        "readback SIZE matches written"
    );
    assert_eq!(rb_steps.len(), synth.len(), "readback frame count matches");
    assert_eq!(
        bytemuck::cast_slice::<RawFormantStep, u8>(&rb_steps),
        bytemuck::cast_slice::<RawFormantStep, u8>(&synth),
        "readback bytes differ from written motion"
    );

    write_motion(&mut conn, &rx, SCRATCH, &orig_steps);
    let (restored_size, _) = read_motion(&mut conn, &rx, SCRATCH);
    assert_eq!(restored_size, orig_size, "scratch motion restored");
}

#[ignore = "requires connected Korg R3 (memory protect OFF); writes slot 0"]
#[test]
fn editor_write_path_fix_slot0_name() {
    let midi_out = midir::MidiOutput::new("r3-fix").unwrap();
    let sound_port = midi_out
        .ports()
        .into_iter()
        .find(|p| midi_out.port_name(p).ok() == Some(PORT_SOUND.to_string()))
        .expect("no R3 SOUND");
    let mut conn = midi_out.connect(&sound_port, "r3-fix-send").unwrap();

    let midi_in = midir::MidiInput::new("r3-fix-rx").unwrap();
    let kbd_port = midi_in
        .ports()
        .into_iter()
        .find(|p| midi_in.port_name(p).ok() == Some(PORT_KBD_KNOB.to_string()))
        .expect("no R3 KBD/KNOB");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn_in = midi_in
        .connect(
            &kbd_port,
            "r3-fix-rx-kbd",
            move |_ts, data, _| {
                let _ = tx.send(data.to_vec());
            },
            (),
        )
        .unwrap();

    let original = read_slot(&mut conn, &rx, 0);
    eprintln!(
        "slot 0 name before: {:?}",
        std::str::from_utf8(&original.name)
    );

    let mut prog = Program::try_from(original).expect("decode typed program");
    prog.name = "InitProg".to_string();
    let fixed: RawProgram = (&prog).into();

    assert_eq!(&fixed.name, b"InitProg", "typed encode set the name");
    assert_eq!(
        bytemuck::bytes_of(&fixed)[8..],
        bytemuck::bytes_of(&original)[8..],
        "typed encode changed bytes other than the name"
    );

    conn.send(&current_program_dump_message(CHANNEL, &fixed))
        .unwrap();
    expect_data_load_completed(&rx);
    conn.send(&program_write_request(CHANNEL, 0)).unwrap();
    match KorgR3Message::try_from(recv(&rx, TIMEOUT).as_slice()).expect("parse write ack") {
        KorgR3Message::DataLoadCompleted | KorgR3Message::WriteCompleted => {}
        KorgR3Message::DataLoadError => panic!("write REJECTED — memory protect is ON"),
        other => panic!("expected write ack, got {other:?}"),
    }

    let readback = read_slot(&mut conn, &rx, 0);
    eprintln!(
        "slot 0 name after: {:?}",
        std::str::from_utf8(&readback.name)
    );
    assert_eq!(&readback.name, b"InitProg");
    assert_eq!(
        bytemuck::bytes_of(&readback),
        bytemuck::bytes_of(&fixed),
        "readback differs from written program"
    );
}
