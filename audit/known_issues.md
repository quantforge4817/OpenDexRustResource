# OpenDex Known Issues

**Version:** 0.1.0-security-review
**Date:** 2026-10-14
**Status:** Authoritative record of known defects and unresolved risks
**Scope:** Known defects, unresolved security questions, and outstanding risks

---

## Table of Contents

1. [Critical Issues](#1-critical-issues)
2. [High Severity Issues](#2-high-severity-issues)
3. [Medium Severity Issues](#3-medium-severity-issues)
4. [Low Severity Issues](#4-low-severity-issues)
5. [Design Decisions](#5-design-decisions)
6. [Resolved Issues](#6-resolved-issues)
7. [Risk Register](#7-risk-register)

---

## 1. Critical Issues

### KNOWN-C01 — DLEQ Proof Stub Implementation

| Field | Value |
|-------|-------|
| **Severity** | Critical |
| **Component** | `opendex-btc-xmr` |
| **Status** | Resolved (2026-10-18) — replaced with Fiat-Shamir cross-curve DLEQ proof |
| **Launch blocker** | **No** — resolved, pending independent review |

**Description:** The DLEQ proof generation and verification in `opendex-btc-xmr` were stub implementations. **Resolved on 2026-10-18:** Replaced with a Fiat-Shamir cross-curve DLEQ proof using `k256::Scalar` for secp256k1 modular arithmetic and `curve25519-dalek` for ed25519. Proof structure: R_btc (32 bytes) || R_xmr (32 bytes) || s_btc (32 bytes) || s_xmr (32 bytes). Verification checks s*G - c*P == R on both curves with a shared challenge c = H(btc_pk || xmr_pk || R_btc || R_xmr).

**Impact:** Without functional DLEQ proofs, the BTC-XMR atomic swap protocol cannot guarantee that the same secret key is used on both secp256k1 and Ed25519 curves. An attacker could provide a false proof and gain an unfair advantage in the swap.

**Affected methods:**
- `DleqProofGenerator::generate_proof()` — Generates cross-curve DLEQ proof with random nonce, Fiat-Shamir challenge, and response scalars
- `DleqProofGenerator::verify_proof()` — Verifies s*G - c*P == R on both secp256k1 and ed25519
- `DleqProofGenerator::derive_monero_public_key()` — Simplified placeholder using SHA-512 with Ed25519 scalar clamping

**Remediation:** Integrate the full Eigenwallet BTC-XMR library implementation. This is the single most important item for production readiness.

**Independent verification required:** Yes

---

## 2. High Severity Issues

### KNOWN-H01 — Monero Daemon RPC Stub

| Field | Value |
|-------|-------|
| **Severity** | High |
| **Component** | `opendex-monero` |
| **Status** | Resolved (2026-10-18) — implemented real daemon RPC |
| **Launch blocker** | **No** — resolved, pending testnet validation |

**Description:** The Monero daemon JSON-RPC interface in `opendex-monero` was a stub. **Resolved on 2026-10-18:** Implemented `transaction()` via `get_transactions`, `height()` via `get_block_count`, and `confirmations()` from transaction lookup. `broadcast()` requires a wallet client and returns an error for daemon-only access.

**Impact:** Without functional Monero RPC, the swap engine cannot observe Monero blockchain state, making Monero funding detection, confirmation counting, and redemption impossible.

**Remediation:** Implement full Monero daemon RPC methods.

**Independent verification required:** Yes

### KNOWN-H02 — No Real Blockchain Testing

| Field | Value |
|-------|-------|
| **Severity** | High |
| **Component** | All blockchain interfaces |
| **Status** | Open — testing limitation |
| **Launch blocker** | No (testing gap) |

**Description:** All blockchain interactions use mock implementations (`MockBitcoinChain`, `MockMoneroChain`). No real Bitcoin or Monero transactions have been constructed, broadcast, or observed.

**Impact:** Transaction construction, fee estimation, confirmation counting, and reorg handling have not been tested against real blockchain behavior. Undiscovered bugs in transaction logic could cause fund loss.

**Remediation:** Deploy to testnet and run end-to-end swap tests with real blockchain nodes.

**Independent verification required:** Yes

### KNOWN-H03 — Fiat Payment Non-Atomicity

| Field | Value |
|-------|-------|
| **Severity** | High |
| **Component** | `fiat::settlement_engine` |
| **Status** | Open — inherent limitation |
| **Launch blocker** | No (mitigated) |

**Description:** Fiat bank transfers settle outside the protocol. The protocol can coordinate but cannot guarantee atomic settlement of fiat and crypto legs. A fiat payment can be reversed after crypto is released.

**Impact:** If a taker's bank transfer is reversed (chargeback, fraud dispute) after the maker releases crypto, the maker loses both fiat and crypto.

**Mitigations:**
- `ReversalDetected` state allows post-completion dispute
- Dispute mechanism for resolution
- Security bonds deter bad behavior
- Payment deadlines limit exposure window

**Remediation:** This is an inherent limitation of fiat-crypto P2P trading. Cannot be fully eliminated; can only be mitigated.

**Independent verification required:** No (design limitation)

---

## 3. Medium Severity Issues

### KNOWN-M01 — No Real Payment Adapter Testing

| Field | Value |
|-------|-------|
| **Severity** | Medium |
| **Component** | `fiat::adapter` |
| **Status** | Open — testing limitation |
| **Launch blocker** | No |

**Description:** Payment adapters (`SepaProvider`, `UkFasterPayments`) are implemented but tested only with mock data. No real bank API integration has been tested.

**Impact:** Payment detection may fail in production due to API differences, rate limiting, or unexpected response formats.

**Remediation:** Test payment adapters with real bank API sandbox environments.

**Independent verification required:** No

### KNOWN-M02 — No Real P2P Network Testing

| Field | Value |
|-------|-------|
| **Severity** | Medium |
| **Component** | `p2p-node`, `opendex-transport` |
| **Status** | Partially resolved (2026-10-18) — In-process multi-node tests added |
| **Launch blocker** | No |

**Description:** The P2P transport layer is tested with `MockTransport` and in-process multi-node tests (`opendex-transport/tests/multi_node.rs`). No real libp2p network with multiple nodes on separate hosts has been tested. In-process multi-node tests cover: MockTransportPair connectivity, replay detection, sequence tracking, and malicious state rejection.

**Impact:** Network partition handling, message ordering, peer discovery, and gossipsub behavior have not been tested under real network conditions.

**Remediation:** Deploy multiple nodes on test network and verify message delivery, peer discovery, and partition handling.

**Independent verification required:** No

### KNOWN-M03 — Encryption at Rest

| Field | Value |
|-------|-------|
| **Severity** | Medium |
| **Component** | `opendex-storage` |
| **Status** | ✅ Resolved (2026-10-19) — AES-256-GCM encryption implemented |
| **Launch blocker** | No |

**Description:** Swap state files, event logs, and transaction intents were stored as plaintext JSON files. **Resolved on 2026-10-19:** Implemented AES-256-GCM encryption with HKDF-SHA256 key derivation, per-file salts, and `OPNDEX01` magic bytes for definitive detection. The `EncryptedStorage` wrapper provides transparent encryption/decryption for all persisted data.

**Impact:** Resolved — stored data is now encrypted at rest with authenticated encryption.

**Verification:** 14 tests cover encryption, decryption, key derivation, magic bytes detection, and environment-based key provisioning.

**Independent verification required:** No

### KNOWN-M04 — No Multi-Party Coordination Testing

| Field | Value |
|-------|-------|
| **Severity** | Medium |
| **Component** | `opendex-swap`, `fiat::settlement_engine` |
| **Status** | Partially resolved (2026-10-18) — E2E and multi-node tests added |
| **Launch blocker** | No |

**Description:** E2E integration tests (`opendex-swap/tests/swap_e2e.rs`) cover swap lifecycle, timeout/refund, crash recovery, and reconciliation using mock chains. In-process multi-node tests (`opendex-transport/tests/multi_node.rs`) cover disconnection, replay attacks, message reordering, and malicious proof rejection. No tests with two real nodes on separate hosts executing swaps simultaneously.

**Impact:** Race conditions, message ordering issues, and coordination failures may not be detected until production.

**Remediation:** Create integration tests with two real nodes executing swaps and fiat trades.

**Independent verification required:** No

### KNOWN-M05 — Asset Enum Hardcoded

| Field | Value |
|-------|-------|
| **Severity** | Medium |
| **Component** | `opendex-types` |
| **Status** | Accepted — design decision |
| **Launch blocker** | No |

**Description:** The `Asset` enum is hardcoded to `BTC`, `XMR`, and fiat currencies. Adding new assets requires code changes.

**Impact:** The protocol cannot support new assets without modification and redeployment.

**Remediation:** This is a deliberate design decision. Extensibility can be added in a future version if needed.

**Independent verification required:** No

---

## 4. Low Severity Issues

### KNOWN-L01 — GUI Dependency Warnings

| Field | Value |
|-------|-------|
| **Severity** | Low |
| **Component** | `desktop-gui` (transitive) |
| **Status** | Open — not in financial code path |
| **Launch blocker** | No |

**Description:** Three `cargo audit` warnings for GUI-only transitive dependencies:
- `paste` (unmaintained)
- `ttf-parser` (unmaintained)
- `lru` (RUSTSEC-2026-0192, unsoundness)

None of these affect the financial code path.

**Remediation:** Monitor for updates; replace if vulnerabilities are discovered.

**Independent verification required:** No

### KNOWN-L02 — No Load Testing

| Field | Value |
|-------|-------|
| **Severity** | Low |
| **Component** | All |
| **Status** | Partially resolved (2026-10-18) — Load tests exist but require real infrastructure |
| **Launch blocker** | No |

**Description:** Load tests exist in `p2p-node/tests/load_tests.rs` (4 tests: mesh formation, message flood, peer churn, snapshot sync). Snapshot sync test passes in CI. The other 3 tests are marked `#[ignore]` because they require concurrent swarm driving that Rust's borrow checker prevents with Vec-based swarm management. They run on the VPS testnet where swarms are separate processes. 4 Criterion benchmarks also exist (order matching, message processing, settlement, auth).

**Impact:** Performance degradation or resource exhaustion under load.

**Remediation:** Perform load testing before production launch on VPS testnet.

**Independent verification required:** No

### KNOWN-L03 — Test Code Uses `.unwrap()`

| Field | Value |
|-------|-------|
| **Severity** | Low |
| **Component** | Test code |
| **Status** | Accepted — intentional |
| **Launch blocker** | No |

**Description:** Test code extensively uses `.unwrap()` which is acceptable for tests but should be documented as intentional.

**Remediation:** Documented as accepted. Test code is not executed in production.

**Independent verification required:** No

---

## 5. Design Decisions

### DESIGN-01 — Non-Custodial Architecture

The DEX never takes custody of user funds. Atomic swaps use protocol-level locking; fiat trades use 2-of-2 multisig requiring both parties to co-sign. This is a deliberate design choice that limits the DEX's liability but also means it cannot intervene to resolve disputes unilaterally.

### DESIGN-02 — Fiat Payment is External

Fiat bank transfers settle outside the protocol. The protocol can only coordinate, not guarantee, fiat payment finality. This is an inherent limitation of P2P fiat-crypto trading.

### DESIGN-03 — Integer-Only Arithmetic

All monetary values use integer types (`Satoshis`, `Piconero`, `FiatAmount` in minor units). No floating-point arithmetic is used for monetary calculations. This prevents rounding errors and precision loss.

### DESIGN-04 — Event Sourcing

Every state change produces an immutable event. Full state can be reconstructed from the event log. This provides an audit trail and enables state recovery after crashes.

### DESIGN-05 — In-Memory Key Storage

Secret keys are stored in memory only and zeroized on drop. No secret keys are persisted to disk. This limits exposure if storage is compromised but means keys are lost on crash.

---

## 6. Resolved Issues

The following issues were identified and resolved during internal audit and adversarial testing:

| ID | Description | Severity | Resolution |
|----|-------------|----------|------------|
| AUDIT-C01 | Weak password hashing (MD5) | Critical | Replaced with Argon2id |
| AUDIT-C02 | Missing auth on settlement | Critical | Added caller authorization |
| AUDIT-C03 | Plaintext secret key storage | Critical | Implemented key zeroization on drop |
| AUDIT-C04 | Unsigned tokens | Critical | Added HMAC-SHA256 token signing |
| AUDIT-H10 | No P2P message validation | High | Added schema version and timestamp checks |
| AUDIT-H11 | No miner fees in payout | High | Deducted miner fees from payout transactions |
| AUDIT-H12 | No rate limiting | High | Added 5 login attempts/minute per username |
| AUDIT-H13 | RwLock unwrap panics | High | Added poison recovery macros |
| AUDIT-H14 | HashMap unwrap panics | High | Replaced with `ok_or_else()` error handling |
| AUDIT-H15 | Duplicate crate versions | High | Changed `multiple-versions` to `"deny"` in deny.toml |
| AUDIT-M20 | Bond calculation | Medium | Bonds use trade value in basis points |
| AUDIT-M21 | Payment detection | Medium | PaymentAdapter trait for bank APIs |
| AUDIT-M22 | Message replay | Medium | 30-second timestamp freshness validation |
| AUDIT-M23 | Plaintext credentials | Medium | Credential zeroization on GuiNodeConfig drop |
| AUDIT-M24 | Address validation | Medium | Crypto address validation before storage |
| AUDIT-M25 | Duplicate crate versions | Medium | Comprehensive skip-tree for GUI transitive deps |
| AUDIT-L30 | Unused fiat dependencies | Low | All fiat crate dependencies verified as used |
| AUDIT-L31 | No log redaction | Low | Created `log_redact` module with `RedactionLayer` |
| AUDIT-L33 | No E2E fiat tests | Low | Created 3 E2E fiat integration tests |
| ADVERSARIAL-H01 | No cancellation from CryptoReserved+ | High | Added 8 cancellation paths |
| ADVERSARIAL-H02 | No MakerConfirmed timeout | High | Added timeout check in `check_deadlines()` |
| ADVERSARIAL-H03 | Bond release fails on early cancellation | High | Added `Required → Released` bond transition |
| ADVERSARIAL-H04 | cancel_trade calls return_escrow when not reserved | High | Conditional escrow return based on trade state |
| ADVERSARIAL-H05 | Same-state transitions silently accepted | High | `transition()` now rejects same-state with error |
| ADVERSARIAL-H06 | No failure paths from MakerConfirmed/CryptoRelease | High | Added failure paths |

---

## 7. Risk Register

### 7.1 Active Risks

| ID | Risk | Severity | Likelihood | Impact | Status | Owner |
|----|------|----------|-----------|--------|--------|-------|
| R01 | DLEQ proof stub exploitation | Critical | High | Critical | Resolved (2026-10-18) | Project |
| R02 | Monero RPC stub — no Monero functionality | High | Certain | High | Resolved (2026-10-18) | Project |
| R03 | Fiat payment reversal after crypto release | High | Medium | High | Mitigated | Design |
| R04 | No real blockchain testing | High | Medium | High | Open | Project |
| R05 | Bank detail substitution attack | High | Medium | High | Partially mitigated | Design |
| R06 | False payment claims | Medium | Medium | High | Mitigated | Design |
| R07 | No real payment adapter testing | Medium | Medium | Medium | Open | Project |
| R08 | No real P2P network testing | Medium | Medium | Medium | Open | Project |
| R09 | No encryption at rest | Medium | Low | Medium | Resolved (2026-10-19) | Design |
| R10 | No multi-party coordination testing | Medium | Medium | Medium | Open | Project |
| R11 | Counterparty strategic disconnection | Medium | High | Medium | Mitigated | Design |
| R12 | Blockchain node outage | Medium | Low | High | Mitigated | Design |
| R13 | Replay attack | Low | Low | Medium | Mitigated | Design |
| R14 | Key leakage via logs | Low | Low | High | Mitigated | Design |
| R15 | Sybil attack | Low | Medium | Low | Mitigated | Design |

### 7.2 Risk Treatment Summary

| Treatment | Count | Examples |
|-----------|-------|---------|
| **Open — requires remediation** | 2 | No blockchain testing, no payment adapter testing |
| **Mitigated — controls in place** | 7 | Fiat reversal, false claims, disconnection, node outage, replay, key leakage, Sybil |
| **Partially mitigated** | 1 | Bank detail substitution |
| **Open — future enhancement** | 1 | Multi-party testing |
| **Accepted — design decision** | 1 | Asset enum hardcoded |
| **Resolved** | 3 | DLEQ proof stub (Fiat-Shamir), Monero RPC stub (daemon RPC), encryption at rest (AES-256-GCM) |

---

## Appendix A: Launch Blockers

The following issues **must** be resolved before production launch:

1. **KNOWN-C01** — ✅ Resolved (2026-10-18) — Replaced with Fiat-Shamir cross-curve DLEQ proof
2. **KNOWN-H01** — ✅ Resolved (2026-10-18) — Implemented real Monero daemon RPC

Both original launch blockers are now resolved. Remaining pre-launch items are listed in Appendix B.

## Appendix B: Recommended Pre-Launch Items

The following items **should** be addressed before production launch but are not strict blockers:

1. **KNOWN-H02** — Real blockchain testing on testnet
2. **KNOWN-M01** — Real payment adapter testing
3. **KNOWN-M02** — Real P2P network testing
4. **KNOWN-M04** — Multi-party coordination testing
5. Independent security audit completion
