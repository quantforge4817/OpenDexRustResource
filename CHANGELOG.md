# Changelog

All notable changes to the OpenDex project will be documented in this file.

## [Unreleased]

### Added
- Self-hosted Matrix homeserver deployment guide (`docs/matrix/`) — Conduit on cloud VPS (~$3.50–5/mo)
- Community chat link updated to `#opendex:opendex.org` (self-hosted Matrix server)
- Matrix server references added to project documentation (README, CONTRIBUTING, SECURITY, ROADMAP, deployment docs)

## [v0.1.0-rc1] — 2026-10-15

### Release Type
**Release Candidate 1** — First candidate for independent external security review.

### Commit
`43e6118` — feat: add Kani formal verification, fuzz targets, benchmarks, and release documentation

### Changes Since v0.1.0-security-review
- Added Kani formal verification: 24 proof harnesses across 4 components (BondState, FiatTradeState, OrderBook, RateLimiter)
- Added performance benchmarks: 4 Criterion benchmarks (order matching, message processing, settlement, auth)
- Added P2P network load tests: multi-node mesh formation, message flood, peer churn, snapshot sync
- Added 4 extended fuzz targets: fiat_trade, settlement_ops, auth_token, bond (total: 12)
- Added penetration testing suite: 133 tests across 7 phases (network, auth, crypto, financial, data integrity, fiat, DoS, supply chain)
- Fixed 8 testnet deployment bugs (stale snapshot crash, Monero RPC path, Bitcoin blockcount, etc.)
- Resolved all remaining audit findings (5 High, 6 Medium, 4 Low)
- Fixed rate limiter unbounded memory growth: added max_keys cap with LRU eviction (10,000 key limit)
- Fixed zero amount/price order validation: Exchange::submit_order now rejects zero values
- Updated rustls from v0.23.45 to v0.23.46 (RUSTSEC-2026-0346 through 0349)
- Added log redaction framework for sensitive data
- Added per-username login rate limiting (5/min)
- Added application-level P2P message validation (timestamp freshness, schema version)
- Added caller verification on all settlement engine methods
- Added HMAC-signed authentication tokens (replaced unsigned UUIDs)
- Added Argon2id password hashing (replaced DJB2)
- Added secret key zeroization on drop for escrow keys
- Added Bitcoin payout transaction miner fee output
- Added RwLock poison recovery macros
- Added HashMap error handling (ok_or_else instead of unwrap)
- Added crypto address validation
- Added bond amount calculation based on trade value
- Added payment detection via PaymentAdapter trait
- Upgraded deny.toml: duplicate versions now denied (was warn)

### Test Status
- **425 tests passed**, 0 failed (365 library/binary + 60 integration)
- **133 penetration tests** — all pass across 7 phases (network, auth, crypto, financial, data integrity, fiat, DoS, supply chain)
- **12 fuzz targets** — all smoke tests pass
- **24 Kani proofs** — 11 verified, 13 disabled (state space explosion)
- **4 Criterion benchmarks** — all compile
- **~59 Miri tests** — 0 undefined behavior
- `cargo-deny`: **0 RUSTSEC vulnerabilities** across 662 dependencies

### Git History Secrets Scan
- ✅ **No secrets found** — full history scanned for private keys, API keys, passwords, Monero addresses

### Security Notes
- 2 `unsafe` blocks total, both for password zeroization in `p2p-node`
- No custom cryptographic primitives; uses standard secp256k1, Ed25519, SHA-256
- DLEQ proof integration is a stub; full implementation pending Eigenwallet library

### Known Open Items
- External security audit pending
- CI/CD pipeline not yet configured
- Production monitoring infrastructure not yet configured
- Full BTC-XMR atomic swap transaction construction pending

See `RELEASE.md` for full feature status and test results.

---

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
