# midilab-editor
An application for programming MIDI controllers

![MidiLab Editor Screenshot](https://raw.githubusercontent.com/molenick/midilab/main/editor/assets/screenshot.png)

## Supported Controllers
- Akai Mpd226

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

## License
Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.