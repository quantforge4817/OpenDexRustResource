OpenDEX — Security Log
Purpose
This document records security-relevant observations, threat-model concerns, security findings, mitigations, and audit preparation work.
It is intentionally separate from the Development Log and Bug Tracker.
Not every security observation is immediately a bug.
Some entries identify an architectural risk that requires investigation before a vulnerability can be confirmed.
Security-sensitive changes should be supported by regression tests wherever practical.

Security Review Status
Current phase: 🟡 Security architecture preparation
Security hardening: Not yet considered complete
Final audit: Pending
The current priority is to stabilise architectural boundaries before performing the final security-hardening pass.

Security Principles
OpenDEX should assume that network messages are untrusted.
Security-sensitive decisions should therefore be:
• explicit
• locally validated
• deterministic
• bounded by clear domain responsibilities
• resistant to duplicate messages
• resistant to out-of-order delivery
• resistant to malformed input
• independently testable
The P2P layer should not implicitly confer authority on remote peers.

Findings
SEC-001 — Remote message trust boundary
Status: 🔴 Open
Remote GossipSub messages are deserialized and subsequently passed into domain handlers.
The security boundary between:
untrusted network data
↓
protocol decoding
↓
domain state mutation
requires explicit review.
Questions to resolve
• Which fields are trusted?
• Which fields require validation?
• Which operations may a remote peer legitimately request?
• Which operations require local ownership?
• Can malformed values reach persistent state?
• Can a remote peer cause unauthorised state transitions?
• Can replayed messages cause unintended effects?
Priority: Critical

SEC-002 — Remote order cancellation authority
Status: 🔴 Open
OrderCancelled messages can cause local order state to be removed.
The current architecture requires explicit review of whether a remote cancellation is sufficiently authenticated/authorised.
Required investigation
Determine how the system establishes that the sender is authorised to cancel the referenced order.
A peer being able to publish a validly encoded message should not automatically imply authority over another trader's order.
Priority: Critical

SEC-003 — Trade-state transition authority
Status: 🔴 Open
TradeStateChanged messages can advance trade state.
The transition state machine validates whether a transition is structurally legal, but structural validity is not necessarily equivalent to authorisation.
Required investigation
Determine:
• who may advance each state
• whether the sender must be a trade participant
• whether different states require different authorities
• whether a remote peer can force settlement
• whether replayed state changes are harmless
• whether timestamps have security significance
Priority: Critical

SEC-004 — Remote settlement trigger
Status: 🔴 Open
A transition to Settled reaches the local settlement path.
Settlement changes balances and is therefore security-sensitive.
The existing implementation has useful protections:
• local participant checking
• idempotency
• partial-fill accounting
• overflow protection
However, the authority to cause a trade to become Settled through a remote TradeStateChanged message requires dedicated review.
Key question
Can an unauthorised remote peer cause another node to settle a trade?
Priority: Critical

SEC-005 — State synchronization authority
Status: 🔴 Open
Remote StateResponse messages ultimately reach snapshot application.
Snapshot synchronization therefore represents a significant trust boundary.
Required review
• snapshot authenticity
• sender authority
• stale snapshot handling
• conflicting state
• malicious state injection
• order resurrection
• trade resurrection
• persistence corruption
• replay behaviour
apply_snapshot() should be treated as security-sensitive until this review is complete.
Priority: Critical

SEC-006 — Distributed replay and duplicate events
Status: 🟠 Investigating
OpenDEX deliberately supports idempotent and out-of-order event processing.
This is a major correctness requirement, but replay resistance needs to be distinguished from simple idempotency.
The review should determine whether an attacker can repeatedly submit valid historical events to create undesirable state transitions or resource consumption.
Priority: High

SEC-007 — Per-order balance reservation model
Status: 🔴 Open
Aggregate locked balances may make it difficult to prove that a particular trade or cancellation consumes only funds belonging to the correct order.
This is both a correctness and security concern.
Required design
Funds should be attributable to the reservation that created the obligation.
The security review should cover:
• multiple orders
• partial fills
• cancellation
• concurrent trades
• duplicate settlement
• out-of-order events
• restart/recovery
Priority: Critical

SEC-008 — Network message resource consumption
Status: 🟡 Review required
Malformed or excessive network messages may create CPU, memory, logging, or persistence pressure.
The protocol boundary should eventually be reviewed for:
• message size limits
• payload limits
• deserialization cost
• repeated invalid messages
• pending-event growth
• snapshot size
• persistence amplification
Priority: High

Security Controls Already Present
The current implementation already contains several useful defensive properties.
Idempotent settlement
Duplicate settlement does not repeatedly move funds.
Partial-fill accounting
Settlement consumes only the traded quantity.
Overflow protection
Quote calculation overflow is rejected before balance mutation.
Out-of-order event handling
Trade and trade-state events can be retained until their dependencies exist.
Idempotent trade application
Repeated trade delivery does not repeatedly apply the same trade.
Persistence testing
Order/trade persistence and restart behaviour have regression coverage.
Integration testing
Two-node networking behaviour is tested through real libp2p swarms.
These controls reduce risk but do not replace an explicit authority/authentication review.

Security Review Queue
The next dedicated security review should proceed approximately in this order:
1. Remote message authority
2. Trade-state transition authority
3. Settlement authority
4. Remote cancellation authority
5. Snapshot/synchronization authority
6. Per-order balance reservations
7. Replay resistance
8. Resource-exhaustion limits
9. Persistence integrity
10. Multi-node adversarial testing

Audit Preparation
The final security audit should begin only after:
Architectural boundaries stabilised
↓
Known bugs/issues reviewed
↓
Security findings investigated
↓
Security-sensitive invariants documented
↓
Regression tests expanded
↓
Adversarial multi-node testing
↓
Final audit
The objective is not to claim that the system is secure before the audit.
The objective is to make the security model sufficiently explicit that an auditor can identify:
• trust boundaries
• authority boundaries
• state-transition rules
• balance invariants
• persistence boundaries
• network assumptions
• replay behaviour
• failure handling
without reconstructing the architecture from scattered event handlers.

Current Security Principle
Do not treat a valid message as an authorised message.
Protocol validity, state-machine validity, and authority are separate questions.
Every security-sensitive remote operation should eventually have an explicit answer to:
Who is allowed to cause this state transition, and how does the receiving node verify that authority?
Until those answers are established, the relevant paths remain security-review items rather than completed security controls.

OpenDEX — Security Log Update
Security Review Checkpoint
Date: 29 August 2026
The remote-message handling paths have now been reviewed against the current message_handler.rs, orderbook.rs, trades.rs, trade_state.rs, sync.rs, and protocol message definitions.
The review identified that several previously documented security concerns are now sufficiently demonstrated to be treated as active security findings rather than purely architectural questions.
No security-hardening changes have been made yet.
The current objective remains to establish the security invariants before modifying the implementation.

SEC-001 — Remote message trust boundary
Status: 🔴 Open
Priority: Critical
Remote GossipSub messages are deserialized and passed into domain handlers.
The current message boundary is:
GossipSub message
↓
OpenDexMessage deserialization
↓
payload deserialization
↓
domain handler
↓
state mutation
The protocol currently validates message structure and payload decoding, but semantic validation and authorization are incomplete.
The review confirms that protocol validity, domain validity, and authorization must remain separate security questions.
Required review remains:
• authenticated sender identity
• payload validation
• domain invariants
• operation authorization
• replay handling
• persistence safety
• resource limits

SEC-002 — Remote order cancellation authority
Status: 🔴 Confirmed security weakness
Priority: Critical
OrderCancelled messages currently contain only the referenced order_id.
The receive path is:
OrderCancelled
↓
decode payload
↓
handle_order_cancelled()
↓
OrderBook::remove_order()
No authorization check is currently performed.
The receiver therefore does not establish that the sender is authorized to cancel the referenced order.
Additionally, adding a check against OpenDexMessage.sender alone would not solve the problem because that field is not authenticated.
Required investigation:
• establish the authenticated identity of the GossipSub sender
• determine how trader identity maps to authenticated peer identity
• require authorization from the order owner
• define behaviour for cancellation of unknown/already-consumed orders
• add adversarial regression tests

SEC-003 — Trade-state transition authority
Status: 🔴 Confirmed security weakness
Priority: Critical
TradeStateChanged messages are currently authorized using:
Trade buyer/seller
↓
compare against msg.sender
However, msg.sender is an arbitrary string supplied by the remote message itself.
The current implementation therefore does not cryptographically establish that the sender is the claimed trade participant.
This means the existing participant check is a domain-level identity comparison, not an authenticated authorization mechanism.
Further review is required to define whether:
• either trade participant may perform every transition
• different transitions require different authorities
• certain transitions require both participants
• settlement requires stronger authorization
• exception states require separate authorization
The state machine's structural transition rules do not by themselves establish authorization.

SEC-004 — Remote settlement trigger
Status: 🔴 Open — authorization weakness confirmed
Priority: Critical
A remote TradeStateChanged message reaching Settled invokes:
TradeStateChanged
↓
advance_trade_to_state()
↓
Settled
↓
settle_local_trade()
Settlement itself contains useful defensive controls:
• local participant checking
• idempotency
• partial-fill accounting
• overflow protection
However, the authorization preceding settlement is currently based on the attacker-controlled msg.sender field.
Therefore the security question is unresolved:
Can an unauthenticated remote peer cause a valid local participant
to execute settlement by claiming to be an authorized participant?
This path must not be considered secure until authenticated identity and transition authority are established.

SEC-005 — State synchronization authority
Status: 🔴 Open
Priority: Critical
Remote StateResponse messages can reach:
StateResponse
↓
StateSnapshot
↓
apply_snapshot()
↓
OrderBook / TradeBook mutation
↓
persistence
apply_snapshot() currently accepts remote state without an explicit sender-authority check.
Snapshot application can affect:
• active orders
• removed orders
• trades
• applied trade tracking
• pending trade state indirectly through order updates
• persistent local state
The review therefore confirms that snapshot application is a security-sensitive trust boundary.
Required design decisions:
• who is authorized to provide authoritative state
• whether snapshots are advisory or authoritative
• how conflicting snapshots are resolved
• how stale snapshots are rejected
• how removed orders are protected from resurrection
• how trade state conflicts are handled
• whether snapshots require authentication or signatures
• how replayed snapshots are handled

SEC-006 — Distributed replay and duplicate events
Status: 🟠 Investigating
Priority: High
The implementation deliberately supports duplicate and out-of-order delivery.
Existing protections include:
• idempotent trade insertion
• applied-trade tracking
• idempotent trade-state transitions
• pending trade handling
• pending trade-state handling
• idempotent settlement
However, idempotency does not automatically provide replay resistance.
A future review must distinguish:
duplicate event
from:
old but valid event intentionally replayed
Questions remain around:
• stale state transitions
• replayed cancellation messages
• replayed snapshots
• replayed trade events
• pending-event growth
• persistence amplification

SEC-007 — Per-order balance reservation model
Status: 🔴 Open
Priority: Critical
The current balance architecture primarily tracks aggregate available and locked balances.
The security review confirms that settlement correctness cannot be fully established until the relationship between:
order
↓
balance reservation
↓
trade
↓
partial fill
↓
settlement
↓
remaining reservation
is explicit.
Required design review:
• reservation ownership by order ID
• multiple simultaneous orders
• partial fills
• cancellation
• concurrent trades
• duplicate settlement
• out-of-order events
• restart/recovery
This remains one of the highest-priority architectural security concerns.

SEC-009 — Remote TradeCreated semantic validation
Status: 🔴 Confirmed security weakness
Priority: Critical
Remote TradeCreated messages are accepted after payload deserialization and can reach:
handle_trade_created()
↓
OrderBook::apply_trade()
OrderBook::apply_trade() currently establishes only that both referenced order IDs exist.
It does not fully establish that the supplied Trade is consistent with those orders.
The current validation does not explicitly verify all of the following:
• buy_order references a Buy order
• sell_order references a Sell order
• the referenced orders belong to buyer and seller
• buyer matches the buy-order trader
• seller matches the sell-order trader
• market matches both referenced orders
• price satisfies the relevant order-price constraints
• amount is non-zero
• amount does not exceed either order's remaining amount
• maker corresponds to the resting order
• taker corresponds to the incoming order
• the trade identity is consistent with the deterministic matching rules
This creates a potential path for a remote peer to submit a structurally valid but semantically fabricated trade.
Because apply_trade() mutates order quantities and can remove fully consumed orders, this must be treated as a security-sensitive domain boundary.
Required action:
1. Define the authoritative trade invariants.
2. Establish the correct architectural owner for validation.
3. Add regression tests for malformed/contradictory remote trades.
4. Ensure validation occurs before order-book mutation.
5. Test adversarial multi-node delivery.

SEC-010 — Application sender identity is unauthenticated
Status: 🔴 Confirmed security weakness
Priority: Critical
OpenDexMessage currently contains:
sender: String
The sender value is serialized as part of the application message.
On receipt, authorization-sensitive code uses:
msg.sender
as the claimed identity.
However, the GossipSub layer is configured for signed messages and therefore already has an authenticated network-level peer identity.
The current implementation does not establish a cryptographic binding between:
authenticated GossipSub peer
and:
OpenDexMessage.sender
Therefore:
msg.sender == trade participant
does not prove:
authenticated network sender == trade participant
This is the most important identity-boundary finding currently identified.
Required design decision:
authenticated PeerId
↓
identity binding
↓
authorized trader identity
The exact mechanism should be determined before implementing authorization checks.
Until then, application-level sender fields must not be treated as proof of authority.

SEC-011 — Trade-state fast-forward semantics require authorization review
Status: 🟠 Investigating
Priority: High
advance_trade_to_state() intentionally permits a node to advance through the normal happy-path states when intermediate events were not observed.
This supports distributed convergence and out-of-order delivery.
However:
state is reachable
does not necessarily imply:
sender is authorized to assert that state
A remote message requesting a later state can therefore cause local inference of intermediate states.
The security review must determine whether this is an intended protocol property and, if so, which authenticated authority may request each fast-forward.
In particular, settlement must not be authorized merely because the requested target state is structurally reachable.

Current Confirmed Security Boundary
The current review establishes the following distinction:
Protocol validity
≠
Domain validity
≠
Authorization
≠
Settlement authority
A message can currently be:
valid JSON
↓
valid protocol payload
↓
structurally valid state transition
without establishing:
authorized actor
This distinction is now considered a core OpenDEX security invariant.

Revised Security Review Priority
The current review order is:
1. 🔴 Authenticated peer identity and sender binding
2. 🔴 Remote TradeCreated semantic validation
3. 🔴 Trade-state transition authority
4. 🔴 Settlement authorization
5. 🔴 Remote order cancellation authorization
6. 🔴 Snapshot/synchronization authority
7. 🔴 Per-order balance reservations
8. 🟠 Replay and stale-event resistance
9. 🟠 Resource-exhaustion limits
10. 🟠 Persistence integrity
11. Multi-node adversarial testing

Current Security Status
Security hardening remains incomplete.
No claim of security should be made at this stage.
The current implementation has useful defensive properties, particularly:
• idempotent settlement
• partial-fill accounting
• overflow protection
• idempotent trade application
• out-of-order event handling
• persistence testing
• genuine two-node networking tests
However, the newly confirmed identity and remote-state-mutation weaknesses mean that the system is not yet ready for final security audit.
The immediate objective is to establish explicit trust and authority boundaries before modifying security-sensitive domain behaviour.






























OpenDEX — Security Log
SEC-001 — Unauthenticated Application-Level Sender Identity
Date: 2026-08-29
Status: Open
Severity: High
Category: Authentication / Authorization / Peer Identity
Finding
OpenDEX messages contain an application-level sender string:
pub struct OpenDexMessage {
pub message_type: MessageType,
pub sender: String,
pub payload: String,
pub timestamp: u64,
}
Incoming authorization decisions currently rely on this value. For example, TradeStateChanged verifies that msg.sender is a trade participant.
However, the implementation does not currently establish that msg.sender corresponds to the authenticated libp2p peer that actually published the message.
Security Risk
The network layer provides authenticated peer identity through libp2p/Gossipsub signing, but the application layer currently maintains a separate identity supplied by the message itself.
This creates a trust-boundary gap:
Authenticated libp2p PeerId
│
│ NOT explicitly verified against
▼
OpenDexMessage.sender
│
▼
Authorization decision
An attacker may therefore be able to construct messages claiming another participant's application identity.
Confirmed Attack Surface
OrderCancelled is particularly exposed because the cancellation handler currently identifies the target order only by order_id and does not verify that the sender owns that order.
A malicious peer could therefore attempt to cancel another participant's order through the P2P network.
Potential Impact
• Unauthorized order cancellation.
• Trade-participant impersonation.
• Unauthorized trade-state changes if sender spoofing is possible.
• Malicious or inconsistent state propagation.
• Loss of trust in application-level peer authorization.
Required Remediation
Bind application identity to authenticated network identity.
The receiving node should:
1. Obtain the authenticated publisher PeerId.
2. Verify its relationship to OpenDexMessage.sender.
3. Prefer the authenticated identity for authorization decisions.
4. Verify order ownership before processing OrderCancelled.
5. Apply the same principle to trade-state changes and other privileged messages.
6. Add tests for forged sender identities and unauthorized actions.
Current Assessment
No direct balance-theft exploit has been demonstrated from the reviewed code.
The finding is currently classified as an authorization and state-integrity vulnerability, with the highest immediate concern being unauthorized remote order cancellation.
Tracking
Related Bug Tracker issue: BUG-SEC-001 — Application-Level Sender Identity Is Not Bound to Authenticated Peer Identity
Next security milestone: Establish a single authenticated identity model for all P2P messages before considering the protocol authorization layer complete.






Security Log — Peer Authentication & Authorization Hardening
Date: August 29, 2026
Classification: Security hardening / regression prevention
Status: Implemented; dedicated regression coverage pending
Security Finding
Network messages contained a claimed sender identity that could previously be treated as authoritative without explicitly verifying that the claimed identity matched the authenticated libp2p peer.
This created an impersonation/authorization risk for message types that perform state changes.
Security Controls Implemented
Message sender authentication
Incoming gossipsub messages now verify:
OpenDexMessage.sender == authenticated_peer_id
Messages failing this check are rejected before their payload is processed.
Order authorization
Order cancellation requests are now checked against the authenticated peer.
A peer cannot cancel an order belonging to another trader.
Trade authorization
Trade-state changes are restricted to authenticated participants in the trade:
• buyer
• seller
An unrelated peer cannot advance or otherwise modify the trade state.
Pending-state authorization
Trade-state changes received before their corresponding trade are retained with their authenticated sender identity.
When the trade eventually arrives, the pending state change is checked against the trade participants before being applied.
Settlement protections
Settlement now includes:
• idempotency protection against repeated settlement;
• checked quote arithmetic;
• checked balance arithmetic;
• locked-balance validation;
• validation before mutation to prevent partial state changes.
Regression Test Structure
A dedicated security test module has been established:
crates/p2p-node/tests/security.rs
Planned executable regressions include:
• forged sender rejection;
• valid authenticated sender acceptance;
• impersonated order cancellation rejection;
• unauthorized trade-state change rejection;
• unauthorized pending trade-state change rejection;
• legitimate participant state-change acceptance;
• settlement replay/idempotency protection.
Verification
The complete existing test suite was executed successfully with:
cargo check
cargo test
Result: All existing tests passed.
Outstanding Security Work
The implementation-level security controls are in place.
The dedicated tests/security.rs cases still need to be converted from placeholders into executable regression tests.
Security hardening: Implemented
Security regression suite: Open / next-session task
Known test failures: None









OpenDEX — Security Log Update
Security Hardening Verification Checkpoint
Date: 30 August 2026
Classification: Security hardening / regression verification
Status: 🟢 Implemented controls verified; broader security review remains open
The security-hardening work identified during the 29 August review has now been converted into executable regression coverage and verified against the current implementation.
The dedicated security test suite:
crates/p2p-node/tests/security.rs
is now active rather than placeholder coverage.
Verification
The following validation was completed successfully:
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
The complete workspace test suite passed.
Security regression results:
running 14 tests

14 passed
0 failed
The security suite therefore provides executable regression coverage for the principal authentication and authorization controls implemented during this phase.
Verified Security Controls
Authenticated peer / application identity binding
Incoming GossipSub messages now verify that the claimed application sender corresponds to the authenticated peer identity.
Forged sender identities are rejected before privileged message processing can occur.
Order ownership authorization
Remote order cancellation now verifies that the authenticated sender owns the referenced order.
A peer cannot cancel another trader's order merely by knowing its order ID.
Trade participant authorization
Trade-state changes are restricted to authenticated participants in the relevant trade.
An unrelated peer cannot use a fabricated application identity to advance trade state.
Pending trade-state authorization
Trade-state changes received before their corresponding trade are retained together with the authenticated sender identity.
When the trade becomes available, authorization is checked against the actual trade participants before the pending state change is applied.
State-integrity protections
Security regression coverage verifies that rejected messages do not mutate protected local state.
The suite also verifies that synchronization/state-response handling cannot incorrectly resurrect removed orders or replace existing trade state through unauthorized input.
Settlement protections
The previously implemented settlement controls remain verified by the full workspace test suite:
• settlement idempotency
• partial-fill accounting
• checked quote arithmetic
• checked balance arithmetic
• locked-balance validation
• validation before mutation
• local participant restrictions
Security Finding Status
SEC-010 — Application sender identity is unauthenticated
Status: 🟢 Resolved — implemented and regression-tested
Priority: Critical
The previously identified identity-boundary weakness has been addressed.
The receiving path now establishes an authenticated relationship between the network-level peer and the application-level sender before privileged processing occurs.
Regression coverage confirms rejection of forged sender identities.
SEC-002 — Remote order cancellation authority
Status: 🟢 Resolved — implemented and regression-tested
Priority: Critical
Cancellation now requires the authenticated sender to own the referenced order.
Regression coverage confirms that impersonated/non-owner cancellation attempts are rejected without mutating state.
SEC-003 — Trade-state transition authority
Status: 🟢 Resolved for current participant authorization model — regression-tested
Priority: Critical
Trade-state changes now require the authenticated sender to be a participant in the trade.
Regression coverage confirms both legitimate participant acceptance and unauthorized-peer rejection.
The exact authority required for each individual trade-state transition remains subject to the broader protocol review, particularly around fast-forward and settlement semantics.
SEC-004 — Remote settlement authorization
Status: 🟡 Partially resolved / further review required
Priority: Critical
The immediate impersonation path has been hardened because trade-state authorization now relies on authenticated identity.
Settlement remains protected by participant checks, idempotency, arithmetic validation, and mutation ordering.
However, the protocol-level question of which authenticated participant is authorized to cause each settlement transition remains an architectural/security review item.
This should therefore not yet be considered fully closed.
Remaining Security Findings
The following findings remain open or under investigation:
• SEC-001 — Remote message trust boundary
• SEC-005 — State synchronization authority
• SEC-006 — Distributed replay and duplicate-event resistance
• SEC-007 — Per-order balance reservation model
• SEC-008 — Network message resource consumption
• SEC-009 — Remote TradeCreated semantic validation
• SEC-011 — Trade-state fast-forward authorization
These findings require further architectural review and/or adversarial testing.
Updated Security Review Position
The security posture has materially improved.
The previously identified application-identity impersonation path is now backed by executable regression tests, and the full workspace passes with Clippy warnings treated as errors.
The current position is therefore:
Authenticated identity hardening
↓
Implemented
↓
Regression tested
↓
Full workspace validation passed
↓
Remaining semantic/architectural security review
↓
Adversarial multi-node testing
↓
Final audit
Security hardening should therefore be described as verified for the implemented controls, not as complete protocol security.
Current Security Principle
The following principle remains unchanged and is now enforced more strongly by the implementation:
Do not treat a valid message as an authorised message.
The receiving node must distinguish:
Protocol validity
≠
Domain validity
≠
Authenticated identity
≠
Authorization
≠
Settlement authority
The next security phase should focus on semantic trade validation, synchronization authority, reservation/accounting invariants, replay resistance, resource limits, and adversarial multi-node testing.








OpenDEX — Security Log Update
Security Review Checkpoint
Date: 31 August 2026
Classification: Security architecture / semantic validation review
Status: 🟡 Core TradeCreated validation implemented; broader security review remains open
The current implementation of the remote TradeCreated handling path was reviewed against:
• crates/protocol/src/message.rs
• crates/p2p-node/src/orderbook.rs
• crates/p2p-node/src/trades.rs
• crates/p2p-node/src/order_factory.rs
• current TradeCreated call sites and order-book tests
The review confirms that the previously identified remote trade semantic-validation weakness has been materially mitigated.
OrderBook::validate_trade() now performs substantial semantic validation before order-book mutation.
However, several protocol-level trade invariants remain undefined or unenforced. SEC-009 should therefore remain open pending completion of the authoritative trade invariant definition and corresponding regression coverage.

SEC-009 — Remote TradeCreated Semantic Validation
Status: 🟠 Partially resolved — core validation implemented; protocol completeness review remains
Priority: Critical
Implemented Validation
Remote trades reaching OrderBook::apply_trade() are now validated before order quantities are modified.
validate_trade() currently verifies:
• trade amount is greater than zero;
• referenced buy order exists;
• referenced sell order exists;
• the buy order has Side::Buy;
• the sell order has Side::Sell;
• trade.buyer matches the buy-order trader;
• trade.seller matches the sell-order trader;
• trade market matches the buy order market;
• trade market matches the sell order market;
• trade price does not exceed the buy-order limit price;
• trade price is not below the sell-order limit price;
• trade amount does not exceed the remaining buy-order amount;
• trade amount does not exceed the remaining sell-order amount.
Validation occurs before:
• adding the trade to applied_trades;
• changing order quantities;
• removing fully consumed orders;
• removing the trade from pending state.
The effective processing boundary is therefore:
Remote TradeCreated
↓
payload deserialization
↓
referenced orders available?
↓
validate_trade()
↓
┌──────┴──────┐
│             │
reject        accept
│             │
no mutation mark applied
↓
mutate order book
This establishes the important security invariant:
A semantically invalid TradeCreated message must not mutate the order book.
Existing Defensive Properties
The trade application path also provides:
• duplicate trade protection through applied_trades;
• pending handling when referenced orders have not yet arrived;
• quantity bounds preventing a trade from consuming more than either order has remaining;
• permanent tracking of consumed order IDs through removed_orders;
• validation before mutation.
These controls significantly reduce the risk of malformed remote trades directly corrupting order quantities.
Remaining Trade Invariants
The following fields are present in Trade but are not yet completely validated at the remote application boundary.
Maker / taker consistency
Trade contains:
maker
taker
buyer
seller
The current validation verifies buyer and seller against their referenced orders but does not independently establish that maker and taker correspond to the authoritative matching context.
Local matching establishes maker/taker based on the resting and incoming orders, but apply_trade() receives only the serialized Trade.
The protocol must therefore define how maker/taker authority can be independently reconstructed or verified by receiving nodes.
Deterministic trade identity
Local matching derives the trade ID from:
buy_order
sell_order
trade_price
amount
using a deterministic SHA-256-derived UUID.
validate_trade() does not currently verify that a remotely supplied trade.id corresponds to those deterministic inputs.
The existing remaining-order checks prevent the same order quantity from being consumed beyond its available amount, but the protocol should explicitly determine whether deterministic trade-ID validation is a required security invariant.
Initial trade state
Locally generated trades begin in:
TradeState::Created
The remote validation path does not currently require a newly received TradeCreated to have state == Created.
The protocol should explicitly define whether TradeCreated is permitted to introduce a trade in any state other than Created.
This is particularly important because trade state is independently security-sensitive and may eventually influence settlement or other privileged operations.
Timestamp consistency
Trade contains:
created_at
updated_at
but the semantic validation does not currently establish a relationship between these timestamps.
At minimum, the protocol should determine whether:
updated_at >= created_at
is an invariant.
It must also be determined whether timestamps are informational only or have security significance for expiration, replay resistance, dispute handling, ordering, or settlement.
Current Assessment
The previously identified weakness in which a structurally valid but internally contradictory remote trade could directly mutate the order book has been substantially mitigated.
The finding is not yet considered fully resolved because the complete authoritative trade invariant set has not yet been defined and enforced.
Current classification:
Core semantic validation
↓
Implemented
↓
Regression coverage
↓
Required for remaining invariants
↓
Protocol completeness review
↓
Adversarial multi-node testing

SEC-012 — Remote OrderCreated / Existing Order Mutation
Status: 🔴 Open
Priority: Critical
Category: Authorization / State Integrity
The current OrderBook::upsert_order() implementation replaces an existing order when the incoming order has the same UUID:
existing order ID
↓
incoming OrderCreated
↓
same UUID?
↓
existing order replaced
This creates a separate security review requirement for remote order creation and order identity integrity.
The review must establish:
• who is authorized to create an order;
• whether the authenticated peer must correspond to order.trader;
• whether an existing order may ever be modified through OrderCreated;
• whether an order ID is immutable;
• whether duplicate OrderCreated messages must contain identical order data;
• whether price, side, market, trader, or amount may change after publication;
• whether an existing active order may be replaced by a remote peer;
• whether replayed or conflicting order creation messages are rejected.
The existing removed_orders protection prevents orders permanently marked as removed from being resurrected through upsert_order().
However, this does not by itself prevent modification of an existing active order.
This finding should therefore be investigated before considering remote order synchronization fully trusted.

Trade Application Security Boundary
The current implementation now establishes a stronger separation between protocol decoding and domain mutation:
Network message
↓
Protocol decoding
↓
Message-specific handling
↓
Domain semantic validation
↓
State mutation
The security model must continue to maintain the distinction:
Protocol validity
≠
Domain validity
≠
Authenticated identity
≠
Authorization
≠
State-transition authority
≠
Settlement authority
A successfully decoded TradeCreated message must not be considered trustworthy merely because its fields have valid serialization types.

Current Security Review Queue
The updated priority is:
1. 🔴 Remote OrderCreated authorization and order identity integrity
2. 🔴 Complete TradeCreated semantic invariant definition
3. 🔴 Trade-state transition authority
4. 🔴 Settlement authorization
5. 🔴 State synchronization authority
6. 🔴 Per-order balance reservation model
7. 🟠 Deterministic trade identity and replay resistance
8. 🟠 Trade timestamp semantics
9. 🟠 Network message resource limits
10. 🟠 Persistence integrity
11. 🟠 Adversarial multi-node testing

Security Status
The security posture continues to improve, but protocol security hardening remains incomplete.
The current implementation provides meaningful protections for:
• authenticated application/network identity binding;
• order cancellation authorization;
• trade participant authorization;
• pending trade-state authorization;
• remote trade semantic validation;
• trade idempotency;
• partial-fill accounting;
• checked arithmetic;
• locked-balance validation;
• validation-before-mutation;
• out-of-order trade handling;
• removed-order protection;
• persistence and restart behaviour;
• multi-node networking behaviour.
The newly reviewed TradeCreated path now has substantial semantic validation, but additional protocol invariants remain to be established.
A new security concern has also been identified around the ability of upsert_order() to replace an existing active order when an incoming order uses the same UUID.
No claim of complete protocol security should be made at this stage.

Current Security Principle
The following principle remains central to the OpenDEX security model:
Do not treat a valid message as an authorised message.
The receiving node must distinguish:
Protocol validity
≠
Domain validity
≠
Authenticated identity
≠
Authorization
≠
State-transition authority
≠
Settlement authority
For every remotely supplied state-changing object, the implementation should be able to answer:
Who is allowed to create or modify this object, which fields are authoritative, and how does the receiving node independently verify those claims before mutating local state?
The next security phase should therefore focus first on remote order creation/update authority, completion of the authoritative Trade invariant set, and then trade-state and settlement authority.






Security Log Update
Date: 2026-08-31
Branch: peer-dial
Commit: 2c846ac — Harden pending trade state handling
Checkpoint: checkpoint-pending-trade-state
Security Issue: Unauthorized pending trade-state injection
Severity: High
Status: Fixed and verified
Description
A trade-state message may arrive before the corresponding trade has been received. In this situation, the state change must be stored temporarily.
Because the trade does not yet exist locally, authorization against the trade participants cannot be performed at the time the pending state is received.
The pending state therefore needs to retain the authenticated sender so authorization can be performed when the associated trade arrives.
Risk
Without sender authentication being retained and checked later, an unauthorized peer could potentially submit a trade-state change before TradeCreated and have that state applied when the trade subsequently becomes available.
This could allow an attacker to attempt to manipulate the lifecycle state of a trade they do not participate in.
Mitigation
PendingTradeState now retains:
• The pending TradeStateChanged message.
• The authenticated sender associated with that message.
When the corresponding trade arrives:
1. The pending state is retrieved.
2. The stored sender is checked against the trade participants.
3. Unauthorized pending state changes are rejected.
4. Only an authorized state change is passed to the trade-state transition logic.
5. Settlement is only triggered after an authorized transition to Settled.
Additional State-Integrity Protection
Pending happy-path states are explicitly ranked.
Only the linear happy-path states participate in rank comparison. Exceptional states including:
• Disputed
• Cancelled
• Expired
are not treated as numerically ordered states.
This prevents an invalid state progression from being inferred solely from a state rank.
Security Tests
Verified:
• unauthorized_pending_state_is_rejected_when_trade_arrives
• rejects_trade_state_change_from_unauthorized_peer
• handler_rejects_unauthorized_trade_state_change
• accepts_trade_state_change_from_buyer
• accepts_trade_state_change_from_seller
Verification
Full workspace test suite passed with no failures.
Result:
Pending trade-state processing now retains authentication context and verifies authorization before applying delayed state changes.


OpenDEX — Security Log
SL-2026-08-31-001 — Network Sender Authentication Boundary
Date: 31 August 2026
Severity: Security-relevant
Status: Implemented / Requires continued audit
Component: crates/p2p-node/src/message_handler.rs
Finding
Incoming Gossipsub messages now establish an explicit authentication boundary between the network transport identity and the sender identity claimed inside the OpenDEX message.
The handler compares:
OpenDEX message.sender
↕
authenticated libp2p PeerId
Messages with a mismatching claimed sender are rejected before message-specific processing occurs.
Security Impact
This prevents a peer from simply claiming another PeerId in the OpenDEX message envelope.
The check provides an important foundation for later ownership and authorisation checks.
Additional Authorisation Controls Observed
The current handler also performs:
• Order ownership validation for OrderCreated.
• Order ownership validation for OrderCancelled.
• Trade-participant validation for TradeStateChanged.
• Pending trade-state sender validation when the associated trade arrives.
• Protocol-level validation through Trade::transition().
Remaining Security Finding
State synchronisation requires further security review.
StateResponse currently passes a received StateSnapshot into:
handle_state_response()
↓
apply_snapshot()
↓
OrderBook / TradeBook mutation
apply_snapshot() currently accepts remote orders, trades, removed-order identifiers, and applied-trade identifiers and mutates local state accordingly.
This means the state-sync boundary currently has significant authority over local domain state.
The architecture plan specifically identifies this boundary as requiring:
UNTRUSTED NETWORK INPUT
↓
Decode
↓
Validate
↓
Authorise
↓
Domain State
The current implementation does not yet provide a sufficiently explicit validation/authorisation layer for snapshots.
Risk
A malicious authenticated peer may potentially provide state data that is syntactically valid but semantically inappropriate for the local node.
This is currently classified as a security finding requiring investigation, not yet as a confirmed exploitable vulnerability.
Required Follow-up
Audit StateSnapshot handling before further architectural expansion.
Questions to answer:
• Should every snapshot field be trusted?
• Can a remote peer remove arbitrary local orders?
• Can a remote peer introduce arbitrary trades?
• Should snapshots be authoritative or merely advisory?
• Should individual orders/trades be validated before insertion?
• Should snapshot state be reconciled rather than directly applied?
• Should state synchronisation have a separate authorisation policy?
• Can an authenticated peer be trusted to provide canonical exchange state?
No fix should be made until the intended state-sync trust model is established.












































AUD-001 — Settlement trust boundary
Severity: HIGH / potentially CRITICAL
Status: Open
Summary: Settlement consumes Trade fields without independently enforcing
the order-validation boundary.
Action: Verify and harden the network-to-settlement path.




AUD-001 — Harden settlement validation boundary
AUD-002 — Make settlement persistence transactional
AUD-003 — Make snapshot persistence crash-consistent
AUD-005 — Validate financial floating-point inputs
AUD-006 — Harden trade-state ranking API

Implemented AUD-002 settlement transaction handling.
Added temporary BalanceBook mutation and commit-after-save.
Tests added/updated and passing.

AUDIT_FINDINGS
│
├── SECURITY_LOG  → security significance
│
├── BUG_LOG       → work to be done
│
└── DEV_LOG       → what was actually changed

Security Log — Finding 1
Untrusted StateResponse Can Become Authoritative Local State
An authenticated network peer can submit a structurally valid StateResponse, and the receiving node will apply it through apply_snapshot(). Authentication establishes peer identity, but the current source path does not establish authorization to provide authoritative state.

The security significance is that a network identity is currently sufficient to reach a state-mutating synchronization boundary that permits authoritative remote removals. The impact depends on the intended trust model for peer synchronization and therefore requires the protocol invariant to be established before selecting a remediation.

Status: Open — finding phase. No code changed.


