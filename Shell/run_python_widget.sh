#!/usr/bin/env bash
# Launcher script for Python Hotspot & Stats Widget on Linux
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY_DIR="$SCRIPT_DIR/../Python"

cd "$PY_DIR"
echo "Starting Python Hotspot & Stats Widget..."
exec python3 main.py "$@"
