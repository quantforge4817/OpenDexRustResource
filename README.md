<div align="center">

# OpenDex

### Peer-to-Peer Decentralized Exchange for Trustless Bitcoin & Monero Swaps

[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](LICENSE)
[![Security Audit](https://img.shields.io/badge/security%20audit-internal%20complete-green.svg)](audit/audit.md)
[![Tests](https://img.shields.io/badge/tests-380%2B%20passed-brightgreen.svg)](tests/)
[![CI](https://img.shields.io/badge/CI-GitHub%20Actions-brightgreen.svg)](.github/workflows/ci.yml)
[![Clippy](https://img.shields.io/badge/clippy-passing-brightgreen.svg)](.github/workflows/ci.yml)

</div>

OpenDex is an open-source, peer-to-peer decentralized exchange that enables trustless cross-chain swaps between **Bitcoin (BTC)** and **Monero (XMR)** without relying on any central authority or trusted third party.

Built entirely in Rust using the [libp2p](https://libp2p.io/) networking stack, OpenDex uses:

- **GossipSub** for order book synchronization
- **Kademlia DHT** for peer discovery
- **Noise protocol** for encrypted communications
- **Adaptor signatures** for atomic BTC ↔ XMR swaps

Every node maintains a replicated view of orders, trades, balances, and trade state. The architecture is designed around deterministic trade identity, idempotent state application, authenticated actors, and conservative snapshot synchronization.

---

## 🚀 Quick Start

### Prerequisites

- **Rust 1.88+** (stable or nightly)
- **Cargo** package manager

### Building

```bash
# Clone the repository
git clone https://github.com/quantforge4817/OpenDexRustResource.git
cd OpenDexRustResource

# Build the entire workspace
cargo build --workspace

# Run tests
cargo test --workspace
```

### Running the P2P Node

```bash
cargo run -p p2p-node
```

### Running the Desktop GUI

```bash
cargo run -p desktop-gui
```

### Building the Web Portal

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk

cd crates/web-frontend
trunk serve --open
```

---

## 📦 Project Structure

```
OpenDexRustResource/
├── Cargo.toml              # Workspace manifest (23 crates)
├── deny.toml               # cargo-deny configuration
├── crates/
│   ├── common/             # Shared domain types (Asset, Market, Side)
│   ├── protocol/           # Message types: Order, Trade, TradeState, Message
│   ├── p2p-node/           # P2P node: OrderBook, TradeBook, BalanceBook, Exchange
│   │
│   │   # Milestone 1: BTC-XMR atomic swap foundation
│   ├── opendex-types/
│   ├── opendex-errors/
│   ├── opendex-protocol-types/
│   ├── opendex-protocol/
│   ├── opendex-btc-xmr/
│   ├── opendex-bitcoin/
│   ├── opendex-monero/
│   ├── opendex-swap/
│   │
│   │   # Milestone 2: Persistence, watchers, recovery
│   ├── opendex-events/
│   ├── opendex-storage/
│   ├── opendex-watchers/
│   ├── opendex-recovery/
│   │
│   │   # Milestone 3: P2P transport, peer discovery, messaging
│   ├── opendex-transport/
│   ├── opendex-chain/
│   │
│   │   # Regulatory compliance
│   ├── opendex-dao/                # DAO governance interface
│   ├── opendex-fee-treasury/       # Fee & treasury subsystem
│   │
│   │   # User interfaces
│   ├── web-frontend/               # Leptos/WASM static portal
│   └── desktop-gui/                # iced native desktop app
│
├── fuzz/                   # cargo-fuzz targets (7 fuzz targets)
├── tests/                  # Integration tests
├── audit/                  # Security audit, compliance, bug log
└── docs/                   # Documentation
    ├── deployment/         # Web portal deployment scripts
    ├── matrix/             # Cloud-hosted Matrix server deployment
    └── testnet/            # Testnet deployment scripts and results
```

---

## 🔑 Key Features

| Feature | Description |
|---------|-------------|
| **Peer-to-Peer Network** | No central server — every node is equal. Connect directly to other traders via libp2p with encrypted, authenticated communication. |
| **Atomic Swaps** | Trustless BTC ↔ XMR swaps using cross-curve DLEQ proofs (Fiat-Shamir). Your funds are never at risk: either both parties receive their assets, or the swap is refunded. |
| **Decentralized Order Book** | Orders are broadcast across the P2P network using GossipSub. Every node maintains its own copy of the order book. |
| **Privacy-First** | Monero-native privacy. No KYC, no central logs, no tracking. Your trading activity stays between you and the network. |
| **Open Source** | Fully open-source codebase under MIT/Apache-2.0 licenses. Audit all components, run your own node, verify every transaction. |
| **Cross-Platform** | Native desktop app for Linux, macOS, and Windows. Run your own P2P node and trade from anywhere. |
| **CI/CD Pipeline** | Full GitHub Actions pipeline: build, test, clippy, cargo-deny, and formatting on every PR. |

---

## 🛡️ Security

OpenDex takes security seriously:

- **380+ tests passed**, 0 failed across the workspace (including E2E and multi-node tests)
- **cargo-deny** supply chain check: **0 RUSTSEC vulnerabilities**
- **12 fuzz targets** for protocol message parsing and state machine validation
- **Miri** undefined behavior detection: 146 tests, 0 UB
- **24 Kani formal proofs** (11 verified, 13 disabled)
- **133 penetration tests** across 7 attack phases
- **0 `unsafe` blocks** — all code is safe Rust; credential zeroization uses the `zeroize` crate
- No custom cryptographic primitives; uses standard `secp256k1`, `Ed25519`, `SHA-256`
- **Cross-curve DLEQ proofs** using Fiat-Shamir challenge-response (secp256k1 + Ed25519)
- **CI pipeline** enforces clippy, cargo-deny, and formatting on every PR

See [`audit/audit.md`](audit/audit.md) for the full security audit report and [`audit/audit_package/`](audit/audit_package/) for the external audit preparation package.

---

## 📋 Regulatory Compliance

OpenDex has been analyzed for compliance with:

- **EU MiCA** (Markets in Crypto-Assets Regulation)
- **UK FCA** (Financial Conduct Authority) requirements

The non-custodial, decentralized architecture is rated **GREEN** under the project's regulatory compliance framework. See [`audit/regulatory_compliance/`](audit/regulatory_compliance/) for the full analysis.

---

## 🔧 CI/CD Pipeline

Every pull request and push to `main` triggers a full CI pipeline via GitHub Actions:

- **Format check** — `cargo fmt --check` enforces consistent code style
- **Linting** — `cargo clippy --workspace -- -D warnings` catches code quality issues
- **Build** — `cargo build --workspace` verifies compilation
- **Tests** — `cargo test --workspace` runs all 380+ unit, integration, E2E, and multi-node tests
- **Dependency audit** — `cargo deny check` validates licenses and dependency sources

See [`.github/workflows/ci.yml`](.github/workflows/ci.yml) for the full configuration.

---

## 💬 Community

Join the OpenDex community on **Matrix** to ask questions, discuss development, connect with contributors, and share ideas:

- **[Matrix Chat](https://matrix.to/#/#opendex:opendex.org)** — Community discussion and support

We also provide a cloud-hosted Matrix server deployment guide for those who want to run their own instance on a low-cost VPS (~$3.50–5/mo). See **[docs/matrix/](docs/matrix/)** for details.

## 📖 Documentation

- **[Roadmap](docs/ROADMAP.md)** — Development milestones and future plans
- **[Architecture](audit/architecture.md)** — System architecture overview
- **[Protocol](audit/protocol.md)** — Protocol specification
- **[Security](audit/security.md)** — Security analysis and recommendations
- **[Testing](audit/testing.md)** — Testing strategy and coverage
- **[Testnet](docs/testnet/README.md)** — Testnet deployment guide
- **[Deployment](docs/deployment/README.md)** — Web portal deployment guide
- **[Matrix Server](docs/matrix/README.md)** — Cloud-hosted Matrix server setup

---

## 🧪 Testnet

A 3-node testnet is available for development and testing:

- 1 bootstrap node + 2 test nodes
- Monero stagenet + Bitcoin testnet
- Ubuntu 24.04 VPS deployment scripts

See [`docs/testnet/`](docs/testnet/) for setup instructions and test results.

---

## 🤝 Contributing

We welcome contributions! Please see our contributing guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes (`git commit -m 'Add my feature'`)
4. Push to the branch (`git push origin feature/my-feature`)
5. Open a Pull Request

Before submitting, please ensure:
- All tests pass: `cargo test --workspace`
- No compiler warnings: `cargo clippy --workspace`
- Supply chain is clean: `cargo deny check`

---

## 📄 License

This project is dual-licensed under **MIT** or **Apache-2.0** at your option.

See the [LICENSE](LICENSE) file for details.

---

## 🔍 Seeking Independent Review & Security Audit

Contributions and critical feedback from Bitcoin, Monero, Rust, and cryptographic protocol developers are welcome. We are actively seeking **independent security review** of the atomic swap protocol implementation, as well as general code review and testing.

Please report suspected vulnerabilities **privately** using the repository's [security reporting instructions](.github/SECURITY.md) rather than publishing exploitable details immediately.

The goal is to establish what works, identify weaknesses, and improve the design before any consideration of production deployment.

### Review Focus Areas

We are asking reviewers to focus first on:

1. **Atomic-swap correctness** — Verify that the BTC ↔ XMR swap protocol guarantees atomicity under all conditions.
2. **Refund safety** — Ensure that funds are always recoverable if a swap fails or a counterparty disappears.
3. **Adaptor-signature assumptions** — Validate the cryptographic assumptions and their implementation.
4. **Failure recovery** — Test that the system handles network partitions, node crashes, and unexpected state transitions gracefully.

### Testing on Test Networks

Developers are invited to test on **Bitcoin testnet** and **Monero stagenet only**. Before testing, please verify that the test setup cannot accidentally use mainnet funds. See [`docs/testnet/`](docs/testnet/) for the testnet deployment guide and configuration.

**Do not use mainnet funds.** This project is not yet ready for production deployment.

### Security Status

> **Important:** Do not advertise or treat this exchange as safe or ready for trading until the relevant security work has been completed. Independent audit and thorough testing are required before any production use.

---

## ⚠️ Disclaimer

OpenDex is experimental software under active development. Use at your own risk. The developers are not liable for any losses incurred through the use of this software. Always verify smart contracts and cryptographic operations independently.

---

<div align="center">

**Built with ❤️ in Rust**

[Report Bug](https://github.com/quantforge4817/OpenDexRustResource/issues/new) · [Request Feature](https://github.com/quantforge4817/OpenDexRustResource/issues/new) · [View Documentation](docs/)

</div>
# OpenDexRustResource
