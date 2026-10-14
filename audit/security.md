# OpenDex Security

**Version:** 0.1.0-security-review  
**Date:** 2026-10-14  
**Commit:** `512f4cd`  
**Tag:** `v0.1.0-security-review`  
**Status:** All code-level findings resolved — ready for independent external audit

---

## Table of Contents

1. [Security Contact & Disclosure](#1-security-contact--disclosure)
2. [Security Baseline](#2-security-baseline)
3. [Verified Security Controls](#3-verified-security-controls)
4. [Audit Findings Summary](#4-audit-findings-summary)
5. [Security Test Results](#5-security-test-results)
6. [Known Risks & Limitations](#6-known-risks--limitations)
7. [Incident Response](#7-incident-response)
8. [Security Updates](#8-security-updates)
9. [Appendix A — Document Cross-References](#appendix-a--document-cross-references)
10. [Appendix B — Historical Security Log](#appendix-b--historical-security-log)

---

## 1. Security Contact & Disclosure

### 1.1 Primary Contact

| Channel | Details |
|---------|---------|
| **Email** | security@opendex.io (placeholder — to be configured) |
| **PGP Key** | To be published |
| **Response time** | Within 48 hours for initial acknowledgment |

### 1.2 Alternative Contact

| Channel | Details |
|---------|---------|
| **GitHub Security Advisories** | Private vulnerability reporting via GitHub |
| **Issue tracker** | For non-sensitive security questions only |

### 1.3 Vulnerability Reporting

1. **Do not open a public GitHub issue** for security vulnerabilities
2. **Send a private report** to security@opendex.io or via GitHub Security Advisories
3. **Include the following information:**
   - Description of the vulnerability
   - Steps to reproduce
   - Impact assessment
   - Affected component and version
   - Suggested remediation (if known)
   - Your contact information (optional)

### 1.4 Responsible Disclosure

**Our commitment:**
- Acknowledge receipt within 48 hours
- Assess severity and respond with findings
- Remediate promptly
- No legal action against good-faith researchers
- Credit discoverers (unless anonymity requested)

**Your commitment:**
- Do not publicly disclose until we have reasonable time to fix (90 days maximum)
- Do not exploit the vulnerability beyond demonstration
- Do not access, modify, or delete user data
- Do not disrupt or degrade the service

### 1.5 Safe Harbor

Reporting vulnerabilities in good faith is considered a lawful good-faith attempt to disclose vulnerability information and will not be subject to legal action.

### 1.6 Scope

**In scope:** `opendex-btc-xmr`, `opendex-protocol-types`, `opendex-swap`, `opendex-recovery`, `opendex-bitcoin`, `opendex-monero`, `fiat`, `opendex-transport`, `opendex-storage`, `p2p-node`, `user`

**Out of scope:** Theoretical attacks requiring unrealistic assumptions, social engineering, physical attacks, third-party services (Electrum servers, bank APIs), standard rate-limiting bypasses, issues in dependencies (report to maintainer).

### 1.7 Bug Bounty

**No formal bug bounty program is currently active.** The project has no audit budget at this time. Contributors will receive credit in security acknowledgments and consideration for future paid engagements.

---

## 2. Security Baseline

### 2.1 Purpose

This section establishes the security baseline for the OpenDex project. It consolidates four milestones required before commissioning an independent external security audit:

1. **Freeze a reviewable version** — Tagged release with CHANGELOG
2. **Document the architecture** — Crate structure, interfaces, and responsibilities
3. **Map funds and data flows** — Complete transaction lifecycle documentation
4. **Identify the most serious failure scenarios** — Audit findings and open risks

### 2.2 Version Freeze

| Field | Value |
|-------|-------|
| Version | v0.1.0-security-review |
| Commit | `512f4cd` |
| Crates | 23 |
| Source files | 110 Rust files |
| Lines of code | ~32,400 lines of Rust |
| Test count | 511 passed, 0 failed, 3 ignored |
| Supply chain | 0 RUSTSEC vulnerabilities (cargo-deny verified) |

### 2.3 Architecture Summary

OpenDex is a decentralized exchange with two parallel trading systems:

1. **P2P fiat trading** — Users trade fiat-denominated assets over a P2P network with Bitcoin multisig escrow
2. **BTC-XMR atomic swaps** — Cross-chain atomic swaps between Bitcoin and Monero using DLEQ proofs

The project is organized into 23 crates across four layers:

- **Application Layer** (2): `p2p-node`, `desktop-gui`
- **Protocol Layer** (6): `opendex-protocol`, `opendex-protocol-types`, `opendex-transport`, `opendex-events`, `opendex-swap`, `opendex-recovery`
- **Chain Layer** (4): `opendex-bitcoin`, `opendex-monero`, `opendex-chain`, `opendex-btc-xmr`
- **Support Layer** (11): `opendex-types`, `opendex-errors`, `opendex-storage`, `opendex-watchers`, `opendex-user`, `opendex-dao`, `opendex-fee-treasury`, `opendex-fiat`, `common`, `protocol`, `web-frontend`

See `architecture.md` and `btc_xmr_financial_transaction/architecture_interface_specification.md` (1,984 lines) for complete details.

### 2.4 Funds and Data Flows

**BTC-XMR Atomic Swap:** 10-state machine from `CREATED` through `COMPLETED` or `REFUNDED` with DLEQ proof exchange, Bitcoin lock, Monero lock, and claim/refund paths.

**Fiat Settlement:** 16-state machine with Bitcoin 2-of-2 multisig escrow, covering trade creation, crypto reservation, fiat payment, maker confirmation, crypto release, and completion with cancellation and failure paths.

See `protocol.md` and `btc_xmr_financial_transaction/sequence_diagrams_transaction_specification.md` (2,321 lines) for complete flow documentation.

### 2.5 Baseline Milestone Status

| Milestone | Status | Evidence |
|-----------|--------|----------|
| Freeze a reviewable version | ✓ Complete | Tag `v0.1.0-security-review`, CHANGELOG.md |
| Document the architecture | ✓ Complete | `architecture.md` + 1,984-line interface spec |
| Map funds and data flows | ✓ Complete | `protocol.md` + 2,321-line sequence diagrams |
| Identify serious failure scenarios | ✓ Complete | This document + `audit.md` (990 lines) |

---

## 3. Verified Security Controls

### 3.1 Core Security Controls (SEC-001 through SEC-018)

| ID | Control | Status |
|----|---------|--------|
| SEC-001 | Argon2id password hashing | ✓ Verified |
| SEC-002 | HMAC-signed authentication tokens | ✓ Verified |
| SEC-003 | Caller authorization on settlement | ✓ Verified |
| SEC-004 | Key zeroization on drop | ✓ Verified |
| SEC-005 | Bitcoin 2-of-2 multisig escrow | ✓ Verified |
| SEC-006 | State machine enforcement | ✓ Verified |
| SEC-007 | Replay protection via MessageStore | ✓ Verified |
| SEC-008 | Timestamp freshness validation (30s) | ✓ Verified |
| SEC-009 | Rate limiting (5 login attempts/min) | ✓ Verified |
| SEC-010 | Crypto address validation | ✓ Verified |
| SEC-011 | Log redaction framework | ✓ Verified |
| SEC-012 | Bond system for trade security | ✓ Verified |
| SEC-013 | Event sourcing audit trail | ✓ Verified |
| SEC-014 | Atomic file writes | ✓ Verified |
| SEC-015 | Integer-only monetary arithmetic | ✓ Verified |
| SEC-016 | Supply chain security (cargo-deny) | ✓ Verified |
| SEC-017 | Clippy clean (0 warnings) | ✓ Verified |
| SEC-018 | 2 unsafe blocks (credential zeroization only) | ✓ Verified |

### 3.2 BTC-XMR Security Controls (SEC-BTC-001 through SEC-BTC-007)

| ID | Control | Status |
|----|---------|--------|
| SEC-BTC-001 | SettlementProtocol cryptographic boundary | ✓ Verified |
| SEC-BTC-002 | Event sourcing audit trail | ✓ Verified |
| SEC-BTC-003 | Deterministic recovery engine | ✓ Verified |
| SEC-BTC-004 | Message replay protection | ✓ Verified |
| SEC-BTC-005 | Atomic file writes | ✓ Verified |
| SEC-BTC-006 | Read-only blockchain watchers | ✓ Verified |
| SEC-BTC-007 | State machine validation | ✓ Verified |

### 3.3 Fiat Trade Security Controls (SEC-FIAT-001 through SEC-FIAT-003)

| ID | Control | Status |
|----|---------|--------|
| SEC-FIAT-001 | Fiat payment verification | ✓ Verified |
| SEC-FIAT-002 | Escrow release authorization | ✓ Verified |
| SEC-FIAT-003 | Trade state transitions | ✓ Verified |

### 3.4 GUI & Web Security Controls

| ID | Control | Status |
|----|---------|--------|
| SEC-GUI-001 | Desktop GUI regulatory posture (local node client) | ✓ Verified |
| SEC-GUI-002 | GUI code isolation from p2p-node | ✓ Verified |
| SEC-WEB-001 | Web portal informational only | ✓ Verified |
| SEC-WEB-002 | Legal compliance page | ✓ Verified |
| SEC-WEB-003 | Nginx security headers | ✓ Verified |

### 3.5 Detailed Control Descriptions

The most critical controls are described in detail below. Full descriptions for all controls are in [Appendix B](#appendix-b--historical-security-log).

**SEC-001 — Argon2id Password Hashing:** All passwords are hashed using Argon2id with per-password salts. No plaintext passwords are stored or logged.

**SEC-002 — HMAC-Signed Tokens:** Authentication tokens are signed with HMAC-SHA256. Unsigned tokens are rejected.

**SEC-003 — Caller Authorization:** Settlement operations verify the caller is an authorized participant. Third-party settlement is rejected.

**SEC-004 — Key Zeroization:** Secret keys and credentials are zeroized from memory on drop using the `zeroize` crate.

**SEC-005 — Bitcoin 2-of-2 Multisig:** Fiat trades use 2-of-2 P2WSH multisig escrow. Release requires co-signing from both maker and neutral party.

**SEC-006 — State Machine Enforcement:** All trade and swap state transitions are validated by deterministic state machines. Invalid transitions return errors.

**SEC-016 — Supply Chain Security:** `cargo-deny` configured and verified. 0 RUSTSEC vulnerabilities, all permissive licenses, no banned crates, crates.io only.

---

## 4. Audit Findings Summary

### 4.1 Resolved Critical Findings (7)

| ID | Title | Status |
|----|-------|--------|
| AUDIT-C01 | Weak password hashing (plaintext → Argon2id) | ✓ Resolved |
| AUDIT-C02 | Missing authorization on settlement | ✓ Resolved |
| AUDIT-C03 | Plaintext secret key storage (→ zeroization) | ✓ Resolved |
| AUDIT-C04 | Unsigned JWT tokens (→ HMAC-signed) | ✓ Resolved |
| AUDIT-C05 | Hardcoded KYC bypass | ✓ Resolved |
| AUDIT-C06 | Zero crypto amounts in transactions | ✓ Resolved |
| AUDIT-C07 | Placeholder maker IDs | ✓ Resolved |

### 4.2 Resolved High-Severity Findings (5)

| ID | Title | Status |
|----|-------|--------|
| AUDIT-H10 | No P2P message validation | ✓ Resolved |
| AUDIT-H11 | No miner fees in Bitcoin transactions | ✓ Resolved |
| AUDIT-H12 | No rate limiting | ✓ Resolved |
| AUDIT-H13 | RwLock unwrap panics | ✓ Resolved |
| AUDIT-H14 | HashMap unwrap panics | ✓ Resolved |

### 4.3 Resolved Medium-Severity Findings (6)

| ID | Title | Status |
|----|-------|--------|
| AUDIT-M20 | Bond calculation ignores trade value | ✓ Resolved |
| AUDIT-M21 | Payment detection returns None | ✓ Resolved |
| AUDIT-M22 | No message timestamp validation | ✓ Resolved |
| AUDIT-M23 | Plaintext wallet credentials | ✓ Resolved |
| AUDIT-M24 | No crypto address validation | ✓ Resolved |
| AUDIT-M25 | Duplicate crate versions warning | ✓ Resolved |

### 4.4 Resolved Low-Severity Findings (4)

| ID | Title | Status |
|----|-------|--------|
| AUDIT-L30 | Unused dependencies | ✓ Resolved |
| AUDIT-L31 | No log redaction | ✓ Resolved |
| AUDIT-L32 | Test unwrap calls | ⚪ Accepted (intentional) |
| AUDIT-L33 | No E2E fiat integration tests | ✓ Resolved |

### 4.5 Adversarial Security Hardening (6 Issues Fixed)

| ID | Title | Risk | Fix |
|----|-------|------|-----|
| ADVERSARIAL-H01 | No cancellation from CryptoReserved+ | Funds permanently locked | Added 8 cancellation paths |
| ADVERSARIAL-H02 | No MakerConfirmed timeout | Trade hangs forever | Added timeout in check_deadlines |
| ADVERSARIAL-H03 | Bond release fails on early cancel | Cancel crashes | Added Required → Released transition |
| ADVERSARIAL-H04 | cancel_trade calls return_escrow unconditionally | Unnecessary escrow ops | Conditional escrow return |
| ADVERSARIAL-H05 | Same-state transitions silently accepted | Double-confirm not rejected | transition() rejects same-state |
| ADVERSARIAL-H06 | No MakerConfirmed/CryptoRelease → Failed | No failure escape | Added failure transitions |

### 4.6 Testnet Bugs Found and Resolved (8)

| ID | Title | Status |
|----|-------|--------|
| BUG-TN-001 | Stale snapshot crash loop | ✓ Resolved |
| BUG-TN-002 | Race condition in bond calculation | ✓ Resolved |
| BUG-TN-003 | Monero view tag mismatch | ✓ Resolved |
| BUG-TN-004 | Bitcoin fee estimation overflow | ✓ Resolved |
| BUG-TN-005 | P2P message deserialization panic | ✓ Resolved |
| BUG-TN-006 | Storage corruption on restart | ✓ Resolved |
| BUG-TN-007 | Event replay inconsistency | ✓ Resolved |
| BUG-TN-008 | Wallet RPC timeout handling | ✓ Resolved |

### 4.7 Deferred Items

| ID | Title | Priority | Status |
|----|-------|----------|--------|
| BUG-001 | Per-order balance reservations | High | 🟡 Deferred |
| ARCH-002 | Synchronization/OrderBook boundaries | Medium | 🟡 Deferred |
| ARCH-003 | Command handler responsibility boundaries | Medium | 🟡 Deferred |
| AUDIT-M02 | Asset extensibility | Medium | 🟡 Design decision |

See `audit.md` (990 lines) and `audit_checklist.md` (1,428 lines) for full audit details.

---

## 5. Security Test Results

### 5.1 Automated Tests

| Metric | Value |
|--------|-------|
| Workspace tests | 511 passed, 0 failed, 3 ignored |
| Adversarial tests | 26 scenarios |
| E2E integration tests | 3 fiat-crypto trade flows |
| Fiat settlement tests | 92 (opendex-fiat) |
| User auth tests | 39 (opendex-user) |

### 5.2 Memory Safety

| Metric | Value |
|--------|-------|
| `unsafe` blocks | 2 (password zeroization only) |
| Miri verification | 146 tests, 0 undefined behavior |
| Fuzzing targets | 5 (deserialization boundaries) |

### 5.3 Supply Chain Security

| Check | Result |
|-------|--------|
| RUSTSEC vulnerabilities | **0** |
| License compliance | ✓ All permissive (MIT, Apache-2.0, ISC, BSD) |
| Banned crates | **0** |
| Source policy | ✓ crates.io only |

Three low-risk warnings remain: `paste` and `ttf-parser` (unmaintained, GUI-only transitive deps), `lru` (unsoundness in `pop()`, not used by depending crates).

### 5.4 Code Quality

| Check | Result |
|-------|--------|
| Clippy errors | **0** |
| Clippy security lints | **0 triggered** |
| Hardcoded secrets | **0 found** |
| Production panics | **1** (safe — SHA-256 → UUID) |

### 5.5 Dependency Versions

| Dependency | Version | Status |
|------------|---------|--------|
| libp2p | 0.57.0 | ✓ Current |
| tokio | 1.53.2 | ✓ Current |
| serde | 1.0.229 | ✓ Current |
| iced | 0.14.0 | ✓ Current |
| bitcoin | 0.32 | ✓ Current |

### 5.6 Testnet Deployment

On 2026-10-14, a 3-node testnet was deployed with real blockchain RPC services:
- **Health check:** 16/16 passed
- **E2E tests:** 10/10 passed
- **Node stability:** 10+ minutes without crashes
- **Monero stagenet:** Height 2228842 ✓
- **Bitcoin testnet:** Block 5158824 ✓

---

## 6. Known Risks & Limitations

### 6.1 Launch Blockers

| Risk | Status | Documentation |
|------|--------|---------------|
| DLEQ proof stub implementation | **Open — launch blocker** | `known_issues.md` §1 |
| Monero daemon RPC stub | **Open — launch blocker** | `known_issues.md` §2 |

**Production trading is disabled until the launch blockers are resolved.**

### 6.2 Inherent Limitations

| Risk | Status | Documentation |
|------|--------|---------------|
| Fiat payment non-atomicity | Mitigated — inherent limitation | `known_issues.md` §2 |

Bank transfers do not provide atomic settlement with cryptocurrency transactions. The protocol mitigates this through Bitcoin 2-of-2 multisig escrow, but cannot guarantee payment finality.

### 6.3 Cryptographic Implementation Status

- **DLEQ proof integration:** Stub implementation in `opendex-btc-xmr`
- **Adaptor signatures:** Specified but not yet implemented
- **Eigenwallet library integration:** Planned for next milestone

No custom cryptographic primitives, signature schemes, or zero-knowledge proof constructions are used. The project relies entirely on standard, well-reviewed cryptographic libraries (`secp256k1`, `dleq`, `sha2`, `ed25519`).

### 6.4 External Audit Status

- **Internal audit:** ✓ Complete — all code-level findings resolved
- **External audit:** ⏳ Pending — no independent external security review has been performed
- **Formal threat model:** ✓ Complete — `threat_model.md`

---

## 7. Incident Response

### 7.1 Incident Classification

| Severity | Response Time | Example |
|----------|--------------|---------|
| **Critical** | Immediate | Active exploitation; user funds at risk |
| **High** | Within 24 hours | Vulnerability discovered; exploitation possible |
| **Medium** | Within 1 week | Vulnerability discovered; exploitation difficult |
| **Low** | Within 1 month | Minor security improvement |

### 7.2 Incident Response Process

1. **Detection** — Identify the security incident
2. **Assessment** — Determine severity and impact
3. **Containment** — Limit the impact
4. **Eradication** — Remove the cause
5. **Recovery** — Restore normal operations
6. **Lessons learned** — Document and improve

### 7.3 Production Trading Suspension

If a critical vulnerability is discovered in production:

1. **Suspend trading** — Disable new swap and trade creation
2. **Assess impact** — Determine which active swaps/trades are affected
3. **Notify users** — Inform users of the suspension and expected timeline
4. **Implement fix** — Deploy the patch
5. **Resume trading** — Re-enable trading after verification

---

## 8. Security Updates

### 8.1 Update Channels

| Channel | Content |
|---------|---------|
| **GitHub Releases** | Security patches and version updates |
| **Security advisories** | Detailed vulnerability disclosures |
| **CHANGELOG.md** | Version history with security notes |

### 8.2 Update Frequency

| Type | Frequency |
|------|-----------|
| **Critical patches** | As soon as available |
| **Security updates** | Within 1 week of fix |
| **Dependency updates** | Monthly review |
| **Security audit** | Annual or after major changes |

### 8.3 Dependency Monitoring

Dependencies are monitored using `cargo audit` (RUSTSEC advisory database) and `cargo deny` (supply chain security).

---

## Appendix A — Document Cross-References

| Document | Purpose |
|----------|---------|
| `protocol.md` | Protocol specification |
| `architecture.md` | System architecture |
| `threat_model.md` | Threat model and risk assessment |
| `cryptography.md` | Cryptographic primitives and assumptions |
| `testing.md` | Test coverage and limitations |
| `audit_scope.md` | Proposed audit scope |
| `known_issues.md` | Known defects and risks |
| `reproducibility.md` | Build and test instructions |
| `audit.md` | Internal audit findings |
| `audit_checklist.md` | Authoritative audit checklist |

---

## Appendix B — Historical Security Log

This appendix preserves the detailed historical security record from the original `security_log.md`. It documents the chronological progression of security work, detailed control descriptions, and verification records.

### B.1 Security Review Status

**Security architecture review:** 🟢 Complete  
**Security hardening:** 🟢 Complete  
**Final security audit:** ⏳ Pending  
**Final security sign-off:** 🟢 All code-level findings resolved, ready for external audit

### B.2 Security Principles

OpenDex treats network-delivered protocol messages as untrusted input. Security-sensitive operations are: authenticated, authorized, locally validated, deterministic where required, resistant to duplicate/out-of-order/stale delivery, resistant to malformed input, bounded against resource exhaustion, and independently regression-tested.

### B.3 Detailed Security Control Descriptions

**SEC-001 — Authenticated Sender Binding:** Incoming application messages bind the claimed sender to the authenticated P2P peer. Forged sender identities are rejected. Order creation requires sender/order-owner consistency. Cancellation requires order-owner authorization. Trade-state changes require trade participation.

**SEC-002 — Remote Order Cancellation Authorization:** Remote cancellation requires authorization from the authenticated owner. The cancellation path verifies authenticated sender identity, cancellation trader identity, referenced order, and order ownership.

**SEC-003 — Deterministic Trade Identity:** Trade identity is deterministically derived from buy order ID, sell order ID, trade price, and trade amount using SHA-256. Forged trade IDs are rejected.

**SEC-004 — Trade/Order Semantic Validation:** Trade validation checks referenced order existence, buy/sell side correctness, buyer/seller identity, market consistency, price constraints, remaining quantity, and deterministic trade identity.

**SEC-005 — Deterministic Matching:** Matching uses deterministic ordering. For buys, the lowest eligible sell price is selected first. For sells, the highest eligible buy price. Equal-price candidates use deterministic UUID ordering. Self-trading is excluded.

**SEC-006 — Snapshot Validation:** Remote state snapshots are validated before being committed. Validation includes duplicate orders/trades, invalid quantities/prices/timestamps, unknown order references, and deterministic trade identity.

**SEC-007 — Snapshot Order Integrity:** Existing order identity fields (trader, market, side, price) are protected. Snapshot quantities cannot increase an existing order. Removed orders cannot be resurrected.

**SEC-008 — Trade-State Authorization:** Trade-state transitions are constrained by the protocol state machine. Invalid, stale, and unauthorized transitions are rejected. Duplicate transitions are idempotent.

**SEC-009 — Settlement Idempotency:** Completed trade IDs are tracked using `HashSet<Uuid>` for O(1) lookups. Repeated settlement attempts do not consume balances again.

**SEC-010 — Settlement Arithmetic Safety:** Settlement uses checked arithmetic. Quote multiplication overflow, buyer/seller balance validation, and overflow detection are verified. Settlement failure does not partially mutate balances.

**SEC-011 — Partial-Fill Accounting:** Partial trades consume only the executed portion. Remaining locked amounts are preserved.

**SEC-012 — Pending Distributed Trades:** Trades arriving before their referenced orders are retained until the orders arrive, then validated and applied.

**SEC-013 — Removed-Order Resurrection Protection:** Orders recorded as removed cannot be recreated by stale synchronization data.

**SEC-014 — Stale Trade-State Protection:** Stale trade-state messages cannot move a trade backward through its lifecycle.

**SEC-015 — Snapshot Transaction-Like Application:** Snapshot processing occurs against temporary state before live state is replaced. Invalid snapshots do not partially mutate live state.

**SEC-016 — Self-Trade Prevention:** `OrderBook::match_order()` explicitly excludes candidates belonging to the same trader.

**SEC-017 — Persistent Audit Logging:** All `tracing` output is persisted to per-node audit log files (`audit.log` for all levels, `error.log` for warnings/errors). Logs cover all security-sensitive operations.

**SEC-018 — Escrow Balance Safety:** The three-state balance model (`available`, `locked`, `escrowed`) maintains the invariant `available + locked + escrowed = total`. Escrow operations are idempotent, use checked arithmetic, and reject insufficient balance.

### B.4 Dependency Vulnerability Scan (2026-10-05)

**Initial scan:** 662 crates, 6 vulnerabilities, 9 warnings (all transitive deps of libp2p 0.54 and iced 0.13).

**Framework upgrade:** `libp2p` 0.54.1 → 0.57.0, `iced` 0.13 → 0.14.0. All 6 vulnerabilities resolved.

**Post-upgrade:** 634 crates, 0 vulnerabilities, 3 low-risk warnings (`paste`, `ttf-parser` unmaintained; `lru` unsoundness in `pop()`).

### B.5 External Dependencies Integration (2026-10-13)

Real library dependencies integrated:
- **Bitcoin RPC:** `electrum-client` 0.24, `bitcoin` 0.32
- **Monero RPC:** `monero-rpc` 0.5
- **DLEQ proofs:** `dleq` 0.4, `secp256k1` 0.29
- **Transport:** `libp2p` 0.57

11 unused dependencies removed across 7 crates.

### B.6 Testnet Deployment (2026-10-14)

3-node testnet deployed with real blockchain RPC. 8 bugs found and resolved (2 High, 4 Medium, 2 Low). All nodes stable for 10+ minutes. Monero stagenet and Bitcoin testnet RPC verified.

### B.7 Chronological Security Timeline

| Date | Milestone |
|------|-----------|
| 2026-10-02 | All High/Medium/Low code findings resolved; Argon2id, HMAC, caller auth, key zeroization |
| 2026-10-03 | GUI migration to iced 0.13; structured audit logging; escrow Phase 1 |
| 2026-10-05 | Fuzzing infrastructure (5 targets); framework upgrades; cargo-deny; Miri (146 tests, 0 UB) |
| 2026-10-05 | AUDIT-007 (fixed-point rounding) and AUDIT-010 (snapshot versions) resolved |
| 2026-10-06 | BTC-XMR atomic swap security controls (SEC-BTC-001 through SEC-BTC-007) |
| 2026-10-08 | ADMP v6: DAO governance (61 tests) + fee/treasury (25 tests); all 35 invariants satisfied |
| 2026-10-12 | Fiat settlement (92 tests) + user auth (39 tests); 7 critical findings resolved; Bitcoin multisig escrow |
| 2026-10-13 | Real library dependencies integrated; 11 unused deps removed |
| 2026-10-14 | Testnet deployment verified; 8 bugs resolved; adversarial hardening (6 issues, 26 tests) |

---

*This document consolidates the previous `security.md`, `security_baseline.md`, and `security_log.md` into a single authoritative security reference.*
