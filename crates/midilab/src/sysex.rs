use bytemuck::Pod;
pub use midi_io::SysEx;

pub fn pack_u14(val: u16) -> [u8; 2] {
    let high = ((val >> 7) & 0x7F) as u8;
    let low = (val & 0x7F) as u8;
    [high, low]
}

pub fn unpack_u14(bytes: [u8; 2]) -> u16 {
    let high = (bytes[0] & 0x7F) as u16;
    let low = (bytes[1] & 0x7F) as u16;
    (high << 7) | low
}

pub fn pack_u7(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity((data.len() * 8).div_ceil(7));
    for chunk in data.chunks(7) {
        let mut msb_byte: u8 = 0;
        for (i, &byte) in chunk.iter().enumerate() {
            if byte & 0x80 != 0 {
                msb_byte |= 1 << i;
            }
        }
        out.push(msb_byte);
        for &byte in chunk {
            out.push(byte & 0x7F);
        }
    }
    out
}

pub fn unpack_u7(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let msb_byte = data[i];
        i += 1;
        for bit in 0..7 {
            if i >= data.len() {
                break;
            }
            let msb = if msb_byte & (1 << bit) != 0 { 0x80 } else { 0 };
            out.push(data[i] | msb);
            i += 1;
        }
    }
    out
}

pub fn sysex(payload: impl AsRef<[u8]>) -> SysEx {
    SysEx::new(payload.as_ref()).expect("sysex payload must be non-empty and 7-bit clean")
}

pub fn from_header_and_body<H: Pod, B: AsRef<[u8]>>(header: &H, body: B) -> SysEx {
    let mut payload = bytemuck::bytes_of(header).to_vec();
    payload.extend_from_slice(body.as_ref());
    sysex(payload)
}

pub trait SysExPreview {
    fn preview(&self) -> String;
}

impl SysExPreview for SysEx {
    fn preview(&self) -> String {
        const CHUNK_HEAD_LEN: usize = 4;
        const CHUNK_TAIL_LEN: usize = 4;
        const CHUNK_TOTAL_LEN: usize = CHUNK_HEAD_LEN + CHUNK_TAIL_LEN;

        let payload = self.bytes();
        if payload.len() <= CHUNK_TOTAL_LEN {
            format!("{payload:02x?}, {} bytes", payload.len())
        } else {
            let chunk_head = &payload[0..CHUNK_HEAD_LEN];
            let chunk_tail = &payload[payload.len() - CHUNK_TAIL_LEN..];

            let hs: Vec<String> = chunk_head.iter().map(|i| format!("{i:02x}")).collect();
            let hs = hs.join(", ");

            let ts: Vec<String> = chunk_tail.iter().map(|i| format!("{i:02x}")).collect();
            let ts = ts.join(", ");

            format!("[{hs}, ..., {ts}], {} bytes", payload.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use midi_io::SysExError;

    use super::*;

    #[test]
    fn test_sysex_preview_short_payload() {
        let s = sysex([0, 1, 2, 3, 4, 5, 6, 7]);

        assert_eq!(&s.preview(), "[00, 01, 02, 03, 04, 05, 06, 07], 8 bytes");
    }

    #[test]
    fn test_sysex_preview_long_payload() {
        let s = sysex([0, 1, 2, 3, 4, 5, 6, 7, 8]);

        assert_eq!(
            &s.preview(),
            "[00, 01, 02, 03, ..., 05, 06, 07, 08], 9 bytes"
        );
    }

    #[test]
    fn test_pack_u14_zero() {
        let result = pack_u14(0x0000);
        assert_eq!(result, [0x00, 0x00]);
    }

    #[test]
    fn test_pack_u14_max_14bit() {
        let result = pack_u14(0x3FFF);
        assert_eq!(result, [0x7F, 0x7F]);
    }

    #[test]
    fn test_pack_u14_masks_high_bits() {
        let result = pack_u14(0xFFFF);
        assert_eq!(result, [0x7F, 0x7F]);

        let result = pack_u14(0x4000);
        assert_eq!(result, [0x00, 0x00]);

        let result = pack_u14(0x8000);
        assert_eq!(result, [0x00, 0x00]);
    }

    #[test]
    fn test_pack_u14_various_values() {
        assert_eq!(pack_u14(0x0001), [0x00, 0x01]);
        assert_eq!(pack_u14(0x007F), [0x00, 0x7F]);
        assert_eq!(pack_u14(0x0080), [0x01, 0x00]);
        assert_eq!(pack_u14(0x00FF), [0x01, 0x7F]);
        assert_eq!(pack_u14(0x0100), [0x02, 0x00]);
        assert_eq!(pack_u14(0x1000), [0x20, 0x00]);
        assert_eq!(pack_u14(0x2AAA), [0x55, 0x2A]);
    }

    #[test]
    fn test_unpack_u14_zero() {
        let result = unpack_u14([0x00, 0x00]);
        assert_eq!(result, 0x0000);
    }

    #[test]
    fn test_unpack_u14_max() {
        let result = unpack_u14([0x7F, 0x7F]);
        assert_eq!(result, 0x3FFF);
    }

    #[test]
    fn test_unpack_u14_masks_high_bits() {
        let result = unpack_u14([0xFF, 0xFF]);
        assert_eq!(result, 0x3FFF);

        let result = unpack_u14([0x80, 0x00]);
        assert_eq!(result, 0x0000);

        let result = unpack_u14([0x00, 0x80]);
        assert_eq!(result, 0x0000);
    }

    #[test]
    fn test_unpack_u14_various_values() {
        assert_eq!(unpack_u14([0x00, 0x01]), 0x0001);
        assert_eq!(unpack_u14([0x00, 0x7F]), 0x007F);
        assert_eq!(unpack_u14([0x01, 0x00]), 0x0080);
        assert_eq!(unpack_u14([0x01, 0x7F]), 0x00FF);
        assert_eq!(unpack_u14([0x02, 0x00]), 0x0100);
        assert_eq!(unpack_u14([0x20, 0x00]), 0x1000);
        assert_eq!(unpack_u14([0x55, 0x2A]), 0x2AAA);
    }

    #[test]
    fn test_pack_unpack_u14_round_trip() {
        for val in [
            0x0000, 0x0001, 0x007F, 0x0080, 0x00FF, 0x0100, 0x0FFF, 0x1000, 0x1FFF, 0x2000, 0x3FFF,
        ] {
            assert_eq!(unpack_u14(pack_u14(val)), val);
        }
    }

    #[test]
    fn test_sysex_to_wire_bytes() {
        let s = sysex([0x47, 0x00, 0x35]);

        let bytes = s.to_wire_bytes();
        assert_eq!(bytes[0], 0xf0);
        assert_eq!(bytes[1], 0x47);
        assert_eq!(bytes[2], 0x00);
        assert_eq!(bytes[3], 0x35);
        assert_eq!(bytes[4], 0xf7);
        assert_eq!(bytes.len(), 5);
    }

    #[test]
    fn test_sysex_try_from_vec_valid() {
        let data = vec![0xf0, 0x47, 0x00, 0x35, 0xf7];
        let s = SysEx::try_from(data.as_slice()).unwrap();

        assert_eq!(s.bytes(), &[0x47, 0x00, 0x35]);
    }

    #[test]
    fn test_sysex_try_from_empty_payload_is_err() {
        let data = vec![0xf0, 0xf7];
        let result = SysEx::try_from(data.as_slice()).unwrap_err();

        assert!(matches!(result, SysExError::EmptyBody));
    }

    #[test]
    fn test_sysex_try_from_invalid_start_byte() {
        let data = vec![0x00, 0x47, 0x00, 0x35, 0xf7];
        let result = SysEx::try_from(data.as_slice()).unwrap_err();

        assert!(matches!(result, SysExError::MissingStart))
    }

    #[test]
    fn test_sysex_try_from_invalid_end_byte() {
        let data = vec![0xf0, 0x47, 0x00, 0x35, 0x00];
        let result = SysEx::try_from(data.as_slice()).unwrap_err();

        assert!(matches!(result, SysExError::Unterminated))
    }

    #[test]
    fn test_sysex_try_from_empty_data() {
        let data: Vec<u8> = vec![];
        let result = SysEx::try_from(data.as_slice()).unwrap_err();

        assert!(matches!(result, SysExError::MissingStart));
    }

    #[test]
    fn test_sysex_try_from_single_byte() {
        let data = vec![0xf0];
        let result = SysEx::try_from(data.as_slice()).unwrap_err();

        assert!(matches!(result, SysExError::Unterminated));
    }

    #[test]
    fn test_pack_u7_known_vector() {
        let input = [0x80, 0x00, 0x81, 0x00, 0x00, 0x00, 0x00];
        let packed = pack_u7(&input);

        assert_eq!(packed.len(), 8);
        assert_eq!(packed[0], 0x05);
        assert_eq!(packed[1], 0x00);
        assert_eq!(packed[2], 0x00);
        assert_eq!(packed[3], 0x01);
        assert_eq!(packed[4], 0x00);
        assert_eq!(packed[5], 0x00);
        assert_eq!(packed[6], 0x00);
        assert_eq!(packed[7], 0x00);

        let unpacked = unpack_u7(&packed);
        assert_eq!(unpacked, input);
    }

    #[test]
    fn test_pack_u7_global_size() {
        let data = vec![0u8; 80];
        let packed = pack_u7(&data);
        assert_eq!(packed.len(), 92);

        let unpacked = unpack_u7(&packed);
        assert_eq!(unpacked.len(), 80);
    }

    #[test]
    fn test_pack_u7_program_size() {
        let data = vec![0u8; 456];
        let packed = pack_u7(&data);
        assert_eq!(packed.len(), 522);

        let unpacked = unpack_u7(&packed);
        assert_eq!(unpacked.len(), 456);
    }

    #[test]
    fn test_pack_u7_round_trip() {
        let data = (0..=255).cycle().take(456).collect::<Vec<u8>>();
        let packed = pack_u7(&data);
        let unpacked = unpack_u7(&packed);
        assert_eq!(unpacked, data);
    }

    #[test]
    fn test_sysex_round_trip() {
        let original_payload = vec![0x47, 0x00, 0x35, 0x10, 0x08, 0x33];
        let s = sysex(&original_payload);
        let bytes = s.to_wire_bytes();
        let reconstructed = SysEx::try_from(bytes.as_slice()).unwrap();

        assert_eq!(reconstructed.bytes(), &original_payload);
    }
}
