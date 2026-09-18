Yes. Based on the **current code changes you committed** and the test results you supplied, I’d update the checklist so it no longer carries the already-remediated AUDIT-009/AUDIT-008 material as open findings.

 One important correction: the current findings and checklist are internally inconsistent around **AUDIT-008/AUDIT-009**. The latest implementation adds deterministic trade-ID validation and snapshot trade-identity validation, and your 29 orderbook + 17 security + other tests passed. I would record those as **verified/remediated**, while keeping the genuinely unresolved items open.

 Here is a cleaned-up version you can replace `audit/AUDIT_CHECKLIST.md` with:

 # OpenDEX Audit Checklist

 ## 1\. Current Audit Scope

 The current source-level audit covers:

 - `crates/protocol/src/message.rs`
- `crates/protocol/src/state.rs`
- `crates/p2p-node/src/order_factory.rs`
- `crates/p2p-node/src/orderbook.rs`
- `crates/p2p-node/src/trades.rs`
- `crates/p2p-node/src/balances.rs`
- `crates/p2p-node/src/settlement.rs`
- `crates/p2p-node/src/trade_state.rs`
- `crates/p2p-node/src/sync.rs`
- `crates/p2p-node/src/exchange.rs`

 The review covers:

 - Order creation and representation
- Order validation and ownership
- Order cancellation
- Order matching and deterministic trade creation
- Trade identity and semantic validation
- Trade application and idempotency
- Trade-state transitions
- Balance accounting and settlement
- State snapshot creation and application
- Authentication and authorization boundaries
- Replay and stale-event resistance
- Integer overflow/underflow
- Distributed consistency
- Persistence consistency
- Resource-exhaustion considerations

 ### Out of scope

 - Physical/network infrastructure
- Host security
- Key custody and operational security
- Production deployment configuration
- External services
- Cryptographic key management outside the reviewed code
- Live penetration testing
- Production infrastructure configuration

 An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.

---

 # 2\. Audit Status

 - [ ] Audit complete
- [ ] All findings reviewed
- [ ] Critical findings resolved
- [ ] High findings resolved
- [ ] Medium findings resolved
- [ ] Low findings resolved
- [ ] External audit completed
- [ ] Final security sign-off

 **Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF**

 The implementation has passed the current automated regression suite, but several architectural and economic questions remain open.

---

 # 3\. Audit Areas

 ## 3.1 Protocol

 - [x] Message integrity
- [x] Serialization/deserialization
- [x] Price/amount validation
- [x] Timestamp validation
- [x] Authenticated sender binding

 ## 3.2 Orders

 - [x] Order creation
- [x] Order ownership validation
- [x] Order cancellation authorization
- [x] Duplicate order handling
- [x] Quantity accounting
- [x] Removed-order resurrection protection
- [ ] Complete snapshot order-authority model

 ## 3.3 Matching

 - [x] Price matching
- [x] Deterministic price priority
- [x] Deterministic UUID tie-breaking
- [x] Partial fills
- [x] Trade identity
- [x] Trade duplication/replay handling
- [ ] Self-trading policy
- [x] Overflow/underflow checks

 ## 3.4 Balances & Settlement

 - [x] Available/locked accounting
- [x] Buyer settlement
- [x] Seller settlement
- [x] Settlement idempotency
- [x] Checked arithmetic
- [x] Partial-fill settlement accounting
- [ ] Asset-conservation proof across all rounding cases
- [ ] Settlement lifecycle authorization
- [ ] Fixed-point rounding policy
- [ ] Per-order reservation architecture

 ## 3.5 Trade State

 - [x] Valid transitions
- [x] Invalid transitions rejected
- [x] Exceptional states
- [x] State replay/idempotency
- [x] Skipped-state handling
- [x] Stale-state protection
- [x] Unauthorized trade-state changes rejected

 ## 3.6 State Synchronization

 - [x] Snapshot structural validation
- [x] Duplicate trade detection
- [x] Duplicate order handling
- [x] Removed-order protection
- [x] Applied-trade handling
- [x] Transaction-like snapshot application
- [x] Snapshot persistence consistency
- [x] Trade identity validation
- [x] Trade/order semantic validation
- [ ] Snapshot authority model
- [ ] Snapshot freshness/version semantics
- [ ] Complete order replacement authorization

---

 # 4\. Resolved / Verified Findings

 ## AUDIT-001 — Message Sender Authentication

 **Severity:** High\
 **Status:** CLOSED / VERIFIED

 ### Finding

 `OpenDexMessage.sender` is application-level identity and must not be trusted merely because it is contained in a received message.

 ### Remediation

 Incoming messages now bind the claimed application sender to the authenticated libp2p peer.

 Protections include:

 - claimed sender must match the authenticated peer identity;
- forged sender identities are rejected;
- order creation requires sender/order-owner consistency;
- cancellation requires owner authorization;
- trade-state changes require trade participation;
- pending trade-state messages retain authenticated sender identity;
- sender identity is revalidated when the corresponding trade arrives.

 ### Verification

 Security regression coverage includes:

 - forged sender rejection;
- impersonated trader rejection;
- forged cancellation rejection;
- unauthorized trade-state rejection;
- authenticated sender acceptance.

 **Disposition:** CLOSED / VERIFIED

---

 ## AUDIT-008 — Remote Snapshot Trade Validation

 **Severity:** High\
 **Status:** CLOSED / VERIFIED

 ### Finding

 Remote state snapshots must not be allowed to introduce structurally or semantically invalid trades into the local state.

 ### Remediation

 `validate_state_snapshot()` now validates snapshot trades before they are accepted.

 Validation includes:

 - trade structure;
- deterministic trade identity;
- duplicate trade IDs;
- participant relationships;
- referenced orders;
- order side;
- market consistency;
- price constraints;
- trade quantity;
- maker/taker integrity.

 Snapshot application remains transaction-like and invalid snapshots do not partially mutate live state.

 ### Verification

 Regression coverage includes:

 - invalid snapshot rejection;
- invalid maker rejection;
- conflicting snapshot order terms;
- snapshot atomicity;
- materialized snapshot handling;
- repeated snapshot idempotency;
- trade-before-order handling;
- removed-order resurrection protection.

 **Disposition:** CLOSED / VERIFIED

---

 ## AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching

 **Severity:** Critical\
 **Status:** CLOSED / VERIFIED

 ### Finding

 Distributed trade creation required stronger guarantees that independently received trades represent the same deterministic economic event and correspond to their referenced orders.

 ### Remediation

 Trade IDs are deterministically derived from:

 - buy order UUID;
- sell order UUID;
- trade price;
- trade amount.

 SHA-256 is used as the derivation function.

 Incoming trades are checked against the expected deterministic ID before persistence/application.

 Trade validation additionally verifies:

 - maker/taker structure;
- buyer/seller identity;
- referenced order identity;
- order side;
- market;
- price limits;
- remaining quantities.

 Matching is deterministic:

 - buy orders select the lowest eligible sell price;
- sell orders select the highest eligible buy price;
- equal-price orders use deterministic UUID ordering.

 ### Verification

 The implementation is covered by regression tests for:

 - invalid deterministic trade IDs;
- deterministic matching;
- different arrival orders converging to the same result;
- duplicate trade application;
- partial fills;
- multiple fills against one order;
- invalid trade/order relationships;
- invalid trade quantities;
- invalid trade prices;
- out-of-order trade delivery;
- trade-before-order delivery.

 Current supplied verification:

 - Orderbook tests: 29 passed
- Exchange tests: 1 passed
- Security tests: 17 passed
- Trade-state tests: 14 passed
- Settlement tests: 6 passed
- Balance tests: 8 passed
- CLI tests: 11 passed
- Network tests: 1 passed
- Total supplied current suite: 87 individual integration/unit tests plus doctest targets with no failures

 **Disposition:** CLOSED / VERIFIED

---

 ## SEC-002 — Remote Order Cancellation Authority

 **Severity:** Critical\
 **Status:** CLOSED / VERIFIED

 ### Finding

 Remote cancellation must be authorized against the authenticated sender and the owner of the referenced order.

 ### Remediation

 Cancellation now requires:

 - authenticated sender identity;
- matching `OrderCancelled.trader`;
- existing referenced order;
- sender matching the order owner.

 Unauthorized cancellation is rejected before order mutation.

 ### Verification

 Covered by:

 - `rejects_order_cancellation_by_non_owner`
- `handler_rejects_cancellation_by_non_owner`
- `handler_rejects_forged_cancellation_trader_without_mutating_state`

 **Disposition:** CLOSED / VERIFIED

---

 # 5\. Positive Security Controls

 ## CTRL-001 — Trade-State Transition Validation

 **Status:** PASS

 `Trade::transition()` requires `TradeState::can_transition_to()` to approve state changes.

 Duplicate state events are idempotent.

 Skipped-state handling follows protocol transition rules.

---

 ## CTRL-002 — Trade/Order Consistency Validation

 **Status:** PASS

 Trade validation verifies:

 - referenced buy order;
- referenced sell order;
- correct Buy/Sell sides;
- buyer matches buy-order trader;
- seller matches sell-order trader;
- market consistency;
- price limits;
- remaining quantities;
- deterministic trade identity.

---

 ## CTRL-003 — Snapshot Transaction-Like Application

 **Status:** PASS

 Snapshot processing occurs against cloned state.

 Live state is replaced only after validation, processing, and persistence succeed.

 Invalid snapshots therefore do not partially mutate the live books.

---

 ## CTRL-004 — Structural Trade Validation Before Persistence

 **Status:** PASS

 Trade recording validates structural correctness before inserting the trade into `TradeBook`.

 This prevents malformed trades from entering persistent trade state merely because their referenced orders are not yet available.

 Order-dependent validation is subsequently performed when the referenced orders become available.

---

 ## CTRL-005 — Settlement Trade-Level Idempotency

 **Status:** PASS

 Settled trade IDs are recorded.

 Duplicate settlement attempts are ignored.

 This prevents the same trade from consuming funds more than once within the persisted settlement state.

---

 ## CTRL-006 — Settlement Arithmetic Validation

 **Status:** PASS

 Settlement checks:

 - quote multiplication overflow;
- buyer locked quote balance;
- seller locked base balance;
- buyer available-base overflow;
- seller available-quote overflow.

 Balance mutations occur only after validation succeeds.

---

 ## CTRL-007 — Distributed Pending-Trade Handling

 **Status:** PASS

 Trades received before their referenced orders are retained and applied when the required orders arrive.

 Duplicate application is idempotent.

---

 ## CTRL-008 — Pending Trade-State Authorization

 **Status:** PASS

 Pending trade-state messages:

 - retain authenticated sender identity;
- reject unauthorized participants;
- avoid unsafe state regression;
- retain valid later happy-path states;
- revalidate authorization when the trade arrives.

---

 ## CTRL-009 — Removed-Order Resurrection Protection

 **Status:** PASS

 Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.

---

 ## CTRL-010 — Stale Trade-State Protection

 **Status:** PASS

 Stale synchronization cannot move an existing trade backward through its lifecycle.

---

 # 6\. Active Findings

 ## AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics

 **Severity:** High\
 **Status:** OPEN

 ### Issue

 Orders sharing an existing UUID require careful handling when incoming data contains different immutable attributes.

 The current implementation now rejects conflicting immutable order terms rather than blindly replacing the existing order.

 However, the broader audit question remains:

 > What authority does a remote snapshot have to modify an existing order?

 ### Required verification

 Establish and document:

 - which order fields are immutable;
- whether remote `OrderCreated` messages can modify existing orders;
- whether snapshots may reduce quantities;
- whether snapshots may change trader/market/side/price;
- who has authority to update an order;
- whether an authenticated peer is sufficient authority;
- how stale snapshots interact with newer local state.

 ### Current evidence

 Regression coverage now includes:

 - conflicting snapshot order terms;
- stale order delivery;
- removed-order resurrection;
- snapshot quantity reduction.

 **Disposition:** OPEN pending explicit protocol authority decision.

---

 ## AUDIT-003 — Self-Trading Policy

 **Severity:** Medium\
 **Status:** OPEN

 ### Issue

 The matching engine must establish whether a trader is permitted to trade against their own orders.

 ### Required decision

 If self-trading is prohibited:

 - reject matching orders belonging to the same trader;
- add a regression test;
- verify distributed matching applies the same rule.

 If self-trading is intentional:

 - document the economic semantics;
- document settlement behavior;
- verify buyer/seller identity handling;
- verify fees, accounting, and downstream state handling.

 **Disposition:** OPEN — protocol/economic policy decision required.

---

 ## AUDIT-004 — Settlement Lifecycle Authorization

 **Severity:** High\
 **Status:** OPEN

 ### Issue

 `settle_local_trade()` performs economic mutation, but the audit must establish whether settlement itself requires a settlement-authorized trade state.

 ### Required verification

 Trace every caller of `settle_local_trade()` and establish:

 - which state permits settlement;
- whether all callers enforce that state;
- whether remote trade-state messages can cause premature settlement;
- whether settlement can be invoked independently of the state machine;
- whether settlement authorization should be enforced directly at the settlement boundary.

 If every caller independently guarantees authorization, this may become a defense-in-depth finding.

 **Disposition:** OPEN.

---

 ## AUDIT-007 — Fixed-Point Settlement Rounding

 **Severity:** Medium\
 **Status:** OPEN

 ### Issue

 Quote settlement uses:

 `price * amount / PRICE_SCALE`

 Integer division truncates the remainder.

 ### Required verification

 Establish:

 - authoritative protocol rounding rule;
- whether truncation creates economic imbalance;
- minimum-value behavior;
- maximum-value behavior;
- boundary behavior;
- repeated fractional-trade behavior;
- whether rounding systematically favors either participant;
- asset conservation under the defined rounding rule.

 ### Required tests

 Add boundary and conservation tests covering fractional results.

 **Disposition:** OPEN.

---

 ## AUDIT-010 — Snapshot Consistency / Version Semantics

 **Severity:** Medium\
 **Status:** OPEN

 ### Issue

 `StateSnapshot` does not currently provide an explicit state sequence/version or equivalent state commitment.

 ### Required verification

 Determine whether the existing synchronization protocol already provides sufficient freshness and consistency guarantees.

 If not, define explicit snapshot semantics, potentially including:

 - monotonic snapshot version;
- generation number;
- event sequence;
- state commitment;
- freshness rule;
- stale snapshot rejection.

 The mechanism should prevent a peer from unknowingly accepting an internally inconsistent or stale state.

 **Disposition:** OPEN pending synchronization architecture review.

---

 # 7\. Architectural Findings

 ## ARCH-001 — Trade Application Has Multiple Mutation Paths

 **Priority:** High\
 **Status:** INVESTIGATING

 Trade application remains reachable through multiple paths, including:

```
Remote TradeCreated
    ↓
handle_trade_created()
    ↓
OrderBook::apply_trade()
```

 and:

```
Remote OrderCreated
    ↓
matching
    ↓
OrderBook::apply_trade()
```

 as well as synchronization and pending-trade processing.

 ### Required review

 Establish:

 - one clear owner for applying a trade;
- ownership of `pending_trades`;
- ownership of `applied_trades`;
- when orders are removed;
- when trades are persisted;
- whether local and remote trade application share identical invariants;
- whether synchronization can bypass those invariants;
- whether all entry paths enforce trade identity and semantic validation.

 No refactor should be performed merely to reduce duplication.

 **Disposition:** INVESTIGATING.

---

 ## ARCH-002 — `sync.rs` Accesses OrderBook Internals

 **Priority:** Medium\
 **Status:** DEFERRED

 Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

 This review should be coordinated with:

 - AUDIT-002;
- AUDIT-010;
- ARCH-004.

 No refactor should be performed without a demonstrated correctness or security requirement.

---

 ## ARCH-003 — Command Handler Responsibility Boundaries

 **Priority:** Medium\
 **Status:** DEFERRED

 Review responsibility boundaries around:

 - order creation;
- balance locking;
- publication;
- local matching;
- settlement;
- trade-state transitions.

 No split should be performed merely because the command handler is large.

---

 # 8\. Deferred Correctness / Architecture

 ## BUG-001 — Per-Order Balance Reservations

 **Priority:** High\
 **Status:** DEFERRED

 The current BalanceBook primarily tracks aggregate available and locked balances.

 A future reservation model should establish an explicit relationship between locked funds and the order responsible for the reservation.

 Required future work:

 - associate reservations with order IDs;
- consume only the relevant reservation during settlement;
- unlock only the relevant reservation during cancellation;
- support multiple simultaneous orders;
- preserve partial-fill correctness;
- persist reservations across restart;
- handle conflicting/out-of-order events;
- ensure duplicate settlement cannot consume a reservation twice.

---

 # 9\. Resolved Historical Findings

 The following findings are considered resolved and should remain documented for audit history:

 | ID | Issue | Status |
| --- | --- | --- |
| BUG-SEC-001 / AUDIT-001 | Application-level sender impersonation | RESOLVED |
| SEC-002 | Remote order cancellation authority | RESOLVED |
| BUG-003 | Distributed TradeCreated application | RESOLVED |
| BUG-004 | Trade state arriving before TradeCreated | RESOLVED |
| BUG-005 | Local settlement applied to non-participants | RESOLVED |
| BUG-006 | Duplicate settlement | RESOLVED |
| BUG-007 | Partial-fill settlement accounting | RESOLVED |
| BUG-008 | Quote amount overflow | RESOLVED |
| BUG-009 | GossipSub timing-dependent test | RESOLVED |
| BL-2026-08-31-001 | Pending trade-state handling | FIXED |
| AUDIT-008 | Remote snapshot trade validation | CLOSED / VERIFIED |
| AUDIT-009 / ARCH-004 | Trade identity/order integrity/deterministic matching | CLOSED / VERIFIED |

---

 # 10\. Current Finding Summary

 | ID | Severity | Status |
| --- | --- | --- |
| AUDIT-002 | High | 🔴 OPEN |
| AUDIT-004 | High | 🔴 OPEN |
| AUDIT-003 | Medium | 🟠 OPEN |
| AUDIT-007 | Medium | 🟠 OPEN |
| AUDIT-010 | Medium | 🟠 OPEN |
| ARCH-001 | High | 🟠 INVESTIGATING |
| BUG-001 | High | 🟡 DEFERRED |
| ARCH-002 | Medium | 🟡 DEFERRED |
| ARCH-003 | Medium | 🟡 DEFERRED |

**Previously critical trade semantic-validation finding:** CLOSED / VERIFIED

---

 # 11\. Current Audit Priorities

 The next review sequence should be:

 1. **AUDIT-002 — Order UUID / snapshot order authority**
2. **AUDIT-004 — Settlement lifecycle authorization**
3. **AUDIT-010 — Snapshot consistency and freshness semantics**
4. **AUDIT-003 — Self-trading policy**
5. **AUDIT-007 — Fixed-point rounding and conservation**
6. **ARCH-001 — Trade application ownership**
7. **BUG-001 — Per-order balance reservations**
8. **ARCH-002 / ARCH-003 — Deferred architecture reviews**

 The priority is to resolve protocol invariants before undertaking broad architectural refactoring.

---

 # 12\. Verification Record

 The current implementation has been verified with:

```
cargo fmt --check
cargo test -p p2p-node
git diff --check
```

 The supplied `cargo test -p p2p-node` run completed successfully with:

 - Balance tests: 8 passed
- CLI tests: 11 passed
- Exchange tests: 1 passed
- Network tests: 1 passed
- Orderbook tests: 29 passed
- Security tests: 17 passed
- Settlement tests: 6 passed
- Trade-state tests: 14 passed
- Unit-test targets: passed
- Doc-test target: passed
- Failures: 0

 The repository was subsequently committed in two commits:

```
1997c06 Harden distributed trade and snapshot validation
0172e2b Update audit findings for verified remediations
```

 Working tree status at the checkpoint:

```
nothing to commit, working tree clean
```

---

 # 13\. Audit Readiness

 **Current status: NOT READY FOR FINAL SECURITY SIGN-OFF**

Current Findings Summary
Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current source review has verified several previously open findings and
reduced the remaining substantive audit scope to three findings.

AUDIT-001 — Message sender authentication
Severity: High

Status: CLOSED / VERIFIED ✅

Files:

crates/p2p-node/src/message_handler.rs

crates/protocol/src/message.rs

Verification:

Incoming OpenDexMessage messages are checked against the authenticated
P2P peer identity using is_authenticated_sender().

OrderCreated messages additionally verify that the order trader matches
the authenticated sender using is_order_owner().

Unauthorized messages/orders are rejected before protocol processing.

Disposition: The previously identified sender-impersonation path is
mitigated at the message-handler authentication boundary.

AUDIT-002 — Order UUID reuse permits unvalidated order replacement
Severity: High

Status: CLOSED / VERIFIED ✅

Files:

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

Verification:

Remote OrderCreated messages do not directly perform an unrestricted
upsert_order() operation.

handle_order_created() checks whether the order UUID is already known and
does not replace an existing order.

New remote orders pass through Exchange::submit_order(), which verifies:

actor == order.trader

Snapshot synchronization uses the separate
update_order_from_snapshot() path.

That path:

rejects conflicting trader, market, side, or price fields;

rejects snapshot quantity increases;

ignores orders already marked as removed;

only permits the snapshot quantity to remain the same or decrease.

Disposition: The original unrestricted remote UUID-replacement concern
is not supported by the current production call paths.

AUDIT-003 — Matching engine permits self-trading
Severity: Medium

Status: OPEN

File: crates/p2p-node/src/orderbook.rs

Location: match_order()

Current assessment:

The matching path has not yet been conclusively verified to reject
orders belonging to the same trader.

Required verification:

Inspect the complete match_order() candidate predicate and determine whether
it explicitly rejects:

existing.trader == incoming.trader

If no such check exists, add the protocol-required self-trade prevention
rule and regression coverage.

Status: OPEN pending direct inspection of match_order().

AUDIT-004 — Settlement does not independently require terminal trade state
Severity: Low / Defense-in-depth

Status: CLOSED / VERIFIED ✅

Files:

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/command_handler.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/trade_state.rs

Verification:

All identified production settlement callers first advance the trade using
advance_trade_to_state().

Settlement is invoked only after the requested/accepted state is
TradeState::Settled.

The local command path performs:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
if state == Settled
        ↓
settle_local_trade(...)

The remote TradeStateChanged path performs the same ordering after
verifying that the sender is a participant in the trade.

Pending trade-state changes follow the same transition-before-settlement
pattern.

advance_trade_to_state() enforces the protocol's permitted trade-state
transitions.

Disposition: No confirmed settlement-before-terminal-state vulnerability
was identified in the current production call paths.

Defense-in-depth recommendation: settle_local_trade() may additionally
verify trade.state == TradeState::Settled internally so future callers
cannot accidentally bypass the lifecycle requirement.

AUDIT-007 — Fixed-point settlement truncates quote amount
Severity: Medium

Status: OPEN

File: crates/p2p-node/src/settlement.rs

Observation:

Quote settlement is calculated using integer division:

price * amount / PRICE_SCALE

Any remainder is truncated.

Potential impact:

Repeated trades with fractional fixed-point results may produce rounding
differences between the protocol's mathematical trade value and the actual
settled amount.

Required verification:

Define the protocol's exact rounding rule.

Determine whether truncation is intentional.

Prove asset conservation under the defined rule.

Test boundary and fractional-price/amount cases.

Determine whether repeated rounding can produce a meaningful economic
imbalance.

Status: OPEN pending economic/conservation review.

AUDIT-008 — Snapshot / Synchronization Validation
Severity: N/A

Status: CLOSED / VERIFIED ✅

File: crates/p2p-node/src/sync.rs

Verification:

Remote StateResponse messages are validated before snapshot application.

Validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts/prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned order/trade books and is persisted
before committing the merged state to the live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED.

AUDIT-009 — Snapshot trust boundary depends on order validation
Severity: High

Status: CLOSED / VERIFIED ✅

Files:

crates/p2p-node/src/sync.rs

crates/p2p-node/src/orderbook.rs

Verification:

Snapshot orders are processed through
update_order_from_snapshot() rather than unrestricted upsert_order().

Existing order identity fields are protected:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Removed orders cannot be resurrected.

Snapshot processing is performed transactionally against temporary state
before the live books are committed.

Disposition: The AUDIT-002 order-replacement concern does not propagate
through the snapshot synchronization path.

AUDIT-010 — State snapshots lack explicit consistency/version marker
Severity: Medium

Status: OPEN

File: crates/protocol/src/state.rs

Observation:

StateSnapshot contains orders, trades, removed orders, and applied trades,
but does not currently contain an explicit snapshot sequence number,
monotonic state version, or equivalent state commitment.

Potential impact:

Without explicit version semantics, peers may have difficulty determining
whether a snapshot is stale or represents a coherent point-in-time state.

Depending on synchronization behavior, stale snapshots could potentially be
merged with newer local state.

Required verification/design review:

Determine the intended synchronization consistency model and establish whether
the protocol needs:

a monotonic snapshot/state version;

stale-snapshot rejection;

explicit snapshot ancestry;

or another consistency mechanism.

Status: OPEN pending protocol consistency review.

Current Findings Table
ID	Severity	Component	Status
AUDIT-001	High	Message authentication	CLOSED / VERIFIED
AUDIT-002	High	Order UUID handling	CLOSED / VERIFIED
AUDIT-003	Medium	Matching / self-trading	OPEN
AUDIT-004	Low / Defense-in-depth	Settlement state enforcement	CLOSED / VERIFIED
AUDIT-007	Medium	Fixed-point settlement	OPEN
AUDIT-008	—	Snapshot validation	CLOSED / VERIFIED
AUDIT-009	High	Snapshot order trust	CLOSED / VERIFIED
AUDIT-010	Medium	Snapshot consistency/versioning	OPEN

Current Audit Position
Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

Verified / closed
AUDIT-001 — Message sender authentication

AUDIT-002 — Order UUID replacement

AUDIT-004 — Settlement lifecycle enforcement through production callers

AUDIT-008 — Snapshot validation

AUDIT-009 — Snapshot order trust boundary

Remaining substantive findings
AUDIT-003 — Self-trading

AUDIT-007 — Fixed-point rounding/conservation

AUDIT-010 — Snapshot consistency/versioning

Defense-in-depth recommendations
Add an explicit TradeState::Settled check inside settle_local_trade().

Continue maintaining regression tests around authentication, snapshot
atomicity, order immutability, trade transitions, and settlement
idempotency.

Next audit action: Complete direct inspection of match_order() for
self-trade prevention and update AUDIT-003 based on the actual matching
predicate.
