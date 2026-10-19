#!/bin/bash
#
# OpenDex Red Team VPS Provisioning Script
#
# This script provisions a single VPS instance for the OpenDex testnet.
# Run it on each VPS instance with the appropriate role:
#   - honest-node (run on 4 instances)
#   - attacker-node (run on 1 instance)
#   - monitoring (run on 1 instance)
#
# Usage:
#   ./vps_setup.sh <role> [options]
#
# Options:
#   --node-id <id>        Node ID (1-4 for honest nodes, default: 1)
#   --attacker-ip <ip>    Attacker node IP (for honest nodes)
#   --honest-ips <ips>    Comma-separated honest node IPs (for attacker)
#   --monitoring-ip <ip>  Monitoring node IP (for all nodes)
#
# Example:
#   # On honest node 1
#   ./vps_setup.sh honest-node --node-id 1 --attacker-ip 10.0.1.100 --monitoring-ip 10.0.1.200
#
#   # On attacker node
#   ./vps_setup.sh attacker-node --honest-ips 10.0.1.10,10.0.1.11,10.0.1.12,10.0.1.13 --monitoring-ip 10.0.1.200
#
#   # On monitoring node
#   ./vps_setup.sh monitoring

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    log_error "Please run as root (use sudo)"
    exit 1
fi

# Parse arguments
ROLE=""
NODE_ID=1
ATTACKER_IP=""
HONEST_IPS=""
MONITORING_IP=""

if [ $# -lt 1 ]; then
    echo "Usage: $0 <role> [options]"
    echo "Roles: honest-node, attacker-node, monitoring"
    exit 1
fi

ROLE="$1"
shift

while [[ $# -gt 0 ]]; do
    case $1 in
        --node-id)
            NODE_ID="$2"
            shift 2
            ;;
        --attacker-ip)
            ATTACKER_IP="$2"
            shift 2
            ;;
        --honest-ips)
            HONEST_IPS="$2"
            shift 2
            ;;
        --monitoring-ip)
            MONITORING_IP="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate role
case $ROLE in
    honest-node|attacker-node|monitoring)
        log_info "Role: $ROLE"
        ;;
    *)
        log_error "Invalid role: $ROLE. Must be: honest-node, attacker-node, monitoring"
        exit 1
        ;;
esac

# ============================================================
# 1. System Updates
# ============================================================
log_info "Updating system packages..."
apt-get update -y
apt-get upgrade -y -o Dpkg::Options::="--force-confdef" -o Dpkg::Options::="--force-confold"

# ============================================================
# 2. Install Dependencies
# ============================================================
log_info "Installing system dependencies..."
apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    ufw \
    htop \
    vim \
    net-tools \
    jq

# ============================================================
# 3. Install Rust Toolchain
# ============================================================
if command -v rustup &> /dev/null; then
    log_info "Rust toolchain already installed, skipping..."
else
    log_info "Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Verify Rust installation
rustup default stable
rustc --version
cargo --version

# ============================================================
# 4. Install Docker (for all roles)
# ============================================================
if command -v docker &> /dev/null; then
    log_info "Docker already installed, skipping..."
else
    log_info "Installing Docker..."
    curl -fsSL https://get.docker.com -o get-docker.sh
    sh get-docker.sh
    usermod -aG docker $SUDO_USER || true
fi

# Install Docker Compose
if ! docker compose version &> /dev/null; then
    log_info "Installing Docker Compose..."
    DOCKER_COMPOSE_VERSION=$(curl -s https://api.github.com/repos/docker/compose/releases/latest | jq -r .tag_name)
    curl -SL "https://github.com/docker/compose/releases/download/${DOCKER_COMPOSE_VERSION}/docker-compose-$(uname -s)-$(uname -m)" \
        -o /usr/local/bin/docker-compose
    chmod +x /usr/local/bin/docker-compose
fi

# ============================================================
# 5. Configure Firewall
# ============================================================
log_info "Configuring firewall..."
ufw --force enable

# Allow SSH
ufw allow 22/tcp

# Role-specific firewall rules
case $ROLE in
    honest-node)
        # Allow libp2p ports from attacker node only
        if [ -n "$ATTACKER_IP" ]; then
            log_info "Allowing libp2p ports from attacker node: $ATTACKER_IP"
            ufw allow from $ATTACKER_IP to any port 4000:5000 proto tcp
            ufw allow from $ATTACKER_IP to any port 4000:5000 proto udp
        else
            log_warn "No attacker IP specified; allowing libp2p ports from anywhere (INSECURE)"
            ufw allow 4000:5000/tcp
            ufw allow 4000:5000/udp
        fi
        # Allow Prometheus metrics from monitoring node
        if [ -n "$MONITORING_IP" ]; then
            ufw allow from $MONITORING_IP to any port 9100 proto tcp
        fi
        ;;
    attacker-node)
        # Allow SSH only (inbound)
        log_info "Attacker node: SSH only inbound"
        # Outbound connections are allowed by default
        ;;
    monitoring)
        # Allow Prometheus, Grafana, Loki
        ufw allow 9090/tcp   # Prometheus
        ufw allow 3000/tcp   # Grafana
        ufw allow 3100/tcp   # Loki
        ufw allow 9100/tcp   # Node exporter (from all nodes)
        ;;
esac

log_info "Firewall rules:"
ufw status verbose

# ============================================================
# 6. Clone OpenDex Repository
# ============================================================
log_info "Cloning OpenDex repository..."
cd /opt
if [ -d "OpenDexRustSource" ]; then
    log_info "Repository already exists, pulling latest..."
    cd OpenDexRustSource
    git pull
else
    git clone https://github.com/your-org/OpenDexRustSource.git
    cd OpenDexRustSource
fi

# Build the project
log_info "Building OpenDex..."
source $HOME/.cargo/env
cargo build --release

# ============================================================
# 7. Role-Specific Configuration
# ============================================================
case $ROLE in
    honest-node)
        log_info "Configuring honest node $NODE_ID..."
        
        # Create testnet configuration
        mkdir -p /opt/opendex-testnet
        cat > /opt/opendex-testnet/node-${NODE_ID}.toml << EOF
# OpenDex Honest Node Configuration
# Node ID: $NODE_ID

[node]
id = "$NODE_ID"
name = "honest-node-${NODE_ID}"
listen_address = "0.0.0.0:400${NODE_ID}"

[libp2p]
listen_addresses = ["/ip4/0.0.0.0/tcp/400${NODE_ID}"]
protocol_version = "opendex-testnet/1.0.0"

[logging]
level = "info"
file = "/var/log/opendex/node-${NODE_ID}.log"
EOF
        
        # Create log directory
        mkdir -p /var/log/opendex
        touch /var/log/opendex/node-${NODE_ID}.log
        
        # Create systemd service
        cat > /etc/systemd/system/opendex-node@.service << EOF
[Unit]
Description=OpenDex Honest Node %i
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/OpenDexRustSource
ExecStart=/opt/OpenDexRustSource/target/release/p2p-node --config /opt/opendex-testnet/node-%i.toml
Restart=always
RestartSec=10
StandardOutput=append:/var/log/opendex/node-%i.log
StandardError=append:/var/log/opendex/node-%i.log

[Install]
WantedBy=multi-user.target
EOF
        
        systemctl daemon-reload
        log_info "Honest node $NODE_ID configured. Start with: systemctl start opendex-node@$NODE_ID"
        ;;
    
    attacker-node)
        log_info "Configuring attacker node..."
        
        # Create testnet configuration
        mkdir -p /opt/opendex-testnet
        cat > /opt/opendex-testnet/attacker.toml << EOF
# OpenDex Attacker Node Configuration

[node]
id = "attacker"
name = "red-team-agent"

[libp2p]
listen_addresses = ["/ip4/0.0.0.0/tcp/4000"]
protocol_version = "opendex-testnet/1.0.0"

[honest_nodes]
addresses = [$(echo $HONEST_IPS | sed 's/,/","/g' | sed 's/^/"/' | sed 's/$/"/')]

[logging]
level = "debug"
file = "/var/log/opendex/attacker.log"
EOF
        
        # Create log directory
        mkdir -p /var/log/opendex
        touch /var/log/opendex/attacker.log
        
        log_info "Attacker node configured. Run with: cd redteam/agent && cargo run --release"
        ;;
    
    monitoring)
        log_info "Configuring monitoring node..."
        
        # Install Prometheus
        log_info "Installing Prometheus..."
        PROMETHEUS_VERSION="2.51.0"
        curl -SL "https://github.com/prometheus/prometheus/releases/download/v${PROMETHEUS_VERSION}/prometheus-${PROMETHEUS_VERSION}.linux-amd64.tar.gz" \
            -o /tmp/prometheus.tar.gz
        tar xzf /tmp/prometheus.tar.gz -C /opt/
        mv /opt/prometheus-*.linux-amd64 /opt/prometheus
        
        # Create Prometheus configuration
        cat > /opt/prometheus/prometheus.yml << EOF
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  - job_name: 'opendex-nodes'
    static_configs:
      - targets: [$(echo $HONEST_IPS | sed 's/,"/","/g' | sed 's/^/"/' | sed 's/$/":9100"/')]
        labels:
          group: 'honest-nodes'
      - targets: ["${ATTACKER_IP}:9100"]
        labels:
          group: 'attacker-node'
EOF
        
        # Create Prometheus systemd service
        cat > /etc/systemd/system/prometheus.service << EOF
[Unit]
Description=Prometheus
After=network.target

[Service]
Type=simple
User=root
ExecStart=/opt/prometheus/prometheus --config.file=/opt/prometheus/prometheus.yml --storage.tsdb.path=/opt/prometheus/data
Restart=always

[Install]
WantedBy=multi-user.target
EOF
        
        # Install Grafana
        log_info "Installing Grafana..."
        apt-get install -y apt-transport-https software-properties-common
        curl -s https://packages.grafana.com/gpg.key | apt-key add -
        add-apt-repository "deb https://packages.grafana.com/oss/deb stable main"
        apt-get update
        apt-get install -y grafana
        
        systemctl daemon-reload
        systemctl enable prometheus
        systemctl enable grafana-server
        
        log_info "Monitoring node configured. Start with: systemctl start prometheus grafana-server"
        log_info "Grafana available at: http://$(hostname -I | awk '{print $1}'):3000"
        log_info "Prometheus available at: http://$(hostname -I | awk '{print $1}'):9090"
        ;;
esac

# ============================================================
# 8. Install Prometheus Node Exporter (all roles)
# ============================================================
log_info "Installing Prometheus node exporter..."
NODE_EXPORTER_VERSION="1.7.0"
curl -SL "https://github.com/prometheus/node_exporter/releases/download/v${NODE_EXPORTER_VERSION}/node_exporter-${NODE_EXPORTER_VERSION}.linux-amd64.tar.gz" \
    -o /tmp/node_exporter.tar.gz
tar xzf /tmp/node_exporter.tar.gz -C /opt/
mv /opt/node_exporter-*.linux-amd64 /opt/node_exporter

cat > /etc/systemd/system/node_exporter.service << EOF
[Unit]
Description=Prometheus Node Exporter
After=network.target

[Service]
Type=simple
User=root
ExecStart=/opt/node_exporter/node_exporter
Restart=always

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable node_exporter
systemctl start node_exporter

# ============================================================
# 9. Summary
# ============================================================
log_info "========================================="
log_info "VPS provisioning complete!"
log_info "========================================="
log_info "Role: $ROLE"
log_info "Node ID: $NODE_ID"
log_info ""
log_info "Next steps:"
case $ROLE in
    honest-node)
        log_info "1. Start the node: systemctl start opendex-node@$NODE_ID"
        log_info "2. Check status: systemctl status opendex-node@$NODE_ID"
        log_info "3. View logs: tail -f /var/log/opendex/node-${NODE_ID}.log"
        ;;
    attacker-node)
        log_info "1. Connect to honest nodes: cd redteam/agent && cargo run --release -- --connect <honest-node-multiaddr>"
        log_info "2. Run attack scenarios: cargo run --release -- --category protocol"
        log_info "3. View logs: tail -f /var/log/opendex/attacker.log"
        ;;
    monitoring)
        log_info "1. Start services: systemctl start prometheus grafana-server"
        log_info "2. Access Grafana: http://$(hostname -I | awk '{print $1}'):3000"
        log_info "3. Access Prometheus: http://$(hostname -I | awk '{print $1}'):9090"
        ;;
esac
log_info ""
log_info "Node exporter metrics: http://$(hostname -I | awk '{print $1}'):9100/metrics"
