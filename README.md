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
| **midilab** | `core` | Core library |
| **midilab-editor** | `editor` | An application for programming MIDI controllers |
| **midilab-io** | `io` | Mediates access to I/O resources such as sending and receiving SysEx to MIDI ports |
| **midilab-gui** | `gui` | Graphical user interface library |
| **midilab-sim** | `sim` | Hardware device simulations |


## Supported devices

### Akai Mpd226 (work in progress)
#### Completed features:
    - Sysex deserialization into domain types
    - Pad Mapping
    - Global Mapping
    - Preset dump
    - Preset send
    - Basic note mapping
    - Basic LED color mapping

#### Feature roadmap:
    - Dial mapping
    - Fader mapping
    - Switch mapping
    - Additional note/led pattern mapping options
    - Local persistence of presets

## Editor usage

Install and run midilab-editor:

```bash
cargo install midilab-editor
midilab
```

Optionally, you use a device simulator if you want to try the editor without real hardware:
```bash
cargo install midilab-sim
akai_mpd226_sim
```

With both running, you can merrily send data back and forth between the editor and simulator.

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

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
