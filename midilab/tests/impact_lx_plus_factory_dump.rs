//! Byte-perfect verification of the Impact LX+ model against a real factory
//! memory dump captured from an unmodified LX61+ (2026-07-16).

use midilab::manufacturer::nektar::impact_lx_plus::DUMP_MESSAGE_COUNT;
use midilab::manufacturer::nektar::impact_lx_plus::DeviceStatus;
use midilab::manufacturer::nektar::impact_lx_plus::Dump;
use midilab::manufacturer::nektar::impact_lx_plus::DumpAssembler;

const FACTORY_DUMP: &[u8] = include_bytes!("data/impact_lx61plus_factory_dump.syx");

fn factory_messages() -> Vec<Vec<u8>> {
    FACTORY_DUMP
        .split_inclusive(|b| *b == 0xF7)
        .map(<[u8]>::to_vec)
        .collect()
}

#[test]
fn factory_dump_has_expected_message_count() {
    assert_eq!(factory_messages().len(), DUMP_MESSAGE_COUNT);
}

#[test]
fn factory_dump_parses_completely() {
    let mut assembler = DumpAssembler::default();
    for message in factory_messages() {
        let status = DeviceStatus::try_from(message.as_slice()).unwrap();
        assembler.apply(&status);
    }
    assert!(assembler.is_complete());
    assembler.try_into_dump().unwrap();
}

#[test]
fn factory_dump_decodes_to_default_dump() {
    let mut assembler = DumpAssembler::default();
    for message in factory_messages() {
        assembler.apply(&DeviceStatus::try_from(message.as_slice()).unwrap());
    }
    assert_eq!(assembler.try_into_dump().unwrap(), Dump::default());
}

#[test]
fn default_dump_encodes_byte_perfect_factory_dump() {
    let encoded = Dump::default().to_messages();
    let captured = factory_messages();

    assert_eq!(encoded.len(), captured.len());
    for (index, (encoded, captured)) in encoded.iter().zip(captured.iter()).enumerate() {
        assert_eq!(encoded, captured, "message {index} differs");
    }
}
