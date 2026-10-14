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

# --- Detect architecture and OS ---
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  ARCH_SUFFIX="x64" ;;
    aarch64) ARCH_SUFFIX="arm64" ;;
    *) error "Unsupported architecture: $ARCH" ;;
esac

# Detect OS for binary compatibility
if [[ -f /etc/os-release ]]; then
    . /etc/os-release
    OS_ID="$ID"
    OS_VERSION="$VERSION_ID"
else
    OS_ID="unknown"
    OS_VERSION="unknown"
fi

# --- Monero Stagenet ---
info "=== Installing Monero Stagenet ==="

MONERO_VERSION="0.18.5.3"
MONERO_URL="https://downloads.getmonero.org/cli/monero-linux-${ARCH_SUFFIX}-v${MONERO_VERSION}.tar.bz2"

install_monero_binaries() {
    info "Downloading Monero v${MONERO_VERSION}..."
    cd /tmp
    if ! wget --timeout=60 --tries=2 -O "monero-${MONERO_VERSION}.tar.bz2" "$MONERO_URL"; then
        warn "Monero download failed. Trying apt-get package instead..."
        apt-get install -y monero
        return 1
    fi
    tar xjf "monero-${MONERO_VERSION}.tar.bz2"
    # getmonero.org archive structure: monero-x86_64-linux-gnu-v0.18.5.3/ (binaries directly in folder)
    MONERO_EXTRACT_DIR=$(tar tjf "monero-${MONERO_VERSION}.tar.bz2" | head -1 | sed 's|/.*||')
    cp "${MONERO_EXTRACT_DIR}/monerod" /usr/local/bin/
    cp "${MONERO_EXTRACT_DIR}/monero-wallet-rpc" /usr/local/bin/
    chmod +x /usr/local/bin/monerod /usr/local/bin/monero-wallet-rpc
    rm -rf "${MONERO_EXTRACT_DIR}" "monero-${MONERO_VERSION}.tar.bz2"
    info "Monero binaries installed."
    return 0
}

# Check if Monero is installed and if it's a recent enough version
# Debian apt package (v0.18.4.0 from 2020) crashes on modern CPUs with "Error generating CryptonightR code"
NEED_UPGRADE=false
if command -v monerod &>/dev/null; then
    EXISTING_VERSION=$(monerod --version 2>/dev/null | grep -oP 'v\K[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+' || echo "")
    if [[ -n "$EXISTING_VERSION" ]]; then
        # Compare versions: Debian package is 0.18.4.0, we need at least 0.18.4.3+
        # Simple string comparison works for this case
        if [[ "$EXISTING_VERSION" < "0.18.4.3" ]]; then
            warn "Monero v${EXISTING_VERSION} is too old (crashes on modern CPUs)."
            warn "Upgrading to v${MONERO_VERSION}..."
            NEED_UPGRADE=true
        else
            info "Monero v${EXISTING_VERSION} already installed, skipping download."
        fi
    else
        info "Monero installed at $(which monerod), skipping download."
    fi
else
    NEED_UPGRADE=true
fi

if [[ "$NEED_UPGRADE" == "true" ]]; then
    # Try downloading from GitHub first; fall back to apt-get if download fails
    if ! install_monero_binaries; then
        # apt-get installed monero, verify it works
        if command -v monerod &>/dev/null; then
            info "Monero installed via apt-get package."
        else
            error "Monero installation failed completely."
            error "Please install manually: apt-get install monero"
            exit 1
        fi
    fi
    # Verify the binary works
    if ! /usr/local/bin/monerod --version &>/dev/null; then
        warn "Monero binary failed to run (likely glibc incompatibility)."
        warn "Trying apt-get package instead..."
        apt-get install -y monero
        if ! command -v monerod &>/dev/null; then
            error "Monero installation failed. Please install manually and retry."
            exit 1
        fi
    fi
fi

# Create data directory on /data partition (NVMe, 503GB free)
mkdir -p /data/monero-stagenet
chown -R monero:monero /data/monero-stagenet

# Detect actual monerod path — prefer /usr/local/bin (manual install) over /usr/bin (apt-get)
if [[ -x /usr/local/bin/monerod ]]; then
    MONEROD_PATH="/usr/local/bin/monerod"
else
    MONEROD_PATH=$(command -v monerod 2>/dev/null || echo '/usr/local/bin/monerod')
fi

# Install systemd service — replace placeholder with actual binary path
info "Installing Monero stagenet systemd service..."
sed "s|__MONEROD_PATH__|${MONEROD_PATH}|g" \
    "${SCRIPT_DIR}/systemd/monero-stagenet.service" > /etc/systemd/system/monero-stagenet.service
systemctl daemon-reload
systemctl enable monero-stagenet

# --- Monero Wallet RPC ---
info "=== Installing Monero Wallet RPC ==="

# Detect monero-wallet-rpc path
if [[ -x /usr/local/bin/monero-wallet-rpc ]]; then
    MONERO_WALLET_RPC_PATH="/usr/local/bin/monero-wallet-rpc"
else
    MONERO_WALLET_RPC_PATH=$(command -v monero-wallet-rpc 2>/dev/null || echo '/usr/local/bin/monero-wallet-rpc')
fi

# Create wallet directory
mkdir -p /data/monero-stagenet/wallets
chown -R monero:monero /data/monero-stagenet/wallets

# Install systemd service
info "Installing Monero Wallet RPC systemd service..."
sed "s|__MONERO_WALLET_RPC_PATH__|${MONERO_WALLET_RPC_PATH}|g" \
    "${SCRIPT_DIR}/systemd/monero-wallet-rpc.service" > /etc/systemd/system/monero-wallet-rpc.service
systemctl daemon-reload
systemctl enable monero-wallet-rpc

# --- Bitcoin Testnet ---
info "=== Installing Bitcoin Testnet ==="

BTC_VERSION="28.0"
BTC_URL="https://bitcoincore.org/bin/bitcoin-core-${BTC_VERSION}/bitcoin-${BTC_VERSION}-x86_64-linux-gnu.tar.gz"

install_bitcoin_binaries() {
    info "Downloading Bitcoin Core v${BTC_VERSION}..."
    cd /tmp
    if ! wget --timeout=60 --tries=2 -O "bitcoin-${BTC_VERSION}.tar.gz" "$BTC_URL"; then
        warn "Bitcoin download failed. Trying apt-get package instead..."
        apt-get install -y bitcoin-d
        return 1
    fi
    tar xzf "bitcoin-${BTC_VERSION}.tar.gz"
    cp "bitcoin-${BTC_VERSION}/bin/bitcoind" /usr/local/bin/
    cp "bitcoin-${BTC_VERSION}/bin/bitcoin-cli" /usr/local/bin/
    chmod +x /usr/local/bin/bitcoind /usr/local/bin/bitcoin-cli
    rm -rf "bitcoin-${BTC_VERSION}" "bitcoin-${BTC_VERSION}.tar.gz"
    info "Bitcoin binaries installed."
    return 0
}

if ! command -v bitcoind &>/dev/null; then
    # Try downloading from Bitcoin Core first; fall back to apt-get if download fails
    if ! install_bitcoin_binaries; then
        # apt-get installed bitcoin-d, verify it works
        if command -v bitcoind &>/dev/null; then
            info "Bitcoin installed via apt-get package."
        else
            error "Bitcoin installation failed completely."
            error "Please install manually: apt-get install bitcoin-d"
            exit 1
        fi
    fi
    # Verify the binary works
    if ! /usr/local/bin/bitcoind -version &>/dev/null; then
        warn "Bitcoin binary failed to run (likely glibc incompatibility)."
        warn "Trying apt-get package instead..."
        apt-get install -y bitcoin-d
        if ! command -v bitcoind &>/dev/null; then
            error "Bitcoin installation failed. Please install manually and retry."
            exit 1
        fi
    fi
else
    info "Bitcoin already installed at $(which bitcoind), skipping download."
fi

# Create data directory on /data partition (NVMe, 503GB free)
mkdir -p /data/bitcoin-testnet
chown -R bitcoin:bitcoin /data/bitcoin-testnet

# Create Bitcoin testnet config
info "Creating Bitcoin testnet configuration..."
cat > /data/bitcoin-testnet/bitcoin.conf <<'EOF'
[test]
server=1
rpcport=18332
rpcallowip=127.0.0.1
rpcuser=opendex
rpcpassword=opendex_testnet
prune=550
daemon=0
EOF
chown bitcoin:bitcoin /data/bitcoin-testnet/bitcoin.conf

# Detect actual bitcoind path (apt-get installs to /usr/bin, manual to /usr/local/bin)
BITCOIND_PATH=$(command -v bitcoind 2>/dev/null || echo '/usr/local/bin/bitcoind')

# Install systemd service — replace placeholder with actual binary path
info "Installing Bitcoin testnet systemd service..."
sed "s|__BITCOIND_PATH__|${BITCOIND_PATH}|g" \
    "${SCRIPT_DIR}/systemd/bitcoin-testnet.service" > /etc/systemd/system/bitcoin-testnet.service
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

# --- Create Monero stagenet wallet ---
info "=== Creating Monero Stagenet Wallet ==="
MONERO_WALLET_FILE="/data/monero-stagenet/wallets/opendex-stagenet-wallet"
if [[ ! -f "$MONERO_WALLET_FILE" ]]; then
    info "Creating new Monero stagenet wallet..."
    # Use start-stop-daemon to run monero-wallet-rpc as the monero user,
    # then wait for the wallet file to appear, then stop the process.
    # This is more reliable than background processes under sudo.
    start-stop-daemon --start --background --make-pidfile \
        --pidfile /tmp/monero-wallet-rpc-init.pid \
        --chuid monero:monero \
        --exec /usr/local/bin/monero-wallet-rpc \
        -- \
        --stagenet \
        --daemon-address 127.0.0.1:18082 \
        --wallet-file "$MONERO_WALLET_FILE" \
        --generate-new-wallet "opendex-stagenet-wallet" \
        --password "" \
        --trusted-daemon \
        --disable-rpc-login \
        --non-interactive
    # Wait up to 30 seconds for the wallet file to be created
    for i in $(seq 1 30); do
        if [[ -f "$MONERO_WALLET_FILE" ]]; then
            break
        fi
        sleep 1
    done
    # Stop the temporary process
    start-stop-daemon --stop --pidfile /tmp/monero-wallet-rpc-init.pid 2>/dev/null || true
    rm -f /tmp/monero-wallet-rpc-init.pid
    # Check if wallet was created
    if [[ -f "$MONERO_WALLET_FILE" ]]; then
        chown monero:monero "$MONERO_WALLET_FILE" 2>/dev/null || true
        chown monero:monero "${MONERO_WALLET_FILE}.keys" 2>/dev/null || true
        info "Monero stagenet wallet created."
    else
        warn "Monero wallet creation failed."
        warn "Create it manually:"
        warn "  sudo -u monero /usr/local/bin/monero-wallet-rpc --stagenet --daemon-address 127.0.0.1:18082 --wallet-file $MONERO_WALLET_FILE --generate-new-wallet opendex-stagenet-wallet --password '' --trusted-daemon --disable-rpc-login --non-interactive &"
        warn "  sleep 5 && kill \$!"
    fi
else
    info "Monero stagenet wallet already exists."
fi

info "Starting Monero Wallet RPC..."
systemctl start monero-wallet-rpc
sleep 2
if systemctl is-active --quiet monero-wallet-rpc; then
    info "Monero Wallet RPC started successfully."
else
    warn "Monero Wallet RPC failed to start. Check logs: sudo journalctl -u monero-wallet-rpc -n 20"
fi

info "Starting Bitcoin testnet daemon..."
systemctl start bitcoin-testnet
sleep 2
if systemctl is-active --quiet bitcoin-testnet; then
    info "Bitcoin testnet daemon started successfully."
else
    error "Bitcoin testnet daemon failed to start. Check logs: journalctl -u bitcoin-testnet"
fi

# Create Bitcoin testnet wallet
info "=== Creating Bitcoin Testnet Wallet ==="
BITCOIND_PATH=$(command -v bitcoind 2>/dev/null || echo '/usr/local/bin/bitcoind')
BITCOIN_CLI_PATH=$(command -v bitcoin-cli 2>/dev/null || echo '/usr/local/bin/bitcoin-cli')

# Wait for Bitcoin to be ready
for i in $(seq 1 30); do
    if $BITCOIN_CLI_PATH --datadir=/data/bitcoin-testnet -testnet getblockcount &>/dev/null; then
        break
    fi
    sleep 2
done

# Create wallet if it doesn't exist
if ! $BITCOIN_CLI_PATH --datadir=/data/bitcoin-testnet -testnet -rpcuser=opendex -rpcpassword=opendex_testnet createwallet opendex 2>/dev/null; then
    # Wallet may already exist
    $BITCOIN_CLI_PATH --datadir=/data/bitcoin-testnet -testnet -rpcuser=opendex -rpcpassword=opendex_testnet loadwallet opendex 2>/dev/null
fi
info "Bitcoin testnet wallet created or loaded."

# --- Sync status ---
info ""
info "=== Wallet Daemon Status ==="
info "Monero stagenet:  $(systemctl is-active monero-stagenet 2>/dev/null || echo 'unknown')"
info "Monero Wallet RPC: $(systemctl is-active monero-wallet-rpc 2>/dev/null || echo 'unknown')"
info "Bitcoin testnet:  $(systemctl is-active bitcoin-testnet 2>/dev/null || echo 'unknown')"
info ""
info "IMPORTANT: Blockchain sync is required before using P2P nodes."
info "  - Monero stagenet sync: ~30-60 minutes"
info "  - Bitcoin testnet sync: ~10-30 minutes (pruned to 550MB)"
info ""
info "Check sync progress:"
info "  Monero:  sudo journalctl -u monero-stagenet -f | grep 'Synced'"
info "  Bitcoin: bitcoin-cli --datadir=/data/bitcoin-testnet -testnet getblockcount"
info ""
info "Next step: Run setup-nodes.sh to deploy P2P nodes."
