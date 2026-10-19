# OpenDex Red Team Infrastructure

This directory contains all infrastructure for red team penetration testing of the OpenDex P2P network. It provides the tooling to deploy a 5-node testnet, execute adversarial scenarios, and document findings.

## Overview

The red team infrastructure consists of:

1. **Testnet deployment**: 4 honest nodes + 1 attacker node connected via libp2p/GossipSub
2. **Red team agent**: Rust crate that connects as a legitimate peer, then injects attacks
3. **Scenario definitions**: Detailed attack vectors with expected behaviors and validation criteria
4. **Reporting**: Structured templates for documenting findings

## Prerequisites

- **Cloud VPS**: 5 instances (see `audit/red_team_plan.md` §2.1 for specs)
- **Domain name**: For testnet bootstrap (e.g., `testnet.opendex.io`)
- **Rust toolchain**: Stable 1.75+ with `rustup`
- **Docker**: For local testnet emulation
- **Prometheus + Grafana**: For monitoring (optional, recommended)

## Quick Start

### Local Development (Docker)

For local testing without cloud VPS, use Docker Compose to emulate the 5-node testnet:

```bash
# Build and start all nodes
cd redteam/infra
docker-compose up -d

# View logs
docker-compose logs -f

# Run the attacker agent
docker-compose exec attacker cargo run --bin red-team-agent

# Stop all nodes
docker-compose down
```

### Cloud Deployment (VPS)

For real network-level testing, deploy on cloud VPS instances:

```bash
# 1. Provision VPS instances (see infra/vps_setup.sh for automated setup)
# 2. Install Rust toolchain on all instances
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 3. Configure firewall rules
#    - Honest nodes: Allow inbound from attacker node only (libp2p ports)
#    - Attacker node: Allow inbound SSH only

# 4. Deploy honest nodes
cd crates/p2p-node
cargo build --release
./target/release/p2p-node --config testnet-node1.toml

# 5. Deploy attacker agent
cd redteam/agent
cargo build --release
./target/release/red-team-agent --connect <honest-node-multiaddr>
```

## Directory Structure

```
redteam/
├── README.md                   # This file
├── infra/                      # Infrastructure setup
│   ├── vps_setup.sh            # VPS provisioning script
│   ├── docker-compose.yml      # Local testnet emulation
│   └── monitoring/             # Prometheus/Grafana configs
├── agent/                      # Attacker agent crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # Agent entry point
│       ├── protocol_attacks.rs # Protocol attack scenarios
│       ├── crypto_attacks.rs   # Cryptographic attack scenarios
│       ├── dos_attacks.rs      # Denial of service scenarios
│       └── fiat_attacks.rs     # Fiat payment attack scenarios
├── scenarios/                  # Test scenario definitions
│   ├── network.md              # Network-level attack scenarios
│   ├── crypto.md               # Cryptographic attack scenarios
│   ├── application.md          # Application-level attack scenarios
│   └── fiat.md                 # Fiat-specific attack scenarios
└── reports/                    # Test results and findings
    └── template.md             # Report template
```

## Network Topology

```
                    ┌─────────────────┐
                    │  Attacker Node  │
                    │  (red-team-agent)│
                    └────────┬────────┘
                             │ libp2p/GossipSub
              ┌──────────────┼──────────────┐
              │              │              │
    ┌─────────▼────┐ ┌──────▼──────┐ ┌─────▼────────┐
    │ Honest Node 1│ │ Honest Node 2│ │ Honest Node 3│
    │ (p2p-node)   │ │ (p2p-node)   │ │ (p2p-node)   │
    └──────────────┘ └──────────────┘ └──────────────┘
              │
    ┌─────────▼────┐
    │ Honest Node 4│
    │ (p2p-node)   │
    └──────────────┘
```

## Attack Scenario Categories

| Category | Scenarios | Target Crates | Description |
|----------|-----------|---------------|-------------|
| Protocol | 8 | `p2p-node`, `protocol` | Malformed messages, replay, sequence manipulation |
| Crypto | 7 | `user`, `fiat` | Token replay, timing analysis, key extraction |
| DoS | 6 | `p2p-node`, `opendex-transport` | Connection exhaustion, state bloat, resource starvation |
| Fiat | 7 | `fiat` | Payment spoofing, race conditions, reversals |
| Transport | 4 | `opendex-transport` | Envelope tampering, MessageStore bypass |

## Running Attack Scenarios

The red team agent executes scenarios by category:

```bash
# Run all protocol attack scenarios
cargo run --bin red-team-agent -- --category protocol

# Run a specific scenario
cargo run --bin red-team-agent -- --scenario PA-001

# Run all scenarios
cargo run --bin red-team-agent -- --all
```

## Reporting

After each test cycle, document findings using the report template:

```bash
# Copy template for new report
cp redteam/reports/template.md redteam/reports/2026-10-19-protocol-attacks.md

# Edit with findings
$EDITOR redteam/reports/2026-10-19-protocol-attacks.md
```

## Safety Guidelines

1. **Isolation**: Never run red team tests against production or mainnet nodes
2. **Key management**: Use separate test keys, never reuse production keys
3. **Resource limits**: DoS tests may crash nodes; ensure auto-restart is configured
4. **Network isolation**: All testnet nodes must be in isolated VPC
5. **Data cleanup**: Remove test data after each test cycle

## Monitoring

For real-time monitoring during tests:

1. **Prometheus**: Scrapes metrics from all nodes at `http://<monitoring-ip>:9090`
2. **Grafana**: Dashboards at `http://<monitoring-ip>:3000`
3. **Logs**: Aggregated via Vector/Loki at `http://<monitoring-ip>:3100`

## Related Documentation

- **Red Team Plan**: `audit/red_team_plan.md` — Complete action plan
- **Security Overview**: `audit/security.md` — Security baseline and controls
- **Threat Model**: `audit/threat_model.md` — STRIDE-based threat analysis
- **Testing Guide**: `audit/testing.md` — Test infrastructure overview

## Support

For questions or issues with the red team infrastructure, refer to the main project's issue tracker or contact the security team.
