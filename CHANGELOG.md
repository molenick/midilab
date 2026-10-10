# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Korg R3 support: `midilab::manufacturer::korg::r3` with `Program`, `Global` and
  `FormantMotion` models, dump and write requests, parameter change messages and a
  `live` module for parameter diffs. Editor binary `korg_r3_editor`.
- Arturia MiniLab mkII support: `midilab::manufacturer::arturia::minilab_mk2` with
  param, global, memory, identity and pad color messages, `Preset`, `Global` and
  `ParamStore`. Editor binary `arturia_minilab_mk2_editor`. Simulator binary
  `arturia_minilab_mk2_sim`.
- Nektar Impact LX+ support: `midilab::manufacturer::nektar::impact_lx_plus` with
  preset control, pad, global setting and global control messages, `DumpAssembler`
  and `Dump`. Editor binary `nektar_impact_lx_plus_editor`.
- `midilab::sysex`: free functions `sysex`, `from_header_and_body`, `pack_u7` and
  `unpack_u7`, and the `SysExPreview` trait.
- `mpd226::reply_to` to match a device reply to its request.
- `Mpd226Error::InvalidLength`.
- `midilab-io`: `Listener` and `Link` for sysex transport on all ports.
- Hardware-in-the-loop test harness for the Korg R3.
- Factory dump fixture test for the Nektar Impact LX+.

### Changed

- All crates moved under `crates/`.
- `midilab` now depends on `midi-io` and uses its `SysEx` type end-to-end.
- `midilab::sysex::SysEx` is a re-export of `midi_io::SysEx`.
- MPD226 message builders return `SysEx` instead of `Vec<u8>`.
- `MidiError::InvalidSysex` wraps `midi_io::SysExError`.
- `MidiError::Send(String)` replaces the removed transport error variants.
- Rust toolchain updated to 1.94.0.
- egui updated to 0.35.
- Custom `PHI` constant replaced by the Rust standard constant.
- Dependencies updated to address cargo deny RUSTSEC advisories.

### Removed

- `midilab-gui` crate. Editors now live in `midilab-editor`.
- `midilab::sysex::Sysex`, `START_BYTE` and `END_BYTE`.
- `midilab::error::SysexParseError`.
- `MidiError` variants `WritePreset`, `DumpPreset`, `OutputConnection`,
  `InputConnection`, `ResponseTimeout` and `ChannelClosed`.
- `midilab` modules `config`, `message` and `state`.
- `midilab` dependencies `dirs`, `serde`, `serde_json` and `midir`.
- `midilab-io::fs` module.
- `ConfigError`.

## [0.2.6] - 2026-03-09

### Added

- Setting to sync with the device on app start when the device is connected.
- Superscript display for pitch octaves.
- Documentation for `midi` and `music::theory`.

### Changed

- App config loading improved.

## [0.2.5] - 2026-03-01

### Added

- Save and load a single binary preset file.
- Load a preset through a file dialog.
- Custom and default preset filenames.
- Global settings persistence.
- `Display` implementation for `Pitch`.

### Changed

- Sysex construction improvements.
- MIDI API maintenance.

### Removed

- `AppMsg` from the UI.

## [0.2.4] - 2026-02-25

### Added

- Change aftertouch for all pads in two taps.
- Unit tests for `midilab-sim`.
- Qwen UI feature.

### Changed

- `music` split into `theory` and `generation` modules.
- `midilab-io` split into `fs` and `midi` modules.
- MIDI note representation standardized on `MidiNote`.
- Pad `trigger` property renamed to `mode`.
- GUI modules reorganized.

### Fixed

- Dial CC values.
- Threshold rendering now matches the range 1 to 10.

## [0.2.3] - 2026-02-20

### Added

- Preset name in quick actions.
- Default note color mapping with improved colors.

### Changed

- `ChordRow` sequences rewritten.
- Control bank refactored.
- Note mapping UX improvements.
- CI runs cargo deny on branches.

## [0.2.2] - 2026-02-18

### Changed

- Pads are shown to users with `ui_id()` instead of the raw id.

## [0.2.1] - 2026-02-17

### Added

- Menu bar.
- Editing in a modal window.
- Note color mapping.
- `OctaveRowSequence`.
- Reset a preset to a blank state.
- First UI test with `egui_kittest`.
- Keyboard navigation at the top level of the app.
- Pad spacing in the GUI.

### Changed

- `u8` MIDI values replaced with `MidiValue`.
- Bespoke control editor functions replaced with a generic replacement.
- GUI repaints only on receipt of a `UiMsg`.
- Duplicate comboboxes reduced.
- Simulator fixes and improvements.
- The unmaintained `paste` crate is allowed in cargo deny.

## [0.2.0] - 2026-02-07

### Added

- Global settings write and dump.
- Preset persistence backend.
- Basic layout and pad design for the next version.
- Spacing function.

### Changed

- MPL-2.0 licenses allowed in cargo deny.
- Dependencies updated.

## [0.1.7] - 2026-02-02

### Fixed

- Preset `settings` were misinterpreted as `global`.

## [0.1.6] - 2026-02-01

### Changed

- Color variants replaced with `ColorPattern::Repeating(vec![ColorSequence])`.
- Supported controllers listed in the editor README.

## [0.1.5] - 2026-02-01

### Added

- Publishing script.

### Changed

- Crate names use hyphens instead of underscores.
- CI caches apt packages, tests and clippy.

## [0.1.4] - 2026-02-01

### Added

- Dial bank rendering.
- Fader and switch editor rendering.
- Editor screenshot in `editors/assets`.

### Changed

- Control mapping data types improved.
- Default preset uses the manufacturer default values for the MPD226 editor.

## [0.1.3] - 2026-01-31

### Fixed

- Links stay relative within their hosted platform.

## [0.1.2] - 2026-01-31

### Added

- Screenshot of `midilab-editor`.
- Tested code example in the `midilab` README.

### Changed

- GUI copy improved.
- Legacy naming removed.

### Fixed

- HIL test referenced a deleted package name.

## [0.1.1] - 2026-01-31

### Changed

- Workspace README and per-crate READMEs.
- CI runs cargo deny only for `main`.

### Fixed

- Version information restored for publishing.

## [0.1.0] - 2026-01-31

Initial release.

[Unreleased]: https://github.com/molenick/midilab/compare/v0.2.6...HEAD
[0.2.6]: https://github.com/molenick/midilab/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/molenick/midilab/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/molenick/midilab/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/molenick/midilab/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/molenick/midilab/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/molenick/midilab/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/molenick/midilab/compare/v0.1.7...v0.2.0
[0.1.7]: https://github.com/molenick/midilab/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/molenick/midilab/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/molenick/midilab/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/molenick/midilab/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/molenick/midilab/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/molenick/midilab/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/molenick/midilab/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/molenick/midilab/releases/tag/v0.1.0
