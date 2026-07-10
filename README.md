# MidiLab
Software for programming midi controllers via Sysex messages

[![Crates.io][crates-badge]][crates-url]
[![License][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/midilab.svg
[crates-url]: https://crates.io/crates/midilab
[license-badge]: https://img.shields.io/crates/l/midilab.svg
[license-url]: https://github.com/molenick/midilab#license
[actions-badge]: https://github.com/molenick/midilab/actions/workflows/ci.yml/badge.svg
[actions-url]: https://github.com/molenick/midilab/actions/workflows/ci.yml

## Crate Overview
| Crate | Directory | Description |
|-|-|-|
| **[midilab](./midilab)** | `midilab` | Device definitions by manufacturer and APIs for sysex message de/serialiation |
| **[akai_mpd226_editor](./akai_mpd226_editor)** | `akai_mpd226_editor` | An application for programming the Akai MPD226 |
| **[korg_r3_editor](./korg_r3_editor)** | `korg_r3_editor` | An application for programming the Korg R3 |
| **[midilab-io](./io)** | `io` | Mediates access to I/O resources such as sending and receiving SysEx to MIDI ports for the midilab editors |
| **[midilab-sim](./sim)** | `sim` | Hardware device simulations for the midilab editors |

## Supported devices

Akai Mpd226 (work in progress)

Korg R3 (work in progress)

## Credits

Thanks to [mpd-utils](https://github.com/mungewell/mpd-utils) for providing a starting point for understanding the Sysex payload deserialization.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
