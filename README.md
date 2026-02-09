# Voice to Text

Speech-to-text desktop app using [whisper.cpp](https://github.com/ggerganov/whisper.cpp) with GPU acceleration (CUDA). Built with [Tauri](https://tauri.app/).

**Double-press Alt** to start/stop recording. Transcribed text is automatically typed into the active text field.

## Requirements

- Linux with X11 (GNOME, KDE, etc.)
- NVIDIA GPU with CUDA support
- CUDA Toolkit 12.x
- System packages:

```bash
sudo apt install -y cmake libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev \
  libayatana-appindicator3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev \
  xdotool xclip
```

## Quick Start

```bash
# Clone
git clone https://github.com/antoszy/voice-to-text.git
cd voice-to-text

# Install everything (deps check, model download, build, autostart)
scripts/install.sh
```

## Manual Setup

```bash
# 1. Download whisper model (~1.6 GB)
scripts/download-model.sh

# 2. Build
cd src-tauri
cargo build --release

# 3. Run
./target/release/voice-to-text
```

## Usage

| Action | Effect |
|---|---|
| Double-press **Alt** | Start/stop recording |
| Click tray icon | Show/hide settings window |
| Right-click tray | Menu (Show/Quit) |

The app runs in the system tray. When recording stops, audio is transcribed and the result is pasted into the currently focused text field.

## Configuration

- **Language**: Polish (default), English, German, Ukrainian, or auto-detect
- **Model**: whisper large-v3-turbo (stored in `~/.local/share/voice-to-text/models/`)

## Architecture

```
src-tauri/src/
  lib.rs          — Tauri app setup, state management, tray
  audio.rs        — Microphone recording (cpal)
  transcribe.rs   — Whisper.cpp transcription (whisper-rs + CUDA)
  hotkey.rs       — Double-Alt detection (rdev)
  typing.rs       — Text insertion (xclip + xdotool)
```

## License

MIT
