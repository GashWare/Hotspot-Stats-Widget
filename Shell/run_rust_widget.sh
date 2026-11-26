#!/usr/bin/env bash
# Launcher script for Rust Hotspot & Stats Widget on Linux
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$SCRIPT_DIR/../Rust/hotspot_stats_widget"

if [ ! -f "$BIN" ]; then
    BIN="$SCRIPT_DIR/../Rust/test_gui/target/release/hotspot_stats_widget"
fi

if [ -f "$BIN" ]; then
    chmod +x "$BIN"
    echo "Starting Rust Hotspot & Stats Widget..."
    exec "$BIN" "$@"
else
    echo "Binary not found. Please build first using: ./Shell/build_rust.sh"
    exit 1
fi
