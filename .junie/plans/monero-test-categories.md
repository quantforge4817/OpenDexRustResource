---
sessionId: session-261015-152132-h3dw
---

# Requirements

### Overview & Goals

Add Kani formal verification proof harnesses for the four highest-value components identified in the previous analysis:

1. **`BondState` state machine** — 6 states with 10 valid transitions in `crates/fiat/src/bond.rs`
2. **`FiatTradeState` state machine** — 16 states with ~45 valid transitions in `crates/fiat/src/trade.rs`
3. **`OrderBook` arithmetic and validation** — trade validation, idempotency, and amount checks in `crates/p2p-node/src/orderbook.rs`
4. **`RateLimiter` arithmetic** — sliding window cutoff calculation in `crates/user/src/auth.rs`

Kani uses symbolic execution to prove properties hold for **all possible inputs** — complementing the existing unit tests (specific inputs), fuzzing (random inputs), and benchmarks (performance).

### Scope

**In Scope:**
- Kani proof harnesses for state machine transition graphs (proving only valid transitions return true)
- Kani proof harnesses for arithmetic properties (no overflow, no underflow)
- Kani proof harnesses for idempotency guarantees (apply_trade called twice)
- Kani proof harnesses for terminal state properties (no outgoing transitions)
- Documentation in `audit/audit.md`

**Out of Scope:**
- Async code verification (Kani doesn't support async/await — P2P network layer excluded)
- External library verification (libp2p, secp256k1, Monero RPC)
- Concurrency verification (use Miri for that)
- Memory safety verification (0 `unsafe` blocks — all safe Rust, use Miri)

### Acceptance Criteria

- All Kani proofs compile and pass: `cargo kani` on each target crate
- No unreachable assertions or counterexamples
- All proofs documented in `audit/audit.md`
- Proofs cover all state pairs for both state machines (36 for BondState, 256 for FiatTradeState)

# Technical Design

### Current Implementation

The project has:
- **42+ inline `#[cfg(test)]` modules** — cover specific scenarios with concrete inputs
- **12 fuzz targets** — find edge cases via random inputs
- **4 Criterion benchmarks** — measure throughput/latency
- **1 load test file** — multi-node P2P stress tests
- **No Kani proofs** — no mathematical guarantees about properties holding for all inputs

**Target components and their properties:**

| Component | File | States | Valid Transitions | Key Properties |
|---|---|---|---|---|
| `BondState` | `crates/fiat/src/bond.rs` | 6 | 10 | No self-transition, terminal states have no outgoing |
| `FiatTradeState` | `crates/fiat/src/trade.rs` | 16 | ~45 | No self-transition, lifecycle paths valid, dispute from active |
| `OrderBook::validate_trade()` | `crates/p2p-node/src/orderbook.rs` | — | — | Rejects zero amounts, rejects amounts > order limits |
| `OrderBook::apply_trade()` | `crates/p2p-node/src/orderbook.rs` | — | — | Idempotent, rejects zero price, fill sequence monotonic |
| `RateLimiter::is_allowed()` | `crates/user/src/auth.rs` | — | — | Cutoff never underflows, enforces limit, window expiry works |

### Key Decisions

1. **Separate `*_kani.rs` files** — Proof harnesses in dedicated files (e.g., `bond_kani.rs`) rather than inline `#[cfg(test)]` modules, since Kani uses `#[kani::proof]` attributes incompatible with `cargo test`
2. **`kani` as dev-dependency** — Added to each crate's `Cargo.toml` rather than a workspace-level setup, following Kani's per-crate invocation model
3. **`kani::any()` for symbolic states** — Each proof harness uses `kani::any()` to create symbolic enum values, allowing Kani to explore all combinations
4. **No loop unwinding needed** — Target functions contain no loops (state machines use `matches!()`, rate limiter uses `retain()`), avoiding Kani's loop unwinding limitations

### Proposed Changes

#### 1. Bond State Machine Proofs (`crates/fiat/src/bond_kani.rs`)

- **`verify_bond_transition_graph()`** — Use `kani::any()` for both current and next `BondState`. Assert that `can_transition_to()` returns true only for the 10 valid transitions among 36 possible pairs (6×6)
- **`verify_bond_no_self_transition()`** — Assert `!current.can_transition_to(&current)` for any symbolic `BondState`
- **`verify_bond_terminal_states()`** — Assert that `Released` and `Slashed` cannot transition to any state except `Refunded`
- **`verify_bond_lock_safety()`** — Create symbolic `SecurityBond`, assert `lock()` fails unless state is `Required`
- **`verify_bond_slash_safety()`** — Assert `slash()` fails unless state is `Locked` or `PartiallySlashed`

#### 2. Fiat Trade State Machine Proofs (`crates/fiat/src/trade_kani.rs`)

- **`verify_fiat_trade_transition_graph()`** — Use `kani::any()` for both states. Assert `can_transition_to()` returns true only for valid transitions among 256 pairs (16×16)
- **`verify_fiat_trade_no_self_transition()`** — Assert no state transitions to itself
- **`verify_fiat_trade_terminal_states()`** — Assert `Complete`, `Failed`, and `Cancelled` have no outgoing transitions (except `Complete → ReversalDetected`)
- **`verify_fiat_trade_dispute_from_active()`** — Assert `Disputed` is reachable from all active states
- **`verify_fiat_trade_transition_rejects_invalid()`** — Create symbolic `FiatTrade`, assert `transition()` returns Err for invalid next states

#### 3. OrderBook Proofs (`crates/p2p-node/src/orderbook_kani.rs`)

- **`verify_validate_trade_rejects_zero_amount()`** — Create `OrderBook` with orders, symbolic `Trade` with amount 0, assert `validate_trade()` returns Err
- **`verify_validate_trade_rejects_excess_amount()`** — Symbolic trade amount > order amount, assert rejection
- **`verify_apply_trade_idempotency()`** — Apply valid trade twice, assert second call returns false
- **`verify_apply_trade_rejects_zero_price()`** — Symbolic trade with price 0, assert rejection via `validate_trade_structure()`
- **`verify_fill_sequence_monotonicity()`** — Apply two trades to the same resting order, assert fill sequence increased

#### 4. Rate Limiter Proofs (`crates/user/src/auth_kani.rs`)

- **`verify_rate_limiter_cutoff_no_underflow()`** — Symbolic `current_time` and `window_seconds`, assert cutoff calculation doesn't underflow (checked via the `if` guard)
- **`verify_rate_limiter_enforces_limit()`** — Create limiter with limit N, make N+1 symbolic requests, assert at least one is rejected
- **`verify_rate_limiter_window_expiry()`** — Fill limiter to capacity, advance time past window, assert next request is allowed

### File Structure

```
crates/fiat/
├── src/
│   └── bond_kani.rs             (NEW: BondState Kani proofs)
├── src/
│   └── trade_kani.rs            (NEW: FiatTradeState Kani proofs)
└── Cargo.toml                   (MODIFIED: add kani dev-dep)

crates/p2p-node/
├── src/
│   └── orderbook_kani.rs        (NEW: OrderBook Kani proofs)
└── Cargo.toml                   (MODIFIED: add kani dev-dep)

crates/user/
├── src/
│   └── auth_kani.rs             (NEW: RateLimiter Kani proofs)
└── Cargo.toml                   (MODIFIED: add kani dev-dep)

audit/
└── audit.md                     (MODIFIED: document Kani verification)
```

### Architecture Diagram

``` mermaid
graph LR
    CK[cargo kani] --> BK[bond_kani.rs]
    CK --> TK[trade_kani.rs]
    CK --> OK[orderbook_kani.rs]
    CK --> AK[auth_kani.rs]
    BK --> BS[BondState]
    TK --> FS[FiatTradeState]
    OK --> OB[OrderBook]
    AK --> RL[RateLimiter]
```

### Risks

- **Kani installation** — Requires `cargo install cargo-kani` and `kani setup` before first use. If Kani is not installed, proofs won't run.
- **UUID generation** — `SecurityBond::new()` calls `Uuid::new_v4()`. Kani cannot model random number generation. Workaround: construct `SecurityBond` fields directly in proofs instead of using `new()`.
- **`anyhow::Result` returns** — Kani may not follow error paths through `anyhow::bail!`. Proofs will check return values explicitly rather than relying on `unwrap()`.
- **HashMap iteration order** — `OrderBook` proofs construct specific orders rather than relying on iteration order.

# Delivery Steps

### ✓ Step 1: Add BondState and FiatTradeState Kani proofs
### ✓ Step 2: Add OrderBook and RateLimiter Kani proofs
### ✓ Step 3: Document Kani verification in audit.md
Create proof harnesses for both state machines in the `fiat` crate, covering all state pairs and terminal state properties.

- Install Kani: `cargo install cargo-kani` and `kani setup` if not already installed.
- Add `kani` dev-dependency to `crates/fiat/Cargo.toml`.
- Create `crates/fiat/src/bond_kani.rs` with 5 proof harnesses:
  - `verify_bond_transition_graph()`: Prove only 10 of 36 state pairs are valid transitions.
  - `verify_bond_no_self_transition()`: Prove no state transitions to itself.
  - `verify_bond_terminal_states()`: Prove `Released` and `Slashed` have no outgoing except to `Refunded`.
  - `verify_bond_lock_safety()`: Prove `lock()` only succeeds from `Required`.
  - `verify_bond_slash_safety()`: Prove `slash()` only succeeds from `Locked` or `PartiallySlashed`.
- Create `crates/fiat/src/trade_kani.rs` with 5 proof harnesses:
  - `verify_fiat_trade_transition_graph()`: Prove only valid transitions among 256 pairs (16×16).
  - `verify_fiat_trade_no_self_transition()`: Prove no self-transition.
  - `verify_fiat_trade_terminal_states()`: Prove `Complete`, `Failed`, `Cancelled` are terminal (except `Complete → ReversalDetected`).
  - `verify_fiat_trade_dispute_from_active()`: Prove `Disputed` reachable from all active states.
  - `verify_fiat_trade_transition_rejects_invalid()`: Prove `transition()` returns Err for invalid states.
- Add modules to `crates/fiat/src/lib.rs`: `mod bond_kani;` and `mod trade_kani;`.
- Run proofs: `cargo kani --manifest-path crates/fiat/Cargo.toml`.

###   Step 2: Add OrderBook and RateLimiter Kani proofs
Create proof harnesses for OrderBook arithmetic/validation and RateLimiter arithmetic properties.

- Add `kani` dev-dependency to `crates/p2p-node/Cargo.toml`.
- Create `crates/p2p-node/src/orderbook_kani.rs` with 5 proof harnesses:
  - `verify_validate_trade_rejects_zero_amount()`: Prove zero-amount trades are rejected.
  - `verify_validate_trade_rejects_excess_amount()`: Prove trades exceeding order amounts are rejected.
  - `verify_apply_trade_idempotency()`: Prove applying the same trade twice returns false the second time.
  - `verify_apply_trade_rejects_zero_price()`: Prove zero-price trades are rejected.
  - `verify_fill_sequence_monotonicity()`: Prove fill sequences only increase.
- Add module to `crates/p2p-node/src/lib.rs`: `mod orderbook_kani;`.
- Add `kani` dev-dependency to `crates/user/Cargo.toml`.
- Create `crates/user/src/auth_kani.rs` with 3 proof harnesses:
  - `verify_rate_limiter_cutoff_no_underflow()`: Prove cutoff calculation never underflows.
  - `verify_rate_limiter_enforces_limit()`: Prove requests beyond limit are rejected.
  - `verify_rate_limiter_window_expiry()`: Prove requests allowed after window expires.
- Add module to `crates/user/src/lib.rs`: `mod auth_kani;`.
- Run proofs: `cargo kani --manifest-path crates/p2p-node/Cargo.toml` and `cargo kani --manifest-path crates/user/Cargo.toml`.

###   Step 3: Document Kani verification in audit.md
Document all Kani proofs, their properties, and how to run them.

- Add Section 5 "Kani Formal Verification" to `audit/audit.md` before Section 4.
- List all 17 proof harnesses with their proven properties.
- Include run commands for each crate.
- Update the overall assessment table to include a "Formal verification" row with 🟢 GREEN status.
- Update test coverage totals to include Kani proofs.