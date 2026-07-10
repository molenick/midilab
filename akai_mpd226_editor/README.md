# akai_mpd226_editor
An application for programming the Akai MPD226 MIDI controller

![Akai MPD226 Editor Screenshot](https://raw.githubusercontent.com/molenick/midilab/main/akai_mpd226_editor/assets/screenshot.png)

## Editor usage
Run the editor from the workspace:

```bash
cargo run -p akai_mpd226_editor
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
