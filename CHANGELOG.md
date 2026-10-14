# Changelog

All notable changes to the OpenDex project will be documented in this file.

## [v0.1.0-security-review] — 2026-10-14

### Release Type
**Security Review Baseline** — Frozen snapshot for independent external security audit.

### Commit
`512f4cd` — chore: prepare v0.1.0-security-review baseline snapshot

### Scope
23 Rust crates, 110 source files, ~32,400 lines of Rust code covering:
- BTC-XMR atomic swap protocol with DLEQ proofs
- Fiat settlement with Bitcoin 2-of-2 multisig escrow
- P2P network layer with event sourcing and state machine enforcement
- User authentication, reputation, and DAO governance

### Crates Included

| Crate | Purpose |
|-------|---------|
| `common` | Shared utilities and types |
| `desktop-gui` | Desktop GUI application (Iced) |
| `opendex-bitcoin` | Bitcoin chain operations |
| `opendex-btc-xmr` | BTC-XMR adapter with DLEQ proof integration |
| `opendex-chain` | Chain abstraction layer |
| `opendex-dao` | DAO governance logic |
| `opendex-errors` | Error handling |
| `opendex-events` | Event system |
| `opendex-fee-treasury` | Fee calculation and treasury isolation |
| `opendex-fiat` | Fiat settlement engine |
| `opendex-monero` | Monero chain operations |
| `opendex-protocol` | Protocol message types and state machine |
| `opendex-protocol-types` | Protocol message types |
| `opendex-recovery` | Recovery mechanisms |
| `opendex-storage` | Persistent state storage |
| `opendex-swap` | Swap orchestration logic |
| `opendex-transport` | P2P transport layer |
| `opendex-types` | Core type definitions |
| `opendex-user` | User management |
| `opendex-watchers` | Chain watchers |
| `p2p-node` | P2P node application |
| `protocol` | Protocol implementation |
| `web-frontend` | Web frontend (Yew) |

### Changes Since Last Commit
- Integrated real external dependencies (`dleq`, `secp256k1`, `bitcoin`, `sha2`)
- Completed audit cleanup across all 23 crates
- Updated fiat settlement engine with Bitcoin multisig escrow
- Added Monero wallet RPC systemd service configuration
- Updated internal audit documentation (AUDIT.md, SECURITY_LOG.md, AUDIT_CHECKLIST.md)

### Test Status
- **480 tests passed**, 0 failed, 2 ignored
- `cargo-deny` supply chain check: **0 RUSTSEC vulnerabilities**
- All crates compile cleanly with no warnings on test targets

### Known Open Findings
- 5 High-severity findings (AUDIT-H11 through AUDIT-H15)
- 6 Medium-severity findings (AUDIT-M21 through AUDIT-M26)
- 4 Low-severity findings (AUDIT-L01 through AUDIT-L04)
- See `audit/audit.md` and `audit/security.md` for details

### Security Notes
- 2 `unsafe` blocks total, both for password zeroization in `p2p-node`
- No custom cryptographic primitives; uses standard secp256k1, Ed25519, SHA-256
- DLEQ proof integration is a stub; full implementation pending Eigenwallet library integration
- Cryptographic adapter specification is documented in `audit/BTC_XMR_financial_transaction/`
