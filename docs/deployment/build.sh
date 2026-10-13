#!/bin/bash
# OpenDex Web Portal Build Script
# Builds the Leptos static site and prepares the dist/ folder for deployment.
#
# Usage (from project root):
#   cd docs/deployment
#   ./build.sh              # Debug build
#   ./build.sh --release    # Release build (recommended for production)
#
# Prerequisites:
#   - Rust 1.88+ installed
#   - wasm32-unknown-unknown target: rustup target add wasm32-unknown-unknown
#   - Trunk: cargo install trunk

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
FRONTEND_DIR="$PROJECT_ROOT/crates/web-frontend"
DIST_DIR="$SCRIPT_DIR/dist"

MODE="debug"
if [[ "${1:-}" == "--release" ]]; then
    MODE="release"
fi

echo "============================================"
echo "  OpenDex Web Portal Build"
echo "  Mode: $MODE"
echo "============================================"

# Check prerequisites
if ! command -v rustc &> /dev/null; then
    echo "ERROR: Rust is not installed. Install from https://rustup.rs/"
    exit 1
fi

if ! command -v trunk &> /dev/null; then
    echo "ERROR: Trunk is not installed. Run: cargo install trunk"
    exit 1
fi

if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "ERROR: wasm32-unknown-unknown target not installed."
    echo "Run: rustup target add wasm32-unknown-unknown"
    exit 1
fi

# Verify frontend crate exists
if [[ ! -d "$FRONTEND_DIR" ]]; then
    echo "ERROR: Web frontend directory not found: $FRONTEND_DIR"
    exit 1
fi

# Clean previous build
echo ""
echo "Cleaning previous build..."
rm -rf "$DIST_DIR"

# Build using trunk from the web-frontend crate directory
echo "Building web portal ($MODE mode)..."
cd "$FRONTEND_DIR"

if [[ "$MODE" == "release" ]]; then
    trunk build --release
else
    trunk build
fi

# Copy the dist output from web-frontend to docs/deployment
echo ""
echo "Copying build output to deployment folder..."
cp -r "$FRONTEND_DIR/dist" "$SCRIPT_DIR/dist"
rm -rf "$FRONTEND_DIR/dist"

echo ""
echo "Build complete!"
echo "Output directory: $DIST_DIR"
echo ""

# Show build summary
if [[ -d "$DIST_DIR" ]]; then
    FILE_COUNT=$(find "$DIST_DIR" -type f | wc -l)
    TOTAL_SIZE=$(du -sh "$DIST_DIR" | cut -f1)
    echo "Files: $FILE_COUNT"
    echo "Total size: $TOTAL_SIZE"
    echo ""
    echo "To deploy, copy the $DIST_DIR directory to your web server."
    echo "See DEPLOY.md for detailed deployment instructions."
else
    echo "ERROR: Build failed — dist/ directory not found."
    exit 1
fi
