#!/usr/bin/env bash
set -euo pipefail

echo "Building Voice-typeless (macOS)..."

cd "$(dirname "$0")/../frontend"
npm run build

cd "../src-tauri"
cargo tauri build

echo "macOS build complete!"
