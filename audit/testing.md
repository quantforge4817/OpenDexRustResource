# OpenDex Testing Document

**Version:** 0.1.0-security-review
**Date:** 2026-10-14
**Status:** Authoritative testing reference
**Scope:** Automated tests, integration tests, fuzzing, adversarial scenarios, and testing limitations

---

## Table of Contents

1. [Test Overview](#1-test-overview)
2. [Unit Tests](#2-unit-tests)
3. [Integration Tests](#3-integration-tests)
4. [Adversarial Tests](#4-adversarial-tests)
5. [Fuzz Testing](#5-fuzz-testing)
6. [Security Checks](#6-security-checks)
7. [Test Coverage by Component](#7-test-coverage-by-component)
8. [Testing Limitations](#8-testing-limitations)
9. [Test Environment](#9-test-environment)

---

## 1. Test Overview

### 1.1 Test Statistics

| Metric | Value |
|--------|-------|
| Total tests | 511 passed, 0 failed, 3 ignored |
| Unit tests | ~480 across all 23 crates |
| Integration tests | 3 E2E fiat integration tests |
| Adversarial tests | 26 scenarios |
| Fuzz targets | 8 |
| Unsafe blocks | 2 (both credential zeroization) |
| RUSTSEC vulnerabilities | 0 (cargo-deny verified) |

### 1.2 Test Command

```bash
cargo test --workspace
```

**Result:** 511 passed, 0 failed, 3 ignored

### 1.3 Test Philosophy

- **State machines are tested exhaustively:** Every valid and invalid transition is tested
- **Adversarial scenarios are prioritized:** Tests focus on failure modes, not just happy paths
- **Fuzzing targets deserialization:** Untrusted input parsing is fuzz-tested
- **Integration tests verify end-to-end flows:** Full trade lifecycles are tested

---

## 2. Unit Tests

### 2.1 State Machine Tests

**Atomic Swap State Machine** (`opendex-protocol-types`)

| Test | Description |
|------|-------------|
| Valid transitions | Every permitted transition is tested |
| Invalid transitions | Every forbidden transition returns error |
| Recovery paths | All states can transition to `Recovering` |
| Failure paths | All active states can transition to `Failed` |
| Peer disconnection | `WaitingForPeer` transitions tested |

**Fiat Trade State Machine** (`fiat::trade`)

| Test | Description |
|------|-------------|
| Automatic lifecycle | `Created → Accepted → CryptoReserved → PaymentRequested → PaymentPending → Detected → Confirming → Confirmed → CryptoRelease → Complete` |
| Manual lifecycle | `Created → Accepted → CryptoReserved → PaymentRequested → PaymentSubmitted → MakerConfirmed → CryptoRelease → Complete` |
| Cancellation paths | All 10 active states can transition to `Cancelled` |
| Failure paths | Timeout states transition to `Failed` |
| Dispute paths | Most active states can transition to `Disputed` |
| Dispute resolution | `Disputed` can resolve to `Complete`, `Failed`, or `Cancelled` |
| Same-state rejection | Transitioning to the current state returns error |
| Reversal detection | `Complete → ReversalDetected → Disputed` |

### 2.2 Cryptographic Tests

| Test | Description |
|------|-------------|
| Key generation | secp256k1 key pairs generate valid keys |
| Key zeroization | `Drop` implementation zeros secret key bytes |
| Multisig script | 2-of-2 P2WSH script construction is valid |
| Partial signatures | Partial signatures can be combined |

### 2.3 Authentication Tests

| Test | Description |
|------|-------------|
| Password hashing | Argon2id produces consistent hashes |
| Token signing | HMAC-SHA256 tokens verify correctly |
| Token forgery | Forged tokens are rejected |
| Rate limiting | Excessive login attempts are rejected |

### 2.4 Storage Tests

| Test | Description |
|------|-------------|
| Atomic writes | Write-to-temp-then-rename pattern |
| State persistence | Swap state survives restart |
| Event log | Events can be replayed to reconstruct state |

### 2.5 Transport Tests

| Test | Description |
|------|-------------|
| Message sending | Messages are delivered via transport |
| Replay protection | Duplicate message IDs are rejected |
| Sequence ordering | Messages are processed in sequence order |
| Envelope validation | Malformed envelopes are rejected |

### 2.6 Bond Tests

| Test | Description |
|------|-------------|
| Bond calculation | Bonds are calculated as basis points of trade value |
| Bond states | `Pending → Locked → Required → Released` |
| Early cancellation | `Required → Released` transition works |
| Zero bond | Zero-value bond is handled |
| Maximum bond | Maximum-value bond is handled |

### 2.7 Address Validation Tests

| Test | Description |
|------|-------------|
| Valid BTC address | Bitcoin addresses pass validation |
| Valid XMR address | Monero addresses pass validation |
| Invalid BTC address | Invalid Bitcoin addresses are rejected |
| Invalid XMR address | Invalid Monero addresses are rejected |
| Empty address | Empty addresses are rejected |

---

## 3. Integration Tests

### 3.1 E2E Fiat Integration Tests

**File:** `crates/fiat/tests/e2e_fiat.rs`

| Test | Description |
|------|-------------|
| Complete fiat trade | Full lifecycle from offer to completion |
| Dispute resolution | Trade disputed and resolved |
| Cancellation flow | Trade cancelled with bond return |

### 3.2 Swap Engine Tests

| Test | Description |
|------|-------------|
| Swap creation | New swap is created and persisted |
| Message processing | Protocol messages trigger correct actions |
| Chain observation | Blockchain observations advance state |
| Recovery trigger | Failed swaps trigger recovery engine |

---

## 4. Adversarial Tests

**File:** `crates/fiat/tests/adversarial_tests.rs`

**26 adversarial scenarios covering:**

### 4.1 State Machine Violations

| Test | Scenario | Expected Result |
|------|----------|-----------------|
| Missing cancellation path | Cancel from `CryptoReserved` | Transition succeeds |
| Missing cancellation path | Cancel from `PaymentRequested` | Transition succeeds |
| Missing cancellation path | Cancel from `PaymentSubmitted` | Transition succeeds |
| Missing cancellation path | Cancel from `PaymentPending` | Transition succeeds |
| Missing cancellation path | Cancel from `Detected` | Transition succeeds |
| Missing cancellation path | Cancel from `Confirming` | Transition succeeds |
| Missing cancellation path | Cancel from `MakerConfirmed` | Transition succeeds |
| Missing cancellation path | Cancel from `Confirmed` | Transition succeeds |
| Same-state transition | Transition to current state | Error returned |

### 4.2 Unauthorized Access

| Test | Scenario | Expected Result |
|------|----------|-----------------|
| Third-party crypto release | Unauthorized party releases crypto | Rejected by 2-of-2 multisig |
| Third-party trade completion | Unauthorized party completes trade | State machine rejects |
| Third-party trade cancellation | Unauthorized party cancels trade | Authorization check |

### 4.3 Payment Amount Mismatch

| Test | Scenario | Expected Result |
|------|----------|-----------------|
| Too-low payment | Taker sends less than agreed | Amount verification fails |
| Too-high payment | Taker sends more than agreed | Amount verification fails |

### 4.4 Bond Calculation Edge Cases

| Test | Scenario | Expected Result |
|------|----------|-----------------|
| Zero bond | Bond calculated for zero-value trade | Handled correctly |
| Maximum bond | Bond calculated for maximum-value trade | No overflow |

### 4.5 Rate Limiter Bypass

| Test | Scenario | Expected Result |
|------|----------|-----------------|
| Timestamp manipulation | Attacker sets future timestamps | Rejected by freshness check |
| Excessive attempts | More than 5 login attempts in 60 seconds | Rejected by rate limiter |

### 4.6 Crypto Address Validation

| Test | Scenario | Expected Result |
|------|----------|-----------------|
| Invalid BTC address | Malformed Bitcoin address | Rejected before storage |
| Invalid XMR address | Malformed Monero address | Rejected before storage |
| Empty address | Empty string address | Rejected before storage |

### 4.7 Dispute Edge Cases

| Test | Scenario | Expected Result |
|------|----------|-----------------|
| Double dispute | Second dispute on already-disputed trade | State machine rejects |

### 4.8 Fiat Payment Coordination

| Test | Scenario | Expected Result |
|------|----------|-----------------|
| Double-confirm payment | Same payment confirmed twice | State machine rejects |
| Release before maker confirm | Crypto released without maker confirmation | State machine rejects |
| Operations on failed trade | Attempt state change on failed trade | State machine rejects |
| Operations on cancelled trade | Attempt state change on cancelled trade | State machine rejects |

---

## 5. Fuzz Testing

### 5.1 Fuzz Targets

**Directory:** `fuzz/fuzz_targets/`

| Target | Input | Purpose |
|--------|-------|---------|
| `message.rs` | Raw bytes | P2P message deserialization |
| `order.rs` | Raw bytes | Order parsing |
| `p2p_offer.rs` | Raw bytes | P2P offer deserialization |
| `p2p_trade.rs` | Raw bytes | P2P trade deserialization |
| `p2p_trade_state.rs` | Raw bytes | P2P trade state deserialization |
| `state_response.rs` | Raw bytes | State response deserialization |
| `trade.rs` | Raw bytes | Trade message deserialization |
| `trade_state.rs` | Raw bytes | Trade state deserialization |

### 5.2 Fuzz Testing Strategy

All fuzz targets focus on **deserialization boundaries** — the point where untrusted external input (network messages) is parsed into internal structures. This is the most common attack surface for memory safety violations and logic bugs.

### 5.3 Running Fuzz Tests

```bash
cargo fuzz run message -- -max_total_time=300
cargo fuzz run order -- -max_total_time=300
cargo fuzz run p2p_offer -- -max_total_time=300
cargo fuzz run p2p_trade -- -max_total_time=300
cargo fuzz run p2p_trade_state -- -max_total_time=300
cargo fuzz run state_response -- -max_total_time=300
cargo fuzz run trade -- -max_total_time=300
cargo fuzz run trade_state -- -max_total_time=300
```

Each target runs for 300 seconds (5 minutes) by default.

### 5.4 Fuzz Testing Results

No crashes or security issues have been found in the fuzz targets to date.

---

## 6. Security Checks

### 6.1 Clippy

```bash
cargo clippy --workspace -- -D warnings
```

**Result:** Clean — 0 warnings across all 23 crates.

### 6.2 Cargo Audit

```bash
cargo audit
```

**Result:** 3 warnings — all GUI-only transitive dependencies:

| Warning | Crate | Severity | In Financial Code? |
|---------|-------|----------|-------------------|
| `paste` (unmaintained) | GUI transitive | Low | No |
| `ttf-parser` (unmaintained) | GUI transitive | Low | No |
| `lru` (RUSTSEC-2026-0192, unsoundness) | GUI transitive | Medium | No |

None of these affect the financial code path.

### 6.3 Cargo Deny

```bash
cargo deny check
```

**Result:** Clean

| Check | Status |
|-------|--------|
| Advisories | ✓ OK (0 RUSTSEC vulnerabilities) |
| Bans | ✓ OK (no duplicate direct dependencies) |
| Licenses | ✓ OK (all licenses approved) |
| Sources | ✓ OK (all sources approved) |

### 6.4 Unsafe Code Review

**Total `unsafe` blocks:** 2

| Location | Purpose | Justification |
|----------|---------|---------------|
| `p2p-node::GuiNodeConfig::drop()` | Zeroize RPC password byte arrays | Credential security |
| `fiat::multisig::EscrowKeyPair::drop()` | Zeroize secp256k1 secret key bytes | Key security |

Both are for credential zeroization and are necessary for security.

### 6.5 Miri (Memory Safety)

**Status:** 146 tests verified with Miri. No memory safety violations found.

---

## 7. Test Coverage by Component

### 7.1 Coverage Summary

| Component | Tests | Coverage | Notes |
|-----------|-------|----------|-------|
| `opendex-protocol-types` | ~50 | High | State machine transitions exhaustively tested |
| `fiat` | ~80 | High | Trade lifecycle, bonds, disputes, adversarial tests |
| `opendex-swap` | ~30 | Medium | Swap engine coordination |
| `opendex-transport` | ~20 | Medium | Message delivery, replay protection |
| `opendex-storage` | ~20 | Medium | Atomic writes, persistence |
| `opendex-recovery` | ~15 | Medium | Recovery decisions |
| `opendex-btc-xmr` | ~10 | Low | Stub implementation; limited test value |
| `opendex-bitcoin` | ~15 | Medium | Chain interface, mock tests |
| `opendex-monero` | ~10 | Low | Stub implementation; limited test value |
| `opendex-watchers` | ~10 | Medium | Chain observation |
| `opendex-events` | ~10 | Medium | Event sourcing |
| `opendex-types` | ~10 | High | Type validation |
| `opendex-errors` | ~5 | High | Error handling |
| `p2p-node` | ~30 | Medium | Network, persistence, log redaction |
| `user` | ~20 | Medium | Authentication, reputation |
| Other crates | ~10 | Low | Supporting utilities |

### 7.2 Low-Coverage Areas

| Area | Reason | Risk |
|------|--------|------|
| `opendex-btc-xmr` | Stub DLEQ implementation | **Critical** — no real cryptographic testing |
| `opendex-monero` | Stub RPC implementation | **High** — no real Monero testing |
| Blockchain integration | Mock chains only | **High** — no real blockchain testing |
| Payment adapters | Mock providers only | **Medium** — no real bank API testing |
| P2P network | Mock transport only | **Medium** — no real network testing |

---

## 8. Testing Limitations

### 8.1 What Is NOT Tested

| Gap | Description | Risk |
|-----|-------------|------|
| **Real DLEQ proofs** | DLEQ generation and verification are stubs | **Critical** — no cryptographic security testing |
| **Real blockchain transactions** | All chain interactions use mocks | **High** — transaction construction untested |
| **Real Monero RPC** | Monero daemon interface is stub | **High** — Monero operations untested |
| **Real payment adapters** | Bank APIs are mocked | **Medium** — payment detection untested |
| **Real P2P network** | Transport uses mock implementation | **Medium** — network behavior untested |
| **Multi-party coordination** | Tests are single-node | **Medium** — race conditions untested |
| **Long-running swaps** | No endurance testing | **Low** — memory leaks undetected |
| **Blockchain reorganizations** | Reorg handling not tested with real chains | **High** — reorg recovery untested |
| **Fee market changes** | Fee estimation not tested under pressure | **Medium** — stuck transactions possible |
| **Cross-chain timing** | No test for asymmetric confirmation times | **Medium** — timing attacks possible |

### 8.2 Testing Environment Limitations

| Limitation | Impact |
|------------|--------|
| No testnet deployment | Real network conditions not tested |
| No mainnet testing | Production behavior unknown |
| Mock blockchain nodes | Node behavior may differ from reality |
| Single-machine testing | Network partition scenarios simulated only |
| No load testing | Performance under load unknown |

### 8.3 Recommendations for Reviewers

1. **Focus on DLEQ proof integration** — The stub implementation means the core cryptographic security is untested
2. **Review transaction construction** — No real Bitcoin or Monero transactions have been built or broadcast
3. **Test with real blockchain nodes** — Mock chains may not reflect real node behavior
4. **Adversarial multi-party testing** — Single-node tests cannot catch race conditions or coordination failures
5. **Fuzz the DLEQ proof interface** — Once implemented, fuzz the proof generation and verification

---

## 9. Test Environment

### 9.1 Build and Test Commands

```bash
# Full test suite
cargo test --workspace

# With output
cargo test --workspace -- --nocapture

# Specific crate
cargo test -p fiat

# Specific test
cargo test -p fiat complete_fiat_trade_lifecycle

# Clippy
cargo clippy --workspace -- -D warnings

# Security audit
cargo audit

# Dependency check
cargo deny check

# Fuzz testing
cargo fuzz run message -- -max_total_time=300
```

### 9.2 Test Dependencies

| Dependency | Purpose |
|------------|---------|
| `tokio` | Async runtime for tests |
| `tempfile` | Temporary directories for storage tests |
| `uuid` | Test UUID generation |
| `anyhow` | Error handling in tests |

### 9.3 Mock Infrastructure

| Mock | Replaces | Purpose |
|------|----------|---------|
| `MockBitcoinChain` | Real Bitcoin node | Test chain observations |
| `MockMoneroChain` | Real Monero node | Test chain observations |
| `MockTransport` | Real P2P network | Test message delivery |
| `MockCryptoWallet` | Real wallet | Test escrow operations |
| `MockWatcher` | Real blockchain watcher | Test chain monitoring |
| `SimulatedXmrWallet` | Real Monero wallet | Test Monero operations |

---

## Appendix A: Test Failure History

| Finding | Test Added | Status |
|---------|-----------|--------|
| ADVERSARIAL-H01: No cancellation from CryptoReserved+ | 8 cancellation path tests | ✓ Fixed |
| ADVERSARIAL-H02: No MakerConfirmed timeout | Timeout test | ✓ Fixed |
| ADVERSARIAL-H03: Bond release fails on early cancellation | Bond transition test | ✓ Fixed |
| ADVERSARIAL-H04: cancel_trade calls return_escrow when not reserved | Conditional escrow test | ✓ Fixed |
| ADVERSARIAL-H05: Same-state transitions silently accepted | Same-state rejection test | ✓ Fixed |
| ADVERSARIAL-H06: No failure paths from MakerConfirmed/CryptoRelease | Failure path tests | ✓ Fixed |
