#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BIN_NAME="voice-to-text"

echo "=== Voice to Text — Installer ==="
echo ""

# Check system dependencies
MISSING=()
for pkg in cmake libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev \
           libayatana-appindicator3-dev libsoup-3.0-dev \
           libjavascriptcoregtk-4.1-dev xdotool xclip; do
    if ! dpkg -l "$pkg" &>/dev/null; then
        MISSING+=("$pkg")
    fi
done

if [ ${#MISSING[@]} -gt 0 ]; then
    echo "Missing system packages: ${MISSING[*]}"
    echo "Run: sudo apt install -y ${MISSING[*]}"
    exit 1
fi

# Check Rust
if ! command -v cargo &>/dev/null; then
    echo "Rust not found. Install via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Download model if needed
"$SCRIPT_DIR/download-model.sh"

# Build
echo ""
echo "Building application (release mode with CUDA)..."
cd "$PROJECT_DIR/src-tauri"
WHISPER_CUDA=1 cargo build --release

# Install binary
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp "target/release/$BIN_NAME" "$INSTALL_DIR/"
echo "Binary installed to $INSTALL_DIR/$BIN_NAME"

# Setup autostart
AUTOSTART_DIR="$HOME/.config/autostart"
mkdir -p "$AUTOSTART_DIR"
sed "s|Exec=voice-to-text|Exec=$INSTALL_DIR/$BIN_NAME|" \
    "$PROJECT_DIR/voice-to-text.desktop" > "$AUTOSTART_DIR/$BIN_NAME.desktop"
echo "Autostart configured in $AUTOSTART_DIR/$BIN_NAME.desktop"

echo ""
echo "=== Installation complete ==="
echo "Start with: $BIN_NAME"
echo "Or log out and back in for autostart."
