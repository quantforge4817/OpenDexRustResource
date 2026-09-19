# OpenDEX — Security Log

## Purpose

This document records the current security posture, verified security controls, significant remediation work, and remaining security-review items.

The authoritative status of individual audit findings is maintained in:

- `audit/AUDIT_FINDINGS.md`
- `audit/AUDIT_CHECKLIST.md`

This document is intentionally concise. Historical implementation details that are no longer active findings are not repeated here.

---

# 1. Current Security Review Status

**Security architecture review:** 🟡 In progress

**Security hardening:** 🟡 In progress

**Final security audit:** ⏳ Pending

**Final security sign-off:** 🔴 Not yet ready

The current implementation has substantially strengthened authentication, authorization, distributed-state handling, trade validation, settlement safety, and replay resistance.

The remaining substantive audit work concerns:

- fixed-point settlement rounding and asset conservation;
- snapshot consistency and version/freshness semantics;
- architectural ownership of trade application.

Deferred architectural work includes:

- per-order balance reservations;
- synchronization/domain-boundary review;
- command-handler responsibility-boundary review.

Self-trade prevention has been directly verified in the current matching implementation and is therefore no longer an open finding.

---

# 2. Security Principles

OpenDEX treats network-delivered protocol messages as untrusted input.

Security-sensitive operations should therefore be:

- authenticated;
- authorized;
- locally validated;
- deterministic where required;
- resistant to duplicate delivery;
- resistant to out-of-order delivery;
- resistant to stale state;
- resistant to malformed input;
- bounded against resource exhaustion;
- independently regression-tested.

Authentication of a network peer does not by itself grant authority over another trader's state.

---

# 3. Verified Security Controls

## SEC-001 — Authenticated Sender Binding

**Status:** 🟢 CLOSED / VERIFIED

Incoming application messages bind the claimed application sender to the authenticated P2P peer.

Verified protections include:

- claimed sender must match the authenticated peer identity;
- forged sender identities are rejected;
- order creation requires sender/order-owner consistency;
- cancellation requires order-owner authorization;
- trade-state changes require trade participation;
- pending trade-state messages retain authenticated sender identity;
- pending state is revalidated when the associated trade arrives.

Regression coverage exists for forged senders, impersonated traders, unauthorized cancellations, and unauthorized trade-state changes.

---

## SEC-002 — Remote Order Cancellation Authorization

**Status:** 🟢 CLOSED / VERIFIED

Remote cancellation requires authorization from the authenticated owner of the referenced order.

The cancellation path verifies:

- authenticated sender identity;
- cancellation trader identity;
- referenced order;
- order ownership.

Unauthorized cancellation is rejected before order-book mutation.

Regression coverage verifies that unauthorized and forged cancellation requests do not mutate state.

---

## SEC-003 — Deterministic Trade Identity

**Status:** 🟢 CLOSED / VERIFIED

Trade identity is deterministically derived from the economic inputs defining the trade:

- buy order ID;
- sell order ID;
- trade price;
- trade amount.

SHA-256 is used for deterministic trade-ID derivation.

Incoming trades are checked against the expected deterministic identity.

Incorrect or forged trade IDs are rejected.

---

## SEC-004 — Trade / Order Semantic Validation

**Status:** 🟢 CLOSED / VERIFIED

Trade validation checks the relationship between a trade and its referenced orders.

Validation includes:

- referenced order existence;
- Buy/Sell side correctness;
- buyer identity;
- seller identity;
- market consistency;
- price constraints;
- remaining quantity;
- trade amount;
- deterministic trade identity.

Invalid trades are rejected without partially mutating live state.

---

## SEC-005 — Deterministic Matching

**Status:** 🟢 CLOSED / VERIFIED

Matching uses deterministic ordering.

For buys:

- the lowest eligible sell price is selected first.

For sells:

- the highest eligible buy price is selected first.

Equal-price candidates use deterministic UUID ordering.

Self-trading candidates are explicitly excluded.

Partial fills are handled using temporary matching state before live order-book mutation.

Regression coverage verifies deterministic matching, partial fills, multiple fills, self-trade rejection, and different event-arrival orders.

---

## SEC-006 — Snapshot Validation

**Status:** 🟢 CLOSED / VERIFIED

Remote state snapshots are validated before being committed to live state.

Validation includes:

- duplicate orders;
- duplicate trades;
- invalid order quantities;
- invalid trade quantities;
- invalid prices;
- invalid timestamps;
- invalid participants;
- unknown order references;
- order-side consistency;
- buyer/seller identity;
- market consistency;
- price-limit consistency;
- applied-trade references;
- deterministic trade identity;
- trade/order semantic relationships.

Snapshot processing occurs against temporary/cloned state.

Invalid snapshots therefore do not partially mutate the live order or trade books.

---

## SEC-007 — Snapshot Order Integrity

**Status:** 🟢 CLOSED / VERIFIED

Snapshot synchronization does not perform unrestricted replacement of existing orders.

Existing order identity fields are protected, including:

- trader;
- market;
- side;
- price.

Snapshot quantities cannot increase an existing order.

Orders previously recorded as removed cannot be resurrected through stale snapshot data.

---

## SEC-008 — Trade-State Authorization and Transition Validation

**Status:** 🟢 CLOSED / VERIFIED

Trade-state transitions are constrained by the protocol state machine.

Verified protections include:

- invalid transitions rejected;
- stale transitions rejected;
- duplicate transitions idempotent;
- permitted forward transitions accepted;
- unauthorized participants rejected;
- pending trade-state messages retain sender authorization;
- authorization is revalidated when pending state is applied.

---

## SEC-009 — Settlement Idempotency

**Status:** 🟢 CLOSED / VERIFIED

Settlement records completed trade IDs.

Repeated settlement attempts for the same trade do not consume balances again.

This protects against duplicate distributed delivery and replay within the persisted settlement state.

---

## SEC-010 — Settlement Arithmetic Safety

**Status:** 🟢 CLOSED / VERIFIED

Settlement uses checked arithmetic and validates the relevant balances before mutation.

Verified protections include:

- quote multiplication overflow detection;
- buyer locked-quote validation;
- seller locked-base validation;
- buyer available-base overflow detection;
- seller available-quote overflow detection.

Settlement failure does not partially mutate balances.

---

## SEC-011 — Partial-Fill Accounting

**Status:** 🟢 CLOSED / VERIFIED

Partial trades consume only the amount associated with the executed portion of the trade.

Regression coverage verifies partial-fill settlement behavior and prevents unintended consumption of the remaining locked amount.

---

## SEC-012 — Pending Distributed Trades

**Status:** 🟢 CLOSED / VERIFIED

A trade may arrive before its referenced orders.

Such trades are retained until the required orders are available.

Once the orders arrive:

- the trade is validated;
- the trade is applied;
- duplicate application is prevented.

Out-of-order delivery is covered by regression tests.

---

## SEC-013 — Removed-Order Resurrection Protection

**Status:** 🟢 CLOSED / VERIFIED

Orders recorded as removed cannot simply be recreated by stale synchronization data.

This protection applies to delayed and out-of-order distributed events.

---

## SEC-014 — Stale Trade-State Protection

**Status:** 🟢 CLOSED / VERIFIED

Stale trade-state messages cannot move an existing trade backward through its lifecycle.

The protocol state machine remains the authority for permitted transitions.

---

## SEC-015 — Snapshot Transaction-Like Application

**Status:** 🟢 CLOSED / VERIFIED

Snapshot processing occurs against temporary state before the live state is replaced.

The implementation therefore avoids partially committing an invalid snapshot.

Regression coverage verifies snapshot failure does not partially mutate live state.

---

## SEC-016 — Self-Trade Prevention

**Status:** 🟢 CLOSED / VERIFIED

`OrderBook::match_order()` explicitly excludes candidates belonging to the same trader.

The matching candidate predicate requires:

```text
existing.trader != incoming.trader
A dedicated regression test is present:

matching_rejects_self_trade

The test verifies that a buy and sell order belonging to the same trader do not generate a trade.

Self-trading is therefore no longer an open audit finding in the current implementation.

4. Current Open Security / Correctness Findings
AUDIT-007 — Fixed-Point Settlement Rounding
Severity: Medium

Status: 🟠 OPEN

Quote settlement currently uses fixed-point integer arithmetic equivalent to:

price * amount / PRICE_SCALE

Integer division truncates any remainder.

Required review:

define the authoritative rounding rule;

verify asset conservation;

define minimum-value behavior;

define maximum-value behavior;

test exact division;

test non-exact division;

test fractional values;

test boundary cases;

test repeated fractional trades;

determine whether rounding creates a systematic economic imbalance;

determine whether remainder/dust accumulation is possible.

Required tests should cover:

exact division;

non-exact division;

smallest valid amounts;

largest safe values;

values close to arithmetic limits;

repeated fractional trades;

buyer/seller accounting;

conservation before and after settlement.

Disposition: OPEN — Economic/conservation review required.

AUDIT-010 — Snapshot Consistency / Version Semantics
Severity: Medium

Status: 🟠 OPEN

The current snapshot structure does not provide an explicit monotonic state version, sequence number, or equivalent state commitment.

Required architectural review:

define snapshot freshness semantics;

determine whether stale snapshots can be identified;

determine whether snapshots require monotonic versions;

consider event sequence, generation number, ancestry, or state commitment;

define behavior for equal-version snapshots;

define behavior for conflicting snapshots;

ensure peers cannot unknowingly accept an inconsistent or stale state;

determine how restart and recovery interact with snapshot freshness.

The existing snapshot validation protects against malformed and conflicting content, but content validation alone does not establish global freshness or ordering.

Disposition: OPEN — Protocol consistency review required.

5. Architectural Review Items
ARCH-001 — Trade Application Ownership
Priority: High

Status: 🟠 INVESTIGATING

Trade application is reachable through multiple paths, including:

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

The security requirement is not merely to remove duplicated code.

The review must establish:

ownership of trade application;

ownership of pending_trades;

ownership of applied_trades;

when orders are removed;

when trades are persisted;

whether every path applies identical invariants;

whether synchronization can bypass trade validation;

whether local and remote trade application have equivalent security properties.

No refactor should be performed merely for code organization.

BUG-001 — Per-Order Balance Reservations
Priority: High

Status: 🟡 DEFERRED

The current balance architecture primarily tracks aggregate available and locked balances.

Future reservation architecture should associate locked funds with individual order IDs.

Required future properties include:

multiple simultaneous orders;

partial fills;

cancellation;

settlement;

concurrent trades;

out-of-order events;

restart persistence;

duplicate settlement protection.

This remains deferred because the current implementation has regression coverage for the existing settlement model and no specific exploit has been established from the aggregate reservation architecture.

ARCH-002 — Synchronization / OrderBook Boundaries
Priority: Medium

Status: 🟡 DEFERRED

sync.rs accesses OrderBook state directly in several places.

Future review should determine whether these accesses:

are necessary for synchronization;

bypass domain invariants;

duplicate existing OrderBook functionality;

make future invariant enforcement harder.

This review should be coordinated with AUDIT-010 and ARCH-001.

ARCH-003 — Command Handler Responsibility Boundaries
Priority: Medium

Status: 🟡 DEFERRED

Future architectural review should examine responsibility boundaries around:

order creation;

balance locking;

publication;

matching;

settlement;

trade-state transitions.

No refactor is required merely because a command handler is large.

6. Resolved Historical Security Issues
The following issues are no longer active findings:

ID	Issue	Status
BUG-SEC-001 / AUDIT-001	Application-level sender impersonation	🟢 Resolved
SEC-002	Unauthorized remote cancellation	🟢 Resolved
BUG-003	Distributed TradeCreated application	🟢 Resolved
BUG-004	Trade-state delivery before TradeCreated	🟢 Resolved
BUG-005	Settlement applied to non-participant	🟢 Resolved
BUG-006	Duplicate settlement	🟢 Resolved
BUG-007	Partial-fill settlement accounting	🟢 Resolved
BUG-008	Quote amount overflow	🟢 Resolved
BUG-009	GossipSub timing-dependent test	🟢 Resolved
BL-2026-08-31-001	Pending trade-state handling	🟢 Fixed
AUDIT-002	Order UUID replacement / snapshot order semantics	🟢 Closed / Verified
AUDIT-003	Self-trading prevention	🟢 Closed / Verified
AUDIT-004	Settlement lifecycle authorization	🟢 Closed / Verified
AUDIT-008	Remote snapshot validation	🟢 Closed / Verified
AUDIT-009 / ARCH-004	Trade identity/order integrity/deterministic matching	🟢 Closed / Verified
AUDIT-009B	Snapshot order trust boundary	🟢 Closed / Verified

These remain in the historical record but should not be treated as current vulnerabilities.

7. Verification Record
The current implementation has been regression-tested with:

cargo test --workspace

The supplied workspace run completed successfully.

Verified test groups included:

Common: 1 passed;

Protocol: 3 passed;

Balances: 8 passed;

CLI: 11 passed;

Exchange: 1 passed;

Network: 1 passed;

Orderbook: 30 passed;

Security: 17 passed;

Settlement: 6 passed;

Trade lifecycle: 14 passed;

Doc-test targets: passed.

Total individual tests reported: 91 passed, 0 failed.

The workspace test run reported no test failures or compiler warnings in the supplied output.

Additional direct source inspection verified self-trade prevention in OrderBook::match_order().

The relevant candidate predicate explicitly rejects:

existing.trader == incoming.trader

The dedicated regression test is:

matching_rejects_self_trade

The current repository checkpoint was also reported as clean:

On branch peer-dial
nothing to commit, working tree clean

Current commits associated with the audit documentation and remediation include:

54f8234 Harden orderbook sync and update security audit
0172e2b Update audit findings for verified remediations

8. Current Audit Position
Status: 🟡 Security hardening in progress

Final security sign-off: 🔴 Not ready

Closed / verified
authenticated sender binding;

order ownership;

cancellation authorization;

deterministic trade identity;

trade/order semantic validation;

deterministic matching;

self-trade prevention;

snapshot validation;

snapshot order integrity;

trade-state authorization;

stale-state protection;

settlement idempotency;

settlement arithmetic safety;

partial-fill accounting;

pending distributed trade handling;

removed-order resurrection protection;

snapshot transaction-like application.

Remaining substantive review
AUDIT-007 — Fixed-point rounding/conservation;

AUDIT-010 — Snapshot consistency/version semantics.

Architectural investigation
ARCH-001 — Trade application ownership.

Deferred
BUG-001 — Per-order balance reservations;

ARCH-002 — Synchronization/OrderBook boundaries;

ARCH-003 — Command handler boundaries.

9. Next Security Review Actions
The next review sequence is:

Establish the fixed-point rounding and asset-conservation policy.

Define snapshot freshness/version semantics.

Complete the trade-application ownership review.

Re-run the complete workspace regression suite after any changes.

Update AUDIT_FINDINGS.md and AUDIT_CHECKLIST.md to reflect verified results.

Prepare the resulting implementation for independent external audit.

No broad architectural refactoring should be undertaken merely for code organization. Security and correctness invariants should be established first.

10. External Audit Boundary
An independent auditor should still verify:

complete network-to-state mutation paths;

remote trade validation;

synchronization authority;

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

live penetration testing;

adversarial multi-node testing.

The source-level audit should remain open until the remaining rounding/conservation and snapshot-consistency questions have been resolved or explicitly accepted by the protocol owners and independently reviewed.

11. Current Conclusion
OpenDEX has materially strengthened its security boundaries and has regression coverage for the principal implemented controls.

The previously identified authentication, order-authority, deterministic trade identity, trade/order integrity, deterministic matching, snapshot validation, settlement-lifecycle, and self-trading findings are documented as remediated and verified.

The remaining substantive work is concentrated on:

fixed-point economic semantics and asset conservation;

snapshot consistency and freshness/version semantics;

architectural ownership of trade application;

future per-order reservation accounting.

Current conclusion: OpenDEX remains in audit progress and is not yet ready for final security sign-off.

The authoritative detailed finding status remains in:

audit/AUDIT_FINDINGS.md

The authoritative audit control/checklist remains in:

audit/AUDIT_CHECKLIST.md

This security log is intentionally kept concise and should not be expanded back into the previous large historical log unless new security evidence requires it.
