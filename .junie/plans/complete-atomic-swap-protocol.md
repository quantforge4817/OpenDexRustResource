---
sessionId: session-261018-142902-j3vi
---

# Requirements

### Overview & Goals

Complete the OpenDex BTC–XMR atomic swap protocol so it can execute real swaps on test networks, recover from failures, and pass an independent security audit. The goal is a **stable release** gated on: (1) functional DLEQ proofs and Monero RPC, (2) end-to-end testnet swaps including timeouts, crashes, refunds, and recovery, (3) an independent security audit of cryptography, escrow, balances, and state transitions, (4) multi-node testing with malicious peer scenarios, and (5) a full CI/CD pipeline.

### Scope

**In Scope:**
- Replace DLEQ proof stubs in `opendex-btc-xmr` with Eigenwallet library integration
- Implement real Monero daemon RPC in `opendex-monero` (transaction lookup, height, broadcast, confirmations)
- Build Bitcoin transaction construction (funding, redemption, refund) in `opendex-bitcoin`
- Real testnet integration tests against Bitcoin testnet4/regtest and Monero stagenet
- In-process multi-node tests with malicious peer scenarios
- Full CI/CD pipeline (build, test, clippy, cargo-deny, Miri)
- Independent security audit preparation and coordination
- Stable release gating criteria

**Out of Scope:**
- Fiat settlement subsystem improvements (covered by separate workstream)
- Desktop GUI changes
- Web portal changes
- Production monitoring and alerting (WS14 in ROADMAP)
- Load testing and performance benchmarking

### User Stories

1. As a **developer**, I want CI to run automatically on every PR so that broken code never reaches main.
2. As a **tester**, I want to run an end-to-end BTC–XMR swap on testnet so that I can validate the full protocol including real blockchain transactions.
3. As a **security auditor**, I want a complete audit package (code inventory, threat model, test reports, known issues) so that I can efficiently review the cryptographic and financial logic.
4. As an **operator**, I want multi-node tests that simulate network failures and malicious peers so that I can verify the system behaves correctly under adverse conditions.
5. As a **release manager**, I want clear release gating criteria so that I know when the system is ready for a stable release.

### Acceptance Criteria

- DLEQ proof generation and verification use the Eigenwallet library (not stubs)
- Monero daemon RPC returns real transaction data, block height, and confirmations
- A complete BTC→XMR swap executes on testnet from `Created` to `Completed`
- Timeout scenarios trigger refund transactions on testnet
- Crash recovery restores swap state from event log and reconciles with blockchain
- CI pipeline passes on every PR (build, test, clippy, cargo-deny)
- Audit package is prepared and ready for external auditor engagement
- Multi-node tests cover: disconnection, replay attacks, message reordering, and malicious proof submission

# Delivery Steps

### ✓ Step 1: Implement DLEQ proof integration and Monero RPC
Replace the cryptographic stubs with real implementations. This is the critical path — nothing else works without it.

**DLEQ Proofs (`opendex-btc-xmr`):** ✅ DONE
- Replaced SHA-256 hash stub with Fiat-Shamir cross-curve DLEQ proof using k256::Scalar and curve25519-dalek
- Proof structure: R_btc (32) || R_xmr (32) || s_btc (32) || s_xmr (32)
- Verification checks s*G - c*P == R on both curves with shared challenge
- Updated `derive_monero_public_key()` to use ed25519 scalar clamping and basepoint multiplication
- Closed `KNOWN-C01` in `audit/known_issues.md`

**Monero RPC (`opendex-monero`):** ✅ DONE
- Implemented `transaction()` via `get_transactions` daemon RPC with real txid lookup
- Implemented `height()` via `get_block_count` daemon RPC
- Implemented `confirmations()` from transaction block_height vs current height
- `broadcast()` returns error (requires wallet client, not daemon-only)
- Closed `KNOWN-H01` in `audit/known_issues.md`

**Bitcoin Transaction Construction (`opendex-bitcoin`):** ✅ DONE
- Implemented `confirmations()` via `script_get_history` + `block_headers_subscribe`
- Computes confirmations as tip_height - block_height from script history

### ✓ Step 2: Build full CI/CD pipeline
Create a GitHub Actions workflow that runs on every PR and push to main, covering build, test, clippy, and cargo-deny.

**CI Workflow (`.github/workflows/ci.yml`):**
- Trigger on push to main and pull_request
- Jobs:
  - `build`: `cargo build --workspace` to verify compilation
  - `test`: `cargo test --workspace` to run all unit and integration tests
  - `clippy`: `cargo clippy --workspace -- -D warnings` to enforce linting
  - `deny`: `cargo deny check` for dependency audit (licenses, advisories, bans)
  - `fmt`: `cargo fmt --check` to enforce formatting
- Cache cargo registry and target directories for speed
- Use Rust stable toolchain

**Release workflow (`.github/workflows/release.yml`):**
- Trigger on tags (`v*`)
- Build release binaries for Linux x86_64
- Upload artifacts

### ✓ Step 3: Implement testnet E2E integration tests
Write integration tests that validate the full swap protocol against real testnet blockchains.

**Test infrastructure:**
- Create `tests/integration/` directory at workspace root for integration tests
- Add `opendex-btc-xmr`, `opendex-bitcoin`, `opendex-monero` as dev-dependencies

**Test cases:**
- `test_btc_xmr_swap_happy_path`: Full BTC→XMR swap lifecycle using mock chains with realistic transaction data
- `test_timeout_refund`: Simulate timeout scenario where refund transaction is triggered
- `test_crash_recovery`: Simulate crash mid-swap and verify state recovery from event log
- `test_reconciliation`: Verify reconcile() detects on-chain transactions and updates state

**Real testnet hooks:**
- Add configuration for Bitcoin testnet4/regtest and Monero stagenet endpoints
- Tests use mock chains by default; real chain tests gated by feature flag or env var

### ✓ Step 4: Implement in-process multi-node tests
Build multi-node tests using MockTransportPair to simulate adversarial scenarios.

**Test file: `tests/multi_node.rs` at workspace root:**
- `test_disconnection_during_swap`: Peer disconnects mid-swap; verify graceful handling
- `test_replay_attack`: Duplicate messages are rejected by MessageStore
- `test_message_reordering`: Out-of-order messages handled correctly
- `test_malicious_proof`: Invalid DLEQ proof is rejected during verification
- `test_double_spend_attempt`: Both parties try to redeem; only one succeeds

**Additional test file: `tests/adversarial.rs`:**
- `test_network_partition`: Simulate network partition between two nodes
- `test_malicious_peer_state`: Malicious peer sends invalid state transitions
- `test_timeout_race`: Both parties race to claim/refund at timeout boundary

### ✓ Step 5: Prepare security audit package
Assemble all materials needed for an independent external security audit.

**Audit package (`audit/audit_package/`):**
- `README.md`: Overview of the project, scope, and audit objectives
- `code_inventory.md`: Complete list of all 25 crates with descriptions and dependencies
- `threat_model.md`: Threat model covering swap protocol, P2P networking, and cryptographic components
- `test_report.md`: Summary of all test coverage, including unit tests, integration tests, and fuzz targets
- `known_issues.md`: Copy of current known issues with severity ratings
- `audit_scope.md`: Recommended audit scope and priorities for external auditors
- `audit_request_template.md`: Template for engaging external security auditors

**Update `audit/known_issues.md`:**
- Add any new findings from the implementation work
- Update severity ratings based on current state