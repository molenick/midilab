# midilab
Software library for programming midi controllers via Sysex messages

Device definitions by manufacturer and APIs for sysex message de/serialiation

## Library usage:
Add midilab to your Cargo.toml:

```bash
cargo add midilab
```

### Re-exports
midilab re-exports `strum::IntoEnumIterator` as `midilab::IntoEnumIterator` so consumers of this
library can take advantage of derived EnumIter features in many of midilab's enums.

## License
Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
