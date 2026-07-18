# midilab-editor

GUI applications for programming supported MIDI controllers via Sysex. This
crate ships one binary per device, all built on the protocol definitions in the
[`midilab`](../midilab) library:

| Binary | Device |
|-|-|
| `akai_mpd226_editor` | Akai MPD226 |
| `arturia_minilab_mk2_editor` | Arturia MiniLab mkII |
| `korg_r3_editor` | Korg R3 |
| `nektar_impact_lx_plus_editor` | Nektar Impact LX+ (LX25+/49+/61+/88+) |

Run any editor from the workspace with `cargo run --bin <binary>`, e.g.:

```bash
cargo run --bin akai_mpd226_editor
```

## Akai MPD226

![Akai MPD226 Editor Screenshot](https://raw.githubusercontent.com/molenick/midilab/main/crates/midilab-editor/assets/akai_mpd226_screenshot.png)

```bash
cargo run --bin akai_mpd226_editor
```

Optionally, use a device simulator if you want to try the editor without real
hardware:

```bash
cargo install midilab-sim
akai_mpd226_sim
```

With both running, you can merrily send data back and forth between the editor
and simulator.

## Nektar Impact LX+

```bash
cargo run --bin nektar_impact_lx_plus_editor
```

Connect the keyboard over USB. The editor uses the `MIDI1` port; `MIDI2` is
the DAW-integration port and carries no sysex.

### Reading the device

The LX+ has no dump-request sysex, so reading is triggered from the keyboard:
press **[Setup]**, then the key labeled **Memory Dump** (G2). The display
reads `SYS` while the 182-message dump is sent; the editor assembles it
automatically and updates all tabs.

### Writing the device

Writes are silent (the device sends no acknowledgement):

- **Global settings and wheel/transport writes apply instantly.**
- **Preset and pad map writes only update stored memory** — they take effect
  the next time that preset or pad map is loaded from the panel.

### Gotcha: DAW-control mode

With the faders in Mixer/Instrument mode (rather than Preset mode) the faders
and pots send fixed channel-16 CCs and preset assignments are not in effect,
which can make edits appear to do nothing.

### Files

Whole-device dumps can be saved to and loaded from `.dump` files
(File → Save Dump / Load Dump). Edit → Factory Dump resets the editor to the
factory state, which is byte-perfect-verified against real hardware.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
