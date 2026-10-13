#!/usr/bin/env bash
# Build the OpenDex desktop application for Linux
#
# Usage:
#   ./build/build-linux.sh              # Debug build
#   ./build/build-linux.sh --release    # Release build
#
# Output:
#   target/release/opendex-desktop      # Release binary
#   target/debug/opendex-desktop        # Debug binary
#   build/opendex-desktop-linux.tar.gz  # Release archive

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

RELEASE=0
if [[ "${1:-}" == "--release" ]]; then
    RELEASE=1
fi

echo "=== OpenDex Desktop — Linux Build ==="
echo "Project root: $PROJECT_ROOT"

# Check for required tools
command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install Rust first."; exit 1; }

cd "$PROJECT_ROOT"

# Build
if [[ $RELEASE -eq 1 ]]; then
    echo "Building release..."
    cargo build --release -p desktop-gui
    BUILD_DIR="target/release"
else
    echo "Building debug..."
    cargo build -p desktop-gui
    BUILD_DIR="target/debug"
fi

BINARY="$BUILD_DIR/opendex-desktop"

if [[ ! -f "$BINARY" ]]; then
    echo "ERROR: Binary not found at $BINARY"
    exit 1
fi

echo "Binary built: $BINARY"
echo "Size: $(du -h "$BINARY" | cut -f1)"

# Create release archive
if [[ $RELEASE -eq 1 ]]; then
    ARCHIVE="$SCRIPT_DIR/opendex-desktop-linux-x86_64.tar.gz"
    echo "Creating archive: $ARCHIVE"

    STAGING="$(mktemp -d)"
    mkdir -p "$STAGING/opendex-desktop/bin"
    cp "$BINARY" "$STAGING/opendex-desktop/bin/"

    # Include desktop integration files
    if [[ -f "$SCRIPT_DIR/../resources/opendex-desktop.desktop" ]]; then
        mkdir -p "$STAGING/opendex-desktop/share/applications"
        cp "$SCRIPT_DIR/../resources/opendex-desktop.desktop" \
           "$STAGING/opendex-desktop/share/applications/"
    fi

    tar -czf "$ARCHIVE" -C "$STAGING" opendex-desktop
    rm -rf "$STAGING"

    echo "Archive created: $ARCHIVE"
    echo "Size: $(du -h "$ARCHIVE" | cut -f1)"
fi

echo ""
echo "=== Build complete ==="
echo "Run: $BINARY"
