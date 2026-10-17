# OpenDex Mainnet Deployment

Production deployment guide for OpenDex on mainnet. This document assumes you have already validated your infrastructure on testnet and are ready to launch with real funds.

## Prerequisites

- **Testnet validation complete** — All testnet health checks and E2E tests passing
- **VPS**: Ubuntu 24.04 LTS, minimum 16GB RAM, 16+ CPU cores recommended
- **Disk space**: 500+ GB free (NVMe preferred) for full blockchain data
  - Monero mainnet: ~200-400GB (with pruning: ~100-200GB)
  - Bitcoin mainnet: ~500GB+ (with pruning: ~550MB)
- **Domain** with DNS configured
- **SSH access** with sudo privileges
- **Strong RPC passwords** for wallet daemons
- **Legal compliance** reviewed — especially if enabling fiat payments

## Network Configuration Differences

| Component | Testnet | Mainnet |
|-----------|---------|---------|
| Monero network | `--stagenet` flag | No flag (mainnet default) |
| Monero P2P port | 18089 | 18080 |
| Monero RPC port | 18082/18083 | 18081/18082 |
| Monero data dir | `~/.bitmonero/stagenet` | `~/.bitmonero/lmdb` |
| Bitcoin network | `--testnet` flag | No flag (mainnet default) |
| Bitcoin RPC port | 18332 | 8332 |
| Bitcoin P2P port | 18333 | 8333 |
| Bitcoin data dir | `~/.bitcoin/testnet3` | `~/.bitcoin` |
| Blockchain sync | 10-60 min | **Hours to days** (full chain) |

## Deployment Phases

### Phase 1: Swaps-Only Mainnet (Recommended First Step)

Launch with BTC/XMR atomic swaps only, without fiat-to-crypto P2P trading. This is the lower-risk entry point.

```bash
# 1. VPS setup
sudo bash docs/testnet/setup-vps.sh

# 2. Wallet daemons (mainnet configuration required — see below)
# You need to modify the wallet setup for mainnet (see "Mainnet Wallet Setup" section)

# 3. P2P nodes (swaps-only, no fiat)
cargo build --release -p p2p-node
sudo bash docs/testnet/setup-nodes.sh /home/opendex/opendex
```

### Phase 2: Fiat Payments (Optional, Later)

When legally ready, rebuild with fiat support:

```bash
cargo build --release -p p2p-node --features fiat
# Replace the binary and restart services
systemctl restart opendex-bootstrap opendex-testnode1 opendex-testnode2
```

**Regulatory warning**: The fiat subsystem requires CASP + Payment Institution authorization under MiCA/PSD2. Consult legal counsel before enabling.

## Mainnet Wallet Setup

The testnet wallet scripts (`setup-wallets.sh`) are configured for Monero stagenet and Bitcoin testnet. For mainnet, you need to modify the configuration:

### Monero Mainnet Daemon

```ini
# /etc/systemd/system/monero-mainnet.service
[Unit]
Description=Monero Mainnet Daemon
After=network-online.target

[Service]
User=monero
ExecStart=/usr/local/bin/monerod \
    --prune-blockchain=1 \
    --data-dir=/data/monero-mainnet \
    --rpc-bind-port=18081 \
    --rpc-bind-ip=127.0.0.1 \
    --restricted-rpc \
    --confirm-external-bind \
    --max-log-files=5
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Key differences from testnet:
- **No `--stagenet` flag** — runs mainnet by default
- **`--prune-blockchain=1`** — reduces disk usage to ~100-200GB
- **`--restricted-rpc`** — production security hardening
- **Port 18081** (mainnet RPC) instead of 18082 (stagenet)

### Monero Mainnet Wallet RPC

```ini
# /etc/systemd/system/monero-wallet-rpc-mainnet.service
[Unit]
Description=Monero Wallet RPC (Mainnet)
After=monero-mainnet.service
Requires=monero-mainnet.service

[Service]
User=monero
ExecStart=/usr/local/bin/monero-wallet-rpc \
    --wallet-file=/home/monero/wallets/mainnet-wallet \
    --password-file=/home/monero/wallets/password.txt \
    --daemon-address=127.0.0.1:18081 \
    --rpc-bind-port=18082 \
    --rpc-bind-ip=127.0.0.1 \
    --disable-rpc-ban \
    --confirm-external-bind
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### Bitcoin Mainnet Daemon

```ini
# /etc/systemd/system/bitcoin-mainnet.service
[Unit]
Description=Bitcoin Mainnet Daemon
After=network-online.target

[Service]
User=bitcoin
ExecStart=/usr/local/bin/bitcoind \
    -datadir=/data/bitcoin-mainnet \
    -rpcport=8332 \
    -rpcbind=127.0.0.1 \
    -rpcuser=opendex \
    -rpcpassword=<STRONG_PASSWORD> \
    -prune=550 \
    -server=1 \
    -daemon=0
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Key differences from testnet:
- **No `--testnet` flag** — runs mainnet by default
- **`-prune=550`** — reduces disk usage to ~550MB
- **Port 8332** (mainnet RPC) instead of 18332 (testnet)
- **Strong RPC password** — required for production

## Mainnet Configuration File

Create `docs/mainnet/mainnet-config.env`:

```bash
# OpenDex Mainnet Configuration
BOOTSTRAP_PORT=4001
BOOTSTRAP_PEER_ID=
MONERO_RPC_URL=http://127.0.0.1:18082
BTC_RPC_URL=http://127.0.0.1:8332
BTC_RPC_USER=opendex
BTC_RPC_PASSWORD=<STRONG_PASSWORD>
```

## Security Hardening Checklist

Before launching mainnet, ensure:

- [ ] **Strong RPC passwords** — All wallet RPC credentials are strong and unique
- [ ] **Firewall rules** — Only necessary ports exposed (4001-4003 for P2P, 80/443 for web)
- [ ] **Wallet RPC binding** — Wallet RPCs bound to `127.0.0.1` only, not `0.0.0.0`
- [ ] **System users** — Dedicated users for each service (monero, bitcoin, opendex)
- [ ] **File permissions** — Wallet files and keys are not world-readable
- [ ] **TLS for wallet RPCs** — Consider TLS for wallet RPC communication
- [ ] **Backup strategy** — Regular backups of wallet files and node data
- [ ] **Monitoring** — WS14 from ROADMAP: alerting on service failures
- [ ] **Log rotation** — Configured for all services
- [ ] **DDoS protection** — Consider Cloudflare or similar for public-facing services

## Blockchain Sync Considerations

Mainnet blockchain sync is significantly longer than testnet:

| Chain | Full sync | Pruned sync |
|-------|-----------|-------------|
| Monero mainnet | 200-400GB, hours to days | 100-200GB, hours |
| Bitcoin mainnet | 500GB+, days | 550MB, hours |

**Recommendation**: Use pruning for both chains to minimize sync time and disk usage.

Monitor sync progress:
```bash
# Monero
journalctl -u monero-mainnet -f | grep 'Height'

# Bitcoin
bitcoin-cli getblockcount
```

## P2P Node Configuration for Mainnet

The P2P node configuration in `mainnet-config.env` points to mainnet wallet RPCs. The node binary is built the same way:

```bash
# Swaps-only (default)
cargo build --release -p p2p-node

# With fiat support
cargo build --release -p p2p-node --features fiat
```

The node connects to wallet RPCs via the configured URLs — no code changes needed between testnet and mainnet.

## Multi-VPS Deployment

For production, consider distributing nodes across multiple VPS instances:

1. **Bootstrap node** — Dedicated VPS with static IP
2. **Test/worker nodes** — Separate VPS instances
3. **Wallet daemons** — Can co-locate with nodes or run separately

Key configuration change for multi-VPS:
```bash
# On bootstrap node, bind to all interfaces
--listen-address 0.0.0.0

# On worker nodes, connect to bootstrap by public IP
--peer /ip4/<BOOTSTRAP_PUBLIC_IP>/tcp/4001/p2p/<BOOTSTRAP_PEER_ID>
```

## Pre-Launch Checklist

- [ ] Testnet validation complete and stable
- [ ] Mainnet wallet daemons synced
- [ ] P2P nodes built and configured for mainnet
- [ ] Security hardening applied
- [ ] Monitoring and alerting configured
- [ ] Backup strategy in place
- [ ] Legal compliance reviewed
- [ ] WS12 (CI/CD) automated tests passing
- [ ] WS13 (QA/UAT) formal acceptance testing complete
- [ ] WS08 (External audit) security review completed

## Rollback Plan

If issues arise after mainnet launch:

1. **Stop P2P nodes**: `systemctl stop opendex-bootstrap opendex-testnode1 opendex-testnode2`
2. **Preserve data**: Node data in `~/.opendex/` contains order book and trade state
3. **Rebuild if needed**: `cargo build --release -p p2p-node`
4. **Restart**: `systemctl start opendex-bootstrap opendex-testnode1 opendex-testnode2`

Node persistence ensures order book and trade state survive restarts.

## ROADMAP Alignment

Mainnet deployment corresponds to **WS15 (Production Readiness)** in `docs/ROADMAP.md`. Before mainnet launch, ensure these workstreams are complete:

| Workstream | Priority |
|------------|----------|
| WS12: CI/CD | **High** — Automated testing |
| WS14: Monitoring | **High** — Production alerting |
| WS08: External audit | **High** — Security review |
| WS11: Performance | Medium — Capacity planning |
| WS13: QA/UAT | **High** — Acceptance testing |

## Compliance Notes

- **Crypto-to-crypto swaps**: Rated 🟢 GREEN — potentially outside MiCA scope under Recital 22 if genuinely decentralized
- **Fiat subsystem**: Rated 🔴 RED — requires CASP + Payment Institution authorization
- See `audit/regulatory_compliance/regulatory_compliance.md` for detailed compliance analysis
