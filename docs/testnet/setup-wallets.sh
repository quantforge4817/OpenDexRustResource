#!/usr/bin/env bash
# setup-wallets.sh — Install and configure Monero stagenet and Bitcoin testnet daemons
#
# Downloads binaries, creates systemd services, and starts blockchain sync.
# Run as root after running setup-vps.sh.
#
# Usage:
#   sudo bash setup-wallets.sh

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

# Check that users exist
for user in monero bitcoin; do
    if ! id -u "$user" &>/dev/null; then
        error "User '$user' not found. Run setup-vps.sh first."
        exit 1
    fi
done

info "=== OpenDex Testnet Wallet Setup ==="

# --- Detect architecture ---
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  ARCH_SUFFIX="x64" ;;
    aarch64) ARCH_SUFFIX="arm64" ;;
    *) error "Unsupported architecture: $ARCH" ;;
esac

# --- Monero Stagenet ---
info "=== Installing Monero Stagenet ==="

MONERO_VERSION="0.18.3.3"
MONERO_URL="https://github.com/monero-project/monero/releases/download/v${MONERO_VERSION}/monero-linux-${ARCH_SUFFIX}-${MONERO_VERSION}.tar.bz2"

if ! command -v monerod &>/dev/null; then
    info "Downloading Monero v${MONERO_VERSION}..."
    cd /tmp
    wget -q --show-progress "$MONERO_URL" -O "monero-${MONERO_VERSION}.tar.bz2"
    tar xjf "monero-${MONERO_VERSION}.tar.bz2"
    cp "monero-${MONERO_VERSION}/bin/monerod" /usr/local/bin/
    cp "monero-${MONERO_VERSION}/bin/monero-wallet-rpc" /usr/local/bin/
    chmod +x /usr/local/bin/monerod /usr/local/bin/monero-wallet-rpc
    rm -rf "monero-${MONERO_VERSION}" "monero-${MONERO_VERSION}.tar.bz2"
    info "Monero binaries installed."
else
    info "Monero already installed at $(which monerod), skipping download."
fi

# Create data directory
mkdir -p /home/monero/.bitmonero/stagenet
chown -R monero:monero /home/monero/.bitmonero

# Install systemd service
info "Installing Monero stagenet systemd service..."
cp "${SCRIPT_DIR}/systemd/monero-stagenet.service" /etc/systemd/system/
systemctl daemon-reload
systemctl enable monero-stagenet

# --- Bitcoin Testnet ---
info "=== Installing Bitcoin Testnet ==="

BTC_VERSION="26.0"
BTC_URL="https://bitcoincore.org/bin/bitcoin-core-${BTC_VERSION}/bitcoin-${BTC_VERSION}-x86_64-linux-gnu.tar.gz"

if ! command -v bitcoind &>/dev/null; then
    info "Downloading Bitcoin Core v${BTC_VERSION}..."
    cd /tmp
    wget -q --show-progress "$BTC_URL" -O "bitcoin-${BTC_VERSION}.tar.gz"
    tar xzf "bitcoin-${BTC_VERSION}.tar.gz"
    cp "bitcoin-${BTC_VERSION}/bin/bitcoind" /usr/local/bin/
    cp "bitcoin-${BTC_VERSION}/bin/bitcoin-cli" /usr/local/bin/
    chmod +x /usr/local/bin/bitcoind /usr/local/bin/bitcoin-cli
    rm -rf "bitcoin-${BTC_VERSION}" "bitcoin-${BTC_VERSION}.tar.gz"
    info "Bitcoin binaries installed."
else
    info "Bitcoin already installed at $(which bitcoind), skipping download."
fi

# Create data directory
mkdir -p /home/bitcoin/.bitcoin/testnet3
chown -R bitcoin:bitcoin /home/bitcoin/.bitcoin

# Create Bitcoin testnet config
info "Creating Bitcoin testnet configuration..."
cat > /home/bitcoin/.bitcoin/testnet3/bitcoin.conf <<'EOF'
[test]
server=1
rpcport=18332
rpcallowip=127.0.0.1
rpcuser=opendex
rpcpassword=opendex_testnet
prune=550
daemon=0
EOF
chown bitcoin:bitcoin /home/bitcoin/.bitcoin/testnet3/bitcoin.conf

# Install systemd service
info "Installing Bitcoin testnet systemd service..."
cp "${SCRIPT_DIR}/systemd/bitcoin-testnet.service" /etc/systemd/system/
systemctl daemon-reload
systemctl enable bitcoin-testnet

# --- Start wallet daemons ---
info "=== Starting Wallet Daemons ==="

info "Starting Monero stagenet daemon..."
systemctl start monero-stagenet
sleep 2
if systemctl is-active --quiet monero-stagenet; then
    info "Monero stagenet daemon started successfully."
else
    error "Monero stagenet daemon failed to start. Check logs: journalctl -u monero-stagenet"
fi

info "Starting Bitcoin testnet daemon..."
systemctl start bitcoin-testnet
sleep 2
if systemctl is-active --quiet bitcoin-testnet; then
    info "Bitcoin testnet daemon started successfully."
else
    error "Bitcoin testnet daemon failed to start. Check logs: journalctl -u bitcoin-testnet"
fi

# --- Sync status ---
info ""
info "=== Wallet Daemon Status ==="
info "Monero stagenet:  $(systemctl is-active monero-stagenet 2>/dev/null || echo 'unknown')"
info "Bitcoin testnet:  $(systemctl is-active bitcoin-testnet 2>/dev/null || echo 'unknown')"
info ""
info "IMPORTANT: Blockchain sync is required before using P2P nodes."
info "  - Monero stagenet sync: ~30-60 minutes (use --fast-block-sync for faster sync)"
info "  - Bitcoin testnet sync: ~10-30 minutes (pruned to 550MB)"
info ""
info "Check sync progress:"
info "  Monero:  journalctl -u monero-stagenet -f | grep 'Height'"
info "  Bitcoin: bitcoin-cli -testnet getblockcount"
info ""
info "Next step: Run setup-nodes.sh to deploy P2P nodes."
