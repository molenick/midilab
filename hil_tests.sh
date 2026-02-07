#!/usr/bin/env bash
set -euo pipefail

# Run hardware-in-loop tests (normally ignored)
cargo test --package midilab --test hil_tests -- --ignored --test-threads=1 "$@"
