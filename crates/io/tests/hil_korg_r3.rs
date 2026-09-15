use std::time::Duration;

use bytemuck::Zeroable;
use midi_io::Client;
use midi_io::SysEx;
use midilab::manufacturer::korg::r3::KorgR3Message;
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
use midilab::manufacturer::korg::r3::reply_to;
use midilab::manufacturer::korg::r3::wrappers::Global;
use midilab::manufacturer::korg::r3::wrappers::Program;
use midilab_io::midi::Link;

const TIMEOUT: Duration = Duration::from_secs(5);
const CHANNEL: u8 = 0x00;

async fn connect(name: &str) -> Client {
    Client::new(name).await.unwrap()
}

async fn try_request(client: &Client, message: SysEx, wait: Duration) -> Option<KorgR3Message> {
    let mut link = Link::open(client).await;
    link.send(&message).await.unwrap();
    link.recv(wait, |s| reply_to(&message, s)).await
}

async fn request(client: &Client, message: SysEx) -> KorgR3Message {
    try_request(client, message, TIMEOUT)
        .await
        .expect("timed out waiting for sysex response")
}

async fn read_global(client: &Client) -> RawGlobal {
    match request(client, global_dump_request(CHANNEL)).await {
        KorgR3Message::GlobalDump(g) => *g,
        other => panic!("expected GlobalDump, got {other:?}"),
    }
}

async fn read_current_program(client: &Client) -> RawProgram {
    match request(client, current_program_dump_request(CHANNEL)).await {
        KorgR3Message::CurrentProgramDump(p) => *p,
        other => panic!("expected CurrentProgramDump, got {other:?}"),
    }
}

async fn read_slot(client: &Client, slot: u16) -> RawProgram {
    match request(client, program_dump_request(CHANNEL, slot)).await {
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

async fn read_motion(client: &Client, motion_no: u8) -> (u16, Vec<RawFormantStep>) {
    match request(client, formant_motion_dump_request(CHANNEL, motion_no)).await {
        KorgR3Message::FormantMotionDump {
            motion_no: n,
            size,
            steps,
        } => {
            assert_eq!(n, motion_no);
            (size, steps)
        }
        other => panic!("expected FormantMotionDump, got {other:?}"),
    }
}

async fn load(client: &Client, data: SysEx) {
    match request(client, data).await {
        KorgR3Message::DataLoadCompleted => {}
        other => panic!("expected DataLoadCompleted, got {other:?}"),
    }
}

async fn write(client: &Client, write_request: SysEx) {
    match request(client, write_request).await {
        KorgR3Message::WriteCompleted => {}
        other => panic!("expected WriteCompleted, got {other:?}"),
    }
}

async fn write_motion(client: &Client, motion_no: u8, steps: &[RawFormantStep]) {
    load(client, current_formant_motion_dump_message(CHANNEL, steps)).await;
    write(client, formant_motion_write_request(CHANNEL, motion_no)).await;
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn global_dump_discovery() {
    let client = connect("r3-disc").await;

    eprintln!("Scanning channels 0-15...");
    let mut found_ch: Option<u8> = None;
    for try_ch in 0u8..=15 {
        let reply = try_request(&client, global_dump_request(try_ch), TIMEOUT).await;
        if reply.is_some() {
            eprintln!("  *** RESPONSE ch={try_ch} ***");
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
    match request(&client, global_dump_request(ch)).await {
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
    let client = connect("r3-pd").await;

    match request(&client, current_program_dump_request(CHANNEL)).await {
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
    let client = connect("r3-gt").await;

    let original = read_global(&client).await;
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

    load(&client, global_dump_message(CHANNEL, &modified)).await;

    let readback = read_global(&client).await;
    assert_eq!(readback.master_tune, new_tune);

    load(&client, global_dump_message(CHANNEL, &original)).await;

    let restored = read_global(&client).await;
    assert_eq!(bytemuck::bytes_of(&restored), bytemuck::bytes_of(&original));
    eprintln!("Global round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn program_round_trip() {
    let client = connect("r3-pt").await;

    let original = read_current_program(&client).await;
    eprintln!(
        "name = {:?}",
        std::str::from_utf8(&original.name).unwrap_or("<non-utf8>")
    );

    let mut modified = original;
    modified.name = *b"HILTEST ";

    load(&client, current_program_dump_message(CHANNEL, &modified)).await;

    let readback = read_current_program(&client).await;
    assert_eq!(&readback.name, b"HILTEST ");

    load(&client, current_program_dump_message(CHANNEL, &original)).await;

    let restored = read_current_program(&client).await;
    assert_eq!(bytemuck::bytes_of(&restored), bytemuck::bytes_of(&original));
    eprintln!("Program round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn parameter_change_program() {
    let client = connect("r3-pc").await;

    let original = read_current_program(&client).await;
    let orig_name0 = original.name[0];

    let new_name0: u8 = if orig_name0 != b'Z' { b'Z' } else { b'Y' };
    Link::open(&client)
        .await
        .send(&parameter_change_message(
            CHANNEL,
            0x00,
            0x00,
            new_name0 as u16,
        ))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let changed = read_current_program(&client).await;
    assert_eq!(
        changed.name[0], new_name0,
        "parameter change did not update current program name[0]"
    );

    load(&client, current_program_dump_message(CHANNEL, &original)).await;
    let restored = read_current_program(&client).await;
    assert_eq!(bytemuck::bytes_of(&restored), bytemuck::bytes_of(&original));
    eprintln!("Parameter change (program) OK");
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn program_dump_slot() {
    let client = connect("r3-sl").await;

    for slot in [0, 1, 32, 64] {
        match request(&client, program_dump_request(CHANNEL, slot)).await {
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
    let client = connect("r3-ws").await;

    let original = read_current_program(&client).await;
    eprintln!(
        "original name = {:?}",
        std::str::from_utf8(&original.name).unwrap_or("<non-utf8>")
    );

    let mut modified = original;
    modified.name = *b"WRITESL ";

    match request(&client, current_program_dump_message(CHANNEL, &modified)).await {
        KorgR3Message::DataLoadCompleted => {}
        KorgR3Message::DataLoadError => {
            eprintln!("load REJECTED (check memory protect)");
            return;
        }
        other => panic!("expected DataLoadCompleted or DataLoadError, got {other:?}"),
    }

    let target_slot: u16 = 0;
    match request(&client, program_write_request(CHANNEL, target_slot)).await {
        KorgR3Message::WriteCompleted => {
            eprintln!("write to slot {target_slot} succeeded");

            let readback = read_slot(&client, target_slot).await;
            assert_eq!(&readback.name, b"WRITESL ");

            load(&client, current_program_dump_message(CHANNEL, &original)).await;
            write(&client, program_write_request(CHANNEL, target_slot)).await;

            let restored = read_slot(&client, target_slot).await;
            assert_eq!(bytemuck::bytes_of(&restored), bytemuck::bytes_of(&original));
        }
        KorgR3Message::WriteError => {
            eprintln!("write REJECTED (check memory protect)");
        }
        other => panic!("expected WriteCompleted or WriteError, got {other:?}"),
    }
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn program_model_round_trip() {
    let client = connect("r3-mrt").await;

    let original = read_current_program(&client).await;
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

    load(&client, current_program_dump_message(CHANNEL, &raw2)).await;
    let readback = read_current_program(&client).await;
    let prog_rb = Program::try_from(readback).expect("decode device readback");
    assert_eq!(
        prog_rb.as_bytes(),
        encoded,
        "modeled parameters did not survive a real device round-trip"
    );

    load(&client, current_program_dump_message(CHANNEL, &original)).await;
    let restored = read_current_program(&client).await;
    assert_eq!(bytemuck::bytes_of(&restored), orig_bytes.as_slice());
    eprintln!("Program model round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn global_model_round_trip() {
    let client = connect("r3-gm").await;

    let original = read_global(&client).await;
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
    load(&client, global_dump_message(CHANNEL, &raw2)).await;
    let readback = read_global(&client).await;
    assert_eq!(
        bytemuck::bytes_of(&readback),
        encoded.as_slice(),
        "global did not survive device round-trip"
    );
    load(&client, global_dump_message(CHANNEL, &original)).await;
    eprintln!("Global model round-trip OK");
}

#[ignore = "requires connected Korg R3"]
#[tokio::test]
async fn tempo_encoding_probe() {
    let client = connect("r3-tp").await;

    for slot in [0u16, 1, 16, 32, 64, 99, 127] {
        match request(&client, program_dump_request(CHANNEL, slot)).await {
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
    let client = connect("r3-mo").await;

    match request(&client, current_formant_motion_dump_request(CHANNEL)).await {
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
    let client = connect("r3-mo").await;

    for i in 0u8..16 {
        let (size, steps) = read_motion(&client, i).await;
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
    let client = connect("r3-mo").await;

    const SCRATCH: u8 = 15;

    let (orig_size, orig_steps) = read_motion(&client, SCRATCH).await;
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

    write_motion(&client, SCRATCH, &synth).await;

    let (rb_size, rb_steps) = read_motion(&client, SCRATCH).await;
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

    write_motion(&client, SCRATCH, &orig_steps).await;
    let (restored_size, _) = read_motion(&client, SCRATCH).await;
    assert_eq!(restored_size, orig_size, "scratch motion restored");
}

#[ignore = "requires connected Korg R3 (memory protect OFF); writes slot 0"]
#[tokio::test]
async fn editor_write_path_fix_slot0_name() {
    let client = connect("r3-fix").await;

    let original = read_slot(&client, 0).await;
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

    match request(&client, current_program_dump_message(CHANNEL, &fixed)).await {
        KorgR3Message::DataLoadCompleted => {}
        KorgR3Message::DataLoadError => panic!("load REJECTED — memory protect is ON"),
        other => panic!("expected DataLoadCompleted, got {other:?}"),
    }
    match request(&client, program_write_request(CHANNEL, 0)).await {
        KorgR3Message::WriteCompleted => {}
        KorgR3Message::WriteError => panic!("write REJECTED — memory protect is ON"),
        other => panic!("expected WriteCompleted, got {other:?}"),
    }

    let readback = read_slot(&client, 0).await;
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
