<div align="center">

# OpenDex

### Peer-to-Peer Decentralized Exchange for Trustless Bitcoin & Monero Swaps

[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](LICENSE)
[![Security Audit](https://img.shields.io/badge/security%20audit-in%20progress-yellow.svg)](audit/audit.md)
[![Tests](https://img.shields.io/badge/tests-480%20passed-brightgreen.svg)](tests/)

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
git clone https://github.com/your-org/OpenDexRustSource.git
cd OpenDexRustSource

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
OpenDexRustSource/
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
    └── testnet/            # Testnet deployment scripts and results
```

---

## 🔑 Key Features

| Feature | Description |
|---------|-------------|
| **Peer-to-Peer Network** | No central server — every node is equal. Connect directly to other traders via libp2p with encrypted, authenticated communication. |
| **Atomic Swaps** | Trustless BTC ↔ XMR swaps using adaptor signatures. Your funds are never at risk: either both parties receive their assets, or the swap is refunded. |
| **Decentralized Order Book** | Orders are broadcast across the P2P network using GossipSub. Every node maintains its own copy of the order book. |
| **Privacy-First** | Monero-native privacy. No KYC, no central logs, no tracking. Your trading activity stays between you and the network. |
| **Open Source** | Fully open-source codebase under MIT/Apache-2.0 licenses. Audit all components, run your own node, verify every transaction. |
| **Cross-Platform** | Native desktop app for Linux, macOS, and Windows. Run your own P2P node and trade from anywhere. |

---

## 🛡️ Security

OpenDex takes security seriously:

- **480 tests passed**, 0 failed across the workspace
- **cargo-deny** supply chain check: **0 RUSTSEC vulnerabilities**
- **7 fuzz targets** for protocol message parsing and state machine validation
- **Miri** undefined behavior detection
- **2 `unsafe` blocks** total, both for password zeroization
- No custom cryptographic primitives; uses standard `secp256k1`, `Ed25519`, `SHA-256`

See [`audit/audit.md`](audit/audit.md) for the full security audit report and [`audit/threat_model.md`](audit/threat_model.md) for the threat model analysis.

---

## 📋 Regulatory Compliance

OpenDex has been analyzed for compliance with:

- **EU MiCA** (Markets in Crypto-Assets Regulation)
- **UK FCA** (Financial Conduct Authority) requirements

The non-custodial, decentralized architecture is rated **GREEN** under the project's regulatory compliance framework. See [`audit/regulatory_compliance/`](audit/regulatory_compliance/) for the full analysis.

---

## 📖 Documentation

- **[Roadmap](docs/ROADMAP.md)** — Development milestones and future plans
- **[Architecture](audit/architecture.md)** — System architecture overview
- **[Protocol](audit/protocol.md)** — Protocol specification
- **[Security](audit/security.md)** — Security analysis and recommendations
- **[Testing](audit/testing.md)** — Testing strategy and coverage
- **[Testnet](docs/testnet/README.md)** — Testnet deployment guide
- **[Deployment](docs/deployment/README.md)** — Web portal deployment guide

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

## ⚠️ Disclaimer

OpenDex is experimental software under active development. Use at your own risk. The developers are not liable for any losses incurred through the use of this software. Always verify smart contracts and cryptographic operations independently.

---

<div align="center">

**Built with ❤️ in Rust**

[Report Bug](https://github.com/your-org/OpenDexRustSource/issues/new) · [Request Feature](https://github.com/your-org/OpenDexRustSource/issues/new) · [View Documentation](docs/)

</div>
# OpenDexRustResource
