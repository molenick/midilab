# midilab
Software library for programming midi controllers via Sysex messages

Device definitions by manufacturer and APIs for sysex message de/serialiation

## Library usage:
Add midilab to your Cargo.toml:

```bash
cargo add midilab
```

### Examples

Parse raw bytes into a SysEx message:
```rust
use midilab::sysex::SysEx;

let bytes: &[u8] = &[0xf0, 0x47, 0x00, 0x35, 0xf7];
let sysex = SysEx::try_from(bytes).unwrap();
assert_eq!(sysex.bytes(), &[0x47, 0x00, 0x35]);
```

Deserialize an Akai MPD226 preset acknowledgment:
```rust
use midilab::sysex::SysEx;
use midilab::manufacturer::akai::mpd226::DeviceStatus;

// Deserialize an Akai Mpd226's raw PresetAck bytes into SysEx
let bytes: &[u8] = &[0xf0, 0x47, 0x00, 0x35, 0x11, 0x00, 0x01, 0x00, 0x00, 0xf7];
let sysex = SysEx::try_from(bytes).unwrap();
// Deserialize the SysEx into a DeviceStatus variant
let status = DeviceStatus::try_from(sysex).unwrap();
```

### Re-exports
midilab re-exports `strum::IntoEnumIterator` as `midilab::IntoEnumIterator` so consumers of this
library can take advantage of derived EnumIter features in many of midilab's enums.

## License
Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/molenick/midilab/blob/main/LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](https://github.com/molenick/midilab/blob/main/LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
