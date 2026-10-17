# OpenDex Testnet Deployment

Complete testnet environment for OpenDex on a single VPS, including the P2P protocol nodes and wallet RPCs.

## Overview

The testnet deploys:

- **P2P Protocol** — 3 nodes (1 bootstrap + 2 test) running as systemd services
- **Wallet RPCs** — Monero stagenet wallet and Bitcoin testnet wallet for on-chain operations
- **Web Frontend** (optional) — Static informational portal (Leptos/WASM) served via Nginx with HTTPS
- **Matrix Server** (optional) — Self-hosted Conduit homeserver for community chat

See also **[docs/matrix/](../matrix/README.md)** for the Matrix homeserver deployment guide.

## Phased Deployment

OpenDex supports a **two-phase deployment** strategy, allowing you to launch with core swap functionality first, then add fiat-to-crypto P2P trading later.

### Phase 1: Swaps-Only (Default)

The default deployment builds `p2p-node` **without** the `fiat` feature, providing:
- BTC/XMR atomic swaps via the P2P network
- Order book, trade matching, and settlement
- Monero stagenet + Bitcoin testnet wallet integration
- Lower regulatory risk (crypto-to-crypto swaps rated 🟢 GREEN)

### Phase 2: Fiat Payments (Optional)

When ready, rebuild with the `fiat` feature enabled to add:
- Fiat-to-crypto P2P trading (buy/sell crypto for bank transfer, cash, etc.)
- Payment adapters (SEPA, UK Faster Payments, cash deposit, in-person cash)
- Dispute resolution and security bonds
- Crypto escrow for fiat trades
- **Regulatory note**: Fiat subsystem rated 🔴 RED — requires separate legal compliance workstream (CASP + Payment Institution authorization under MiCA/PSD2)

To enable fiat support, rebuild the node with `--features fiat`:
```bash
cargo build --release -p p2p-node --features fiat
```

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
│  │ :18080/:18083 │  │  :18332                  │ │
│  └──────────────┘  └──────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

## Prerequisites

- **VPS/Home PC**: Ubuntu 24.04 LTS or Debian 11+, minimum 8GB RAM, 12+ CPU cores recommended
- **Disk space**: 100+ GB free for blockchain data (Monero stagenet + Bitcoin testnet)
  - **Recommended**: Use a separate `/data` partition (NVMe preferred) for blockchain storage
  - Blockchain data is stored in `/data/monero-stagenet/` and `/data/bitcoin-testnet/`
  - If no `/data` partition exists, the scripts will use `/home/<user>/.bitmonero/` and `/home/<user>/.bitcoin/` instead
- **Domain** (optional): For HTTPS via Let's Encrypt
- **SSH access** with sudo privileges
- **Rust toolchain**: Will be auto-installed if missing

### Debian 11 Notes

Debian 11 is fully supported. The scripts automatically detect your OS and adjust package names accordingly. However, pre-built Monero and Bitcoin binaries may have glibc compatibility issues on older Debian versions. If this occurs:

- **Monero**: Install via `sudo apt-get install monero` or compile from source
- **Bitcoin**: Install via `sudo apt-get install bitcoin-d` or compile from source

The wallet setup script will detect binary failures and provide clear instructions.

### Home Computer Setup

Running on a home computer (e.g., laptop) is supported. Key considerations:

- **Power management**: Disable sleep/hibernate for 24/7 operation
- **Network**: Home internet may have dynamic IP (use DDNS) and asymmetric bandwidth
- **Ports**: ISP may block ports 80/443; use 8080/8443 as alternatives
- **Resources**: Your machine should have at least 8GB RAM and 100GB free disk space

## Quick Start

### Swaps-Only Deployment (Default)

The default deployment builds the P2P nodes without fiat support, providing core BTC/XMR swap functionality.

#### One-Command Deployment

```bash
# Clone the repository on the VPS
git clone <repo-url> /home/opendex/opendex

# Run the deployment script (swaps-only, no fiat)
sudo bash docs/testnet/deploy-testnet.sh [domain] [/path/to/repo]
```

Example:

```bash
sudo bash docs/testnet/deploy-testnet.sh testnet.opendex.io /home/opendex/opendex
```

#### Manual Step-by-Step

```bash
# 1. VPS setup (users, packages, firewall)
sudo bash docs/testnet/setup-vps.sh

# 2. Wallet daemons (Monero stagenet + Bitcoin testnet)
sudo bash docs/testnet/setup-wallets.sh

# 3. P2P nodes (build without fiat + deploy + start)
sudo bash docs/testnet/setup-nodes.sh [repo-path]

# 4. Web frontend (optional, static site + Nginx + HTTPS)
sudo bash docs/testnet/setup-web.sh [domain] [repo-path]
```

### Full Deployment with Fiat Support

To include fiat-to-crypto P2P trading, build the nodes with the `fiat` feature:

```bash
# Build with fiat support
sudo bash docs/testnet/setup-nodes.sh [repo-path] --with-fiat
```

Or manually:

```bash
cargo build --release -p p2p-node --features fiat
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
| Monero RPC | 18083 | Monero wallet RPC |
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
# Monero wallet RPC (port 18083, requires /json_rpc path)
curl -X POST http://127.0.0.1:18083/json_rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":"0","method":"get_height","params":{}}'

# Bitcoin testnet RPC
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

- `audit/regulatory_compliance/` — Compliance documentation and fiat subsystem separation boundary
- `audit/audit.md` — Security audit report
- `audit/escrow_design.md` — Escrow mechanism design

The fiat subsystem is rated 🔴 RED for regulatory compliance. Launching swaps-only (Phase 1) is the recommended path, as crypto-to-crypto swaps are rated 🟢 GREEN.

## Deployment Status

**Last verified:** 2026-10-14

- **Health check:** 16/16 passed
- **E2E tests:** 10/10 passed
- **Node stability:** All 3 nodes running stably
- **Wallet RPCs:** Monero stagenet ✓, Bitcoin testnet ✓

## Risks & Limitations

- **Blockchain sync time**: Monero stagenet and Bitcoin testnet require initial sync (10-60 minutes)
- **Resource usage**: All services on one machine require ~8GB RAM minimum
- **Storage**: Blockchain data stored on `/data` partition (~100-150GB total with pruning enabled)
  - Monero stagenet: ~20-40GB with `prune-blockchain=1`
  - Bitcoin testnet: ~550MB with `prune=550`
- **Testnet only**: This is for testing purposes — not for production use
- **Single machine**: All nodes run on localhost; for multi-VPS, use `--listen-address 0.0.0.0`
- **Fiat feature**: Default build excludes fiat-to-crypto P2P trading. Use `--with-fiat` flag or `--features fiat` to include it.

## Path to Mainnet

Once testnet validation is complete, the path to mainnet requires only deployment configuration changes — no protocol code changes:

| Component | Testnet | Mainnet |
|-----------|---------|---------|
| Monero network | `--stagenet` flag | No flag (mainnet default) |
| Monero P2P port | 18089 | 18080 |
| Monero RPC port | 18082/18083 | 18081/18082 |
| Bitcoin network | `--testnet` flag | No flag (mainnet default) |
| Bitcoin RPC port | 18332 | 8332 |
| Bitcoin P2P port | 18333 | 8333 |
| Blockchain sync | 10-60 min | **Hours to days** (full chain) |
| Disk space | ~100 GB | **500+ GB** for full chains |

See `docs/mainnet/README.md` for the mainnet deployment guide.

## Known Issues

- **Monero wallet RPC path**: The Monero wallet RPC requires the `/json_rpc` path (e.g., `http://127.0.0.1:18083/json_rpc`). Requests to the root path will fail with 404.
- **Node restart order**: Restart services in dependency order: wallet daemons first, then bootstrap node (with 2s delay), then test nodes, then nginx.
