#!/bin/bash
# Compress the release-small binary with UPX if available

BINARY="target/release-small/rusttools-tauri"

if [ ! -f "$BINARY" ]; then
    echo "Binary not found: $BINARY"
    echo "Run ./build-single-file.sh first"
    exit 1
fi

if command -v upx &> /dev/null; then
    echo "Compressing with UPX..."
    cp "$BINARY" "${BINARY}.bak"
    upx --best "$BINARY"
    echo ""
    echo "Before:"
    ls -lh "${BINARY}.bak"
    echo "After:"
    ls -lh "$BINARY"
    rm "${BINARY}.bak"
else
    echo "UPX not installed. Install with:"
    echo "  sudo apt-get install upx-ucl    # Debian/Ubuntu"
    echo "  brew install upx                 # macOS"
    echo ""
    echo "Current binary size:"
    ls -lh "$BINARY"
fi
