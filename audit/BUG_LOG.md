OpenDEX — Bug & Issue Tracker
Purpose
This document tracks bugs, technical issues, architectural concerns, deferred fixes, regressions, and items requiring later investigation.
It is deliberately separate from the Development Log and Security Log.
The Development Log records milestones.
The Security Log records security findings, threat-model concerns, mitigations, and audit preparation.
This document records implementation and architectural issues that may require attention.
Issues should remain here until they are:
• fixed and verified;
• explicitly accepted as intentional;
• superseded by another design; or
• determined not to be an issue.
Status Definitions
• 🔴 Open — requires investigation or action
• 🟠 Investigating — actively being reviewed
• 🟡 Deferred — known issue intentionally postponed
• 🟢 Resolved — fixed and verified
• ⚪ Accepted — known behaviour considered intentional

Open Issues
ARCH-004 — Trade/Order Consistency Validation Is Incomplete
Status: 🔴 Open
Priority: Critical
Category: Correctness / Security Boundary
OrderBook::apply_trade() currently verifies that the referenced orders exist before applying the trade, but the complete semantic relationship between a remotely supplied Trade and those orders has not yet been established.
The following invariants require explicit validation before any order-book mutation:
• buy order has Side::Buy;
• sell order has Side::Sell;
• buy order trader matches trade.buyer;
• sell order trader matches trade.seller;
• trade market matches both referenced orders;
• trade amount is non-zero;
• trade amount does not exceed either order's remaining amount;
• trade price satisfies the relevant order-price constraints;
• maker corresponds to the resting order;
• taker corresponds to the incoming order;
• the trade identity is consistent with the deterministic matching rules where independently verifiable;
• remotely supplied trades cannot introduce contradictory order-book state.
Required action
1. Document the authoritative trade/order invariants.
2. Determine the architectural owner of semantic trade validation.
3. Validate the complete trade before mutating order state.
4. Add unit tests for contradictory trade/order combinations.
5. Add adversarial distributed tests.
6. Ensure pending-trade processing uses the same validation path.
7. Ensure synchronization cannot bypass the validation boundary.
Security relationship
This issue is related to SEC-009 — Remote TradeCreated Semantic Validation.
Authenticated sender identity does not establish that the contents of a TradeCreated message are truthful or consistent with the referenced orders.

ARCH-001 — Trade Application State Has Overlapping Mutation Paths
Status: 🟠 Investigating
Priority: High
Trade application remains reachable through multiple paths:
Remote TradeCreated
↓
handle_trade_created()
↓
OrderBook::apply_trade()
Remote OrderCreated
↓
matching
↓
OrderBook::apply_trade()
Order arrival during synchronization
↓
upsert_order()
↓
pending trade re-check
↓
OrderBook::apply_trade()
Existing idempotency mechanisms reduce the risk of duplicate mutation, and current regression tests pass.
However, architectural ownership remains distributed.
Required review
• establish one clear owner for applying a trade;
• define ownership of pending_trades;
• define ownership of applied_trades;
• define when orders are removed;
• define when trade persistence occurs;
• ensure local matching and remote trade application use the same invariants;
• ensure synchronization cannot bypass those invariants;
• ensure ARCH-004 validation is performed regardless of entry path.
No refactor should be performed merely to reduce code duplication. The intended invariant and ownership model should be documented first.

BUG-001 — Per-Order Balance Reservations
Status: 🟡 Deferred
Priority: High
The current BalanceBook architecture primarily tracks aggregate available and locked balances.
The system still requires an explicit relationship between the locked funds and the order that created the reservation.
This becomes important when multiple orders, partial fills, cancellation, settlement, concurrent trades, and out-of-order distributed events occur.
Required future work
• associate reservations with order IDs;
• consume only the relevant reservation during settlement;
• unlock only the relevant remaining reservation during cancellation;
• support multiple simultaneous orders;
• preserve partial-fill correctness;
• persist reservations across restart;
• test conflicting and out-of-order events;
• verify duplicate settlement cannot consume a reservation twice.
This issue remains relevant to both accounting correctness and the security properties described by SEC-007 — Per-Order Balance Reservation Model.

BUG-002 — transfer_locked() Currently Unused in Production Path
Status: 🟡 Deferred
Priority: Low
BalanceBook::transfer_locked() remains covered by tests but is not currently part of the production execution path.
The compiler warning should remain deferred until the balance and settlement architecture stabilises.
The method should not be removed merely to silence the warning because it may represent a legitimate future settlement primitive.

Resolved Issues
BUG-SEC-001 — Application-Level Sender Identity Is Not Bound to Authenticated Peer Identity
Status: 🟢 Resolved
Severity: High
Category: Security / Authorization / State Integrity
Component: crates/p2p-node/src/message_handler.rs, crates/protocol/src/message.rs
Discovered: 2026-08-29
Resolved: 2026-08-30
Commit: d79be81
Branch: peer-dial
Summary
OpenDEX previously relied on the application-level sender field in OpenDexMessage for authorization without establishing that the claimed identity corresponded to the authenticated libp2p peer.
This created an application-level identity spoofing risk.
Resolution
The message-processing path now establishes the authenticated peer identity before applying authorization checks.
Implemented protections include:
• incoming GossipSub messages require the claimed OpenDexMessage.sender identity to correspond to the authenticated PeerId;
• forged sender identities are rejected before state mutation;
• order creation requires the claimed trader to match the authenticated sender;
• order cancellation requires authorization by the order owner;
• trade-state changes require the authenticated sender to be a participant in the trade;
• unauthorized trade-state changes are rejected without mutating the existing trade;
• pending trade-state changes retain their authenticated sender and are revalidated when the corresponding trade arrives;
• previously removed orders cannot be resurrected through later synchronization;
• stale synchronization cannot roll existing trade state backward.
Regression coverage
Dedicated security regression coverage has been added in:
crates/p2p-node/tests/security.rs
The security suite contains 14 executable tests covering, among other cases:
• forged sender rejection;
• authenticated sender acceptance;
• order ownership validation;
• unauthorized order cancellation;
• buyer trade-state authorization;
• seller trade-state authorization;
• unauthorized trade-state rejection;
• forged-message rejection without state mutation;
• impersonated order rejection without state mutation;
• stale state synchronization;
• removed-order resurrection;
• stale trade-state synchronization.
Verification
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
Results:
• Workspace tests: 56 passed / 0 failed
• Security tests: 14 passed / 0 failed
• Clippy: clean with -D warnings
• Formatting: clean
• Network integration test: passed
Current assessment
Fix: Complete
Regression verification: Complete
Dedicated security coverage: Complete
This issue is resolved.
The corresponding security finding should remain historically documented in the Security Log, but it should not be treated as an active implementation issue.

SEC-002 — Remote order cancellation authority
Priority: Critical
Status: 🟢 Closed — Remediated / Verified
Original issue:
Remote OrderCancelled messages could previously cause removal of an order without verifying that the authenticated network sender was authorized to cancel that order.
Fix implemented:
Cancellation processing now requires:
The GossipSub message to originate from an authenticated libp2p PeerId.
OpenDexMessage.sender to match the authenticated PeerId.
OrderCancelled.trader to match the authenticated sender.
The referenced order to exist.
The authenticated sender to match the order owner (order.trader).
Unauthorized requests are rejected before OrderBook::remove_order() is reached.
Verification performed:
Non-owner cancellation rejected.
Forged cancellation trader identity rejected.
Forged message sender identity rejected.
Rejected cancellation does not mutate order state.
Unknown cancellation handled safely.
Duplicate/stale cancellation behaviour verified.
Stale orders cannot be resurrected after cancellation.
Dedicated cancellation security tests: 3 passed, 0 failed.
Full workspace test suite: all tests passed, 0 failed.
Verification result:
🟢 SEC-002 successfully remediated and verified.
Residual note:
The current protocol uses the authenticated libp2p PeerId as the trader identity. If the protocol later introduces a separate trader identity, an authenticated PeerId-to-trader identity binding will need to be defined and verified.

Disposition:
CLOSED. Do not reopen SEC-002 for unrelated observations. Any additional findings identified during subsequent review must be recorded separately in the review queue after SEC-002.
BUG-003 — Distributed TradeCreated Application
Status: 🟢 Resolved
Trades arriving before their corresponding orders are retained and applied when the required orders become available.
Duplicate application is idempotent.
Regression coverage is present.
Note: this does not resolve ARCH-004. Distributed delivery correctness and semantic validation are separate concerns.

BUG-004 — Trade State Arriving Before TradeCreated
Status: 🟢 Resolved
A TradeStateChanged event received before the corresponding trade is retained and applied once the trade becomes available.
Pending-state authorization is also covered by the security regression suite.

BUG-005 — Local Settlement Applied to Non-Participants
Status: 🟢 Resolved
Settlement now changes balances only when the local node participates in the trade.

BUG-006 — Duplicate Settlement
Status: 🟢 Resolved
Settlement is idempotent and duplicate Settled events do not move funds twice.

BUG-007 — Partial-Fill Settlement Accounting
Status: 🟢 Resolved
Settlement consumes only the quantity actually traded rather than the entire original order amount.

BUG-008 — Quote Amount Overflow
Status: 🟢 Resolved
Settlement rejects quote calculations that would overflow rather than mutating balances incorrectly.
Checked balance arithmetic is also used during settlement.

BUG-009 — GossipSub Test Relied on Timing
Status: 🟢 Resolved
The network integration test now observes actual subscription events rather than depending on an arbitrary sleep.

Architectural Issues
ARCH-002 — sync.rs Accesses OrderBook Internals
Status: 🟡 Deferred
Priority: Medium
sync.rs is currently cohesive, but snapshot application accesses several OrderBook internals directly.
The architectural review should determine whether these accesses are necessary for synchronization or whether they bypass domain invariants.
This review should be coordinated with the outstanding synchronization-security work.

ARCH-003 — Command Handler Review
Status: 🟡 Deferred
Priority: Medium
command_handler.rs has grown as command functionality has expanded.
It should eventually be reviewed for responsibility boundaries, particularly around:
• order creation;
• balance locking;
• publication;
• local matching;
• settlement;
• trade-state transitions.
No split should be performed merely because the file is large.

Current Review Queue
The following areas remain active before final security/audit readiness:
1. 🔴 Trade/order semantic consistency validation — ARCH-004
2. 🟡 Per-order balance reservations — BUG-001
3. 🟠 Trade application ownership — ARCH-001
4. 🟡 Synchronization invariants — ARCH-002
5. 🟡 Cancellation semantics
6. 🟡 Persistence/restart behaviour
7. 🟠 Replay and stale-event resistance
8. 🟠 Resource-exhaustion limits
9. 🟠 Adversarial multi-node testing
10. 🟡 Remaining dead code
11. 🟡 Command-handler coupling — ARCH-003
12. 🟠 Full remote-message semantic validation

Completed Security/Correctness Work
The following areas have received implementation changes and regression verification:
• authenticated sender identity;
• forged sender rejection;
• order ownership validation;
• cancellation authorization;
• trade participant authorization;
• pending trade-state authorization;
• stale trade-state protection;
• removed-order resurrection protection;
• stale state synchronization protection;
• settlement idempotency;
• settlement overflow protection;
• checked balance arithmetic;
• partial settlement accounting;
• distributed pending-trade handling;
• GossipSub subscription synchronization.
These completed controls do not imply that all remote message semantics are trusted.
In particular, authenticated origin does not establish semantic validity of a remotely supplied trade.

Verification Baseline
Commit: d79be81
Branch: peer-dial
cargo fmt --all                                  PASS
cargo test --workspace                           PASS
cargo clippy --workspace --all-targets \
--all-features -- -D warnings                  PASS

Workspace tests:     56 passed / 0 failed
Security tests:      14 passed / 0 failed
This remains the current clean reproducible baseline.

Issue Discipline
A discovered issue should not automatically become a code change.
Before changing behaviour:
1. reproduce or demonstrate the issue;
2. identify the invariant involved;
3. determine the correct architectural owner;
4. add regression coverage where appropriate;
5. make the smallest justified change;
6. run complete workspace validation;
7. record the result here.
A known issue is preferable to an undocumented change in security-sensitive distributed behaviour.

Current Assessment
OpenDEX has successfully closed the previously identified application-level sender impersonation issue and has dedicated regression coverage for that boundary.
The principal unresolved security-sensitive implementation issue is now trade/order semantic validation.
The current architecture should therefore be treated as:
Authenticated peer
↓
Application identity binding
↓
Authorization
↓
?
Trade semantic validation
↓
OrderBook mutation
↓
Settlement
The ? represents the next major review boundary.
The system is not yet ready for final security audit. The next security/correctness milestone should establish and test the complete invariants governing remotely supplied TradeCreated messages before further architectural refactoring is undertaken.




















Bug Log Update
Date: 2026-08-31
Branch: peer-dial
Commit: 2c846ac — Harden pending trade state handling
Checkpoint: checkpoint-pending-trade-state
Bug: Pending trade state handling during out-of-order message delivery
Status: Fixed and verified
Description:
Trade-state messages can arrive before the corresponding TradeCreated message. The node therefore needs to temporarily retain the state change and apply it once the trade becomes available.
The pending-state handling also needed to correctly deal with duplicate messages, missed intermediate states, and exceptional trade states without incorrectly ordering them.
Cause:
Pending trade states required additional logic to determine which incoming state should replace an existing pending state. Treating every TradeState as a simple ranked sequence could incorrectly order exceptional states such as Disputed, Cancelled, and Expired.
Fix:
• Added explicit identification of the linear happy-path trade states.
• Allowed a direct valid transition to replace the currently pending state.
• Allowed a later happy-path state to replace an earlier happy-path state when intermediate messages were missed.
• Prevented exceptional states from being ordered using the happy-path ranking.
• Preserved the authenticated sender associated with the pending state.
• Added validation when the pending state is eventually applied to the received trade.
Regression Tests Added:
• pending_trade_state_advances_through_happy_path
• pending_trade_state_keeps_highest_happy_path_state
• unauthorized_pending_state_is_rejected_when_trade_arrives
• settled_state_before_trade_created_is_applied_when_trade_arrives
Verification:
• cargo fmt — passed
• cargo test -p p2p-node — passed
• cargo test --workspace — passed
• cargo check — passed
• git diff --check — passed
Result:
Bug fixed. Pending trade states now handle out-of-order delivery and missing intermediate states while avoiding invalid state ordering.









OpenDEX — Bug Log
BL-2026-08-31-001 — Pending Trade State Handling
Date: 31 August 2026
Severity: Medium
Status: Fixed
Component: crates/p2p-node/src/message_handler.rs
Related Components: trade_state.rs, trades.rs
Commit: 2c846ac
Checkpoint: checkpoint-pending-trade-state
Description
Pending TradeStateChanged messages could arrive before the corresponding TradeCreated message.
The previous handling could retain an earlier pending state when a later valid state was received, particularly when intermediate state messages had been missed.
Resolution
Pending trade-state handling was hardened so that:
• Duplicate pending states are ignored safely.
• Directly valid state transitions replace the previous pending state.
• Later states on the normal happy path can replace earlier pending states when intermediate messages were missed.
• Exceptional states such as Disputed, Cancelled, and Expired are not ordered using the normal state ranking.
• The authenticated sender is retained with the pending state.
• The sender is revalidated against the trade participants when the trade is eventually received.
• Actual state transitions continue to pass through the protocol state machine.
Verification
Working tree is clean after commit.
2c846ac (HEAD -> peer-dial) Harden pending trade state handling
checkpoint-pending-trade-state
Follow-up
Continue the message_handler.rs structural audit before making additional changes.

Bug Log — Finding 1
StateResponse Authorization / Authority Boundary
Finding:
StateResponse is structurally validated and then applied as an authoritative snapshot without a separate authorization or provenance check.

Remediation action:
Before changing implementation, establish the intended synchronization invariant:

Determine whether every authenticated peer is permitted to provide authoritative state.
If not, define the authorization/trust mechanism for authoritative synchronization.
If yes, define how conflicting or stale snapshots are prevented from overwriting valid local state.
Add regression tests for the chosen invariant before implementing the remediation.
Implementation status: Not remediated. No code change made as part of this finding.
BUG-SEC-002 — StateResponse Synchronization Authority
Date: 2026-09-02
Status: Open
Severity: High / potentially Critical
Category: Security / Authorization / State Integrity
Component: crates/p2p-node/src/sync.rs, apply_snapshot()

Finding
The remote StateResponse path currently reaches:

StateResponse
↓
StateSnapshot
↓
apply_snapshot()
↓
OrderBook / TradeBook mutation
↓
persistence

Network authentication establishes the identity of the sending peer, but the current protocol does not yet establish whether that peer is authorized to provide authoritative synchronization state.

apply_snapshot() therefore represents a significant security-sensitive state mutation boundary.

Required Work
Before implementing a narrow code-level fix, establish the intended synchronization trust model:

Define which peers are authorized to provide authoritative state.
Determine whether snapshots are authoritative or advisory.
Define which snapshot fields may affect local state.
Validate orders and trades before insertion or replacement.
Define stale-snapshot handling.
Define conflicting-snapshot resolution.
Prevent unauthorized order/trade resurrection.
Define whether remote removals are authoritative.
Define replay behaviour.
Ensure rejected snapshots cause no protected-state mutation.
Add adversarial regression tests.
Verify crash-consistent persistence during snapshot application.
Security Invariant
An authenticated peer must not automatically be treated as an authorized state authority.

The receiving path should ultimately enforce:

UNTRUSTED NETWORK INPUT
↓
Authenticated sender
↓
Synchronization authorization
↓
Snapshot validation
↓
State reconciliation
↓
Domain mutation
↓
Crash-consistent persistence

Current Assessment
Open — investigation required.

No implementation change is recorded by this checkpoint.

The security log classifies this as a security finding requiring investigation rather than a confirmed exploitable vulnerability because the final impact depends on the intended OpenDEX synchronization trust model.

Related Security Review
Security checkpoint: 2026-09-02 — State synchronization trust boundary

Next Action
Define the synchronization authority model and authoritative StateSnapshot invariants before modifying apply_snapshot().
