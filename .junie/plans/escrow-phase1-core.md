---
sessionId: session-261003-105459-ey97
---

# Requirements

### Overview

Implement **Phase 1: Core Escrow** of the OpenDex escrow facility. This adds a third balance state (`escrowed`) to the `BalanceBook`, enabling funds to be moved from `locked` into escrow when a trade reaches `EscrowFunded`, and released back on settlement or dispute cancellation.

### Goals

- Extend the balance model from two states (`available`, `locked`) to three states (`available`, `locked`, `escrowed`)
- Add three escrow operations: `escrow_lock` (locked → escrowed), `escrow_release` (escrowed → available), `escrow_return` (escrowed → locked)
- Wire escrow fund movements into the existing trade state machine transitions
- Follow the existing plan-then-commit atomicity pattern from `settlement.rs`
- Ensure the invariant `available + locked + escrowed = total` holds after every operation
- Persist the `escrowed` field automatically via serde (no schema migration needed)

### Scope

**In Scope:**
- Add `escrowed: u64` to the `Balance` struct in `balances.rs`
- Implement `escrow_lock`, `escrow_release`, `escrow_return` in `BalanceBook`
- Create an `EscrowPlan` struct and plan/apply/commit functions in a new `escrow.rs` module
- Extend `advance_trade_and_settle` in `message_handler.rs` to trigger escrow on `EscrowFunded` and release on `Settled`
- Handle escrow return on `Disputed → Cancelled`
- Update GUI balance display to show the `escrowed` state
- Comprehensive unit and integration tests

**Out of Scope (future phases):**
- Dispute resolution message types (`DisputeRaised`, `DisputeResolved`)
- Timeout mechanisms (deadlines on Trade struct)
- Multi-party arbitration
- On-chain Bitcoin multisig integration

### Acceptance Criteria

1. A trade reaching `EscrowFunded` moves the seller's locked base-asset amount to `escrowed`
2. A trade reaching `Settled` releases the escrowed amount (buyer receives base asset via normal settlement)
3. A trade transitioning `Disputed → Cancelled` returns escrowed funds to the seller's locked balance
4. The balance invariant `available + locked + escrowed = total` holds after every operation
5. Insufficient locked balance rejects `escrow_lock` with an error
6. Escrow funding is idempotent — applying `EscrowFunded` twice does not double-spend
7. All operations use checked arithmetic to prevent overflow
8. The `escrowed` field survives node restart via persistence

# Technical Design

### Current Implementation

**Balance model** (`crates/p2p-node/src/balances.rs`):
- `Balance` struct has `available: u64` and `locked: u64`
- Operations: `credit`, `lock`, `unlock`, `consume_locked`
- All use checked arithmetic; no negative balances possible

**Settlement** (`crates/p2p-node/src/settlement.rs`):
- `SettlementPlan` struct describes debit/credit mutations
- `plan_local_settlement` validates and creates a plan
- `commit_local_settlement` applies to a cloned `BalanceBook`, persists, then swaps
- `settle_local_trade` orchestrates plan → commit

**Trade state machine** (`crates/protocol/src/message.rs`):
- `TradeState` enum includes `EscrowFunding`, `EscrowFunded`, `Disputed`, etc.
- `can_transition_to()` validates transitions
- `Trade::transition()` enforces valid transitions

**Message handler** (`crates/p2p-node/src/message_handler.rs`):
- `advance_trade_and_settle()` calls `advance_trade_to_state()` then triggers settlement on `Settled`
- Currently only triggers `settle_local_trade()` when state is `Settled`

### Key Decisions

1. **New `escrow.rs` module** — Following the pattern of `settlement.rs`, escrow logic gets its own module with `EscrowPlan`, plan/apply/commit functions. This keeps escrow concerns separate from settlement.

2. **Escrow triggered from `advance_trade_and_settle`** — Rather than a separate message type, escrow fund movements are triggered by the same state transition messages. When `EscrowFunded` is reached, the seller's node moves funds. When `Settled` is reached, escrow is released.

3. **Dual-ledger escrow** — Each node only mutates its own `BalanceBook`. The seller's node moves its own locked funds to escrowed. The buyer's node does not move funds at `EscrowFunded` — it only tracks the trade state. On settlement, both nodes independently release their respective balances.

4. **Idempotent escrow funding** — The `EscrowPlan` will check whether the trade has already been escrow-funded (via a new `escrowed_trades: HashSet<Uuid>` in `BalanceBook`, similar to `settled_trades`).

### Proposed Changes

#### 1. `crates/p2p-node/src/balances.rs`
- Add `escrowed: u64` to `Balance` struct (with `Default = 0`)
- Add `escrowed_trades: HashSet<Uuid>` to `BalanceBook` (serde default)
- Add `escrow_lock(trader, asset, amount)` — moves from `locked` to `escrowed`
- Add `escrow_release(trader, asset, amount)` — moves from `escrowed` to `available`
- Add `escrow_return(trader, asset, amount)` — moves from `escrowed` to `locked`

#### 2. New `crates/p2p-node/src/escrow.rs`
- `EscrowPlan` struct: `trade_id`, `trader`, `action` (Fund/Release/Return), `asset`, `amount`
- `plan_local_escrow(balance_book, local_peer_id, trade)` — validates the trade is local, not already escrowed, and has sufficient locked balance
- `apply_escrow_plan(balance_book, plan)` — applies the plan mutations
- `commit_local_escrow(balance_book, plan)` — plan-then-commit pattern (clone → apply → save → swap)
- `fund_local_escrow(balance_book, local_peer_id, trade)` — orchestrates funding on `EscrowFunded`
- `release_local_escrow(balance_book, local_peer_id, trade)` — orchestrates release on `Settled`
- `return_local_escrow(balance_book, local_peer_id, trade)` — orchestrates return on dispute cancel

#### 3. `crates/p2p-node/src/message_handler.rs`
- Extend `advance_trade_and_settle()` to:
  - Call `fund_local_escrow()` when state becomes `EscrowFunded`
  - Call `release_local_escrow()` when state becomes `Settled` and trade was in escrow
  - Call `return_local_escrow()` when state becomes `Cancelled` and trade was in escrow

#### 4. `crates/p2p-node/src/lib.rs`
- Register the new `escrow` module

#### 5. `crates/p2p-node/src/gui_model.rs`
- Update `BalanceView` to include `escrowed` field

#### 6. `crates/p2p-node/src/gui_view/wallet.rs`
- Display `escrowed` balance alongside `available` and `locked`

#### 7. `crates/p2p-node/tests/balances.rs`
- Tests for `escrow_lock`, `escrow_release`, `escrow_return`
- Insufficient balance rejection tests
- Overflow protection tests
- Balance invariant tests

#### 8. `crates/p2p-node/tests/escrow.rs` (new)
- Integration test: trade reaches `EscrowFunded` → seller's balance shows escrowed amount
- Integration test: trade reaches `Settled` → escrow released
- Integration test: dispute → cancelled → escrow returned
- Idempotent escrow funding test

### Data Models

```rust
// balances.rs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Balance {
    pub available: u64,
    pub locked: u64,
    pub escrowed: u64,
}

// escrow.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscrowAction {
    Fund,      // locked → escrowed
    Release,   // escrowed → available
    Return,    // escrowed → locked
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowPlan {
    pub trade_id: Uuid,
    pub trader: String,
    pub action: EscrowAction,
    pub asset: Asset,
    pub amount: u64,
}
```

### Architecture Diagram

``` mermaid
graph LR
    A[TradeStateChanged message] --> B[advance_trade_and_settle]
    B --> C{State?}
    C -->|EscrowFunded| D[fund_local_escrow]
    C -->|Settled| E[release_local_escrow]
    C -->|Cancelled + was in escrow| F[return_local_escrow]
    C -->|Settled| G[settle_local_trade]
    D --> H[EscrowPlan: Fund]
    E --> I[EscrowPlan: Release]
    F --> J[EscrowPlan: Return]
    H --> K[commit_local_escrow]
    I --> K
    J --> K
    K --> L[BalanceBook clone → apply → save → swap]
```

### Risks

| Risk | Mitigation |
|------|------------|
| `escrowed` field breaks existing persisted balances | Serde `#[serde(default)]` ensures old files load with `escrowed = 0` |
| Double escrow funding on replay | `escrowed_trades` HashSet prevents duplicate funding |
| Seller's node restarts between `EscrowFunding` and `EscrowFunded` | Snapshot sync replays trade state, triggering escrow funding on recovery |
| Settlement and escrow release conflict | Escrow release runs before settlement; settlement uses `consume_locked` on base asset, not escrowed |

# Delivery Steps

### ✓ Step 1: Add escrowed balance state and operations to BalanceBook
Extend the Balance struct with `escrowed: u64` and add three escrow operations to BalanceBook.
- Add `escrowed: u64` field to `Balance` in `balances.rs` with `#[serde(default)]`
- Add `escrowed_trades: HashSet<Uuid>` to `BalanceBook` for idempotency tracking
- Implement `escrow_lock()` — moves from locked to escrowed, checks sufficient locked balance
- Implement `escrow_release()` — moves from escrowed to available
- Implement `escrow_return()` — moves from escrowed to locked
- Update `BalanceView` in `gui_model.rs` to include `escrowed` field
- Add unit tests in `tests/balances.rs` for all three operations including error cases and invariant checks

### ✓ Step 2: Create escrow module with plan-then-commit pattern
Create a new `escrow.rs` module following the `settlement.rs` pattern.
- Define `EscrowAction` enum (Fund, Release, Return) and `EscrowPlan` struct
- Implement `plan_local_escrow()` — validates trade is local, not already escrowed, sufficient locked balance
- Implement `apply_escrow_plan()` — applies mutations to BalanceBook
- Implement `commit_local_escrow()` — clone → apply → save → swap atomicity pattern
- Implement `fund_local_escrow()`, `release_local_escrow()`, `return_local_escrow()` orchestration functions
- Register the module in `lib.rs`

### ✓ Step 3: Wire escrow into trade state transitions
Extend `advance_trade_and_settle` in `message_handler.rs` to trigger escrow operations on relevant state transitions.
- On `EscrowFunded`: call `fund_local_escrow()` to move seller's locked base asset to escrowed
- On `Settled` with prior escrow: call `release_local_escrow()` before normal settlement
- On `Cancelled` with prior escrow: call `return_local_escrow()` to return funds to seller
- Update `GuiCommand::AdvanceTradeState` handler in `main.rs` to trigger escrow on `EscrowFunded`
- Update `wallet.rs` GUI to display escrowed balance
- Add integration tests in `tests/escrow.rs` covering happy path, dispute cancellation, and idempotency