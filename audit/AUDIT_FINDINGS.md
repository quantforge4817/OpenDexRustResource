OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

The remaining substantive protocol questions are:

AUDIT-007 — fixed-point settlement rounding and economic conservation;

AUDIT-010 — snapshot freshness, consistency, and version semantics.

ARCH-001 remains under investigation as an architectural ownership question.

BUG-001, ARCH-002, and ARCH-003 remain deferred.

Current conclusion:

OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The next substantive audit work should focus on AUDIT-007, followed by AUDIT-010, before broad architectural refactoring.
OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

The remaining substantive protocol questions are:

AUDIT-007 — fixed-point settlement rounding and economic conservation;

AUDIT-010 — snapshot freshness, consistency, and version semantics.

ARCH-001 remains under investigation as an architectural ownership question.

BUG-001, ARCH-002, and ARCH-003 remain deferred.

Current conclusion:

OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The next substantive audit work should focus on AUDIT-007, followed by AUDIT-010, before broad architectural refactoring.
OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

The remaining substantive protocol questions are:

AUDIT-007 — fixed-point settlement rounding and economic conservation;

AUDIT-010 — snapshot freshness, consistency, and version semantics.

ARCH-001 remains under investigation as an architectural ownership question.

BUG-001, ARCH-002, and ARCH-003 remain deferred.

Current conclusion:

OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The next substantive audit work should focus on AUDIT-007, followed by AUDIT-010, before broad architectural refactoring.
OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

The remaining substantive protocol questions are:

AUDIT-007 — fixed-point settlement rounding and economic conservation;

AUDIT-010 — snapshot freshness, consistency, and version semantics.

ARCH-001 remains under investigation as an architectural ownership question.

BUG-001, ARCH-002, and ARCH-003 remain deferred.

Current conclusion:

OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The next substantive audit work should focus on AUDIT-007, followed by AUDIT-010, before broad architectural refactoring.
OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

The remaining substantive protocol questions are:

AUDIT-007 — fixed-point settlement rounding and economic conservation;

AUDIT-010 — snapshot freshness, consistency, and version semantics.

ARCH-001 remains under investigation as an architectural ownership question.

BUG-001, ARCH-002, and ARCH-003 remain deferred.

Current conclusion:

OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The next substantive audit work should focus on AUDIT-007, followed by AUDIT-010, before broad architectural refactoring.
OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

The remaining substantive protocol questions are:

AUDIT-007 — fixed-point settlement rounding and economic conservation;

AUDIT-010 — snapshot freshness, consistency, and version semantics.

ARCH-001 remains under investigation as an architectural ownership question.

BUG-001, ARCH-002, and ARCH-003 remain deferred.

Current conclusion:

OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The next substantive audit work should focus on AUDIT-007, followed by AUDIT-010, before broad architectural refactoring.
OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

The remaining substantive protocol questions are:

AUDIT-007 — fixed-point settlement rounding and economic conservation;

AUDIT-010 — snapshot freshness, consistency, and version semantics.

ARCH-001 remains under investigation as an architectural ownership question.

BUG-001, ARCH-002, and ARCH-003 remain deferred.

Current conclusion:

OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The next substantive audit work should focus on AUDIT-007, followed by AUDIT-010, before broad architectural refactoring.
OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

The remaining substantive protocol questions are:

AUDIT-007 — fixed-point settlement rounding and economic conservation;

AUDIT-010 — snapshot freshness, consistency, and version semantics.

ARCH-001 remains under investigation as an architectural ownership question.

BUG-001, ARCH-002, and ARCH-003 remain deferred.

Current conclusion:

OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The next substantive audit work should focus on AUDIT-007, followed by AUDIT-010, before broad architectural refactoring.
OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

The remaining substantive protocol questions are:

AUDIT-007 — fixed-point settlement rounding and economic conservation;

AUDIT-010 — snapshot freshness, consistency, and version semantics.

ARCH-001 remains under investigation as an architectural ownership question.

BUG-001, ARCH-002, and ARCH-003 remain deferred.

Current conclusion:

OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The next substantive audit work should focus on AUDIT-007, followed by AUDIT-010, before broad architectural refactoring.
OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

OpenDEX Audit Checklist

Audit type: Source-level security, correctness, accounting, and distributed-state review
Project: OpenDEX
Status: In progress
Last updated: 2026-09-19


1. Current Audit Scope

The current source-level audit covers:

crates/protocol/src/message.rs

crates/protocol/src/state.rs

crates/p2p-node/src/order_factory.rs

crates/p2p-node/src/orderbook.rs

crates/p2p-node/src/trades.rs

crates/p2p-node/src/balances.rs

crates/p2p-node/src/settlement.rs

crates/p2p-node/src/trade_state.rs

crates/p2p-node/src/sync.rs

crates/p2p-node/src/exchange.rs

crates/p2p-node/src/message_handler.rs

crates/p2p-node/src/command_handler.rs

The review covers:

Order creation and representation

Order validation and ownership

Order cancellation

Order matching and deterministic trade creation

Self-trade prevention

Trade identity and semantic validation

Trade application and idempotency

Trade-state transitions

Balance accounting and settlement

State snapshot creation and application

Authentication and authorization boundaries

Replay and stale-event resistance

Integer overflow/underflow

Distributed consistency

Persistence consistency

Resource-exhaustion considerations


Out of scope

Physical/network infrastructure

Host security

Key custody and operational security

Production deployment configuration

External services

Cryptographic key management outside the reviewed code

Live penetration testing

Production infrastructure configuration

An independent external auditor is expected to perform infrastructure, deployment, operational-security, and independent verification checks.


2. Audit Status

Audit complete for the currently reviewed source-level controls

Previously identified critical findings remediated where documented below

High authentication/authorization findings reviewed

Deterministic trade identity reviewed

Snapshot validation reviewed

Self-trade prevention verified

Fixed-point settlement rounding/conservation review remains open

Snapshot freshness/version semantics remain open

External audit pending

Final security sign-off pending

Current overall status: NOT READY FOR FINAL SECURITY SIGN-OFF

The current implementation has closed and verified the previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings.

Two substantive audit questions remain:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural and deferred findings remain documented separately.


3. Audit Areas

3.1 Protocol

Message integrity

Serialization/deserialization

Price/amount validation

Timestamp validation

Authenticated sender binding


3.2 Orders

Order creation

Order ownership validation

Order cancellation authorization

Duplicate order handling

Quantity accounting

Removed-order resurrection protection

Conflicting snapshot order terms rejected

Snapshot quantity increases rejected

Snapshot order identity protected


3.3 Matching

Price matching

Deterministic price priority

Deterministic UUID tie-breaking

Partial fills

Trade identity

Trade duplication/replay handling

Self-trade prevention

Overflow/underflow checks


3.4 Balances & Settlement

Available/locked accounting

Buyer settlement

Seller settlement

Settlement idempotency

Checked arithmetic

Partial-fill settlement accounting

Asset-conservation proof across all rounding cases

Settlement lifecycle enforced through production callers

Fixed-point rounding policy

Per-order reservation architecture


3.5 Trade State

Valid transitions

Invalid transitions rejected

Exceptional states

State replay/idempotency

Skipped-state handling

Stale-state protection

Unauthorized trade-state changes rejected


3.6 State Synchronization

Snapshot structural validation

Duplicate trade detection

Duplicate order handling

Removed-order protection

Applied-trade handling

Transaction-like snapshot application

Snapshot persistence consistency

Trade identity validation

Trade/order semantic validation

Snapshot authority model

Snapshot freshness/version semantics

Complete order replacement authorization through current snapshot path


4. Resolved / Verified Findings

AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Finding

Application-level sender identity must not be trusted merely because it is contained in a received message.

Remediation

Incoming messages bind the claimed application sender to the authenticated libp2p peer.

Protections include:

claimed sender must match the authenticated peer identity;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification

Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition: CLOSED / VERIFIED


AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote order synchronization must not permit unrestricted replacement of an existing order using the same UUID.

Remediation

The current remote order path does not perform unrestricted order replacement.

Snapshot updates use update_order_from_snapshot() and protect immutable order identity fields:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Orders already marked as removed cannot be resurrected.

Conflicting snapshot terms are rejected.

Verification

Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition: CLOSED / VERIFIED


AUDIT-003 — Self-Trading
Severity: Medium
Status: CLOSED / VERIFIED

Finding

The matching engine must prevent a trader's order from matching against another order belonging to the same trader if self-trading is prohibited by protocol policy.

Verified implementation

OrderBook::match_order() explicitly excludes candidates belonging to the same trader:

existing.trader != incoming.trader

The condition is applied as part of the matching candidate predicate before price selection and trade creation.

Therefore an order from trader A cannot match a resting order from trader A.

Regression coverage

A dedicated test exists:

matching_rejects_self_trade

The test:

creates a resting sell order;

creates a buy order;

assigns the same trader identity to both;

attempts matching;

verifies that no self-trade is generated.

Disposition

CLOSED / VERIFIED

The previously open self-trading concern is resolved in the current matching implementation.


AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Finding

Settlement performs economic mutation and must not be reachable through an unchecked path that permits settlement before the required trade lifecycle state.

Verification

All identified production settlement callers first advance the trade using the protocol state machine.

The production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

The remote trade-state path additionally verifies that the sender is a participant before advancing the trade.

Pending trade-state handling follows the same transition-before-settlement model.

Disposition

No confirmed settlement-before-terminal-state vulnerability was identified in the current production call paths.

CLOSED / VERIFIED

Defense-in-depth recommendation

settle_local_trade() may additionally verify:

trade.state == TradeState::Settled

internally, preventing future callers from accidentally bypassing the lifecycle requirement.


AUDIT-008 — Snapshot / Synchronization Validation
Severity: High
Status: CLOSED / VERIFIED

Finding

Remote snapshots must not introduce structurally or semantically invalid trades or orders into local state.

Remediation

Snapshot validation covers:

duplicate orders;

invalid order amounts;

empty traders;

invalid markets;

duplicate trades;

invalid trade amounts;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

applicable limit-price consistency;

duplicate applied-trade references.

Snapshot processing occurs against cloned state before committing the resulting state to live books.

Materialized snapshot trades are not replayed as historical economic events.

Repeated application of the same materialized snapshot is idempotent.

Disposition: CLOSED / VERIFIED


AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Finding

Distributed trade creation required deterministic trade identity and strong validation that received trades correspond to the referenced orders and economic event.

Remediation

Trade IDs are deterministically derived from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for derivation.

Incoming trades are checked against the expected deterministic identity.

Trade validation verifies:

maker/taker structure;

buyer/seller identity;

referenced orders;

order side;

market;

price constraints;

remaining quantities.

Matching is deterministic:

buy orders select the lowest eligible sell price;

sell orders select the highest eligible buy price;

equal-price orders use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Verification

Regression coverage includes:

invalid deterministic trade IDs;

deterministic matching;

different arrival orders;

duplicate trade application;

partial fills;

multiple fills;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order trade delivery;

trade-before-order delivery;

self-trade rejection.

Disposition: CLOSED / VERIFIED


5. Positive Security Controls

CTRL-001 — Trade-State Transition Validation
Status: PASS

Trade::transition() requires TradeState::can_transition_to() to approve state changes.

Duplicate state events are idempotent.


CTRL-002 — Trade/Order Consistency Validation
Status: PASS

Trade validation verifies:

referenced buy order;

referenced sell order;

correct Buy/Sell sides;

buyer matches buy-order trader;

seller matches sell-order trader;

market consistency;

price limits;

remaining quantities;

deterministic trade identity.


CTRL-003 — Snapshot Transaction-Like Application
Status: PASS

Snapshot processing occurs against cloned state.

Live state is replaced only after validation, processing, and persistence succeed.


CTRL-004 — Structural Trade Validation Before Persistence
Status: PASS

Trade recording validates structural correctness before inserting a trade into persistent trade state.

Malformed trades are rejected before they can become persistent economic state.


CTRL-005 — Settlement Trade-Level Idempotency
Status: PASS

Settled trade IDs are recorded.

Duplicate settlement attempts are ignored.


CTRL-006 — Settlement Arithmetic Validation
Status: PASS

Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.


CTRL-007 — Distributed Pending-Trade Handling
Status: PASS

Trades received before their referenced orders are retained and applied when the required orders arrive.

Duplicate application is idempotent.


CTRL-008 — Pending Trade-State Authorization
Status: PASS

Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

avoid unsafe state regression;

retain valid later states;

revalidate authorization when the trade arrives.


CTRL-009 — Removed-Order Resurrection Protection
Status: PASS

Orders explicitly recorded as removed cannot simply be resurrected by stale synchronization data.


CTRL-010 — Stale Trade-State Protection
Status: PASS

Stale synchronization cannot move an existing trade backward through its lifecycle.


CTRL-011 — Self-Trade Prevention
Status: PASS

The matching candidate predicate explicitly requires:

existing.trader != incoming.trader

A dedicated regression test verifies that same-trader buy/sell orders do not generate a trade.


6. Active Findings

AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN

Issue

Quote settlement uses:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required verification

Establish:

authoritative protocol rounding rule;

whether truncation is intentional;

whether asset conservation holds;

minimum-value behavior;

maximum-value behavior;

boundary behavior;

repeated fractional-trade behavior;

whether rounding systematically favors one participant;

whether dust/remainder accumulation is possible.

Required tests

Add tests covering:

exact division;

non-exact division;

minimum valid price/amount;

maximum safe values;

repeated fractional trades;

conservation before and after settlement;

buyer/seller accounting under rounding.

Disposition: OPEN pending economic/conservation review.


AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium
Status: OPEN

Issue

StateSnapshot does not currently provide an explicit state sequence/version or equivalent state commitment.

Potential impact

Without explicit version semantics, peers may have difficulty determining whether a snapshot is:

stale;

newer than local state;

derived from a coherent point in time;

based on a known state ancestry.

The current validation protects against malformed and conflicting content, but content validation does not by itself establish freshness or global ordering.

Required design review

Determine whether the synchronization protocol requires:

monotonic snapshot version;

generation number;

event sequence;

state commitment;

snapshot ancestry;

freshness rule;

stale snapshot rejection.

If the existing synchronization architecture already guarantees these properties through another mechanism, that mechanism should be explicitly documented and tested.

Disposition: OPEN pending synchronization consistency review.


7. Architectural / Deferred Findings

ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Required review

Establish:

one clear owner for applying a trade;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all entry paths enforce deterministic trade identity and semantic validation.

No refactor should be performed merely to reduce duplication.

Disposition: INVESTIGATING


BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook primarily tracks aggregate available and locked balances.

Future reservation architecture should establish an explicit relationship between locked funds and the order responsible for the reservation.

Required future work:

associate reservations with order IDs;

consume only the relevant reservation during settlement;

unlock only the relevant reservation during cancellation;

support multiple simultaneous orders;

preserve partial-fill correctness;

persist reservations across restart;

handle conflicting/out-of-order events;

ensure duplicate settlement cannot consume a reservation twice.

Disposition: DEFERRED


ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Determine whether direct access to OrderBook internals is required by synchronization or bypasses domain invariants.

This review should be coordinated with:

AUDIT-010;

ARCH-001.

No refactor should be performed without a demonstrated correctness or security requirement.

Disposition: DEFERRED


ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
Status: DEFERRED

Review responsibility boundaries around:

order creation;

balance locking;

publication;

local matching;

settlement;

trade-state transitions.

No split should be performed merely because the command handler is large.

Disposition: DEFERRED


8. Resolved Historical Findings

ID      Issue   Status

BUG-SEC-001 / AUDIT-001
Application-level sender impersonation
RESOLVED

SEC-002
Remote order cancellation authority
RESOLVED

BUG-003
Distributed TradeCreated application
RESOLVED

BUG-004
Trade state arriving before TradeCreated
RESOLVED

BUG-005
Local settlement applied to non-participants
RESOLVED

BUG-006
Duplicate settlement
RESOLVED

BUG-007
Partial-fill settlement accounting
RESOLVED

BUG-008
Quote amount overflow
RESOLVED

BUG-009
GossipSub timing-dependent test
RESOLVED

BL-2026-08-31-001
Pending trade-state handling
FIXED

AUDIT-002
Order UUID replacement
CLOSED / VERIFIED

AUDIT-003
Self-trading
CLOSED / VERIFIED

AUDIT-004
Settlement lifecycle authorization
CLOSED / VERIFIED

AUDIT-008
Remote snapshot validation
CLOSED / VERIFIED

AUDIT-009 / ARCH-004
Trade identity/order integrity/deterministic matching
CLOSED / VERIFIED


9. Current Finding Summary

ID      Severity        Status

AUDIT-001       High      CLOSED / VERIFIED

AUDIT-002       High      CLOSED / VERIFIED

AUDIT-003       Medium    CLOSED / VERIFIED

AUDIT-004       High      CLOSED / VERIFIED

AUDIT-007       Medium    OPEN

AUDIT-008       High      CLOSED / VERIFIED

AUDIT-009 / ARCH-004    Critical    CLOSED / VERIFIED

AUDIT-010       Medium    OPEN

ARCH-001        High      INVESTIGATING

BUG-001         High      DEFERRED

ARCH-002        Medium    DEFERRED

ARCH-003        Medium    DEFERRED

Previously identified security findings AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-008, AUDIT-009, and SEC-002 are closed or verified as documented above.


10. Current Audit Priorities

The next review sequence should be:

1. AUDIT-007 — Fixed-point settlement rounding and asset conservation

2. AUDIT-010 — Snapshot freshness/version semantics

3. ARCH-001 — Trade application ownership and invariant enforcement

4. BUG-001 — Per-order balance reservations

5. ARCH-002 / ARCH-003 — Deferred architecture reviews

6. Independent adversarial multi-node testing

7. Persistence/crash-recovery testing beyond current automated coverage

8. Resource-exhaustion review

9. External infrastructure and deployment audit

The priority is to resolve remaining protocol and economic invariants before undertaking broad architectural refactoring.


11. Verification Record

The current implementation has been verified with:

cargo test --workspace

The supplied workspace run completed successfully with:

BUILD: PASS

FAILURES: 0

Workspace test results:

Common: 1 passed

Protocol: 3 passed

Balances: 8 passed

CLI: 11 passed

Exchange: 1 passed

Network: 1 passed

Orderbook: 30 passed

Security: 17 passed

Settlement: 6 passed

Trade lifecycle: 14 passed

Doc-tests: all passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade


12. Audit Readiness

Current status: NOT READY FOR FINAL SECURITY SIGN-OFF

The previously identified authentication, order-authority, deterministic trade, snapshot-validation, settlement-lifecycle, and self-trading findings have been remediated and verified.

The remaining substantive audit questions are:

AUDIT-007 — Fixed-point settlement rounding and economic conservation

AUDIT-010 — Snapshot consistency, freshness, and version semantics

Architectural/deferred findings remain under separate investigation.

The next audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.

Broad architectural refactoring should not precede these invariant decisions unless a demonstrated security or correctness requirement makes the refactor necessary.


13. Audit Documentation Authority

The audit documentation has the following roles:

audit/AUDIT_CHECKLIST.md

Authoritative checklist of audit controls, verification areas, active findings, resolved findings, and current audit priorities.

audit/AUDIT_FINDINGS.md

Authoritative detailed finding record containing finding descriptions, remediation details, verification evidence, dispositions, architectural issues, and external-audit handoff requirements.

audit/SECURITY_LOG.md

Concise security posture and historical security record.

The SECURITY_LOG.md document should not duplicate the full historical audit record.

When finding status changes, AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md must be updated together.

SECURITY_LOG.md should then be updated only as necessary to reflect the current security posture.

This prevents stale duplicated findings from remaining in historical documentation.


14. Current Conclusion

OpenDEX has closed and verified the previously identified authentication, authorization, deterministic trade identity, trade/order integrity, snapshot validation, settlement lifecycle, and self-trading findings.

The current source-level audit is no longer centered on those resolved security boundaries.

The remaining substantive protocol questions are:

AUDIT-007 — fixed-point settlement rounding and economic conservation;

AUDIT-010 — snapshot freshness, consistency, and version semantics.

ARCH-001 remains under investigation as an architectural ownership question.

BUG-001, ARCH-002, and ARCH-003 remain deferred.

Current conclusion:

OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The next substantive audit work should focus on AUDIT-007, followed by AUDIT-010, before broad architectural refactoring.
The previously identified gap between authenticated network identity and semantic trade validation has been substantially addressed.

Remote trades are now checked for deterministic identity and consistency with their referenced orders before they can become accepted state.

Remote snapshots are validated before their contents are committed to live state.

4. Verified / Closed Findings
AUDIT-001 — Message Sender Authentication
Severity: High
Status: CLOSED / VERIFIED

Original issue
Application-level sender identity could not safely be trusted merely because it was contained in a received message.

Remediation
Incoming OpenDEX messages are bound to the authenticated libp2p peer identity.

Relevant controls include:

claimed sender must match the authenticated peer;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Verification
Security regression coverage includes:

forged sender rejection;

impersonated trader rejection;

forged cancellation rejection;

unauthorized trade-state rejection;

authenticated sender acceptance.

Disposition
CLOSED / VERIFIED

AUDIT-002 — Order UUID Replacement / Snapshot Order Semantics
Severity: High
Status: CLOSED / VERIFIED

Original issue
Remote order synchronization required protection against unrestricted replacement of an existing order using the same UUID.

Remediation
Remote OrderCreated processing does not blindly replace an existing order.

Snapshot synchronization uses the dedicated order-update path rather than unrestricted insertion.

Snapshot updates:

reject conflicting trader identity;

reject conflicting market;

reject conflicting side;

reject conflicting price;

reject quantity increases;

ignore orders already marked removed;

permit only valid quantity reductions or equivalent state.

Verification
Regression coverage includes:

conflicting snapshot order terms;

stale order delivery;

removed-order resurrection;

snapshot quantity reduction.

Disposition
CLOSED / VERIFIED

The original unrestricted remote UUID-replacement concern is not supported by the current reviewed production call paths.

AUDIT-003 — Self-Trading Prevention
Severity: Medium
Status: CLOSED / VERIFIED

Original issue
The audit required confirmation that the matching engine does not match an order against another order belonging to the same trader.

Verification
OrderBook::match_order() explicitly requires:

existing.trader != incoming.trader

as part of the candidate predicate.

Therefore a resting order belonging to the same trader as the incoming order is excluded from matching.

The regression suite includes:

matching_rejects_self_trade

The test verifies that an incoming order belonging to the same trader as a resting opposing order produces no trade.

Disposition
CLOSED / VERIFIED

Self-trading is currently prevented at the matching-engine candidate-selection boundary.

AUDIT-004 — Settlement Lifecycle Authorization
Severity: High / Defense-in-depth
Status: CLOSED / VERIFIED

Original issue
Settlement performs economic mutation and therefore required verification that settlement cannot be reached through an unauthorized or premature trade state.

Verification
Reviewed production settlement callers advance the trade through the trade-state machinery before invoking settlement.

The effective production flow is:

advance_trade_to_state(...)
        ↓
retrieve updated trade
        ↓
verify TradeState::Settled
        ↓
settle_local_trade(...)

Remote trade-state handling also verifies authorization before applying the requested lifecycle transition.

Trade-state transitions are constrained by the protocol state machine.

Result
No confirmed production call path was identified that permits settlement before the required terminal state.

Defense-in-depth recommendation
settle_local_trade() may additionally enforce:

trade.state == TradeState::Settled

internally.

This would protect the economic mutation boundary against future callers accidentally bypassing the higher-level lifecycle checks.

Disposition
CLOSED / VERIFIED

The remaining internal state check is a defense-in-depth recommendation rather than an identified exploitable production path.

AUDIT-008 — Remote Snapshot Validation
Severity: High
Status: CLOSED / VERIFIED

Original issue
Remote state snapshots must not introduce invalid orders or trades into local persistent state.

Remediation
Snapshot validation now checks the relevant structural and semantic invariants before committing the resulting state.

Validation includes:

duplicate orders;

invalid order quantities;

invalid traders;

invalid markets;

duplicate trades;

invalid trade quantities;

invalid trade prices;

invalid participants;

invalid timestamps;

unknown order references;

order-side consistency;

buyer/seller identity;

market consistency;

price-limit consistency;

duplicate applied-trade references;

deterministic trade identity;

trade/order semantic relationships.

Snapshot application is performed against temporary/cloned state.

The live state is replaced only after validation and persistence succeed.

Verification
Regression coverage includes:

invalid snapshot rejection;

invalid maker rejection;

conflicting snapshot order terms;

snapshot atomicity;

materialized snapshot handling;

repeated snapshot idempotency;

trade-before-order handling;

removed-order resurrection protection.

Disposition
CLOSED / VERIFIED

AUDIT-009 / ARCH-004 — Trade Identity, Trade/Order Integrity and Deterministic Matching
Severity: Critical
Status: CLOSED / VERIFIED

Original issue
Distributed trade creation required stronger guarantees that independently received trades represented the same deterministic economic event and corresponded to their referenced orders.

Remediation
Trade IDs are derived deterministically from:

buy order UUID;

sell order UUID;

trade price;

trade amount.

SHA-256 is used for the deterministic derivation.

Incoming trades are validated against the expected deterministic ID.

Trade validation additionally checks:

referenced buy order;

referenced sell order;

correct order sides;

buyer identity;

seller identity;

maker/taker relationship;

market consistency;

price limits;

remaining quantity;

trade amount;

deterministic trade identity.

Deterministic matching
Matching uses deterministic price priority:

buys select the lowest eligible sell price;

sells select the highest eligible buy price;

equal prices use deterministic UUID ordering;

self-trading candidates are explicitly excluded.

Matching tracks remaining quantities without prematurely mutating the live order book.

apply_trade() remains responsible for materializing order-book changes.

Distributed handling
Regression coverage verifies:

duplicate trade application;

partial fills;

multiple fills against one order;

invalid trade/order relationships;

invalid quantities;

invalid prices;

out-of-order delivery;

trade-before-order delivery;

different arrival orders converging;

deterministic matching;

deterministic trade identity;

self-trade rejection.

Disposition
CLOSED / VERIFIED

The previously critical trade semantic-validation and deterministic-matching concerns have been remediated and regression-tested.

AUDIT-009B — Snapshot Order Trust Boundary
Severity: High
Status: CLOSED / VERIFIED

Original issue
Snapshot-provided order data required protection against unauthorized replacement or resurrection.

Verification
Snapshot orders are processed through the dedicated snapshot-update path.

Existing immutable order identity fields are protected:

trader;

market;

side;

price.

Snapshot quantity increases are rejected.

Removed orders cannot be resurrected.

Snapshot processing is performed against temporary state before the live state is committed.

Disposition
CLOSED / VERIFIED

5. Active Findings
AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium
Status: OPEN
Category: Economic Correctness
File: crates/p2p-node/src/settlement.rs

Description
Quote settlement uses fixed-point integer arithmetic equivalent to:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Risk
The mathematical trade value and the actual settled integer amount can differ when the division is not exact.

Potential consequences depend on the intended protocol economics and may include:

systematic rounding loss;

accumulation of small discrepancies;

boundary-value inconsistencies;

possible participant asymmetry;

unclear asset-conservation guarantees.

Required review
The protocol specification should explicitly define:

the authoritative rounding rule;

whether truncation is intentional;

which participant receives any remainder, if applicable;

minimum-value behavior;

maximum-value behavior;

zero-result behavior;

fractional-price behavior;

repeated fractional-trade behavior;

asset-conservation requirements.

Required tests
Add boundary and conservation tests covering:

exact division;

non-exact division;

smallest valid amounts;

largest safe amounts;

repeated fractional trades;

values close to u64 limits;

combinations of price and amount that produce remainders;

buyer/seller accounting before and after settlement;

conservation of base and quote assets.

Disposition
OPEN — Economic/conservation review required.

AUDIT-010 — Snapshot Consistency and Version Semantics
Severity: Medium
Status: OPEN
Category: Distributed Consistency
File: crates/protocol/src/state.rs

Description
StateSnapshot contains state data such as:

orders;

trades;

removed orders;

applied trades.

The current snapshot format does not provide an explicit monotonic snapshot sequence, generation number, or equivalent state commitment.

Potential impact
Without explicit version/freshness semantics, peers may have difficulty determining whether a received snapshot:

is newer than local state;

is stale;

represents a coherent point in time;

is derived from an older state;

should replace or merge with local state.

The existing validation and stale-order protections reduce the risk, but they do not by themselves establish a complete snapshot freshness model.

Required protocol review
Determine the intended synchronization consistency model.

Possible mechanisms include:

monotonic snapshot version;

generation number;

event sequence;

state hash/commitment;

snapshot ancestry;

freshness metadata;

explicit merge rules.

The selected mechanism must be consistent with distributed event delivery and persistence behavior.

Required tests
If version semantics are introduced, add tests for:

newer snapshot replacing/merging correctly;

stale snapshot rejection;

equal-version idempotency;

conflicting equal-version snapshots;

restart followed by synchronization;

out-of-order snapshot delivery;

concurrent node updates.

If the existing synchronization architecture already provides equivalent freshness or ancestry guarantees through another mechanism, that mechanism should be explicitly documented and regression-tested instead of introducing redundant version metadata.

Disposition
OPEN — Protocol consistency review required.

6. Architectural / Deferred Issues
ARCH-001 — Trade Application Has Multiple Mutation Paths
Priority: High
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

as well as synchronization and pending-trade processing.

Existing validation and idempotency mechanisms significantly reduce duplicate-mutation risk.

The remaining concern is architectural ownership.

Required review
Establish and document:

ownership of trade application;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether local and remote trade application share identical invariants;

whether synchronization can bypass those invariants;

whether all trade-entry paths enforce deterministic identity and semantic validation.

No refactor should be performed merely to reduce code duplication.

Disposition
INVESTIGATING

BUG-001 — Per-Order Balance Reservations
Priority: High
Status: DEFERRED

The current BalanceBook architecture primarily tracks aggregate available and locked balances.

An explicit relationship between locked funds and the order that created the reservation remains desirable for robust handling of:

multiple simultaneous orders;

partial fills;

cancellation;

settlement;

concurrent trades;

out-of-order distributed events;

restart/recovery.

Future work
Consider associating reservations with order IDs and explicitly tracking:

reserved amount per order;

consumption during partial settlement;

release during cancellation;

persistence across restart;

duplicate settlement;

conflicting distributed events.

Disposition
DEFERRED

This is an architectural/accounting improvement rather than a currently demonstrated exploit.

ARCH-002 — sync.rs Accesses OrderBook Internals
Priority: Medium
Status: DEFERRED

Snapshot application currently accesses several OrderBook internals directly.

The architectural review should determine whether these accesses:

are necessary for synchronization;

bypass domain invariants;

duplicate existing OrderBook functionality;

make future invariant enforcement harder.

This review should be coordinated with:

AUDIT-002;

AUDIT-008;

AUDIT-009;

AUDIT-010;

ARCH-001.

Disposition
DEFERRED

ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium
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

Disposition
DEFERRED

7. Positive Security Controls
Sender Authentication — PASS
Application-level sender identity is bound to the authenticated P2P peer.

Protections include:

claimed sender must match authenticated PeerId;

forged sender identities are rejected;

order creation requires sender/order-owner consistency;

cancellation requires order-owner authorization;

trade-state changes require trade participation;

pending trade-state messages retain authenticated sender identity;

sender identity is revalidated when the corresponding trade arrives.

Trade Identity Validation — PASS
Trade IDs are deterministically derived from the economic trade inputs.

Incoming trades are checked against their expected deterministic identity before acceptance.

Trade/Order Semantic Validation — PASS
Trade validation verifies the relationship between trades and referenced orders, including:

order existence;

order side;

buyer/seller identity;

market;

price limits;

remaining quantity;

trade quantity;

deterministic trade identity.

Deterministic Matching — PASS
Matching is deterministic with:

price priority;

deterministic UUID tie-breaking;

self-trade exclusion;

temporary remaining-quantity tracking;

deterministic trade generation.

Snapshot Transaction-Like Application — PASS
Snapshot processing operates against temporary/cloned state.

Live state is committed only after validation and persistence succeed.

Snapshot Order Immutability — PASS
Snapshot synchronization protects immutable order terms and rejects unauthorized quantity increases.

Removed orders cannot be resurrected.

Settlement Idempotency — PASS
Settled trade IDs are recorded.

Duplicate settlement attempts do not consume the same trade twice.

Settlement Arithmetic — PASS
Settlement checks:

quote multiplication overflow;

buyer locked quote balance;

seller locked base balance;

buyer available-base overflow;

seller available-quote overflow.

Checked arithmetic is used for settlement mutations.

Distributed Pending Trade Handling — PASS
Trades received before their referenced orders are retained and applied when the required orders become available.

Duplicate application is idempotent.

Pending Trade-State Authorization — PASS
Pending trade-state messages:

retain authenticated sender identity;

reject unauthorized participants;

prevent unsafe state regression;

retain valid later happy-path states;

revalidate authorization when the trade arrives;

continue through the protocol state machine.

Removed-Order Resurrection Protection — PASS
Orders recorded as removed cannot simply be reintroduced through stale synchronization data.

Stale Trade-State Protection — PASS
Stale synchronization cannot move an existing trade backward through its lifecycle.

8. Resolved Historical Issues
The following previously identified issues are considered resolved or fixed and should remain historically documented rather than treated as active findings:

ID	Issue	Status
BUG-SEC-001 / AUDIT-001	Application-level sender impersonation	RESOLVED
SEC-002	Remote order cancellation authority	RESOLVED
BUG-003	Distributed TradeCreated application	RESOLVED
BUG-004	Trade state arriving before TradeCreated	RESOLVED
BUG-005	Local settlement applied to non-participants	RESOLVED
BUG-006	Duplicate settlement	RESOLVED
BUG-007	Partial-fill settlement accounting	RESOLVED
BUG-008	Quote amount overflow	RESOLVED
BUG-009	GossipSub timing-dependent test	RESOLVED
BL-2026-08-31-001	Pending trade-state handling	FIXED
AUDIT-002	Order UUID replacement	CLOSED / VERIFIED
AUDIT-003	Self-trading prevention	CLOSED / VERIFIED
AUDIT-004	Settlement lifecycle authorization	CLOSED / VERIFIED
AUDIT-008	Remote snapshot validation	CLOSED / VERIFIED
AUDIT-009 / ARCH-004	Trade identity/order integrity/deterministic matching	CLOSED / VERIFIED

9. Current Finding Summary
ID	Severity / Priority	Status
AUDIT-001	High	CLOSED / VERIFIED
AUDIT-002	High	CLOSED / VERIFIED
AUDIT-003	Medium	CLOSED / VERIFIED
AUDIT-004	High	CLOSED / VERIFIED
AUDIT-007	Medium	OPEN
AUDIT-008	High	CLOSED / VERIFIED
AUDIT-009 / ARCH-004	Critical	CLOSED / VERIFIED
AUDIT-010	Medium	OPEN
ARCH-001	High	INVESTIGATING
BUG-001	High	DEFERRED
ARCH-002	Medium	DEFERRED
ARCH-003	Medium	DEFERRED

10. Current Audit Priorities
The next review sequence is:

AUDIT-007 — Fixed-point rounding and asset conservation

AUDIT-010 — Snapshot consistency and freshness/version semantics

ARCH-001 — Trade application ownership and invariant enforcement

BUG-001 — Per-order balance reservations

ARCH-002 — Synchronization/domain-boundary review

ARCH-003 — Command-handler responsibility review

Independent adversarial multi-node testing

Persistence/crash-recovery testing beyond the current automated coverage

Resource-exhaustion review

External infrastructure and deployment audit

Broad architectural refactoring should not precede resolution of the remaining protocol and accounting invariants unless a demonstrated security requirement makes the refactor necessary.

11. Verification Record
The current implementation was verified with the supplied workspace test run:

cargo test --workspace

Result:

BUILD: PASS
FAILURES: 0

The supplied workspace results include:

Common unit tests: 1 passed

Protocol unit tests: 3 passed

Balance tests: 8 passed

CLI tests: 11 passed

Exchange tests: 1 passed

Network tests: 1 passed

Orderbook tests: 30 passed

Security tests: 17 passed

Settlement tests: 6 passed

Trade lifecycle tests: 14 passed

Doc-test targets: passed

Total individual tests reported: 91 passed, 0 failed.

No test failures were reported.

No compiler warnings were reported in the supplied workspace test output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The matching predicate explicitly rejects candidates where:

existing.trader == incoming.trader

A dedicated regression test is present:

matching_rejects_self_trade

The current verification record therefore supersedes earlier audit documentation that reported 87 or 91 tests based on incomplete test-group totals. The authoritative current workspace result is 91 passed, 0 failed.

12. Audit Readiness Assessment
Current status
NOT READY FOR FINAL SECURITY SIGN-OFF

The current source-level review has verified substantial remediation of the previously identified security boundaries.

Closed and verified areas include:

sender authentication;

remote cancellation authorization;

order UUID replacement protection;

self-trade prevention;

settlement lifecycle enforcement through production callers;

deterministic trade identity;

trade/order semantic validation;

deterministic matching;

snapshot validation;

snapshot order trust;

duplicate/out-of-order trade handling;

pending trade-state authorization;

settlement idempotency;

checked settlement arithmetic;

partial-fill accounting;

stale-state protection.

Remaining substantive findings
AUDIT-007 — Fixed-point rounding/conservation
The protocol still requires an explicit economic decision and conservation analysis for integer fixed-point settlement.

AUDIT-010 — Snapshot consistency/versioning
The synchronization protocol still requires a documented decision on snapshot freshness, versioning, and consistency semantics.

Remaining architectural work
ARCH-001 and BUG-001 remain important architectural/accounting topics, but neither is currently documented as a demonstrated critical exploit.

The final audit should therefore focus on proving the remaining protocol invariants rather than reopening already verified findings without new evidence.

13. External Auditor Handoff
An independent auditor should verify:

complete network-to-order call paths;

complete network-to-trade call paths;

complete trade-to-settlement call paths;

authenticated sender binding;

authorization boundaries;

deterministic trade identity;

trade/order semantic validation;

self-trade prevention;

snapshot validation;

snapshot freshness and consistency;

replay and stale-event handling;

order resurrection prevention;

persistence and crash recovery;

cross-node convergence;

accounting conservation;

fixed-point rounding behavior;

resource-exhaustion limits;

host and network security;

production deployment configuration;

key management and custody;

live penetration and adversarial testing.

The source-level audit should remain open until the remaining rounding/conservation and snapshot-consistency questions have been resolved or explicitly accepted by the protocol owners and independently reviewed.

14. Current Conclusion
OpenDEX has made substantial progress in the security hardening of its distributed trading state.

The previously identified critical trade-identity, trade/order-integrity, deterministic-matching, and snapshot-validation concerns have been remediated and covered by regression tests.

The self-trading finding has also been directly inspected in the current match_order() implementation and is now CLOSED / VERIFIED.

The current review is no longer centered on an unresolved critical remote-trade validation gap.

The remaining substantive work is concentrated on:

fixed-point economic semantics and asset conservation;

snapshot consistency and freshness semantics;

architectural ownership of trade application;

future per-order reservation accounting.

Current conclusion: OpenDEX remains in audit progress and is not yet ready for final security sign-off, but the previously identified critical trade/snapshot validation findings and self-trading finding are now documented as remediated and verified.

The next substantive audit checkpoint should focus on AUDIT-007, followed by AUDIT-010.
