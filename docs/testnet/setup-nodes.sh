#!/usr/bin/env bash
# setup-nodes.sh — Build and deploy P2P nodes as systemd services
#
# Builds the p2p-node binary, generates bootstrap identity, extracts PeerId,
# configures test node services, and starts all nodes in order.
# Run as root after running setup-vps.sh and setup-wallets.sh.
#
# Usage:
#   sudo bash setup-nodes.sh [path-to-opendex-repo]
#
# If no path is given, it assumes the repo is at /home/opendex/opendex

set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# --- Pre-flight checks ---
if [[ $EUID -ne 0 ]]; then
    error "This script must be run as root (sudo)."
    exit 1
fi

if ! id -u opendex &>/dev/null; then
    error "User 'opendex' not found. Run setup-vps.sh first."
    exit 1
fi

# Determine repo path
REPO_PATH="${1:-/home/opendex/opendex}"
if [[ ! -d "$REPO_PATH" ]]; then
    error "Repository not found at $REPO_PATH"
    info "Clone the repo first: sudo -u opendex git clone <repo-url> /home/opendex/opendex"
    exit 1
fi

info "=== OpenDex P2P Node Deployment ==="
info "Repository: $REPO_PATH"

# --- Check for Rust toolchain ---
if ! command -v cargo &>/dev/null; then
    info "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# --- Build p2p-node binary ---
info "Building p2p-node binary (release mode)..."
cd "$REPO_PATH"
sudo -u opendex cargo build --release -p p2p-node
P2P_BINARY="$REPO_PATH/target/release/p2p-node"

if [[ ! -f "$P2P_BINARY" ]]; then
    error "Build failed. Binary not found at $P2P_BINARY"
    exit 1
fi
info "Binary built: $P2P_BINARY"

# --- Generate bootstrap identity ---
info "Generating bootstrap node identity..."
sudo -u opendex "$P2P_BINARY" --port 4001 --new-identity --help &
BOOTSTRAP_PID=$!
sleep 2
kill $BOOTSTRAP_PID 2>/dev/null || true
wait $BOOTSTRAP_PID 2>/dev/null || true

# Read the PeerId from the identity file
IDENTITY_FILE="/home/opendex/.opendex/identity-4001.key"
if [[ ! -f "$IDENTITY_FILE" ]]; then
    error "Bootstrap identity not created at $IDENTITY_FILE"
    exit 1
fi

# Extract PeerId by running the node briefly and capturing the log
info "Extracting bootstrap PeerId..."
PEER_ID_OUTPUT=$(sudo -u opendex timeout 3 "$P2P_BINARY" --port 4001 2>&1 || true)
BOOTSTRAP_PEER_ID=$(echo "$PEER_ID_OUTPUT" | grep -oP 'Peer ID:\s*\K[1-9A-HJ-NP-Za-km-z]+' | head -1)

if [[ -z "$BOOTSTRAP_PEER_ID" ]]; then
    # Fallback: try to extract from logs
    PEER_ID_OUTPUT=$(sudo -u opendex timeout 5 "$P2P_BINARY" --port 4001 2>&1 || true)
    BOOTSTRAP_PEER_ID=$(echo "$PEER_ID_OUTPUT" | grep -oP '12D3[1-9A-HJ-NP-Za-km-z]{50,}' | head -1)
fi

if [[ -z "$BOOTSTRAP_PEER_ID" ]]; then
    error "Could not extract bootstrap PeerId. Please check the node logs."
    info "You can find it by running: sudo -u opendex $P2P_BINARY --port 4001"
    info "Then look for 'PeerId' in the output."
    exit 1
fi

info "Bootstrap PeerId: $BOOTSTRAP_PEER_ID"

# --- Update config file ---
info "Updating testnet configuration..."
sed -i "s/# BOOTSTRAP_PEER_ID=.*/BOOTSTRAP_PEER_ID=${BOOTSTRAP_PEER_ID}/" \
    "${SCRIPT_DIR}/testnet-config.env"

# --- Configure systemd services ---
info "Configuring systemd services..."

# Bootstrap node
cp "${SCRIPT_DIR}/systemd/opendex-bootstrap.service" /etc/systemd/system/

# Test node 1 — replace placeholder with actual PeerId
sed "s/__BOOTSTRAP_PEER_ID__/${BOOTSTRAP_PEER_ID}/g" \
    "${SCRIPT_DIR}/systemd/opendex-testnode1.service" > /etc/systemd/system/opendex-testnode1.service

# Test node 2 — replace placeholder with actual PeerId
sed "s/__BOOTSTRAP_PEER_ID__/${BOOTSTRAP_PEER_ID}/g" \
    "${SCRIPT_DIR}/systemd/opendex-testnode2.service" > /etc/systemd/system/opendex-testnode2.service

# Reload systemd
systemctl daemon-reload

# --- Enable services ---
info "Enabling P2P node services..."
systemctl enable opendex-bootstrap
systemctl enable opendex-testnode1
systemctl enable opendex-testnode2

# --- Start services in order ---
info "=== Starting P2P Nodes ==="

# 1. Bootstrap node
info "Starting bootstrap node (port 4001)..."
systemctl start opendex-bootstrap
sleep 3
if systemctl is-active --quiet opendex-bootstrap; then
    info "Bootstrap node started successfully."
else
    error "Bootstrap node failed to start. Check: journalctl -u opendex-bootstrap"
    exit 1
fi

# 2. Test node 1
info "Starting test node 1 (port 4002)..."
systemctl start opendex-testnode1
sleep 2
if systemctl is-active --quiet opendex-testnode1; then
    info "Test node 1 started successfully."
else
    error "Test node 1 failed to start. Check: journalctl -u opendex-testnode1"
fi

# 3. Test node 2
info "Starting test node 2 (port 4003)..."
systemctl start opendex-testnode2
sleep 2
if systemctl is-active --quiet opendex-testnode2; then
    info "Test node 2 started successfully."
else
    error "Test node 2 failed to start. Check: journalctl -u opendex-testnode2"
fi

# --- Verification ---
info ""
info "=== P2P Node Status ==="
info "Bootstrap:  $(systemctl is-active opendex-bootstrap 2>/dev/null || echo 'unknown')"
info "Test Node 1: $(systemctl is-active opendex-testnode1 2>/dev/null || echo 'unknown')"
info "Test Node 2: $(systemctl is-active opendex-testnode2 2>/dev/null || echo 'unknown')"
info ""
info "Bootstrap PeerId: $BOOTSTRAP_PEER_ID"
info ""
info "Check node logs:"
info "  Bootstrap:  journalctl -u opendex-bootstrap -f"
info "  Test Node 1: journalctl -u opendex-testnode1 -f"
info "  Test Node 2: journalctl -u opendex-testnode2 -f"
info ""
info "Verify peer connections after ~10 seconds:"
info "  journalctl -u opendex-bootstrap | grep -i 'peer\\|connected\\|dial'"
info ""
info "Next step: Run setup-web.sh to deploy the web frontend."
