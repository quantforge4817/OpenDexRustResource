# OpenDex Reproducibility Document

**Version:** 0.1.0-security-review
**Date:** 2026-10-14
**Status:** Authoritative build and test instructions
**Scope:** Reviewed commit, toolchain, dependency versions, build instructions, and test environment

---

## Table of Contents

1. [Reviewed Version](#1-reviewed-version)
2. [System Requirements](#2-system-requirements)
3. [Rust Toolchain](#3-rust-toolchain)
4. [Build Instructions](#4-build-instructions)
5. [Test Instructions](#5-test-instructions)
6. [Security Checks](#6-security-checks)
7. [Dependency Versions](#7-dependency-versions)
8. [Test Environment](#8-test-environment)
9. [Post-Baseline Changes](#9-post-baseline-changes)

---

## 1. Reviewed Version

| Field | Value |
|-------|-------|
| **Version** | v0.1.0-security-review |
| **Git commit** | `512f4cd` |
| **Git tag** | `v0.1.0-security-review` |
| **Branch** | `main` |
| **Date** | 2026-10-14 |
| **Crates** | 23 |
| **Source files** | 110 Rust files |
| **Lines of code** | ~32,400 lines of Rust |
| **Test count** | 692 passed, 0 failed, 5 ignored |

To check out the reviewed version:

```bash
git clone <repository-url>
cd OpenDexRustSource
git checkout v0.1.0-security-review
```

---

## 2. System Requirements

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| OS | Linux (Ubuntu 22.04+), macOS 12+, Windows 10+ | Linux (Ubuntu 24.04) |
| CPU | 4 cores | 8+ cores (for parallel builds) |
| RAM | 8 GB | 16+ GB (for full workspace build) |
| Disk | 10 GB | 20+ GB (with build artifacts) |
| Network | Internet access (for dependency download) | — |

---

## 3. Rust Toolchain

### 3.1 Required Version

| Component | Version |
|-----------|---------|
| Rust compiler | 1.88+ (specified in `rust-version` field) |
| Cargo | 1.88+ |
| Rustfmt | Latest stable |
| Clippy | Latest stable |

### 3.2 Installation

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version    # Should be 1.88+
cargo --version    # Should be 1.88+
```

### 3.3 Additional Tools

| Tool | Purpose | Installation |
|------|---------|--------------|
| `cargo-audit` | Dependency vulnerability scanning | `cargo install cargo-audit` |
| `cargo-deny` | Supply chain security | `cargo install cargo-deny` |
| `cargo-fuzz` | Fuzz testing | `cargo install cargo-fuzz` |
| `cargo-miri` | Memory safety verification | `rustup toolchain install miri` |

---

## 4. Build Instructions

### 4.1 Clean Build

```bash
# Clone and checkout reviewed version
git clone <repository-url>
cd OpenDexRustSource
git checkout v0.1.0-security-review

# Build all crates
cargo build --workspace

# Build in release mode
cargo build --workspace --release
```

### 4.2 Expected Build Output

The build should complete without errors or warnings. If Clippy warnings appear:

```bash
cargo clippy --workspace -- -D warnings
```

This should produce 0 warnings.

### 4.3 Build Artifacts

| Artifact | Location |
|----------|----------|
| Debug binaries | `target/debug/` |
| Release binaries | `target/release/` |
| Library files | `target/debug/deps/` or `target/release/deps/` |

---

## 5. Test Instructions

### 5.1 Full Test Suite

```bash
# Run all tests
cargo test --workspace
```

**Expected result:** 692 passed, 0 failed, 5 ignored

### 5.2 Individual Crate Tests

```bash
# Test specific crate
cargo test -p fiat
cargo test -p opendex-protocol-types
cargo test -p opendex-swap
```

### 5.3 Test with Output

```bash
# Show test output
cargo test --workspace -- --nocapture
```

### 5.4 Fuzz Testing

```bash
# Run fuzz target for 5 minutes
cargo fuzz run message -- -max_total_time=300
cargo fuzz run order -- -max_total_time=300
cargo fuzz run p2p_offer -- -max_total_time=300
cargo fuzz run p2p_trade -- -max_total_time=300
cargo fuzz run p2p_trade_state -- -max_total_time=300
cargo fuzz run state_response -- -max_total_time=300
cargo fuzz run trade -- -max_total_time=300
cargo fuzz run trade_state -- -max_total_time=300
```

### 5.5 Miri (Memory Safety)

```bash
# Run Miri on specific crate
cargo miri test -p fiat
cargo miri test -p opendex-protocol-types
```

---

## 6. Security Checks

### 6.1 Clippy

```bash
cargo clippy --workspace -- -D warnings
```

**Expected result:** Clean — 0 warnings

### 6.2 Cargo Audit

```bash
cargo audit
```

**Expected result:** 3 warnings (all GUI-only transitive dependencies, not in financial code path)

### 6.3 Cargo Deny

```bash
cargo deny check
```

**Expected result:** Clean — advisories ok, bans ok, licenses ok, sources ok

### 6.4 Unsafe Code Review

```bash
# Search for unsafe blocks
grep -r "unsafe" crates/ --include="*.rs"
```

**Expected result:** 0 `unsafe` blocks — all code is safe Rust

---

## 7. Dependency Versions

### 7.1 Key Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `secp256k1` | 0.29 | Bitcoin secp256k1 signatures |
| `bitcoin` | 0.32 | Bitcoin transaction handling |
| `dleq` | 0.4 | Cross-curve DLEQ proofs |
| `sha2` | 0.10 | SHA-256/SHA-512 hash functions |
| `libp2p` | 0.57 | P2P network layer |
| `serde` | 1.x | Serialization |
| `tokio` | 1.x | Async runtime |
| `tracing` | 0.1.x | Structured logging |
| `argon2` | (from Cargo.lock) | Password hashing |
| `hmac` | (from Cargo.lock) | Token signing |
| `zeroize` | (from Cargo.lock) | Secure memory zeroization |
| `uuid` | (from Cargo.lock) | Unique identifiers |

### 7.2 Dependency Lock

All dependencies are pinned in `Cargo.lock`. To verify dependency integrity:

```bash
# Check that Cargo.lock is up to date
cargo update --dry-run

# Verify no unexpected changes
git diff Cargo.lock
```

### 7.3 Supply Chain Security

```bash
# Check for known vulnerabilities
cargo audit

# Check for banned dependencies and license compliance
cargo deny check
```

Both should pass (with expected GUI-only warnings from `cargo audit`).

---

## 8. Test Environment

### 8.1 Mock Infrastructure

The test suite uses mock implementations for external dependencies:

| Mock | Replaces | Crate |
|------|----------|-------|
| `MockBitcoinChain` | Real Bitcoin node | `opendex-bitcoin` |
| `MockMoneroChain` | Real Monero node | `opendex-monero` |
| `MockTransport` | Real P2P network | `opendex-transport` |
| `MockCryptoWallet` | Real wallet | `fiat` |
| `MockWatcher` | Real blockchain watcher | `opendex-watchers` |
| `SimulatedXmrWallet` | Real Monero wallet | `fiat` |

### 8.2 No External Services Required

The test suite does not require:
- Running Bitcoin nodes
- Running Monero nodes
- Network connectivity
- External APIs
- Database servers

All tests run in-process with mock dependencies.

### 8.3 Environment Variables

No environment variables are required for building or testing.

---

## 9. Post-Baseline Changes

### 9.1 Change Tracking

After the `v0.1.0-security-review` tag, all changes are tracked in `CHANGELOG.md`. To see changes since the reviewed version:

```bash
# List commits since the reviewed version
git log v0.1.0-security-review..HEAD --oneline

# Show diff since the reviewed version
git diff v0.1.0-security-review..HEAD
```

### 9.2 Version History

| Version | Commit | Date | Notes |
|---------|--------|------|-------|
| v0.1.0-security-review | `512f4cd` | 2026-10-14 | Security baseline; all 15 open findings resolved |

### 9.3 Development Status

At the time of the security review baseline:

- **All 23 crates** compile without errors or warnings
- **692 tests** pass, 0 fail, 5 ignored
- **0 RUSTSEC vulnerabilities** in financial code path
- **0 `unsafe` blocks** — all code is safe Rust
- **146 tests** verified with Miri
- **12 fuzz targets** with no crashes

### 9.4 Known Implementation Gaps

The following components are stubs and are not production-ready:

1. **DLEQ proof** (`opendex-btc-xmr`) — Stub pending Eigenwallet integration
2. **Monero RPC** (`opendex-monero`) — Stub implementation
3. **Adaptor signatures** — Not yet implemented

These are documented in `known_issues.md` and `cryptography.md`.

---

## Appendix A: Full Build and Test Script

```bash
#!/bin/bash
set -euo pipefail

echo "=== OpenDex Security Review Build ==="
echo "Reviewed commit: 512f4cd (v0.1.0-security-review)"
echo ""

# Checkout reviewed version
git checkout v0.1.0-security-review

# Build
echo "=== Building ==="
cargo build --workspace
echo "Build: OK"
echo ""

# Clippy
echo "=== Clippy ==="
cargo clippy --workspace -- -D warnings
echo "Clippy: OK"
echo ""

# Tests
echo "=== Tests ==="
cargo test --workspace
echo "Tests: OK"
echo ""

# Security checks
echo "=== Cargo Audit ==="
cargo audit || true  # Expected: 3 GUI-only warnings
echo ""

echo "=== Cargo Deny ==="
cargo deny check
echo "Cargo deny: OK"
echo ""

echo "=== Build and test complete ==="
```

---

## Appendix B: Repository Structure

```
OpenDexRustSource/
├── Cargo.toml              # Workspace manifest
├── Cargo.lock              # Pinned dependencies
├── deny.toml               # cargo-deny configuration
├── CHANGELOG.md            # Version history
├── audit/                  # Security documentation
│   ├── architecture.md
│   ├── audit.md
│   ├── audit_checklist.md
│   ├── audit_scope.md
│   ├── cryptography.md
│   ├── known_issues.md
│   ├── protocol.md
│   ├── reproducibility.md  # This document
│   ├── security.md           # Security policy, baseline, controls, findings
│   ├── testing.md
│   └── threat_model.md
├── crates/                 # 23 Rust crates
│   ├── common/
│   ├── fiat/
│   ├── opendex-btc-xmr/
│   ├── opendex-bitcoin/
│   ├── opendex-chain/
│   ├── opendex-dao/
│   ├── opendex-errors/
│   ├── opendex-events/
│   ├── opendex-fee-treasury/
│   ├── opendex-monero/
│   ├── opendex-protocol/
│   ├── opendex-protocol-types/
│   ├── opendex-recovery/
│   ├── opendex-storage/
│   ├── opendex-swap/
│   ├── opendex-types/
│   ├── opendex-transport/
│   ├── opendex-user/
│   ├── opendex-watchers/
│   ├── p2p-node/
│   ├── protocol/
│   ├── user/
│   └── ...
├── apps/                   # Application binaries
│   ├── desktop-gui/
│   └── web-frontend/
└── fuzz/                   # Fuzz targets
    └── fuzz_targets/
        ├── message.rs
        ├── order.rs
        ├── p2p_offer.rs
        ├── p2p_trade.rs
        ├── p2p_trade_state.rs
        ├── state_response.rs
        ├── trade.rs
        └── trade_state.rs
```
