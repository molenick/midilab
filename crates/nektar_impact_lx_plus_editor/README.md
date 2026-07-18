# Nektar Impact LX+ Editor

GUI application for programming Nektar Impact LX+ controllers (LX25+/49+/61+/88+)
via sysex, built on the reverse-engineered protocol in
`midilab::manufacturer::nektar::impact_lx_plus`.

## Usage

```sh
cargo run -p nektar_impact_lx_plus_editor
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

## Files

Whole-device dumps can be saved to and loaded from `.dump` files
(File → Save Dump / Load Dump). Edit → Factory Dump resets the editor to the
factory state, which is byte-perfect-verified against real hardware.
