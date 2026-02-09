# Voice to Text

Speech-to-text desktop app using [whisper.cpp](https://github.com/ggerganov/whisper.cpp) with GPU acceleration (CUDA). Built with [Tauri](https://tauri.app/).

**Double-press Alt** to start/stop recording. Transcribed text is automatically typed into the active text field.

## Modes

- **Streaming** (default) — text appears in real-time as you speak (~6s latency). Whisper re-transcribes the full audio every 3 seconds, confirmed text is typed incrementally.
- **Batch** — records until you double-press Alt again, then transcribes the entire recording at once.

Switch between modes in the Settings window (tray menu → Settings).

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

# 2. Build (with CUDA for NVIDIA GPUs)
cd src-tauri
CMAKE_CUDA_ARCHITECTURES="90-virtual" cargo build --release

# 3. Run
./target/release/voice-to-text
```

## Usage

| Action | Effect |
|---|---|
| Double-press **Alt** | Start recording |
| Double-press **Alt** again | Stop recording (+ transcribe in batch mode) |
| Tray menu → **Settings** | Open settings (mode, language) |
| Tray menu → **Quit** | Exit app |

The app runs in the system tray. In streaming mode, text is typed into the focused field as you speak. In batch mode, text is typed after you stop recording.

## Configuration

- **Mode**: Streaming (real-time) or Batch (after stop)
- **Language**: Polish (default), English, German, Ukrainian, or auto-detect
- **Model**: whisper large-v3-turbo (stored in `~/.local/share/voice-to-text/models/`)

## Architecture

```
src-tauri/src/
  lib.rs          — Tauri app, worker thread, streaming/batch logic, tray
  audio.rs        — Microphone recording + snapshot for streaming (cpal)
  transcribe.rs   — Whisper.cpp transcription (whisper-rs + CUDA)
  hotkey.rs       — Double-Alt detection (rdev)
  typing.rs       — Text insertion (xclip + xdotool)
```

## License

MIT
