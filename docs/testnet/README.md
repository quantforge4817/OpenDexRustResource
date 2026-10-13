# OpenDex Testnet Deployment

Complete testnet environment for OpenDex on a single VPS, including the web frontend, P2P protocol nodes, and wallet RPCs.

## Overview

The testnet deploys:

- **Web Frontend** — Static informational portal (Leptos/WASM) served via Nginx with HTTPS
- **P2P Protocol** — 3 nodes (1 bootstrap + 2 test) running as systemd services
- **Wallet RPCs** — Monero stagenet wallet and Bitcoin testnet wallet for on-chain operations

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  VPS (Ubuntu 24.04)             │
│                                                 │
│  ┌──────────────┐  ┌──────────────────────────┐ │
│  │  Web Portal   │  │  P2P Nodes               │ │
│  │  Nginx        │  │  Bootstrap :4001         │ │
│  │  :80 / :443   │  │  Test Node 1 :4002       │ │
│  └──────────────┘  │  Test Node 2 :4003        │ │
│                     └──────────────────────────┘ │
│                                                 │
│  ┌──────────────┐  ┌──────────────────────────┐ │
│  │ Monero        │  │  Bitcoin                 │ │
│  │ Stagenet      │  │  Testnet                 │ │
│  │ :18080/:18082 │  │  :18332                  │ │
│  └──────────────┘  └──────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

## Prerequisites

- **VPS**: Ubuntu 24.04 LTS, minimum 8GB RAM, 50GB disk
- **Domain** (optional): For HTTPS via Let's Encrypt
- **SSH access** with sudo privileges
- **Rust toolchain**: Will be auto-installed if missing

## Quick Start

### One-Command Deployment

```bash
# Clone the repository on the VPS
git clone <repo-url> /home/opendex/opendex

# Run the deployment script
sudo bash docs/testnet/deploy-testnet.sh [domain] [/path/to/repo]
```

Example:

```bash
sudo bash docs/testnet/deploy-testnet.sh testnet.opendex.io /home/opendex/opendex
```

### Manual Step-by-Step

```bash
# 1. VPS setup (users, packages, firewall)
sudo bash docs/testnet/setup-vps.sh

# 2. Wallet daemons (Monero stagenet + Bitcoin testnet)
sudo bash docs/testnet/setup-wallets.sh

# 3. Web frontend (static site + Nginx + HTTPS)
sudo bash docs/testnet/setup-web.sh [domain] [repo-path]

# 4. P2P nodes (build + deploy + start)
sudo bash docs/testnet/setup-nodes.sh [repo-path]
```

## Port Allocation

| Service | Port | Description |
|---------|------|-------------|
| Nginx (HTTP) | 80 | Web portal HTTP |
| Nginx (HTTPS) | 443 | Web portal HTTPS |
| Bootstrap Node | 4001 | P2P bootstrap node |
| Test Node 1 | 4002 | P2P test node 1 |
| Test Node 2 | 4003 | P2P test node 2 |
| Monero P2P | 18089 | Monero stagenet P2P |
| Monero RPC | 18082 | Monero wallet RPC |
| Bitcoin RPC | 18332 | Bitcoin testnet RPC |
| Bitcoin P2P | 18333 | Bitcoin testnet P2P |

## Service Management

### P2P Nodes

```bash
# Check status
systemctl status opendex-bootstrap opendex-testnode1 opendex-testnode2

# View logs
journalctl -u opendex-bootstrap -f
journalctl -u opendex-testnode1 -f
journalctl -u opendex-testnode2 -f

# Restart a node
systemctl restart opendex-testnode1

# Stop all nodes
systemctl stop opendex-bootstrap opendex-testnode1 opendex-testnode2

# Start all nodes
systemctl start opendex-bootstrap opendex-testnode1 opendex-testnode2
```

### Wallet Daemons

```bash
# Check status
systemctl status monero-stagenet bitcoin-testnet

# View logs
journalctl -u monero-stagenet -f
journalctl -u bitcoin-testnet -f

# Check blockchain sync
journalctl -u monero-stagenet | grep 'Height'
bitcoin-cli -testnet getblockcount
```

### Web Frontend

```bash
# Check Nginx status
systemctl status nginx

# View logs
journalctl -u nginx -f

# Reload configuration
sudo nginx -t && sudo systemctl reload nginx
```

## Verification

### Automated Health Check

Run the built-in health check script to verify all services:

```bash
sudo bash docs/testnet/test-results/verification/health-check.sh
```

This checks systemd services, listening ports, wallet RPC endpoints, and the web portal.

### End-to-End Tests

Run the full E2E test suite after deployment:

```bash
sudo bash docs/testnet/test-results/verification/e2e-test.sh
```

This runs the health check, then tests P2P connectivity, wallet RPCs, and generates a timestamped report in `test-results/reports/`.

### Test Results Directory

All test artifacts are stored in `docs/testnet/test-results/`:

```
test-results/
├── README.md           # Test directory documentation
├── reports/            # Generated test reports (Markdown/JSON)
├── logs/               # Raw test run logs
├── snapshots/          # Screenshots or state snapshots
└── verification/       # Test scripts
    ├── health-check.sh # Quick service health check
    └── e2e-test.sh     # Full end-to-end test suite
```

Test reports and logs are timestamped and saved automatically. See `test-results/README.md` for details.

### Manual Checks

#### Web Portal

```bash
curl http://localhost
curl http://localhost/source
curl http://localhost/legal
```

#### P2P Nodes

```bash
systemctl is-active opendex-bootstrap opendex-testnode1 opendex-testnode2
journalctl -u opendex-bootstrap | grep -i 'peer\|connected\|dial'
journalctl -u opendex-bootstrap -f
```

#### Wallet RPCs

```bash
curl -X POST http://127.0.0.1:18082/json_rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":"0","method":"get_version"}'

bitcoin-cli -testnet getblockcount
```

## Testing P2P Functionality

### Connect to a Node

The nodes run as systemd services and accept commands via their stdin. To interact:

```bash
# View node logs to see current state
journalctl -u opendex-bootstrap -f
```

### End-to-End Test

1. **Fund a node** (test units):
   - The bootstrap node can be funded with test BTC/XMR balances

2. **Place an order**:
   - Create a buy/sell order on the bootstrap node

3. **Verify synchronization**:
   - Check that the order appears on test nodes via gossipsub

4. **Check order book**:
   - Verify the order book is consistent across all 3 nodes

## Troubleshooting

### Node Won't Start

```bash
# Check logs
journalctl -u opendex-bootstrap -n 50

# Common issues:
# - Port already in use: sudo lsof -i :4001
# - Missing binary: Check build output in setup-nodes.sh
# - Wallet RPC not ready: Wait for blockchain sync
```

### Peer Connections Failing

```bash
# Check bootstrap PeerId
grep 'Peer ID' /home/opendex/.opendex/node-4001/audit/audit.log

# Verify test nodes have correct peer address
grep 'peer' /etc/systemd/system/opendex-testnode1.service

# Restart test nodes
systemctl restart opendex-testnode1 opendex-testnode2
```

### Wallet RPC Not Responding

```bash
# Check daemon status
systemctl status monero-stagenet
systemctl status bitcoin-testnet

# Check sync progress
journalctl -u monero-stagenet | grep 'Height' | tail -5
bitcoin-cli -testnet getblockcount

# Wait for sync to complete before using on-chain operations
```

### HTTPS Issues

```bash
# Re-run Certbot
sudo certbot --nginx -d your-domain.com

# Check Nginx configuration
sudo nginx -t

# View Nginx error log
journalctl -u nginx -f
```

## Configuration

All testnet configuration is centralized in `docs/testnet/testnet-config.env`. Key variables:

- `BOOTSTRAP_PORT` — Bootstrap node port (default: 4001)
- `MONERO_RPC_URL` — Monero wallet RPC URL
- `BTC_RPC_URL` — Bitcoin wallet RPC URL
- `BOOTSTRAP_PEER_ID` — Auto-filled during deployment

## File Structure

```
docs/testnet/
├── README.md                  # This file
├── deploy-testnet.sh          # Master deployment script
├── setup-vps.sh               # VPS initial setup
├── setup-wallets.sh           # Monero + Bitcoin daemon setup
├── setup-web.sh               # Web frontend deployment
├── setup-nodes.sh             # P2P node deployment
├── testnet-config.env         # Centralized configuration
├── test-results/              # Test results and verification
│   ├── README.md              # Test directory documentation
│   ├── reports/               # Generated test reports
│   ├── logs/                  # Raw test run logs
│   ├── snapshots/             # Screenshots or state snapshots
│   └── verification/          # Test scripts
│       ├── health-check.sh    # Quick service health check
│       └── e2e-test.sh        # Full end-to-end test suite
└── systemd/
    ├── opendex-bootstrap.service
    ├── opendex-testnode1.service
    ├── opendex-testnode2.service
    ├── monero-stagenet.service
    └── bitcoin-testnet.service
```

## Compliance Notes

For regulatory compliance information, see:

- `audit/Regulatory_Compliance/` — Compliance documentation
- `audit/AUDIT.md` — Security audit report
- `audit/ESCROW_DESIGN.md` — Escrow mechanism design

## Risks & Limitations

- **Blockchain sync time**: Monero stagenet and Bitcoin testnet require initial sync (10-60 minutes)
- **Resource usage**: All services on one VPS require ~8GB RAM minimum
- **Testnet only**: This is for testing purposes — not for production use
- **Single VPS**: All nodes run on localhost; for multi-VPS, use `--listen-address 0.0.0.0`
