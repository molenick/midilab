#!/usr/bin/env bash
set -euo pipefail

# Build WASM package for midilab-editor
# Prerequisites:
#   - Rust toolchain with wasm32-unknown-unknown target
#   - wasm-bindgen-cli (install with: cargo install wasm-bindgen-cli)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PKG_DIR="$PROJECT_DIR/editor/pkg"
TARGET_DIR="$PROJECT_DIR/target/wasm32-unknown-unknown/release"

# Ensure pkg directory exists
mkdir -p "$PKG_DIR"

# Check for wasm-bindgen-cli
if ! command -v wasm-bindgen &> /dev/null; then
    echo "wasm-bindgen-cli not found. Installing..."
    cargo install wasm-bindgen-cli
fi

# Build WASM library
echo "Building WASM library..."
cd "$PROJECT_DIR"
cargo build --target wasm32-unknown-unknown --lib --release

# Generate JS bindings
echo "Generating JS bindings..."
wasm-bindgen \
    --target web \
    --out-name midilab_editor \
    --out-dir "$PKG_DIR" \
    "$TARGET_DIR/midilab_editor.wasm"

# Remove .wasm file - wasm-bindgen generates *_bg.wasm which is the actual wasm
# and midilab_editor.js which is the JS glue code
rm -f "$PKG_DIR/midilab_editor.wasm"

# Copy index.html to pkg directory
sed 's|./pkg/midilab_editor.js|./midilab_editor.js|g' "$PROJECT_DIR/editor/index.html" > "$PKG_DIR/index.html"

echo "Build complete. Output in $PKG_DIR/"
ls -la "$PKG_DIR/"
