#!/bin/bash
# Build script for single-file deployment
# Produces a stripped, size-optimized binary

set -e

echo "Building RustTools (single-file deployment)..."
echo "Profile: release-small (opt-level=z, LTO, strip, panic=abort)"
echo ""

cargo build --profile release-small --bin rusttools-tauri

echo ""
echo "Build complete:"
ls -lh target/release-small/rusttools-tauri
echo ""
echo "Binary size:"
du -sh target/release-small/rusttools-tauri

# Optional: compress with zstd if available
if command -v zstd &> /dev/null; then
    echo ""
    echo "Compressing with zstd -19..."
    zstd -19 --rm target/release-small/rusttools-tauri -o target/release-small/rusttools-tauri.zst
    echo "Compressed size:"
    ls -lh target/release-small/rusttools-tauri.zst
    echo ""
    echo "To decompress: zstd -d target/release-small/rusttools-tauri.zst"
fi
