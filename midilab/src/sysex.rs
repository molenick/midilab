use crate::error::SysexParseError;

pub const START_BYTE: u8 = 0xf0;
pub const END_BYTE: u8 = 0xf7;

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

/// Simple Sysex wrapper for serializing into bytes or as a first step in transforming binary into structured domain messages.
/// See the [spec](http://midi.teragonaudio.com/tech/midispec/sysex.htm) for additional information.
/// Only single-byte manufacturer ids are supported at this time.
#[derive(Debug, Clone)]
pub struct Sysex {
    payload: Vec<u8>,
}

impl Sysex {
    pub fn new(payload: Vec<u8>) -> Self {
        Self { payload }
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.payload.len() + 2);
        data.push(START_BYTE);
        data.extend_from_slice(&self.payload);
        data.push(END_BYTE);
        data
    }

    /// The total length of the sysex payload, including header and footer bytes
    #[expect(
        clippy::len_without_is_empty,
        reason = "valid sysex always has a length of at least 1, so empty is semantically invalid here"
    )]
    pub fn len(&self) -> usize {
        self.payload.len() + 2
    }
}

impl TryFrom<&[u8]> for Sysex {
    type Error = SysexParseError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let (first, remaining) = data.split_first().ok_or(SysexParseError::Empty)?;

        if *first != START_BYTE {
            return Err(SysexParseError::InvalidStart(*first));
        }

        let (last, payload_slice) = remaining
            .split_last()
            .ok_or(SysexParseError::MissingEnding)?;

        if *last != END_BYTE {
            return Err(SysexParseError::InvalidEnding(*last));
        }

        Ok(Sysex {
            payload: payload_slice.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sysex_as_bytes() {
        let payload = vec![0x47, 0x00, 0x35];
        let sysex = Sysex::new(payload);

        let bytes = sysex.as_bytes();
        assert_eq!(bytes[0], START_BYTE);
        assert_eq!(bytes[1], 0x47);
        assert_eq!(bytes[2], 0x00);
        assert_eq!(bytes[3], 0x35);
        assert_eq!(bytes[4], END_BYTE);
        assert_eq!(bytes.len(), 5);
    }

    #[test]
    fn test_sysex_try_from_vec_valid() {
        let data = vec![START_BYTE, 0x47, 0x00, 0x35, END_BYTE];
        let sysex = Sysex::try_from(data.as_slice()).unwrap();

        assert_eq!(sysex.payload(), &[0x47, 0x00, 0x35]);
    }

    #[test]
    fn test_sysex_try_from_slice_valid() {
        let data: &[u8] = &[START_BYTE, 0x47, 0x00, 0x35, END_BYTE];
        let sysex = Sysex::try_from(data).unwrap();

        assert_eq!(sysex.payload(), &[0x47, 0x00, 0x35]);
    }

    #[test]
    fn test_sysex_try_from_empty_payload() {
        let data = vec![START_BYTE, END_BYTE];
        let sysex = Sysex::try_from(data.as_slice()).unwrap();

        assert_eq!(sysex.payload(), &[] as &[u8]);
    }

    #[test]
    fn test_sysex_try_from_invalid_start_byte() {
        let data = vec![0x00, 0x47, 0x00, 0x35, END_BYTE];
        let result = Sysex::try_from(data.as_slice()).unwrap_err();

        assert!(matches!(result, SysexParseError::InvalidStart(0x00)))
    }

    #[test]
    fn test_sysex_try_from_invalid_end_byte() {
        let data = vec![START_BYTE, 0x47, 0x00, 0x35, 0x00];
        let result = Sysex::try_from(data.as_slice()).unwrap_err();

        assert!(matches!(result, SysexParseError::InvalidEnding(0x00)))
    }

    #[test]
    fn test_sysex_try_from_empty_data() {
        let data: Vec<u8> = vec![];
        let result = Sysex::try_from(data.as_slice()).unwrap_err();

        assert!(matches!(result, SysexParseError::Empty));
    }

    #[test]
    fn test_sysex_try_from_single_byte() {
        let data = vec![START_BYTE];
        let result = Sysex::try_from(data.as_slice()).unwrap_err();

        assert!(matches!(result, SysexParseError::MissingEnding));
    }

    #[test]
    fn test_sysex_round_trip() {
        let original_payload = vec![0x47, 0x00, 0x35, 0x10, 0x08, 0x33];
        let sysex = Sysex::new(original_payload.clone());
        let bytes = sysex.as_bytes();
        let reconstructed = Sysex::try_from(bytes.as_slice()).unwrap();

        assert_eq!(reconstructed.payload(), &original_payload);
    }

    #[test]
    fn test_sysex_error_messages() {
        let start_err = SysexParseError::InvalidStart(0x42);
        assert!(start_err.to_string().contains("42"));
        assert!(start_err.to_string().contains("F0"));

        let end_err = SysexParseError::InvalidEnding(0x42);
        assert!(end_err.to_string().contains("42"));
        assert!(end_err.to_string().contains("F7"));
    }
}
