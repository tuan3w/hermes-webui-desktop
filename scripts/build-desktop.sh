#!/usr/bin/env bash
# Build the Hermes desktop app (Linux: .deb + AppImage).
# Requires: Rust, cargo-tauri, python3, ImageMagick (for icon conversion).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# ── 1. Tauri CLI ──────────────────────────────────────────────────────────────
if ! cargo tauri --version &>/dev/null 2>&1; then
  echo "[desktop] Installing tauri-cli..."
  cargo install tauri-cli --version "^2.0" --locked
fi

# ── 2. Icons ──────────────────────────────────────────────────────────────────
ICON_SRC="hermes-webui/static/favicon-512.png"
ICON_DIR="src-tauri/icons"
mkdir -p "$ICON_DIR"

if command -v convert &>/dev/null; then
  echo "[desktop] Generating icons from $ICON_SRC..."
  convert "$ICON_SRC" -resize 32x32   "$ICON_DIR/32x32.png"
  convert "$ICON_SRC" -resize 128x128 "$ICON_DIR/128x128.png"
  convert "$ICON_SRC" -resize 256x256 "$ICON_DIR/256x256.png"
elif command -v sips &>/dev/null; then
  sips -z 32  32  "$ICON_SRC" --out "$ICON_DIR/32x32.png"
  sips -z 128 128 "$ICON_SRC" --out "$ICON_DIR/128x128.png"
  sips -z 256 256 "$ICON_SRC" --out "$ICON_DIR/256x256.png"
elif command -v ffmpeg &>/dev/null; then
  ffmpeg -i "$ICON_SRC" -vf scale=32:32   "$ICON_DIR/32x32.png"   -y -frames:v 1 -q:v 1 2>/dev/null
  ffmpeg -i "$ICON_SRC" -vf scale=128:128 "$ICON_DIR/128x128.png" -y -frames:v 1 -q:v 1 2>/dev/null
  ffmpeg -i "$ICON_SRC" -vf scale=256:256 "$ICON_DIR/256x256.png" -y -frames:v 1 -q:v 1 2>/dev/null
else
  echo "[desktop] Warning: ImageMagick/sips/ffmpeg not found — using source icon for all sizes."
  cp "$ICON_SRC" "$ICON_DIR/32x32.png"
  cp "$ICON_SRC" "$ICON_DIR/128x128.png"
  cp "$ICON_SRC" "$ICON_DIR/256x256.png"
fi
cp "$ICON_SRC" "$ICON_DIR/512x512.png"
cp "$ICON_SRC" "$ICON_DIR/icon.png"

# ── 3. Build ──────────────────────────────────────────────────────────────────
echo "[desktop] Building Tauri app..."
cd src-tauri
cargo tauri build

echo ""
echo "[desktop] Done!  Artifacts:"
find target/release/bundle -name "*.deb" -o -name "*.AppImage" 2>/dev/null | sed 's/^/  /'
