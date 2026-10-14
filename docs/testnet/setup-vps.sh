#!/usr/bin/env bash
# setup-vps.sh — Initial VPS setup for OpenDex testnet
#
# Creates system users, installs required packages, and configures firewall.
# Run as root on a fresh Ubuntu 24.04 LTS instance.
#
# Usage:
#   sudo bash setup-vps.sh

set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }

# --- Pre-flight checks ---
if [[ $EUID -ne 0 ]]; then
    error "This script must be run as root (sudo)."
    exit 1
fi

if [[ ! -f /etc/os-release ]]; then
    error "Cannot detect OS. /etc/os-release not found."
    exit 1
fi

. /etc/os-release
# Support Ubuntu 24.04 and Debian 11+
case "$ID" in
    ubuntu)
        if [[ "$VERSION_ID" == "24.04" ]]; then
            info "Detected Ubuntu 24.04 LTS."
        else
            warn "This script is tested on Ubuntu 24.04 LTS. Detected: $ID $VERSION_ID"
            read -p "Continue anyway? (y/N) " -r
            [[ "$REPLY" =~ ^[Yy]$ ]] || exit 1
        fi
        ;;
    debian)
        if [[ "$VERSION_ID" =~ ^1[1-9]\.|^2[0-9]\.|^13\. ]]; then
            info "Detected Debian $VERSION_ID (supported)."
        else
            warn "This script is tested on Debian 11+. Detected: $ID $VERSION_ID"
            read -p "Continue anyway? (y/N) " -r
            [[ "$REPLY" =~ ^[Yy]$ ]] || exit 1
        fi
        ;;
    *)
        warn "This script is designed for Ubuntu 24.04 or Debian 11+. Detected: $ID $VERSION_ID"
        read -p "Continue anyway? (y/N) " -r
        [[ "$REPLY" =~ ^[Yy]$ ]] || exit 1
        ;;
esac

info "=== OpenDex Testnet VPS Setup ==="

# --- Update system packages ---
info "Updating system packages..."
apt-get update -y
apt-get upgrade -y

# --- Install required packages ---
info "Installing required packages..."

# Detect SSL library package name (Debian 11 uses libssl1.1, Debian 12+ and Ubuntu use libssl3)
SSL_PKG="libssl3"
if [[ "$ID" == "debian" ]] && [[ "$VERSION_ID" =~ ^1[1-9]\. ]]; then
    SSL_PKG="libssl1.1"
fi

apt-get install -y \
    nginx \
    certbot \
    python3-certbot-nginx \
    ufw \
    fail2ban \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    "$SSL_PKG" \
    unzip \
    jq

# --- Create system users ---
info "Creating system users..."

# OpenDex user (for P2P nodes)
if ! id -u opendex &>/dev/null; then
    useradd -m -s /bin/bash opendex
    info "Created user: opendex"
else
    info "User opendex already exists, skipping."
fi

# Monero user
if ! id -u monero &>/dev/null; then
    useradd -m -s /bin/bash monero
    info "Created user: monero"
else
    info "User monero already exists, skipping."
fi

# Bitcoin user
if ! id -u bitcoin &>/dev/null; then
    useradd -m -s /bin/bash bitcoin
    info "Created user: bitcoin"
else
    info "User bitcoin already exists, skipping."
fi

# --- Configure firewall (UFW) ---
info "Configuring firewall..."
ufw --force reset
ufw default deny incoming
ufw default allow outgoing

# SSH
ufw allow 22/tcp

# Web
ufw allow 80/tcp
ufw allow 443/tcp

# P2P node ports
ufw allow 4001:4003/tcp

# Monero stagenet
ufw allow 18089/tcp

# Bitcoin testnet
ufw allow 18333/tcp

# Enable UFW
ufw --force enable
info "Firewall configured and enabled."

# --- Configure fail2ban ---
info "Configuring fail2ban..."
if [[ ! -f /etc/fail2ban/jail.local ]]; then
    cat > /etc/fail2ban/jail.local <<'EOF'
[DEFAULT]
bantime = 3600
findtime = 600
maxretry = 5

[sshd]
enabled = true
EOF
    info "Created /etc/fail2ban/jail.local"
fi
systemctl enable fail2ban
systemctl restart fail2ban

# --- Configure Nginx ---
info "Configuring Nginx..."
systemctl enable nginx
# Default site will be replaced by setup-web.sh

# --- Create data directories ---
info "Creating data directories..."
mkdir -p /home/opendex/.opendex
chown -R opendex:opendex /home/opendex/.opendex

mkdir -p /home/monero/.bitmonero/stagenet
chown -R monero:monero /home/monero/.bitmonero

mkdir -p /home/bitcoin/.bitcoin/testnet3
chown -R bitcoin:bitcoin /home/bitcoin/.bitcoin

# --- Swap space (for smaller VPS) ---
if [[ ! -f /swapfile ]]; then
    info "Creating 2GB swap file..."
    dd if=/dev/zero of=/swapfile bs=1M count=2048
    chmod 600 /swapfile
    mkswap /swapfile
    swapon /swapfile
    echo '/swapfile none swap sw 0 0' >> /etc/fstab
    info "Swap file created and enabled."
else
    info "Swap file already exists, skipping."
fi

# --- Summary ---
info "=== VPS Setup Complete ==="
info "Users created: opendex, monero, bitcoin"
info "Packages installed: nginx, certbot, ufw, fail2ban, build tools"
info "Firewall rules: SSH (22), HTTP (80), HTTPS (443), P2P (4001-4003), Monero (18089), Bitcoin (18333)"
info ""
info "Next steps:"
info "  1. Run setup-wallets.sh to install Monero and Bitcoin daemons"
info "  2. Run setup-web.sh to deploy the web frontend"
info "  3. Run setup-nodes.sh to build and deploy P2P nodes"
info ""
info "Or run deploy-testnet.sh to automate all steps."
