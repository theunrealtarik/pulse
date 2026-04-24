#!/usr/bin/env bash
set -euo pipefail

BIN_NAME="pulse"
TARGET="target/release/$BIN_NAME"
DEST="/usr/local/bin/$BIN_NAME"

echo "[+] Building (release)..."
cargo build --release

if [[ ! -f "$TARGET" ]]; then
    echo "Build failed: binary not found at $TARGET"
    exit 1
fi

echo "Installing to $DEST"
if [[ -w "$(dirname "$DEST")" ]]; then
    mv "$TARGET" "$DEST"
else
    sudo mv "$TARGET" "$DEST"
fi

echo "Installed successfully"
