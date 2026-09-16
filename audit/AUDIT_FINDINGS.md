
OpenDEX Source-Level Security Audit Findings

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Updated: 2026-09-04

1. Audit Scope

The current review covers:

crates/protocol/src/message.rs
crates/protocol/src/state.rs
crates/p2p-node/src/order_factory.rs
crates/p2p-node/src/orderbook.rs
crates/p2p-node/src/trades.rs
crates/p2p-node/src/balances.rs
crates/p2p-node/src/settlement.rs
crates/p2p-node/src/trade_state.rs
crates/p2p-node/src/sync.rs

The review focuses on:

Order creation and representation
Order validation
Order matching and trade creation
Trade validation and application
Trade-state transitions
Balance accounting and settlement
State synchronization
Authentication and authorization boundaries
Idempotency and replay resistance
Integer overflow/underflow
Distributed consistency
Persistence consistency
Resource-exhaustion considerations

Outside this source-level review:

Physical/network infrastructure
Host security
Key custody and operational security
Production deployment configuration
External services
Cryptographic key management outside the reviewed code
Live penetration testing

An independent external auditor is expected to perform infrastructure, deployment, and independent verification checks.

2. Executive Summary

OpenDEX has closed the previously identified application-level sender impersonation vulnerability and has regression coverage for the authenticated-peer/application-identity boundary.

Additional distributed-delivery and accounting protections have also been implemented and verified, including:

authenticated sender binding;
order ownership validation;
cancellation authorization;
trade participant authorization;
pending trade-state authorization;
stale trade-state protection;
removed-order resurrection protection;
stale synchronization protection;
settlement idempotency;
checked settlement arithmetic;
partial-fill settlement accounting;
distributed pending-trade handling;
out-of-order trade-state handling.

The audit remains open.

The two principal unresolved security boundaries are:

The trade actor authorization boundary has now been implemented and regression-tested.
Complete semantic validation of remotely supplied TradeCreated messages remains open.

The current architecture should therefore be treated as:

UNTRUSTED NETWORK INPUT
        ↓
Authenticated peer
        ↓
Application identity binding
        ↓
Authorization
        ↓
Trade / snapshot semantic validation   ← CURRENT MAJOR REVIEW BOUNDARY
        ↓
State reconciliation
        ↓
OrderBook / TradeBook mutation
        ↓
Persistence
        ↓
Settlement


The system is not ready for final security audit sign-off.

3. Active Security and Correctness Findings

AUDIT-009 / ARCH-004 — Status Update
Status: Remediated / Verification Passed

Summary
The identified trade/order integrity and deterministic matching concerns have been addressed and verified across the protocol, order-book, exchange, synchronization, and security test layers.

Implemented Controls
Deterministic trade identity

Trade IDs are derived deterministically from:

buy_order

sell_order

price

amount

SHA-256 is used as the derivation function.

Incoming trades are validated against the expected deterministic ID.

Forged/incorrect trade IDs are rejected before persistence or application.

Trade participant integrity

Trades explicitly identify:

maker

taker

buyer

seller

Trade structure validation ensures maker/taker are valid participants.

Buyer/seller identity is validated against the referenced orders during state synchronization.

Order/trade consistency

Trade validation checks referenced buy/sell orders.

Order side, market, trader identity, price limits, and remaining quantities are validated.

Invalid trades are rejected without partially mutating live state.

Deterministic matching

Matching uses deterministic price priority.

Buy orders select the lowest eligible sell price first.

Sell orders select the highest eligible buy price first.

Equal-price orders use deterministic UUID ordering.

Trade generation therefore converges independently of order arrival sequence.

Partial-fill handling

Temporary matching state tracks remaining quantities without prematurely mutating the live order book.

apply_trade() remains responsible for materializing order-book changes.

Multiple fills against a single resting order are covered by tests.

Trade state-machine enforcement

Trade lifecycle transitions are explicitly constrained.

Invalid transitions are rejected.

Duplicate transitions remain idempotent.

Complete lifecycle progression is covered by protocol tests.

State synchronization validation

State snapshots validate duplicate orders/trades, invalid references, participant identity, market consistency, timestamps, price constraints, and trade quantities.

Invalid snapshots do not partially mutate existing state.

Distributed/out-of-order event delivery is tested for convergence.

Security/authorization coverage

Forged senders, unauthorized cancellations, impersonated traders, unauthorized trade-state changes, and malformed snapshots are covered by security tests.

Pending trade state is bounded and stale/invalid state is rejected.

Verification
Executed:

cargo fmt --all
cargo test --workspace

Result:

BUILD: PASS

All workspace tests passed.

Test coverage included:

Protocol unit tests: 3 passed

Balance tests: 8 passed

CLI tests: 11 passed

Exchange tests: 1 passed

Network tests: 1 passed

Order-book tests: 26 passed

Security tests: 17 passed

Settlement tests: 6 passed

Trade lifecycle tests: 14 passed

Total: 87 tests passed, 0 failed.

Audit Disposition
The previously identified ARCH-004 concerns around deterministic trade identity, trade/order integrity, deterministic matching, lifecycle validation, and state synchronization have now been implemented with corresponding automated regression coverage.

Current disposition: REMEDIATED — TEST VERIFICATION PASSED

Remaining audit work, if any, should be limited to independent review of the implementation and confirmation that the documented controls correspond to the intended protocol/architecture requirements.

SEC-002 — Remote order cancellation authority
Status: 🟢 Remediated / Verified
Priority: Critical

Original weakness:
OrderCancelled messages previously allowed the receiver to reach OrderBook::remove_order() without establishing that the authenticated network sender was authorized to cancel the referenced order.

Remediation implemented:

GossipSub uses signed message authenticity, providing an authenticated libp2p PeerId.
Incoming OpenDexMessage.sender is verified against the authenticated PeerId.
OrderCancelled.trader must match the authenticated sender.
The referenced order must exist before cancellation is processed.
The authenticated sender must match the order's trader/owner.
Unauthorized cancellation requests are rejected before remove_order() is reached.
Unknown cancellations are safely ignored.
Cancellation and stale-order behaviour is covered by regression tests.
Verification:

The dedicated cancellation security tests pass:

rejects_order_cancellation_by_non_owner
handler_rejects_cancellation_by_non_owner
handler_rejects_forged_cancellation_trader_without_mutating_state
The complete workspace test suite also passes:

Security tests: 17 passed
Orderbook tests: 25 passed
Trade tests: 14 passed
Settlement tests: 6 passed
All remaining workspace tests: passed
Failures: 0
Conclusion:
The SEC-002 remote order cancellation authorization weakness has been remediated and adversarially verified. A remote peer can no longer cancel another trader's order merely by referencing its order_id.

Residual architectural note:
The current protocol uses the libp2p PeerId as the trader identity. If a separate cryptographic trader identity is introduced in future, an authenticated PeerId-to-trader identity binding will need to be defined and verified.

Audit disposition:
🟢 CLOSED — Remediated / Verified


Status: OPEN.

AUDIT-004 — Settlement Lifecycle Authorization

Severity: HIGH
Status: OPEN
Category: Security / Accounting Authorization
File: crates/p2p-node/src/settlement.rs
Location: settle_local_trade()

Description

Settlement performs economic mutation based on the supplied Trade.

The audit must establish whether settlement itself enforces the required trade lifecycle state or whether all callers are expected to guarantee that state.

Required verification
Trace every caller of settle_local_trade().
Determine which trade states authorize economic settlement.
Verify callers cannot bypass lifecycle authorization.
Verify remote trade-state messages cannot cause premature settlement.
Determine whether lifecycle authorization should be enforced directly at the settlement boundary.

If callers provide a complete and independently enforced authorization path, the issue may be downgraded to defense-in-depth.

Status: OPEN.

AUDIT-007 — Fixed-Point Settlement Rounding

Severity: MEDIUM
Status: OPEN
Category: Economic Correctness
File: crates/p2p-node/src/settlement.rs

Description

Quote settlement uses integer fixed-point arithmetic:

price * amount / PRICE_SCALE


Integer division truncates any remainder.

Required review

The audit must determine:

the authoritative protocol rounding rule;
whether truncation can create an economic imbalance;
whether asset conservation holds;
minimum-value behavior;
maximum-value behavior;
boundary behavior;
repeated fractional-trade behavior;
whether rounding systematically favors one participant.

Status: OPEN.

AUDIT-003 — Self-Trading Policy

Severity: MEDIUM
Status: OPEN
Category: Matching / Economic Correctness
File: crates/p2p-node/src/orderbook.rs
Location: match_order()

Description

The matching predicate checks compatible markets, opposite sides, and prices, but the audit has not established whether orders belonging to the same trader may match.

Required decision

If self-trading is not permitted:

reject candidates where existing.trader == incoming.trader;
add regression coverage proving self-crossing creates no trade.

If self-trading is intentional:

document the economic semantics;
document settlement semantics;
verify that buyer == seller is safe throughout the complete trade lifecycle.

Status: OPEN.

4. Architectural / Deferred Issues
ARCH-001 — Trade Application Has Overlapping Mutation Paths

Priority: HIGH
Status: INVESTIGATING

Trade application remains reachable through multiple paths, including:

Remote TradeCreated
    ↓
handle_trade_created()
    ↓
OrderBook::apply_trade()


and:

Remote OrderCreated
    ↓
matching
    ↓
OrderBook::apply_trade()


and synchronization/pending-trade processing.

Existing idempotency mechanisms reduce duplicate mutation risk.

The remaining concern is architectural ownership.

Required review
establish one clear owner for applying a trade;
define ownership of pending_trades;
define ownership of applied_trades;
define when orders are removed;
define when trade persistence occurs;
ensure local and remote trade application use the same invariants;
ensure synchronization cannot bypass those invariants;
ensure ARCH-004 validation occurs on every entry path.

No refactor should be performed merely to reduce duplication.

Status: INVESTIGATING.

BUG-001 — Per-Order Balance Reservations

Priority: HIGH
Status: DEFERRED

The current BalanceBook architecture primarily tracks aggregate available and locked balances.

An explicit relationship between locked funds and the order that created the reservation remains desirable for robust handling of:

multiple simultaneous orders;
partial fills;
cancellation;
settlement;
concurrent trades;
out-of-order distributed events.
Future work
associate reservations with order IDs;
consume only the relevant reservation during settlement;
unlock only the relevant reservation during cancellation;
support multiple simultaneous orders;
preserve partial-fill correctness;
persist reservations across restart;
test conflicting and out-of-order events;
verify duplicate settlement cannot consume a reservation twice.

Status: DEFERRED.

ARCH-002 — sync.rs Accesses OrderBook Internals

Priority: MEDIUM
Status: DEFERRED

Snapshot application currently accesses several OrderBook internals directly.

The architectural review should determine whether these accesses are necessary for synchronization or bypass domain invariants.

This review should be coordinated with:

BUG-SEC-002;
ARCH-004.

Status: DEFERRED.

ARCH-003 — Command Handler Responsibility Boundaries

Priority: MEDIUM
Status: DEFERRED

command_handler.rs has grown as command functionality has expanded.

Future review should examine responsibility boundaries around:

order creation;
balance locking;
publication;
local matching;
settlement;
trade-state transitions.

No split should be performed merely because the file is large.

Status: DEFERRED.

5. Positive Security Controls

The following controls have been implemented and/or verified.

Sender Authentication — RESOLVED

Application-level sender identity is now bound to the authenticated P2P peer.

Protections include:

claimed sender must match authenticated PeerId;
forged sender identities are rejected before mutation;
order creation requires sender/order-owner consistency;
cancellation requires order-owner authorization;
trade-state changes require trade participation;
unauthorized trade-state changes do not mutate state;
pending trade-state messages retain authenticated sender identity;
pending state is revalidated when the corresponding trade arrives.

Dedicated security regression coverage exists.

Trade-State Transition Validation — PASS

Trade::transition() requires TradeState::can_transition_to() to approve actual transitions.

Duplicate state events are idempotent.

advance_trade_to_state() uses protocol transition rules rather than arbitrary state assignment.

Basic Trade/Order Consistency Validation — PARTIAL

validate_trade() verifies:

referenced orders exist;
correct Buy/Sell sides;
buyer/seller ownership;
market consistency;
price constraints;
remaining quantity.

This is a strong baseline control.

It is not yet complete semantic validation of remotely supplied trades.

See AUDIT-009 / ARCH-004.

Snapshot Application Transaction-Like Behavior — PASS

Snapshot processing operates against cloned books.

Live books are replaced only after processing and persistence succeed.

This reduces the risk of partially mutating live in-memory state when snapshot processing fails.

This control does not establish snapshot authority or semantic validity.

Settlement Idempotency — PASS

A trade ID is recorded in settled_trades.

Duplicate settlement attempts are ignored, preventing the same trade from moving funds twice within the persisted balance state.

Settlement Arithmetic — PASS

Settlement validates:

quote multiplication overflow;
buyer locked quote balance;
seller locked base balance;
buyer base available-balance overflow;
seller quote available-balance overflow.

Checked balance arithmetic is used during settlement.

Distributed Trade Handling — RESOLVED

Trades arriving before their corresponding orders are retained and applied once the required orders become available.

Duplicate application is idempotent.

Pending Trade-State Handling — RESOLVED

Pending TradeStateChanged messages are retained when the corresponding trade has not yet arrived.

Implemented protections include:

duplicate pending states are ignored safely;
directly valid transitions replace previous pending state;
later valid happy-path states can replace earlier pending states;
exceptional states are not ordered using normal happy-path ranking;
authenticated sender identity is retained;
sender identity is revalidated when the trade arrives;
actual transitions continue through the protocol state machine.
Removed-Order Resurrection Protection — PASS

Orders recorded as removed cannot simply be reintroduced through later synchronization.

Stale Trade-State Protection — PASS

Stale synchronization cannot roll existing trade state backward.

6. Resolved Issues

The following previously identified issues are considered resolved or fixed:

ID	Issue	Status
BUG-SEC-001 / AUDIT-001	Application-level sender impersonation	RESOLVED
BUG-003	Distributed TradeCreated application	RESOLVED
BUG-004	Trade state arriving before TradeCreated	RESOLVED
BUG-005	Local settlement applied to non-participants	RESOLVED
BUG-006	Duplicate settlement	RESOLVED
BUG-007	Partial-fill settlement accounting	RESOLVED
BUG-008	Quote amount overflow	RESOLVED
BUG-009	GossipSub timing-dependent test	RESOLVED
BL-2026-08-31-001	Pending trade-state handling	FIXED

These resolved issues should remain historically documented but should not be treated as active findings.

7. Current Finding Summary
ID	Severity	Status
AUDIT-009 / ARCH-004	CRITICAL	🔴 OPEN
AUDIT-008 / BUG-SEC-002	HIGH / potentially CRITICAL	🔴 OPEN
AUDIT-002	HIGH	🔴 OPEN
AUDIT-004	HIGH	🔴 OPEN
AUDIT-007	MEDIUM	🟠 OPEN
AUDIT-003	MEDIUM	🟠 OPEN
ARCH-001	HIGH	🟠 INVESTIGATING
BUG-001	HIGH	🟡 DEFERRED
ARCH-002	MEDIUM	🟡 DEFERRED
ARCH-003	MEDIUM	🟡 DEFERRED
8. Current Audit Priorities

The current priority order is:

HIGH / CRITICAL — Synchronization authority and snapshot validation (BUG-SEC-002)

HIGH — Order UUID replacement and snapshot order semantics

HIGH — Settlement lifecycle authorization

MEDIUM — Fixed-point rounding/economic correctness

MEDIUM — Self-trading policy

HIGH / DEFERRED — Per-order balance reservations

MEDIUM — Replay and stale-event resistance

MEDIUM — Resource-exhaustion limits

MEDIUM — Adversarial multi-node testing

MEDIUM — Persistence/restart behavior

MEDIUM — Command-handler coupling

ARCH-004 / AUDIT-009 — Remediated / Verification Passed


9. Audit Readiness Assessment

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The sender-authentication vulnerability has been resolved and independently regression-tested within the current source-level review.

Distributed trade delivery, pending trade-state handling, settlement idempotency, settlement arithmetic safety, partial-fill accounting, and stale-state protections have also received implementation and regression verification.

However, the following remain unresolved:

CRITICAL
Complete semantic validation of remotely supplied TradeCreated messages

HIGH / POTENTIALLY CRITICAL
Synchronization authority and authoritative StateSnapshot validation

HIGH
Order UUID replacement authorization and snapshot order semantics

HIGH
Settlement lifecycle authorization


The principal security architecture currently remains:

Authenticated peer
        ↓
Application identity binding
        ↓
Authorization
        ↓
        ?
Trade / snapshot semantic validation
        ↓
State reconciliation
        ↓
OrderBook / TradeBook mutation
        ↓
Persistence
        ↓
Settlement


The ? represents the next major security review boundary.

The next milestone must establish and test:

the complete set of invariants governing remotely supplied TradeCreated messages; and
the synchronization authority model and authoritative StateSnapshot invariants.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security requirement makes the refactor necessary.

10. External Auditor Handoff

The external auditor should independently verify:

complete network-to-trade-to-settlement call paths;
complete remote trade semantic validation;
synchronization authorization and state authority;
snapshot validation and freshness;
replay and stale-event handling;
order resurrection prevention;
persistence and crash recovery;
cross-node consistency;
accounting conservation;
host and network security;
production deployment configuration;
key management and custody;
live penetration and adversarial testing.

The source-level audit should remain open until the critical semantic validation and synchronization authority boundaries have been resolved and regression-tested.

Current conclusion: OpenDEX has made meaningful progress on authentication, distributed delivery, and accounting safety, but the implementation is not yet ready for final security audit approval.

This version should now be treated as the current findings record, while the older settlement-focused AUDIT_FINDINGS.md should no longer be used as the authoritative status. The next code-review checkpoint should be ARCH-004, followed by BUG-SEC-002.
