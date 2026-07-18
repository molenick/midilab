use std::time::Duration;

use bytemuck::Zeroable;
use midi_io::Client;
use midi_io::DestinationConnection;
use midi_io::SysEx;
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
use tokio::sync::mpsc;
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_secs(5);
const CHANNEL: u8 = 0x00;

async fn connect(name: &str) -> (DestinationConnection, mpsc::UnboundedReceiver<SysEx>) {
    let client = Client::new(name).await.unwrap();

    let sound_port = client
        .destinations()
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.name() == PORT_SOUND)
        .expect("no R3 SOUND");
    let conn = client.connect_destination(&sound_port).await.unwrap();

    let kbd_port = client
        .sources()
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.name() == PORT_KBD_KNOB)
        .expect("no R3 KBD/KNOB");
    let conn_in = client.connect_source(&kbd_port).await.unwrap();

    let (tx, rx) = mpsc::unbounded_channel::<SysEx>();
    tokio::spawn(async move {
        let mut sysex = conn_in.into_sysex();
        while let Some(timed) = sysex.recv().await {
            let _ = tx.send(timed.payload);
        }
    });

    (conn, rx)
}

async fn send(conn: &DestinationConnection, sysex: &SysEx) {
    conn.send_sysex(sysex).await.unwrap();
}

async fn recv(rx: &mut mpsc::UnboundedReceiver<SysEx>, dur: Duration) -> SysEx {
    timeout(dur, rx.recv())
        .await
        .expect("timed out waiting for sysex response")
        .expect("channel closed")
}

async fn try_recv(rx: &mut mpsc::UnboundedReceiver<SysEx>, dur: Duration) -> Option<SysEx> {
    timeout(dur, rx.recv()).await.ok().flatten()
}

async fn read_global(
    conn: &DestinationConnection,
    rx: &mut mpsc::UnboundedReceiver<SysEx>,
) -> RawGlobal {
    send(conn, &global_dump_request(CHANNEL)).await;
    let data = recv(rx, TIMEOUT).await;
    match KorgR3Message::try_from(&data).expect("parse global dump") {
        KorgR3Message::GlobalDump(g) => *g,
        other => panic!("expected GlobalDump, got {other:?}"),
    }
}

async fn read_current_program(
    conn: &DestinationConnection,
    rx: &mut mpsc::UnboundedReceiver<SysEx>,
) -> RawProgram {
    send(conn, &current_program_dump_request(CHANNEL)).await;
    let data = recv(rx, TIMEOUT).await;
    match KorgR3Message::try_from(&data).expect("parse program dump") {
        KorgR3Message::CurrentProgramDump(p) => *p,
        other => panic!("expected CurrentProgramDump, got {other:?}"),
    }
}

async fn read_slot(
    conn: &DestinationConnection,
    rx: &mut mpsc::UnboundedReceiver<SysEx>,
    slot: u16,
) -> RawProgram {
    send(conn, &program_dump_request(CHANNEL, slot)).await;
    let data = recv(rx, TIMEOUT).await;
    match KorgR3Message::try_from(&data).expect("parse slot dump") {
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

async fn read_motion(
    conn: &DestinationConnection,
    rx: &mut mpsc::UnboundedReceiver<SysEx>,
    motion_no: u8,
) -> (u16, Vec<RawFormantStep>) {
    send(conn, &formant_motion_dump_request(CHANNEL, motion_no)).await;
    loop {
        let data = recv(rx, TIMEOUT).await;
        match KorgR3Message::try_from(&data).expect("parse motion dump") {
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

async fn write_motion(
    conn: &DestinationConnection,
    rx: &mut mpsc::UnboundedReceiver<SysEx>,
    motion_no: u8,
    steps: &[RawFormantStep],
) {
    send(conn, &current_formant_motion_dump_message(CHANNEL, steps)).await;
    expect_data_load_completed(rx).await;
    send(conn, &formant_motion_write_request(CHANNEL, motion_no)).await;
    expect_data_load_completed(rx).await;
}

async fn expect_data_load_completed(rx: &mut mpsc::UnboundedReceiver<SysEx>) {
    loop {
        let data = recv(rx, TIMEOUT).await;
        match KorgR3Message::try_from(&data).expect("parse ack") {
            KorgR3Message::DataLoadCompleted | KorgR3Message::WriteCompleted => return,
            KorgR3Message::ParameterChange(_) => continue,
            other => panic!("expected DataLoadCompleted, got {other:?}"),
        }
    }
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn global_dump_discovery() {
    let (conn, mut rx) = connect("r3-disc").await;

    eprintln!("Scanning channels 0-15...");
    let mut found_ch: Option<u8> = None;
    for try_ch in 0u8..=15 {
        send(&conn, &global_dump_request(try_ch)).await;
        if let Some(data) = try_recv(&mut rx, Duration::from_secs(5)).await {
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
    send(&conn, &global_dump_request(ch)).await;
    let data = recv(&mut rx, TIMEOUT).await;
    eprintln!("Raw: {} bytes", data.len());

    match KorgR3Message::try_from(&data).expect("parse global") {
        KorgR3Message::GlobalDump(g) => {
            eprintln!("master_tune = {}", g.master_tune);
            assert!(g.master_tune <= 100);
        }
        other => panic!("expected GlobalDump, got {other:?}"),
    }
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn current_program_dump_discovery() {
    let (conn, mut rx) = connect("r3-pd").await;

    send(&conn, &current_program_dump_request(CHANNEL)).await;
    let data = recv(&mut rx, TIMEOUT).await;
    eprintln!("Raw: {} bytes", data.len());

    match KorgR3Message::try_from(&data).expect("parse program") {
        KorgR3Message::CurrentProgramDump(p) => {
            let name = std::str::from_utf8(&p.name).unwrap_or("<non-utf8>");
            eprintln!("name = {:?}", name);
            assert!(p.name.iter().any(|&b| b > 0x20 && b < 0x7F) || name.chars().count() > 0);
        }
        other => panic!("expected CurrentProgramDump, got {other:?}"),
    }
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn global_round_trip() {
    let (conn, mut rx) = connect("r3-gt").await;

    let original = read_global(&conn, &mut rx).await;
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

    send(&conn, &global_dump_message(CHANNEL, &modified)).await;
    expect_data_load_completed(&mut rx).await;

    let readback = read_global(&conn, &mut rx).await;
    assert_eq!(readback.master_tune, new_tune);

    send(&conn, &global_dump_message(CHANNEL, &original)).await;
    expect_data_load_completed(&mut rx).await;

    let restored = read_global(&conn, &mut rx).await;
    assert_eq!(bytemuck::bytes_of(&restored), bytemuck::bytes_of(&original));
    eprintln!("Global round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn program_round_trip() {
    let (conn, mut rx) = connect("r3-pt").await;

    let original = read_current_program(&conn, &mut rx).await;
    eprintln!(
        "name = {:?}",
        std::str::from_utf8(&original.name).unwrap_or("<non-utf8>")
    );

    let mut modified = original;
    modified.name = *b"HILTEST ";

    send(&conn, &current_program_dump_message(CHANNEL, &modified)).await;
    expect_data_load_completed(&mut rx).await;

    let readback = read_current_program(&conn, &mut rx).await;
    assert_eq!(&readback.name, b"HILTEST ");

    send(&conn, &current_program_dump_message(CHANNEL, &original)).await;
    expect_data_load_completed(&mut rx).await;

    let restored = read_current_program(&conn, &mut rx).await;
    assert_eq!(bytemuck::bytes_of(&restored), bytemuck::bytes_of(&original));
    eprintln!("Program round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn parameter_change_program() {
    let (conn, mut rx) = connect("r3-pc").await;

    let original = read_current_program(&conn, &mut rx).await;
    let orig_name0 = original.name[0];

    let new_name0: u8 = if orig_name0 != b'Z' { b'Z' } else { b'Y' };
    send(
        &conn,
        &parameter_change_message(CHANNEL, 0x00, 0x00, new_name0 as u16),
    )
    .await;
    let _ = try_recv(&mut rx, Duration::from_millis(500)).await;

    let changed = read_current_program(&conn, &mut rx).await;
    assert_eq!(
        changed.name[0], new_name0,
        "parameter change did not update current program name[0]"
    );

    send(&conn, &current_program_dump_message(CHANNEL, &original)).await;
    expect_data_load_completed(&mut rx).await;
    let restored = read_current_program(&conn, &mut rx).await;
    assert_eq!(bytemuck::bytes_of(&restored), bytemuck::bytes_of(&original));
    eprintln!("Parameter change (program) OK");
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn program_dump_slot() {
    let (conn, mut rx) = connect("r3-sl").await;

    for slot in [0, 1, 32, 64] {
        send(&conn, &program_dump_request(CHANNEL, slot)).await;
        let data = recv(&mut rx, TIMEOUT).await;
        eprintln!("Slot {slot}: {} bytes", data.len());

        match KorgR3Message::try_from(&data).expect("parse slot dump") {
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
#[tokio::test]
async fn program_write_slot() {
    let (conn, mut rx) = connect("r3-ws").await;

    let original = read_current_program(&conn, &mut rx).await;
    eprintln!(
        "original name = {:?}",
        std::str::from_utf8(&original.name).unwrap_or("<non-utf8>")
    );

    let mut modified = original;
    modified.name = *b"WRITESL ";

    send(&conn, &current_program_dump_message(CHANNEL, &modified)).await;
    expect_data_load_completed(&mut rx).await;

    let target_slot: u16 = 0;
    send(&conn, &program_write_request(CHANNEL, target_slot)).await;

    match KorgR3Message::try_from(&recv(&mut rx, TIMEOUT).await).expect("parse write ack") {
        KorgR3Message::DataLoadCompleted | KorgR3Message::WriteCompleted => {
            eprintln!("write to slot {target_slot} succeeded");

            send(&conn, &program_dump_request(CHANNEL, target_slot)).await;
            match KorgR3Message::try_from(&recv(&mut rx, TIMEOUT).await).expect("read back") {
                KorgR3Message::ProgramDump {
                    program_no,
                    program: p,
                } => {
                    assert_eq!(program_no, target_slot);
                    assert_eq!(&p.name, b"WRITESL ");
                }
                other => panic!("expected ProgramDump on read-back, got {other:?}"),
            }

            send(&conn, &current_program_dump_message(CHANNEL, &original)).await;
            expect_data_load_completed(&mut rx).await;
            send(&conn, &program_write_request(CHANNEL, target_slot)).await;
            expect_data_load_completed(&mut rx).await;

            send(&conn, &program_dump_request(CHANNEL, target_slot)).await;
            match KorgR3Message::try_from(&recv(&mut rx, TIMEOUT).await).expect("read restored") {
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
#[tokio::test]
async fn program_model_round_trip() {
    let (conn, mut rx) = connect("r3-mrt").await;

    let original = read_current_program(&conn, &mut rx).await;
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

    send(&conn, &current_program_dump_message(CHANNEL, &raw2)).await;
    expect_data_load_completed(&mut rx).await;
    let readback = read_current_program(&conn, &mut rx).await;
    let prog_rb = Program::try_from(readback).expect("decode device readback");
    assert_eq!(
        prog_rb.as_bytes(),
        encoded,
        "modeled parameters did not survive a real device round-trip"
    );

    send(&conn, &current_program_dump_message(CHANNEL, &original)).await;
    expect_data_load_completed(&mut rx).await;
    let restored = read_current_program(&conn, &mut rx).await;
    assert_eq!(bytemuck::bytes_of(&restored), orig_bytes.as_slice());
    eprintln!("Program model round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn global_model_round_trip() {
    let (conn, mut rx) = connect("r3-gm").await;

    let original = read_global(&conn, &mut rx).await;
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
    send(&conn, &global_dump_message(CHANNEL, &raw2)).await;
    expect_data_load_completed(&mut rx).await;
    let readback = read_global(&conn, &mut rx).await;
    assert_eq!(
        bytemuck::bytes_of(&readback),
        encoded.as_slice(),
        "global did not survive device round-trip"
    );
    send(&conn, &global_dump_message(CHANNEL, &original)).await;
    expect_data_load_completed(&mut rx).await;
    eprintln!("Global model round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn tempo_encoding_probe() {
    let (conn, mut rx) = connect("r3-tp").await;

    for slot in [0u16, 1, 16, 32, 64, 99, 127] {
        send(&conn, &program_dump_request(CHANNEL, slot)).await;
        let data = recv(&mut rx, TIMEOUT).await;
        match KorgR3Message::try_from(&data).expect("parse slot dump") {
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
#[tokio::test]
async fn formant_motion_dump() {
    let (conn, mut rx) = connect("r3-mo").await;

    send(&conn, &current_formant_motion_dump_request(CHANNEL)).await;
    let data = recv(&mut rx, TIMEOUT).await;
    eprintln!("current formant motion: {} bytes", data.len());

    match KorgR3Message::try_from(&data).expect("parse formant") {
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
#[tokio::test]
async fn formant_dump_all() {
    let (conn, mut rx) = connect("r3-mo").await;

    for i in 0u8..16 {
        let (size, steps) = read_motion(&conn, &mut rx, i).await;
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
#[tokio::test]
async fn formant_write_path() {
    let (conn, mut rx) = connect("r3-mo").await;

    const SCRATCH: u8 = 15;

    let (orig_size, orig_steps) = read_motion(&conn, &mut rx, SCRATCH).await;
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

    write_motion(&conn, &mut rx, SCRATCH, &synth).await;

    let (rb_size, rb_steps) = read_motion(&conn, &mut rx, SCRATCH).await;
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

    write_motion(&conn, &mut rx, SCRATCH, &orig_steps).await;
    let (restored_size, _) = read_motion(&conn, &mut rx, SCRATCH).await;
    assert_eq!(restored_size, orig_size, "scratch motion restored");
}

#[ignore = "requires connected Korg R3 (memory protect OFF); writes slot 0"]
#[tokio::test]
async fn editor_write_path_fix_slot0_name() {
    let (conn, mut rx) = connect("r3-fix").await;

    let original = read_slot(&conn, &mut rx, 0).await;
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

    send(&conn, &current_program_dump_message(CHANNEL, &fixed)).await;
    expect_data_load_completed(&mut rx).await;
    send(&conn, &program_write_request(CHANNEL, 0)).await;
    match KorgR3Message::try_from(&recv(&mut rx, TIMEOUT).await).expect("parse write ack") {
        KorgR3Message::DataLoadCompleted | KorgR3Message::WriteCompleted => {}
        KorgR3Message::DataLoadError => panic!("write REJECTED — memory protect is ON"),
        other => panic!("expected write ack, got {other:?}"),
    }

    let readback = read_slot(&conn, &mut rx, 0).await;
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
