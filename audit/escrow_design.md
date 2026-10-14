# OpenDEX Escrow Design Specification

## Version: 2.0
## Date: 2026-10-02 (Updated 2026-10-12)
## Status: Phase 1 (Core Escrow) implemented — 2026-10-03 | Bitcoin multisig escrow implemented — 2026-10-12

**Phase 1 status:** 🟢 COMPLETE — The three-state balance model, escrow operations, and state machine integration are implemented and tested. See `audit/dev_log.md` for implementation details.

**Bitcoin multisig escrow:** 🟢 COMPLETE — 2-of-2 P2WSH multisig implemented in `crates/fiat/src/multisig.rs` with co-signing flow, key zeroization, and real testnet addresses.

**Phases 2–4 status:** 🟡 PENDING — Dispute resolution, timeouts, and arbitration remain future work for the legacy fiat escrow path.

---

## 1. Executive Assessment

The OpenDEX codebase has an **exceptional foundation** for escrow. The trade state machine already defines the complete escrow lifecycle, the BalanceBook provides atomic balance operations, and the settlement subsystem demonstrates the plan--then-commit pattern. What's missing is the actual escrow fund movement logic — the "plumbing" that moves funds between `locked` and a new `escrowed` state at the right state transitions.

This document describes the most robust escrow design for a P2P exchange, addressing every technical challenge including cross-node consistency, dispute resolution, timeouts, and atomic fund safety.

---

## 2. The Core Challenge: P2P Escrow Without a Trusted Third Party

In a centralized exchange, escrow is trivial: the exchange controls the funds. In a P2P system, **no single node controls the funds** — each participant holds their own balance ledger. The escrow must therefore be a **protocol-level agreement** enforced by both parties' nodes, not by any external authority.

This means the escrow design must solve:

- **How to move funds into escrow** when neither party can force the other
- **How to release funds** when the trade completes
- **How to handle disputes** when parties disagree
- **How to handle timeouts** when a party goes offline
- **How to ensure cross-node consistency** so both parties' ledgers agree

---

## 3. The Three-State Balance Model

### Current State

The BalanceBook currently has two states per asset:

- `available` — spendable funds
- `locked` — reserved for orders

### Required Extension

Add a third state: `escrowed`. The new Balance struct:

```rust
struct Balance {
    available: u64,
    locked: u64,
    escrowed: u64,
}
```

### Why Three States, Not Two

Using only `locked` for both order reservations and escrow would create ambiguity:

- A locked balance for an order reservation can be released by cancelling the order
- An escrowed balance must follow the trade state machine, not the order lifecycle
- Dispute resolution needs to distinguish "funds held for an order" from "funds held in escrow"

The three-state model ensures **clear fund provenance**: you can always answer "why are these funds not available?"

### New BalanceBook Operations

```rust
// Move from locked → escrowed (seller funds the escrow)
fn escrow_lock(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()>

// Move from escrowed → available (escrow release to recipient)
fn escrow_release(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()>

// Move from escrowed → locked (escrow return to original owner on dispute/cancel)
fn escrow_return(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()>
```

Each operation follows the same pattern as existing operations: checked arithmetic, no negative balances, atomic state transitions.

---

## 4. Escrow State Machine Integration

### The Critical Insight

The trade state machine already defines the escrow flow. The escrow fund movements must be **triggered by state transitions**, not by separate messages. This ensures the escrow state is always consistent with the trade state.

### State Transition → Fund Movement Mapping

| Transition | Fund Movement | Who |
|---|---|---|
| `Accepted → EscrowFunding` | No fund movement — signal to seller | System |
| `EscrowFunding → EscrowFunded` | Seller: `locked → escrowed` (base asset amount) | Seller's node |
| `EscrowFunded → PaymentPending` | No fund movement — signal to buyer | System |
| `PaymentPending → PaymentSent` | No fund movement — off-chain payment signal | Buyer's node |
| `PaymentSent → PaymentConfirmed` | No fund movement — buyer confirms receipt | Buyer's node |
| `PaymentConfirmed → PayoutPending` | No fund movement — signal escrow can release | System |
| `PayoutPending → Settled` | Buyer: `escrowed → available` (base asset) + normal settlement | Both nodes |
| `* → Disputed` | No fund movement — escrow held pending resolution | System |
| `Disputed → Settled` | Escrow release per dispute resolution | Both nodes |
| `Disputed → Cancelled` | Seller: `escrowed → locked` (return to seller) | Both nodes |
| `EscrowFunding → Cancelled` | No fund movement — escrow never funded | System |
| `EscrowFunded → Disputed` | No fund movement — escrow already held | System |

### Implementation Pattern

The `advance_trade_and_settle` function in `message_handler.rs` currently only triggers settlement on `Settled` state. It needs to be extended to trigger escrow operations on the relevant transitions:

```rust
fn advance_trade_and_settle(...) -> Result<()> {
    advance_trade_to_state(trade_book, trade_id, target_state, timestamp)?;

    let updated_trade = trade_book.get_trade(trade_id).cloned()
        .expect("Trade disappeared after state transition");

    // Existing: settle on final state
    if updated_trade.state == TradeState::Settled {
        settle_local_trade(balance_book, local_peer_id, &updated_trade)?;
    }

    // New: escrow fund movement on EscrowFunded
    if updated_trade.state == TradeState::EscrowFunded {
        fund_local_escrow(balance_book, local_peer_id, &updated_trade)?;
    }

    // New: escrow release on PayoutPending → Settled
    if updated_trade.state == TradeState::Settled
        && trade_was_in_escrow(trade_id) {
        release_local_escrow(balance_book, local_peer_id, &updated_trade)?;
    }

    // New: escrow return on dispute cancellation
    if updated_trade.state == TradeState::Cancelled
        && trade_was_in_escrow(trade_id) {
        return_local_escrow(balance_book, local_peer_id, &updated_trade)?;
    }
}
```

---

## 5. Cross-Node Consistency: The Hardest Problem

### The Problem

In a P2P network, the seller's node and buyer's node each maintain their own BalanceBook. When the seller's node moves funds to `escrowed`, the buyer's node has no way to force that movement — and vice versa.

### The Solution: Dual-Ledger Escrow with Reconciliation

Each party's node maintains its own escrow balance independently. The escrow is not a shared pool — it's a **mirrored commitment** on both ledgers.

**Seller's node:** When trade reaches `EscrowFunded`, seller's locked base asset moves to `escrowed`.

**Buyer's node:** When trade reaches `EscrowFunded`, buyer records that the seller's escrow is funded (via the `TradeStateChanged` message). The buyer's node does NOT move any funds at this point — the buyer's escrow balance is zero. The buyer's node tracks the escrow state in the trade record, not in the balance book.

**On settlement:** Both nodes independently release their respective escrow balances. The seller's `escrowed` balance is consumed (debited), and the buyer receives the base asset via normal settlement.

### Why This Works

- Each node only mutates its own balance ledger — no cross-node fund movement needed
- The trade state machine ensures both nodes reach the same state (eventual consistency)
- Settlement is idempotent via the `settled_trades` HashSet
- If a node restarts, the persisted trade state + escrow balance reconstruct the correct state

### The Reconciliation Guarantee

The key invariant is: **if the trade state says `EscrowFunded`, the seller's node MUST have the funds in `escrowed` state.** This is enforced by:

1. The seller's own node triggers the escrow funding when it processes the `EscrowFunded` state transition
2. If the seller's node missed the transition (restart), the snapshot synchronization will replay the trade state, triggering the escrow funding
3. The `EscrowFunded → PaymentPending` transition cannot occur without the escrow being funded (enforced by the state machine)

---

## 6. Dispute Resolution: The Most Complex Feature

### Dispute States

The state machine already allows `Disputed` from multiple states: `EscrowFunding`, `EscrowFunded`, `PaymentPending`, `PaymentSent`, `PaymentConfirmed`, `PayoutPending`.

### Dispute Resolution Protocol

A dispute can end in two ways:

1. **`Disputed → Settled`**: Parties reached an agreement; escrow is released per the agreement
2. **`Disputed → Cancelled`**: Parties agree to cancel; escrow is returned to the seller

### The Hard Problem: What if parties disagree about the resolution?

In a pure P2P system without a trusted arbiter, a dispute that reaches an impasse cannot be automatically resolved. The best possible design acknowledges this limitation and provides mechanisms to minimize its impact:

#### Option A: Mutual Consent Resolution (Recommended for V1)

Both parties must agree on the resolution. Either party can propose `Settled` or `Cancelled`, but the other party must acknowledge. This is implemented via a new message type:

```rust
enum MessageType {
    // ... existing ...
    DisputeResolution,  // New: proposes resolution of a disputed trade
}

struct DisputeResolution {
    trade_id: Uuid,
    resolution: TradeState,  // Settled or Cancelled
    timestamp: u64,
}
```

The resolution is only applied if:
- The trade is in `Disputed` state
- The sender is a trade participant
- The resolution is a valid transition from `Disputed`

#### Option B: Timeout-Based Auto-Resolution (Recommended for V2)

Add a dispute timeout to the Trade struct:

```rust
struct Trade {
    // ... existing ...
    dispute_deadline: Option<u64>,  // Unix timestamp after which auto-resolve
}
```

When a trade enters `Disputed` state, a deadline is set (e.g., 7 days). If neither party resolves by the deadline, the trade auto-transitions to `Cancelled`, returning escrow to the seller. This protects the seller from a buyer who disputes and then goes offline.

#### Option C: Multi-Party Arbitration (Most Robust, Most Complex)

Introduce a set of trusted arbitrator nodes. Either party can escalate a dispute to the arbitrators, who vote on the resolution. A supermajority (e.g., 2-of-3) decides the outcome.

This requires:
- Arbitrator discovery and registration protocol
- Vote collection and tallying
- Arbitrator reputation and slashing mechanisms
- New message types: `DisputeEscalation`, `ArbitratorVote`, `ArbitrationResult`

**Recommendation:** Start with Option A (mutual consent) for V1, add Option B (timeout) for V2, and consider Option C (arbitration) only if the use case demands it.

---

## 7. Timeout Mechanisms: Protecting Against Offline Parties

### The Problem

A party can go offline at any point, leaving the trade in an intermediate state indefinitely.

### Timeout Design

Add timeout tracking to the Trade struct:

```rust
struct Trade {
    // ... existing ...
    escrow_funding_deadline: Option<u64>,
    payment_deadline: Option<u64>,
    dispute_deadline: Option<u64>,
}
```

Each deadline is set when entering the corresponding state:

| State Entry | Deadline Set | Duration |
|---|---|---|
| `EscrowFunding` | `escrow_funding_deadline` | 24 hours |
| `PaymentPending` | `payment_deadline` | 7 days |
| `Disputed` | `dispute_deadline` | 7 days |

### Timeout Enforcement

A background task periodically checks for expired deadlines:

```rust
fn check_trade_timeouts(trade_book: &mut TradeBook) -> Vec<TradeTimeout> {
    let now = chrono::Utc::now().timestamp() as u64;

    trade_book.trades.values()
        .filter(|trade| {
            trade.escrow_funding_deadline.map(|d| now > d).unwrap_or(false)
                || trade.payment_deadline.map(|d| now > d).unwrap_or(false)
                || trade.dispute_deadline.map(|d| now > d).unwrap_or(false)
        })
        .map(|trade| TradeTimeout {
            trade_id: trade.id,
            action: determine_timeout_action(trade)
        })
        .collect()
}
```

Timeout actions:
- `EscrowFunding` timeout → transition to `Cancelled`, release order reservation
- `PaymentPending` timeout → transition to `Disputed` (seller can then resolve)
- `Disputed` timeout → transition to `Cancelled`, return escrow to seller

### Persistence

Deadlines must be persisted as part of the Trade struct. On restart, the timeout checker resumes from the persisted state.

---

## 8. Atomic Escrow Operations: Following the Settlement Pattern

### The Pattern

Every escrow operation must follow the same plan-then-commit pattern as settlement:

1. **Plan**: Create an `EscrowPlan` describing the balance mutations
2. **Apply**: Apply to a cloned BalanceBook to verify correctness
3. **Commit**: Persist the clone, then update the live state

### EscrowPlan Structure

```rust
struct EscrowPlan {
    trade_id: Uuid,
    trader: String,
    action: EscrowAction,
    asset: Asset,
    amount: u64,
}

enum EscrowAction {
    Fund,      // locked → escrowed
    Release,   // escrowed → available (to recipient)
    Return,    // escrowed → locked (back to owner)
}
```

### Atomicity Guarantee

If any part of the escrow plan fails (insufficient locked balance, overflow, etc.), the entire plan is rejected and the original balance state is preserved. This is critical because a partial escrow mutation would leave funds in an unrecoverable state.

---

## 9. Escrow Persistence

### What Must Be Persisted

1. **BalanceBook**: The `escrowed` field in the Balance struct (already persisted via serde)
2. **TradeBook**: Trade state including deadlines (already persisted)
3. **Escrow tracking**: Which trades have active escrow (derived from trade state, but could be cached)

### Recovery Scenario

On node restart:
1. BalanceBook loads with `escrowed` balances intact
2. TradeBook loads with trade states intact
3. Any trade in `EscrowFunded` through `PayoutPending` state has active escrow
4. The timeout checker resumes, checking deadlines
5. No special recovery needed — the persisted state is sufficient

### Critical Invariant

**The sum of `available + locked + escrowed` for any trader and asset must equal the total credited amount minus the total consumed amount.** This invariant must hold after every operation and can be checked in tests.

---

## 10. New Message Types

### Minimal V1 Set

```rust
enum MessageType {
    // ... existing ...
    EscrowFunded,       // Seller confirms escrow is funded
    DisputeRaised,      // Either party raises a dispute
    DisputeResolved,    // Mutual consent resolution
}
```

### V2 Extensions

```rust
enum MessageType {
    // ... existing ...
    EscrowTimeout,      // Notification that a deadline has expired
    ArbitrationRequest, // Request arbitrator intervention
    ArbitrationVote,    // Arbitrator's vote on resolution
}
```

### Authorization

All escrow messages follow the existing authorization pattern:
- Sender must be authenticated via libp2p PeerId
- Sender must be a trade participant (buyer or seller)
- For arbitration, sender must be a registered arbitrator

---

## 11. Security Considerations

### Double-Spending Protection

The three-state balance model prevents double-spending:
- A fund in `escrowed` state cannot be simultaneously in `locked` or `available`
- The `escrow_lock` operation checks `locked >= amount` before moving
- The `escrow_release` operation checks `escrowed >= amount` before moving

### Replay Attack Protection

- Escrow operations are keyed by trade ID
- The trade state machine prevents re-entering a state (e.g., can't transition to `EscrowFunded` twice)
- Settlement idempotency via `settled_trades` HashSet prevents duplicate settlement

### DoS Protection

- Pending trade state cache is bounded at 10,000 entries (existing)
- Escrow operations are bounded by the trade amount (can't escrow more than the trade value)
- Timeout mechanisms prevent indefinite escrow holding

### Fund Safety

- Plan-then-commit pattern ensures no partial mutations
- Checked arithmetic prevents overflow
- Persistence ensures survival across restarts
- The invariant `available + locked + escrowed = total` can be audited

---

## 12. Implementation Phases

### Phase 1: Core Escrow (MVP)

1. Add `escrowed` field to Balance struct
2. Implement `escrow_lock`, `escrow_release`, `escrow_return` in BalanceBook
3. Extend `advance_trade_and_settle` to trigger escrow on `EscrowFunded` and `Settled`
4. Add escrow persistence (automatic via serde)
5. Write comprehensive tests covering:
   - Happy path: fund → release
   - Insufficient locked balance
   - Overflow protection
   - Idempotent escrow funding
   - Cross-node consistency (simulated)

### Phase 2: Dispute Resolution

1. Add `DisputeRaised` message type
2. Implement dispute handling in message handler
3. Add `DisputeResolved` message type for mutual consent
4. Extend settlement to handle dispute outcomes
5. Write tests for:
   - Dispute from each state
   - Mutual consent resolution
   - Unauthorized dispute attempts
   - Dispute after settlement (should fail)

### Phase 3: Timeouts

1. Add deadline fields to Trade struct
2. Implement timeout checker background task
3. Add timeout-triggered state transitions
4. Write tests for:
   - Each timeout type
   - Timeout during dispute
   - Timeout persistence across restart

### Phase 4: Arbitration (Optional)

1. Design arbitrator registration protocol
2. Implement vote collection and tallying
3. Add arbitrator message types
4. Write tests for:
   - Vote quorum
   - Arbitrator corruption detection
   - Resolution enforcement

---

## 13. Test Strategy

### Unit Tests (BalanceBook)

- `escrow_lock` moves from locked to escrowed
- `escrow_release` moves from escrowed to available
- `escrow_return` moves from escrowed to locked
- Insufficient locked balance rejects escrow_lock
- Insufficient escrowed balance rejects escrow_release
- Overflow protection on all operations
- Invariant: `available + locked + escrowed = total`

### Integration Tests (Message Handler)

- Trade reaches `EscrowFunded` → seller's balance shows escrowed amount
- Trade reaches `Settled` → escrow released to buyer
- Dispute → escrow held, not released
- Dispute → Cancelled → escrow returned to seller
- Unauthorized escrow funding attempt rejected
- Duplicate escrow funding is idempotent

### Property Tests (Proptest)

- For any sequence of valid state transitions, the balance invariant holds
- For any trade amount, escrow operations never overflow
- For any restart point, the escrow state is recoverable

---

## 14. Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Seller goes offline after `EscrowFunding` | Timeout auto-cancels, releasing reservation |
| Buyer goes offline after `PaymentSent` | Timeout auto-transitions to `Disputed`, seller can resolve |
| Both nodes disagree on trade state | State machine + deterministic trade ID ensures convergence |
| Node restart during escrow operation | Plan-then-commit + persistence ensures atomicity |
| Malicious arbitrator (if arbitration added) | Supermajority voting + reputation system |
| Escrow amount exceeds trade value | Escrow plan validates amount against trade amount |

---

## 15. Conclusion

The OpenDEX codebase is **unusually well-prepared** for escrow implementation. The trade state machine, balance operations, settlement pattern, and authorization model all provide the building blocks. The main work is:

1. Adding the `escrowed` balance state and three operations
2. Wiring escrow fund movements to the existing state transitions
3. Implementing dispute resolution (starting with mutual consent)
4. Adding timeout mechanisms

The design is technically challenging because it must work without a trusted third party, but the P2P architecture's existing patterns — deterministic IDs, idempotent operations, plan-then-commit atomicity, and eventual consistency — provide exactly the tools needed.

**Start with Phase 1 (Core Escrow).** It's self-contained, testable, and provides the foundation for everything else. The existing 114-test suite provides a template for the escrow test strategy.

---

## 16. Relationship to BTC-XMR Atomic Swap — 2026-10-06

The BTC-XMR atomic swap system (Milestones 1-3) introduces a **separate settlement path** that coexists with the fiat-based escrow system. The two systems serve different use cases:

| Feature | Fiat Escrow (existing) | BTC-XMR Atomic Swap (new) |
|---|---|---|
| Settlement | Internal balance transfers | On-chain atomic swap via DLEQ proofs |
| Trust model | Escrow holder (seller's node) | Cryptographic, no trusted third party |
| Assets | Fiat-denominated internal balances | Real BTC and XMR on blockchain |
| State persistence | JSON files via `persistence.rs` | Event sourcing + file-based repositories |
| Recovery | Balance reconciliation | Deterministic recovery engine |
| Network | libp2p GossipSub | Transport trait (libp2p, Tor, etc.) |

### Integration Points

The two settlement paths share:
- **`opendex-types`** — Domain types (Asset, Price, etc.)
- **`opendex-errors`** — Central error hierarchy
- **`p2p-node`** — The GUI application can eventually support both paths

The two settlement paths are independent:
- The fiat escrow uses `BalanceBook`, `settlement.rs`, and `escrow.rs` in `p2p-node`
- The BTC-XMR swap uses `SwapEngine`, `SettlementProtocol`, and `opendex-*` crates

### Future Convergence

A future milestone could unify both paths under the `SettlementProtocol` trait, allowing the fiat escrow to also implement the trait. This would enable:
- Generic swap engine that works with both fiat and crypto settlement
- Unified event sourcing for both paths
- Unified recovery and reconciliation
- Unified transport layer for both paths

Until then, the two paths coexist independently, following the Hybrid approach (Option C).

---

## 17. Bitcoin 2-of-2 Multisig Escrow — Implemented 2026-10-12

The fiat settlement subsystem (`crates/fiat`) now includes a complete Bitcoin 2-of-2 multisig escrow implementation, replacing the original ledger-based escrow design with on-chain cryptographic locking.

### Implementation Details

**Module:** `crates/fiat/src/multisig.rs` (~800 lines, 13 tests)

**Key components:**

- **EscrowKeyPair** — secp256k1 key pair generation and serialization with `Zeroize` on drop
- **RedemptionScript** — `OP_2 <pub1> <pub2> OP_2 OP_CHECKMULTISIG` witness script with P2WSH address derivation
- **PartialSignature** — DER-encoded ECDSA signatures with JSON serialization for P2P exchange
- **MultisigEscrow** — Full escrow context with state machine (AwaitingDeposit → Deposited → MakerSigned → ReadyToBroadcast → Broadcast)
- **MultisigManager** — Multi-trade escrow management with maker/taker co-signing flow

### Co-Signing Flow

The 2-of-2 multisig escrow follows the OpenDex model where **no central company controls the Bitcoin**:

1. **Trade created** — Maker and taker each generate their half of the 2-of-2 multisig key pair
2. **Redemption script** — Combined into P2WSH witness script, producing a testnet escrow address (`tb1q...`)
3. **Seller deposits BTC** — Sends to the 2-of-2 multisig escrow address
4. **Security deposits** — Both parties lock refundable security bonds
5. **Fiat payment** — Buyer sends bank transfer directly to seller (OpenDex never handles fiat)
6. **Seller confirms** — Seller signs the payout transaction
7. **Both sign** — Maker and taker co-sign the payout transaction via P2P signature exchange
8. **BTC released** — Buyer receives BTC, both parties get their security deposits back

### Integration with Crypto Wallet

**Module:** `crates/fiat/src/crypto_wallet.rs` (~500 lines, 12 tests)

The `BtcWalletAdapter` now uses real 2-of-2 P2WSH multisig via `MultisigManager` instead of stub hash strings. Methods include:

- `reserve_escrow()` — Returns real testnet P2WSH addresses
- `record_deposit()` — Records the deposit transaction
- `maker_sign_payout()` — Maker's partial signature
- `taker_sign_and_finalize()` — Taker's signature and transaction finalization
- `create_return_tx()` — Return transaction for escrow cancellation

### Monero Wallet with MultisigMode

The `XmrWalletAdapter` uses a trait-based interface with `MultisigMode` enum:

- **Simulated** — Stub operations for testing
- **NativeMonero** — RPC-based, requires running daemon

### Security Properties

- **No company custody** — Bitcoin locked by cryptography (multisig), not by a custodian
- **Key zeroization** — Secret keys wiped from memory when dropped
- **Co-signing required** — Both parties must sign the payout transaction
- **Security deposits** — Financial incentives for honest behavior
- **Dispute resolution** — Evidence collection and arbitration system
