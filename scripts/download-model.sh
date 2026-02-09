#!/usr/bin/env bash
set -euo pipefail

MODEL_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/voice-to-text/models"
MODEL_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin"
MODEL_FILE="$MODEL_DIR/ggml-large-v3-turbo.bin"

mkdir -p "$MODEL_DIR"

if [ -f "$MODEL_FILE" ]; then
    echo "Model already exists at $MODEL_FILE"
    exit 0
fi

echo "Downloading whisper large-v3-turbo model (~1.6 GB)..."
echo "Destination: $MODEL_FILE"

if command -v wget &>/dev/null; then
    wget -c -O "$MODEL_FILE" "$MODEL_URL"
elif command -v curl &>/dev/null; then
    curl -L -C - -o "$MODEL_FILE" "$MODEL_URL"
else
    echo "Error: wget or curl required"
    exit 1
fi

echo "Done. Model saved to $MODEL_FILE"
