---
sessionId: session-261011-095052-o561
---

# Requirements

### Overview & Goals

Deploy a complete testnet environment for OpenDex on a single VPS, including:
- **Web frontend** — static informational portal (Leptos/WASM) served via Nginx with HTTPS
- **P2P protocol testnet** — 3 nodes (1 bootstrap + 2 test) running as systemd services
- **Wallet RPCs** — Monero stagenet wallet and Bitcoin testnet wallet for on-chain operations

### Scope

**In Scope:**
- VPS provisioning and initial setup (Ubuntu 24.04 LTS)
- Web frontend build and deployment (extend existing `docs/deployment/` scripts)
- P2P node systemd service configuration
- Monero stagenet wallet RPC setup
- Bitcoin testnet wallet RPC setup
- Bootstrap node + 2 test nodes with inter-node connectivity
- HTTPS via Let's Encrypt
- Firewall and SSH hardening
- Testnet deployment documentation

**Out of Scope:**
- Production deployment (this is testnet only)
- Monero mainnet or Bitcoin mainnet integration
- Multi-VPS distribution
- CI/CD pipeline automation
- Desktop GUI application deployment

### User Stories

1. As a developer, I want to deploy the testnet with a single script so I can set up the environment quickly.
2. As a tester, I want to connect to the bootstrap node and verify P2P order book synchronization across 3 nodes.
3. As a developer, I want to test on-chain wallet operations (Monero stagenet, Bitcoin testnet) via the P2P node CLI.
4. As an operator, I want to monitor node status and logs via systemd journal.

### Acceptance Criteria

- Web portal accessible at `https://<domain>` with all pages working (home, source browser, legal)
- Bootstrap node running on port 4001, discoverable by test nodes
- 2 test nodes connected to bootstrap node via `--peer`
- Monero stagenet wallet RPC accessible at `http://127.0.0.1:18082`
- Bitcoin testnet wallet RPC accessible at `http://127.0.0.1:18332`
- All services auto-start on reboot via systemd
- Firewall configured (SSH, HTTP, HTTPS, P2P ports)

# Technical Design

### Current Implementation

**Web Frontend:** Already has deployment infrastructure in `docs/deployment/`:
- `build.sh` — builds the Leptos/WASM static site via Trunk
- `deploy.sh` — uploads to VPS, configures Nginx
- `deploy/nginx.conf` — Nginx config with security headers (WASM-enabled CSP)
- `DEPLOY.md` / `DEPLOYMENT_PLAN.md` — comprehensive deployment documentation

**P2P Protocol:** The `p2p-node` crate (`crates/p2p-node/`) is a CLI-based libp2p application:
- `main.rs` — entry point with CLI parsing, swarm setup, gossipsub topic subscription
- `cli.rs` — CLI args: `--port`, `--peer`, `--new-identity`, `--monero-rpc`, `--btc-rpc`, etc.
- Data directory: `~/.opendex/node-<port>/` (orders, trades, balances, audit logs)
- Identity: `~/.opendex/identity-<port>.key` (ed25519 keypair)
- Listens on `127.0.0.1:<port>` (currently hardcoded in `main.rs:215`)
- GossipSub topic: `opendex-orders`

**Key Finding:** The `main.rs:215` hardcodes listen address to `127.0.0.1`. For a testnet on a single VPS, this is acceptable since all nodes run locally. For multi-VPS, this would need to bind to `0.0.0.0`.

### Key Decisions

1. **Single VPS topology** — All services on one Ubuntu 24.04 LTS VPS (~8GB RAM recommended). Simpler operations, shared localhost networking.
2. **Systemd services** — Native process management with auto-restart, logging via journalctl. No Docker overhead.
3. **Port allocation** — Bootstrap node on 4001, test node 1 on 4002, test node 2 on 4003. Wallet RPCs on standard ports (18082 for Monero, 18332 for Bitcoin).
4. **Monero stagenet** — Use `monerod` on stagenet (not mainnet) with `--stagenet` flag. Wallet RPC via `--rpc-bind-port=18082`.
5. **Bitcoin testnet** — Use `bitcoind` on testnet with `--testnet` flag. Wallet RPC via `--rpcport=18332`.
6. **Bootstrap-first startup** — Bootstrap node starts first, test nodes dial it via `--peer` flag.

### Architecture

```
graph LR
    A[Web Portal<br/>Nginx :80/:443] --> B[VPS]
    C[Bootstrap Node<br/>p2p-node :4001] --> B
    D[Test Node 1<br/>p2p-node :4002] --> B
    E[Test Node 2<br/>p2p-node :4003] --> B
    F[Monero Stagenet<br/>monerod :18080<br/>wallet-rpc :18082] --> B
    G[Bitcoin Testnet<br/>bitcoind :18332] --> B
    C -->|gossipsub| D
    C -->|gossipsub| E
    D -->|monero-rpc| F
    E -->|btc-rpc| G
    C -->|monero-rpc| F
    C -->|btc-rpc| G
```

### Proposed Changes

**New files to create:**

1. **`docs/testnet/`** — Testnet deployment directory
   - `README.md` — Testnet overview and quick start
   - `deploy-testnet.sh` — Main deployment script for the full testnet
   - `setup-vps.sh` — VPS initial setup (firewall, users, packages)
   - `setup-wallets.sh` — Monero stagenet + Bitcoin testnet wallet setup
   - `setup-nodes.sh` — P2P node systemd services
   - `setup-web.sh` — Web frontend deployment (reuses existing scripts)
   - `systemd/` — Systemd unit files
     - `opendex-bootstrap.service` — Bootstrap node
     - `opendex-testnode1.service` — Test node 1
     - `opendex-testnode2.service` — Test node 2
     - `monero-stagenet.service` — Monero daemon on stagenet
     - `bitcoin-testnet.service` — Bitcoin daemon on testnet
   - `testnet-config.env` — Environment variables for testnet configuration

2. **Modify `crates/p2p-node/src/main.rs`** — Add `--listen-address` CLI flag (default `127.0.0.1`) to support binding to `0.0.0.0` for multi-VPS deployments in the future.

3. **Modify `docs/deployment/deploy.sh`** — Update to support testnet domain configuration.

### Service Configuration

| Service | Port | User | Data Directory |
|---------|------|------|----------------|
| Nginx (web) | 80, 443 | www-data | `/var/www/opendex/` |
| Bootstrap node | 4001 | opendex | `/home/opendex/.opendex/node-4001/` |
| Test node 1 | 4002 | opendex | `/home/opendex/.opendex/node-4002/` |
| Test node 2 | 4003 | opendex | `/home/opendex/.opendex/node-4003/` |
| Monero stagenet | 18080, 18082 | monero | `/home/monero/.bitmonero/stagenet/` |
| Bitcoin testnet | 18332 | bitcoin | `/home/bitcoin/.bitcoin/testnet3/` |

### Data Models / Contracts

**Systemd service template** (`opendex-bootstrap.service`):
```
[Unit]
Description=OpenDex Bootstrap Node
After=network.target monero-stagenet.service bitcoin-testnet.service

[Service]
Type=simple
User=opendex
Group=opendex
ExecStart=/home/opendex/opendex/target/release/p2p-node \
  --port 4001 \
  --monero-rpc http://127.0.0.1:18082 \
  --btc-rpc http://127.0.0.1:18332
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=opendex-bootstrap

[Install]
WantedBy=multi-user.target
```

**Test node template** (`opendex-testnode1.service`):
```
[Unit]
Description=OpenDex Test Node 1
After=network.target opendex-bootstrap.service

[Service]
Type=simple
User=opendex
Group=opendex
ExecStart=/home/opendex/opendex/target/release/p2p-node \
  --port 4002 \
  --peer /ip4/127.0.0.1/tcp/4001/p2p/<BOOTSTRAP_PEER_ID> \
  --monero-rpc http://127.0.0.1:18082 \
  --btc-rpc http://127.0.0.1:18332
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=opendex-testnode1

[Install]
WantedBy=multi-user.target
```

### Risks

- **Monero stagenet sync** — `monerod` on stagenet requires initial blockchain sync (~hours). Mitigation: start sync before deploying nodes, or use `--fast-block-sync`.
- **Bitcoin testnet sync** — Similar sync time. Mitigation: use `prune=550` to reduce storage.
- **Bootstrap peer ID** — Test nodes need the bootstrap node's PeerId at startup. Mitigation: deploy script generates identity first, extracts PeerId, then configures test nodes.
- **Resource usage** — Single VPS with all services needs ~8GB RAM. Mitigation: document minimum requirements clearly.
- **Listen address** — `main.rs` hardcodes `127.0.0.1`. Mitigation: add `--listen-address` flag for future multi-VPS support.

# Testing

### Validation Approach

After deployment, verify each component independently, then test end-to-end flows.

### Key Scenarios

1. **Web portal verification:**
   - Visit `https://<domain>` — home page loads
   - Navigate to `/source` — source browser works
   - Navigate to `/legal` — legal page accessible
   - Check browser console — no WASM errors

2. **P2P node verification:**
   - `systemctl status opendex-bootstrap` — running
   - `systemctl status opendex-testnode1` — running
   - `systemctl status opendex-testnode2` — running
   - `journalctl -u opendex-bootstrap -f` — shows "Listening for peers..."
   - Test nodes show peer connections to bootstrap

3. **Wallet RPC verification:**
   - `curl http://127.0.0.1:18082/json_rpc` — Monero wallet RPC responds
   - `bitcoin-cli -testnet getblockcount` — Bitcoin testnet RPC responds

4. **End-to-end P2P test:**
   - Fund bootstrap node: `fund BTC 100000000` (1 BTC in test units)
   - Place order on bootstrap: `buy BTC XMR 50000 0.01`
   - Verify order appears on test nodes via gossipsub sync
   - Check `orders` command on test nodes shows the order

### Edge Cases

- **Node restart** — Verify nodes reconnect after `systemctl restart opendex-testnode1`
- **Bootstrap restart** — Test nodes should handle bootstrap going down and coming back
- **Wallet disconnection** — Nodes should degrade gracefully if wallet RPC is unavailable (already handled in `main.rs:263-264`)
- **Identity persistence** — Nodes should reload saved identities on restart (already in `main.rs:149-171`)

### Test Changes

No unit tests need modification. The testnet deployment is infrastructure-level validation.

# Delivery Steps

### ✓ Step 1: Add --listen-address flag to p2p-node CLI
Extend the p2p-node CLI to support configurable listen addresses, enabling future multi-VPS deployments.
- Add `--listen-address` argument to `CliArgs` in `crates/p2p-node/src/cli.rs` (default: `127.0.0.1`)
- Update `parse_args()` to parse the new flag
- Update `main.rs` to use `cli.listen_address` instead of hardcoded `127.0.0.1` at line 215
- Verify backward compatibility — existing `--port` flag still works

### ✓ Step 2: Create testnet systemd service files
Create systemd unit files for all testnet services in `docs/testnet/systemd/`.
- `opendex-bootstrap.service` — bootstrap node on port 4001, with wallet RPC flags
- `opendex-testnode1.service` — test node on port 4002, dials bootstrap
- `opendex-testnode2.service` — test node on port 4003, dials bootstrap
- `monero-stagenet.service` — monerod on stagenet with wallet RPC
- `bitcoin-testnet.service` — bitcoind on testnet with RPC enabled
- `testnet-config.env` — centralized config (ports, peer IDs, RPC URLs)
- Template peer IDs with placeholders that the deploy script fills in

### ✓ Step 3: Create VPS setup and wallet deployment scripts
Create scripts for VPS provisioning and wallet RPC setup.
- `docs/testnet/setup-vps.sh` — creates users (opendex, monero, bitcoin), installs packages (nginx, certbot, ufw, fail2ban), configures firewall
- `docs/testnet/setup-wallets.sh` — downloads and configures Monero stagenet daemon + wallet RPC, Bitcoin testnet daemon + RPC
- Includes `monerod` with `--stagenet --rpc-bind-port=18082 --non-interactive`
- Includes `bitcoind` with `--testnet --rpcport=18332 --server=1`
- Sets up systemd services for wallet daemons
- Documents sync time expectations

### ✓ Step 4: Create P2P node deployment script
Create the script that builds and deploys P2P nodes as systemd services.
- `docs/testnet/setup-nodes.sh` — builds p2p-node binary, generates bootstrap identity, extracts PeerId, configures test node services with the bootstrap peer address
- Installs systemd service files
- Enables and starts services in order: bootstrap → testnode1 → testnode2
- Verifies peer connections via journalctl
- Includes `verify-nodes.sh` helper for checking node status

### ✓ Step 5: Create web frontend testnet deployment
Extend existing web deployment for testnet.
- `docs/testnet/setup-web.sh` — reuses `docs/deployment/build.sh` and `deploy.sh` logic
- Configures Nginx with testnet domain
- Sets up HTTPS via Certbot
- Deploys static files to `/var/www/opendex/`
- Updates `docs/deployment/deploy.sh` to accept testnet domain parameter

### ✓ Step 6: Create main testnet deploy script and documentation
Create the master deployment script and comprehensive documentation.
- `docs/testnet/deploy-testnet.sh` — orchestrates all setup scripts in order: VPS → wallets → web → nodes
- `docs/testnet/README.md` — testnet overview, architecture diagram, quick start guide
- Includes sections: prerequisites, one-command deploy, manual setup, verification, troubleshooting, node management (start/stop/logs), wallet management
- Documents the testnet topology, port allocation, and peer connectivity
- Includes compliance notes referencing existing `audit/Regulatory_Compliance/` docs