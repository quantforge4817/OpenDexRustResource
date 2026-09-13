# OpenDEX Audit Checklist

## Current Audit Scope

The current audit covers:

- `crates/protocol/src/message.rs`
- `crates/protocol/src/state.rs`
- `crates/p2p-node/src/order_factory.rs`
- `crates/p2p-node/src/orderbook.rs`
- `crates/p2p-node/src/trades.rs`
- `crates/p2p-node/src/balances.rs`
- `crates/p2p-node/src/settlement.rs`
- `crates/p2p-node/src/trade_state.rs`
- `crates/p2p-node/src/sync.rs`
## Audit Boundary

This is a source-level audit of the OpenDEX core trading and settlement logic.

In scope:
- Order creation and representation
- Order matching and trade creation
- Trade validation and application
- Trade state transitions
- Balance accounting and settlement
- State snapshot creation/application
- Protocol message/state definitions
- Idempotency, overflow/underflow, replay, and consistency controls

Out of scope for this review:
- Physical/network infrastructure
- Host security
- Key custody and operational security
- Production deployment configuration
- External services
- Cryptographic key management outside the reviewed code
- Penetration testing and live-network testing

An independent external auditor will perform the physical/infrastructure,
deployment, and independent verification checks.
	
## Audit Status

- [ ] Audit complete
- [ ] Findings reviewed
- [ ] Critical findings resolved
- [ ] High findings resolved
- [ ] Medium findings resolved
- [ ] Low findings resolved

## Audit Areas

### 1. Protocol
- [ ] Message integrity
- [ ] Serialization/deserialization
- [ ] Price/amount validation
- [ ] Timestamp handling

### 2. Orders
- [ ] Order creation
- [ ] Order validation
- [ ] Cancellation
- [ ] Duplicate orders
- [ ] Quantity accounting

### 3. Matching
- [ ] Price matching
- [ ] Partial fills
- [ ] Trade determinism
- [ ] Trade duplication/replay
- [ ] Self-trading
- [ ] Overflow/underflow

### 4. Balances & Settlement
- [ ] Available/locked accounting
- [ ] Buyer settlement
- [ ] Seller settlement
- [ ] Asset conservation
- [ ] Settlement idempotency
- [ ] Overflow/underflow

### 5. Trade State
- [ ] Valid transitions
- [ ] Invalid transitions rejected
- [ ] Exceptional states
- [ ] State replay
- [ ] Skipped-state handling

### 6. State Synchronization
- [ ] Snapshot validation
- [ ] Duplicate trades
- [ ] Removed orders
- [ ] Applied trades
- [ ] Transactional application
- [ ] Persistence consistency

## Findings

### AUDIT-001 — Message sender authentication requires verification

- **Severity:** High (pending verification)
- **Status:** Needs verification
- **File:** `crates/protocol/src/message.rs`
- **Location:** `OpenDexMessage::sender`

**Observation:**

`OpenDexMessage` contains a caller-supplied `sender: String`, but the
protocol message itself does not cryptographically authenticate or bind
that identity to the P2P peer that transmitted the message.

**Security question:**

Does the P2P/network layer verify that `sender` corresponds to the
authenticated peer before processing the message?

**Required verification:**

Trace incoming `OpenDexMessage` handling from network reception through
deserialization and dispatch.

**Impact if unverified:**

An attacker may be able to impersonate another trader/peer and submit
orders, trades, state changes, or other protocol messages under a forged
identity.

**Status:** OPEN

### AUDIT-002 — Order UUID reuse permits unvalidated order replacement

- **Severity:** High (pending message-path verification)
- **Status:** OPEN
- **File:** `crates/p2p-node/src/orderbook.rs`
- **Location:** `upsert_order()`, lines 63-90

**Observation:**

`upsert_order()` replaces an existing order whenever the incoming order
has the same UUID. It does not verify that immutable order attributes
remain unchanged.

Potentially mutable fields include:

- trader
- market
- side
- price
- amount

**Impact:**

If remote peers can submit arbitrary `OrderCreated` messages or snapshot
orders, an attacker may be able to overwrite an existing order by
reusing its UUID.

**Recommendation:**

Define which order fields are immutable after creation. At minimum,
verify the order identity/owner before accepting an update. Prefer
rejecting unauthorized changes rather than treating an `OrderCreated`
message as an unrestricted upsert.

**Required verification:**

Trace every caller of `upsert_order()` and determine whether incoming
orders are authenticated and whether updates are legitimate protocol
operations.

**Status:** OPEN

### AUDIT-003 — Matching engine permits self-trading

- **Severity:** Medium
- **Status:** OPEN
- **File:** `crates/p2p-node/src/orderbook.rs`
- **Location:** `match_order()`, lines 287-401

**Observation:**

The matching predicate verifies opposite sides and compatible markets/prices,
but does not exclude orders belonging to the same trader.

**Impact:**

A trader can potentially match against their own resting order, creating a
trade where `buyer == seller`.

This can distort volume, fees, reputation/accounting, or downstream
settlement assumptions.

**Recommendation:**

Unless self-trading is explicitly intended by the protocol, reject a
candidate when:

`existing.trader == incoming.trader`

Also add a regression test asserting that self-crossing orders produce
no trade.

**Status:** OPEN

### AUDIT-004 — Settlement does not require terminal trade state

- **Severity:** High
- **Status:** OPEN
- **File:** `crates/p2p-node/src/settlement.rs`
- **Location:** `settle_local_trade()`

**Observation:**

`settle_local_trade()` performs economic settlement based on the supplied
`Trade`, but does not verify that the trade is in `TradeState::Settled`
(or otherwise verify that settlement is authorized by the trade lifecycle).

The trade-state machine defines `Settled` as the terminal successful state,
but settlement itself does not enforce that condition.

**Impact:**

If an attacker or incorrect internal caller can invoke settlement with a
trade that has not legitimately completed the required lifecycle, locked
funds could potentially be consumed and assets credited prematurely.

**Recommendation:**

Require the trade to be in the appropriate settlement-authorized state
before performing economic settlement. Ideally, make the transition to
`Settled` and the economic settlement operation part of one well-defined
state transition/authorization path.

**Required verification:**

Trace every caller of `settle_local_trade()` and determine whether callers
already guarantee the trade is `Settled`. If they do, downgrade this from a
confirmed vulnerability to a defense-in-depth finding.

**Status:** OPEN

### CTRL-005 — Settlement has trade-level idempotency

- **Status:** PASS
- **File:** `crates/p2p-node/src/settlement.rs`

A trade ID is recorded in `settled_trades`, and duplicate settlement
attempts are ignored.

This protects against replay of the same settlement event within the
same persisted balance state.

### CTRL-006 — Settlement validates balances and arithmetic before mutation

- **Status:** PASS
- **File:** `crates/p2p-node/src/settlement.rs`

Settlement checks:

- quote multiplication overflow
- buyer locked quote balance
- seller locked base balance
- buyer base available-balance overflow
- seller quote available-balance overflow

These checks occur before balance mutations.

### AUDIT-007 — Fixed-point settlement truncates quote amount

- **Severity:** Medium (pending economic review)
- **Status:** OPEN
- **File:** `crates/p2p-node/src/settlement.rs`
- **Location:** quote amount calculation

**Observation:**

Quote settlement is calculated as:

`price * amount / PRICE_SCALE`

using integer division. Any remainder is truncated.

**Impact:**

Repeated trades with fractional fixed-point results may create systematic
rounding discrepancies between the economically expected quote amount and
the amount actually debited/credited.

**Recommendation:**

Define the protocol's exact fixed-point rounding rule and prove that
rounding cannot create an exploitable imbalance. Add boundary and
conservation tests.

**Status:** OPEN

### AUDIT-008 — Remote snapshot trades require independent validation

- **Severity:** High (pending verification)
- **Status:** OPEN — message-path boundary verified; snapshot path still requires verification

...

**Verification performed:**

The normal `record_trade()` path now performs structural trade validation
before calling `TradeBook::add_trade()`.

Therefore a structurally invalid trade cannot enter `TradeBook` merely
because its referenced orders are missing.

Order-dependent validation remains deferred to `OrderBook::apply_trade()`
when the referenced orders become available.

**Remaining question:**

The snapshot path in `sync.rs` must still be traced independently to confirm
that snapshot-supplied trades pass through the same structural validation
boundary before persistence.

**Status:** OPEN — narrowed pending snapshot-path verification

### AUDIT-009 — Snapshot trust boundary depends on order validation

- **Severity:** High (pending verification)
- **Status:** OPEN
- **File:** `crates/p2p-node/src/sync.rs`
- **Related:** AUDIT-002

**Observation:**

Snapshot orders are passed to `update_order_from_snapshot()`, which
ultimately calls `upsert_order()`.

Therefore the snapshot path inherits the order replacement/trust issue
identified in AUDIT-002.

**Required verification:**

Determine whether snapshots are authenticated/trusted and whether snapshot
orders are independently validated before being merged.

**Status:** OPEN cross reference rather then duplicate finding:

### AUDIT-010 — State snapshots lack an explicit consistency/version marker

- **Severity:** Medium
- **Status:** OPEN
- **File:** `crates/protocol/src/state.rs`

**Observation:**

`StateSnapshot` contains orders, trades, removed orders and applied trades,
but no snapshot sequence number, state version, or cryptographic state
commitment.

**Impact:**

Peers may be unable to determine whether a received snapshot represents
one coherent state or a mixture of states. Depending on the synchronization
protocol, stale or inconsistent snapshots may overwrite or merge with
newer local state.

**Recommendation:**

Define explicit snapshot/version semantics. Consider a monotonically
increasing version or equivalent consistency mechanism and reject stale
or incompatible snapshots.

**Status:** OPEN

| ID | Severity | Component | Status |
|---|---|---|---|
| — | — | — | — |

- [x] `crates/protocol/src/message.rs`
- [x] `crates/protocol/src/state.rs`
- [x] `crates/p2p-node/src/order_factory.rs`
- [x] `crates/p2p-node/src/orderbook.rs`
- [x] `crates/p2p-node/src/trades.rs`
- [x] `crates/p2p-node/src/balances.rs`
- [x] `crates/p2p-node/src/settlement.rs`
- [x] `crates/p2p-node/src/trade_state.rs`
- [x] `crates/p2p-node/src/sync.rs`

## Positive Controls

### CTRL-001 — Trade state transitions are explicitly validated

- **Status:** PASS
- **Files:**
  - `crates/protocol/src/message.rs`
  - `crates/p2p-node/src/trade_state.rs`

`Trade::transition()` requires `TradeState::can_transition_to()` to approve
every actual transition.

Duplicate state events are idempotent.

`advance_trade_to_state()` also uses the protocol transition rules rather
than directly assigning arbitrary states.

### CTRL-002 — Trade validation checks order consistency

- **Status:** PASS
- **File:** `crates/p2p-node/src/orderbook.rs`

`validate_trade()` verifies:

- buy order exists
- sell order exists
- correct Buy/Sell sides
- buyer matches buy-order trader
- seller matches sell-order trader
- trade market matches both orders
- trade price satisfies both limit prices
- trade amount does not exceed either order's remaining amount


### CTRL-003 — Snapshot application is transaction-like

- **Status:** PASS
- **File:** `crates/p2p-node/src/sync.rs`

Snapshot processing occurs against cloned order/trade books.

The live books are replaced only after processing and persistence succeed.

This reduces the risk of leaving the live in-memory state partially
updated when snapshot processing fails.

### CTRL-004 — Structural trade validation occurs before persistence

- **Status:** PASS
- **Files:**
  - `crates/p2p-node/src/trades.rs`
  - `crates/p2p-node/src/orderbook.rs`

`record_trade()` performs structural trade validation before inserting the
trade into `TradeBook`.

This prevents structurally invalid trades from being persisted when their
referenced orders do not yet exist.

Order-dependent validation remains deferred until the referenced orders are
available.

Regression coverage:
`record_trade_rejects_invalid_structure_before_persistence`

Workspace verification:
89 tests passed, 0 failed.
