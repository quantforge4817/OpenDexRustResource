#!/usr/bin/env bash
# deploy-testnet.sh — One-command deployment of the complete OpenDex testnet
#
# Orchestrates all setup scripts in order:
#   1. VPS setup (users, packages, firewall)
#   2. Wallet daemons (Monero stagenet, Bitcoin testnet)
#   3. Web frontend (static site + Nginx + HTTPS)
#   4. P2P nodes (bootstrap + 2 test nodes)
#
# Usage:
#   sudo bash deploy-testnet.sh [domain] [repo-path]
#
# Arguments:
#   domain    — Domain name for HTTPS (e.g., testnet.opendex.io). Default: localhost
#   repo-path — Path to the OpenDex repository. Default: /home/opendex/opendex

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

# --- Arguments ---
DOMAIN="${1:-localhost}"
REPO_PATH="${2:-/home/opendex/opendex}"

info "============================================================"
info "  OpenDex Testnet — Full Deployment"
info "============================================================"
info "Domain:     $DOMAIN"
info "Repository: $REPO_PATH"
info ""

# --- Pre-flight checks ---
if [[ $EUID -ne 0 ]]; then
    error "This script must be run as root (sudo)."
    exit 1
fi

# --- Step 1: VPS Setup ---
info "============================================================"
info "  Step 1/4: VPS Setup"
info "============================================================"
bash "$SCRIPT_DIR/setup-vps.sh"

# --- Step 2: Wallet Daemons ---
info ""
info "============================================================"
info "  Step 2/4: Wallet Daemons (Monero + Bitcoin)"
info "============================================================"
bash "$SCRIPT_DIR/setup-wallets.sh"

info ""
warn "Blockchain sync is in progress. This may take 10-60 minutes."
info "You can monitor sync progress with:"
info "  Monero:  journalctl -u monero-stagenet -f | grep 'Height'"
info "  Bitcoin: bitcoin-cli -testnet getblockcount"
info ""
read -p "Wait for blockchain sync to complete before continuing? (y/N) " -r
if [[ ! "$REPLY" =~ ^[Yy]$ ]]; then
    warn "Continuing without waiting. P2P nodes will start, but on-chain operations may not work until sync is complete."
fi

# --- Step 3: Web Frontend ---
info ""
info "============================================================"
info "  Step 3/4: Web Frontend"
info "============================================================"
bash "$SCRIPT_DIR/setup-web.sh" "$DOMAIN" "$REPO_PATH"

# --- Step 4: P2P Nodes ---
info ""
info "============================================================"
info "  Step 4/4: P2P Nodes"
info "============================================================"
bash "$SCRIPT_DIR/setup-nodes.sh" "$REPO_PATH"

# --- Final Summary ---
info ""
info "============================================================"
info "  OpenDex Testnet Deployment Complete!"
info "============================================================"
info ""
info "Services:"
info "  Web Portal:      http://${DOMAIN}"
info "  Bootstrap Node:  port 4001"
info "  Test Node 1:     port 4002"
info "  Test Node 2:     port 4003"
info "  Monero Stagenet: RPC on port 18082"
info "  Bitcoin Testnet: RPC on port 18332"
info ""
info "Service Management:"
info "  View all:  systemctl list-units | grep -E 'opendex|monero|bitcoin'"
info "  Logs:      journalctl -u opendex-bootstrap -f"
info "  Restart:   systemctl restart opendex-bootstrap"
info "  Stop:      systemctl stop opendex-bootstrap"
info ""
info "Verification:"
info "  1. Check node status:  systemctl status opendex-bootstrap opendex-testnode1 opendex-testnode2"
info "  2. Check peer connections: journalctl -u opendex-bootstrap | grep -i 'peer'"
info "  3. Test web portal:     curl http://${DOMAIN}"
info ""
info "Documentation:"
info "  See docs/testnet/README.md for detailed usage instructions."
