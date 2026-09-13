# MidiLab
A framework for editing and serializing MIDI controller Sysex

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
| **[midilab](./crates/midilab)** | `crates/midilab` | Manufacturer device definitions |
| **[midilab-editor](./crates/midilab-editor)** | `crates/midilab-editor` | Editor applications (`akai_mpd226_editor`, `arturia_minilab_mk2_editor`, `korg_r3_editor`, `nektar_impact_lx_plus_editor`) |
| **[midilab-io](./crates/io)** | `crates/io` | State machines for i/o management |
| **[midilab-sim](./crates/sim)** | `crates/sim` | Hardware device simulations |

## Supported devices

- Akai Mpd226 
- Korg R3
- Nektar Impact LX+ series (LX25+/49+/61+/88+)
- Arturia MiniLab mkII

## Credits

Thanks to [mpd-utils](https://github.com/mungewell/mpd-utils) for providing a starting point for understanding the Akai MPD226 Sysex payload deserialization.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
