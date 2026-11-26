#!/usr/bin/env bash
# Build script for Rust Hotspot & Stats Widget on Linux (Arch, Debian, Kali, Ubuntu)
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="$SCRIPT_DIR/../Rust/test_gui"

cd "$RUST_DIR"
echo "Building Hotspot & Stats Widget in Release mode..."
cargo build --release

echo "Copying binary to Rust directory..."
cp target/release/hotspot_stats_widget ../hotspot_stats_widget
chmod +x ../hotspot_stats_widget

echo "Build complete! Linux native binary is at: Rust/hotspot_stats_widget"
