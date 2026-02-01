# midilab-editor
An application for programming MIDI controllers

![MidiLab Editor Screenshot](https://github.com/user-attachments/assets/d3bacca6-16a6-42d1-a76d-b52eb8e94da0)

The Akai Mpd226 is the only controller available in this early editor release. It's a work-in-progress and only preset Global and Pad information can be programmed at the moment.

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