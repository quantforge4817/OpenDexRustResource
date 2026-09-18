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

SEC-002 — Remote Order Cancellation Authority
Status: 🟠 Remediation implemented — verification pending
Priority: Critical
Category: Security / Authorization / State Integrity

Original Finding
OrderCancelled messages were previously processed without establishing that the authenticated sender was authorized to cancel the referenced order.

The original receive path was:

OrderCancelled
↓
decode payload
↓
handle_order_cancelled()
↓
OrderBook::remove_order()

The original implementation did not establish that the sender was the owner of the referenced order.

Additionally, authorization based only on OpenDexMessage.sender would not have been sufficient because the application-level sender field was previously not cryptographically/authentically bound to the network sender.

Current Implementation
The current message_handler.rs implementation now establishes an authenticated sender identity before dispatching the message.

The GossipSub receive path obtains the authenticated PeerId from the propagation_source and passes it to:

handle_gossipsub_message()

The message's claimed application sender is then checked against the authenticated peer:

OpenDexMessage.sender
        ==
authenticated PeerId

A sender identity mismatch is rejected before message-specific processing or state mutation.

For OrderCancelled, the current receive path additionally performs two authorization checks.

First, the trader contained in the cancellation must correspond to the authenticated sender:

OrderCancelled.trader
        ==
authenticated PeerId

Second, the referenced local order is loaded and its stored owner is checked:

order.trader
        ==
authenticated PeerId

Only after both checks succeed is handle_order_cancelled() invoked.

The resulting security boundary is:

UNTRUSTED NETWORK INPUT
        ↓
Authenticated GossipSub PeerId
        ↓
OpenDexMessage.sender validation
        ↓
OrderCancelled.trader validation
        ↓
Load referenced order
        ↓
Stored order.trader validation
        ↓
Authorized cancellation
        ↓
OrderBook::remove_order()
        ↓
Persistence

Current Security Invariant
The intended invariant is now:

Only the authenticated owner of an existing order
may cause that order to be cancelled.

More precisely:

authenticated_peer_id
        ==
OpenDexMessage.sender
        ==
OrderCancelled.trader
        ==
stored_order.trader

A remote peer therefore cannot cancel another trader's order merely by supplying the victim's order ID.

Likewise, a peer cannot use a forged application-level sender or forged cancellation trader identity to obtain authorization.

State-Mutation Property
Authorization occurs before handle_order_cancelled() and therefore before:

OrderBook::remove_order()

Unauthorized cancellation attempts return without invoking the order-removal operation.

The required security property is:

Unauthorized cancellation
        ↓
Reject
        ↓
No OrderBook mutation

This must be explicitly covered by regression tests.

Remaining Verification
SEC-002 should remain open for verification until the following have been established:

Confirm the current OrderCancelled protocol structure and ensure trader is part of the authenticated/validated cancellation payload.
Trace every caller of handle_order_cancelled().
Trace every relevant caller of OrderBook::remove_order() and determine whether any alternate path can perform remote cancellation without authorization.
Verify cancellation of an unknown order has safe, intentional semantics.
Verify cancellation of an already-consumed/removed order cannot resurrect or otherwise corrupt state.
Add or verify adversarial regression coverage proving an authenticated non-owner cannot cancel another trader's order.
Verify that a forged OpenDexMessage.sender cannot bypass the cancellation authorization boundary.
Verify rejected cancellation attempts leave the order book and related protected state unchanged.
Run the complete workspace verification baseline after any required changes.
Current Assessment
The original SEC-002 weakness — remote cancellation being performed without an authorization check — is no longer present in the current handle_gossipsub_message() path.

The implementation now performs authenticated sender binding followed by explicit order-owner authorization before allowing cancellation to reach OrderBook::remove_order().

However, the finding should not yet be classified as fully resolved until the protocol definition, alternate callers, and regression coverage have been independently verified.

Status: 🟢 Remediated / Verified

This finding should therefore remain in the active security review queue, but the original description should no longer state that the receive path currently performs no authorization check.

POST-SEC-002 REVIEW RULE
------------------------
Findings in this section are deliberately deferred.

They must not be implemented as part of SEC-002 unless the SEC-002 scope is
formally expanded. Each item requires separate review, risk assessment, and
assignment to a future security requirement or remediation cycle.


POST-SEC-002 REVIEW QUEUE
=========================

These findings were identified during the SEC-002 review but are NOT
classified as SEC-002 requirements at this time.

They are recorded for later security review and must not be treated as
approved SEC-002 remediation unless separately assessed and assigned to a
future security requirement/review.


Finding SEC-002-AF-001
Area: Trade state handling
Files:
  - crates/p2p-node/src/message_handler.rs
  - crates/p2p-node/src/trade_state.rs
Status: REVIEW
Severity: Medium

Observation:
TradeStateChanged messages are authenticated to the libp2p sender, but the
current authorization check establishes participant status rather than
role-specific authorization for the requested transition.

Review question:
Should future security requirements distinguish buyer/seller/maker/taker
permissions for individual trade-state transitions?

Action:
No change under SEC-002. Review separately after SEC-002.


Finding SEC-002-AF-002
Area: State synchronization
File:
  - crates/p2p-node/src/sync.rs
Status: REVIEW
Severity: High

Observation:
StateResponse validation establishes structural consistency but does not
establish that the remote snapshot is authoritative, newer than local state,
or otherwise preferred during conflicts.

Review question:
Should future security requirements define snapshot authority, freshness,
versioning, or conflict resolution?

Action:
No change under SEC-002. Review separately after SEC-002.


Finding SEC-002-AF-003
Area: State synchronization / trade history
File:
  - crates/p2p-node/src/sync.rs
Status: REVIEW
Severity: Medium

Observation:
Remote snapshots validate trade structure and order relationships, but the
snapshot validation does not establish consistency between the supplied
Trade.state and a verifiable trade-state history.

Review question:
Should future security requirements validate trade-state history during
snapshot synchronization?

Action:
No change under SEC-002. Review separately after SEC-002.


Finding SEC-002-AF-004
Area: Order synchronization
Files:
  - crates/p2p-node/src/sync.rs
  - crates/p2p-node/src/orderbook.rs
Status: REVIEW
Severity: Medium

Observation:
Snapshot orders can update local order quantities through the snapshot
synchronization path. The current code does not visibly establish
owner authorization or monotonic/versioned legitimacy for those updates.

Review question:
Should future security requirements require authenticated order ownership
and/or versioned monotonic order updates during synchronization?

Action:
No change under SEC-002. Review separately after SEC-002.


Finding SEC-002-AF-005
Area: Pending trade-state cache
File:
  - crates/p2p-node/src/message_handler.rs
Status: REVIEW
Severity: Low/Medium

Observation:
MAX_PENDING_TRADE_STATES limits cache size, but pending entries do not
appear to have a time-based expiration mechanism.

Review question:
Should future security requirements require TTL/expiry for state changes
waiting for their corresponding TradeCreated message?

Action:
No change under SEC-002. Review separately after SEC-002.


Finding SEC-002-AF-006
Area: Trade-state timestamps
Files:
  - crates/p2p-node/src/message_handler.rs
  - crates/p2p-node/src/trade_state.rs
Status: REVIEW
Severity: Medium

Observation:
Remote timestamps are passed into trade-state transitions, but the current
code does not visibly enforce timestamp freshness, monotonicity, or an
acceptable clock-skew window.

Review question:
Should future security requirements address replay, delayed messages, or
timestamp manipulation?

Action:
No change under SEC-002. Review separately after SEC-002.


Finding SEC-002-AF-007
Area: State synchronization / persistence
File:
  - crates/p2p-node/src/sync.rs
Status: REVIEW
Severity: Medium

Observation:
Snapshot application persists the temporary books before committing them to
memory, but persistence currently uses fs::write() rather than an explicitly
crash-safe atomic replacement strategy.

Review question:
Should future security requirements cover crash consistency and protection
against partial state-file writes?

Action:
No change under SEC-002. Review separately after SEC-002.


Finding SEC-002-AF-008
Area: Trade creation / authorization
File:
  - crates/p2p-node/src/orderbook.rs
Status: REVIEW
Severity: High

Observation:
Trade application validates referenced orders, participants, market, price,
and quantity. However, message authentication establishes the identity of
the message sender rather than independently proving that the trade itself
was authorized by the relevant trading parties or matching authority.

Review question:
Should future security requirements require cryptographic authorization
or provenance for TradeCreated messages?

Action:
No change under SEC-002. Review separately after SEC-002.

🔴 SEC-003 — State-specific transition authority missing
Confirmed implementation facts:

Peer authentication exists.
Sender is bound to the authenticated PeerId.
Sender must be a buyer or seller.
State transitions are checked against the protocol transition graph.
Duplicate state transitions are idempotent.
Backward/stale state transitions are rejected by the state graph.
Trade::transition() has no actor parameter.
Trade::transition() performs no state-specific authorization.
TradeStateChanged contains no actor/role information.
Arbitrary remote timestamps are accepted as updated_at.
PayoutPending → Settled is structurally legal.
Disputed → Settled is structurally legal.
Not yet established:

Which participant owns each transition.
Whether buyer and seller have asymmetric authority.
Whether Settled may be directly participant-authorized.
What authorizes Disputed → Settled.
Whether timestamps are trusted anywhere else.
Whether expiration/dispute/settlement should be derived from independently verifiable conditions.
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
Status: 🟢 Mitigated

Remote GossipSub messages are untrusted network input and must not be allowed to assert an identity different from the authenticated libp2p peer or perform operations outside that peer's authorization scope.

Finding
The message protocol contains a caller-supplied sender field. The required security boundary is:

untrusted GossipSub message
        ↓
protocol decoding
        ↓
authenticated PeerId verification
        ↓
operation-specific authorization
        ↓
domain state mutation

The implementation now enforces this boundary before dispatching remote messages to domain handlers.

Controls implemented
msg.sender is authenticated against the PeerId associated with the received GossipSub message.
Messages whose claimed sender does not match the authenticated peer are rejected before domain handling.
OrderCreated requires order.trader to match the authenticated sender.
OrderCancelled requires the cancellation trader to match the authenticated sender and the referenced order owner.
TradeStateChanged requires the authenticated sender to be a participant in the referenced trade.
Rejected forged/unauthorized messages are verified not to mutate the affected domain state.
Legitimate authenticated order ownership and legitimate buyer/seller trade participation are covered by regression tests.
Security test coverage
Test	Control verified	Result
rejects_message_with_forged_sender	Claimed sender cannot differ from authenticated PeerId	✅
accepts_message_from_authenticated_sender	Authenticated sender is accepted	✅
rejects_order_from_impersonated_trader	Order ownership cannot be impersonated	✅
accepts_order_from_authenticated_trader	Legitimate order owner is accepted	✅
rejects_order_cancellation_by_non_owner	Non-owner cannot cancel another trader's order	✅
rejects_trade_state_change_from_unauthorized_peer	Non-participant cannot authorize trade state changes	✅
accepts_trade_state_change_from_buyer	Buyer is an authorized participant	✅
accepts_trade_state_change_from_seller	Seller is an authorized participant	✅
handler_rejects_forged_sender_without_mutating_state	Forged sender is rejected before mutation	✅
handler_rejects_order_from_impersonated_trader_without_mutating_state	Impersonated order cannot mutate order book	✅
handler_rejects_unauthorized_trade_state_change	Unauthorized state change cannot mutate trade	✅
handler_rejects_cancellation_by_non_owner	Unauthorized cancellation cannot mutate order book	✅

Disposition
SEC-001 is mitigated.

The current implementation establishes the required sender-authentication and operation-specific authorization boundary for the reviewed remote message types.

Priority: Critical → Mitigated

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

-----------------------------------------------------------------------------------

SEC-013 — Remote StateSnapshot Synchronization Authority
Status: 🔴 Open — Investigation
Priority: Critical
Category: Authorization / State Integrity / Synchronization

Finding
Remote StateResponse messages can reach the local state-synchronization path:

StateResponse
↓
StateSnapshot
↓
apply_snapshot()
↓
OrderBook / TradeBook mutation
↓
persistence

The receiving node authenticates the network sender, but authenticated peer identity does not by itself establish that the peer is authorized to provide authoritative state.

The security boundary therefore remains:

Authenticated PeerId
        ≠
Synchronization authority

apply_snapshot() can influence authoritative local state, including:

active orders;

removed-order identifiers;

trades;

applied-trade identifiers;

order quantities and related state;

persistent local state.

This makes StateSnapshot handling a distinct authorization and state-integrity boundary.

Security Risk
A malicious or insufficiently trusted authenticated peer may be able to submit a structurally valid snapshot that causes local state to change without establishing authority over the affected state.

Potential consequences include:

unauthorized order introduction or removal;

modification or replacement of local trade state;

stale-state application;

conflicting-state application;

order or trade resurrection;

manipulation of removed-order tracking;

replay of previously valid snapshots;

persistence of attacker-influenced state.

These consequences are not all established as exploitable vulnerabilities at this stage. The confirmed finding is that the synchronization authority model is not yet explicitly defined or enforced.

Required Security Contract
Before apply_snapshot() performs protected state mutation, the receiving node should establish:

UNTRUSTED NETWORK INPUT
        ↓
Authenticated sender
        ↓
Synchronization authorization
        ↓
Snapshot semantic validation
        ↓
State reconciliation
        ↓
Validated domain mutation
        ↓
Crash-consistent persistence

A successfully authenticated StateResponse must not automatically be treated as an authoritative description of local exchange state.

Required Protocol Decisions
The security review must establish:

Whether StateSnapshot is authoritative, advisory, or recovery-oriented.

Which peers, if any, are authorized to provide authoritative state.

Whether synchronization authority is global, scoped, or object-specific.

Which snapshot fields are authoritative.

Whether individual orders require independent ownership and semantic validation.

Whether individual trades require the same validation as remote TradeCreated.

How stale snapshots are detected and rejected.

How conflicting snapshots are resolved.

Whether removed orders can ever be resurrected.

Whether trades can be resurrected or replaced.

Whether a snapshot can remove arbitrary local state.

How replayed snapshots are handled.

Whether snapshot application bypasses normal object-level authorization.

How snapshot persistence remains crash-consistent.

Required Security Invariants
The final protocol should be able to answer, for every remotely supplied snapshot object:

Who is authorized to provide this state?
What authority does that peer possess?
Which fields may the peer change?
How is freshness established?
How are conflicts resolved?
How is resurrection prevented?
How is the resulting mutation persisted safely?

These questions must be answered at the protocol/security-design level before apply_snapshot() is treated as a trusted synchronization mechanism.

Required Verification
Once the synchronization contract is defined, regression coverage should include at minimum:

unauthorized snapshot rejection;

authenticated-but-unauthorized peer rejection;

stale snapshot rejection;

conflicting snapshot handling;

arbitrary order-removal attempts;

order-resurrection attempts;

trade-resurrection attempts;

unauthorized order introduction;

unauthorized trade introduction;

replayed snapshot handling;

state-integrity preservation after rejection;

persistence integrity after accepted snapshot application;

crash/restart behaviour during snapshot application;

adversarial multi-node synchronization tests.

Current Assessment
Status: 🔴 Open — Investigation

No implementation change should be made solely to close SEC-013 until the intended synchronization trust model has been established.

The objective is to define the authoritative synchronization invariant first and then implement the smallest control that enforces that invariant.

Related Findings
SEC-001 — Remote message trust boundary

SEC-009 — Remote TradeCreated semantic validation

SEC-010 — Application sender identity

SEC-012 — Remote OrderCreated / existing order mutation

AUD-002 — Settlement persistence transaction handling

AUD-003 — Snapshot persistence crash consistency

Security Principle
The synchronization boundary must preserve the distinction:

Protocol validity
        ≠
Domain validity
        ≠
Authenticated identity
        ≠
Synchronization authorization
        ≠
Authoritative state

An authenticated peer is not automatically a trusted source of canonical local state.

Next Step
Define the authoritative StateResponse / StateSnapshot trust model.

After the invariant is agreed, convert it into adversarial regression tests and then implement the minimum enforcement required by the protocol.
SEC-013 — State Synchronization Authority Model
Finding: SEC-013 — Remote StateSnapshot Synchronization Authority
Status: 🔴 Open — Investigation
Priority: Critical
Category: Authorization / State Integrity / Synchronization

1. Purpose
This document defines the security model that must govern remote StateResponse / StateSnapshot processing before apply_snapshot() is treated as an authoritative state-mutation boundary.

The objective is to establish what a remote peer is permitted to assert, what the receiving node must independently verify, and which state changes may be performed as a result.

No implementation change is implied by this document.

2. Core Security Principle
An authenticated network identity does not automatically constitute authority over local state.

The receiving node must distinguish:

Authenticated Peer
        ≠
Synchronization Authority
        ≠
Authoritative State

Therefore:

valid StateResponse
        ≠
trusted StateSnapshot
        ≠
authorized state mutation

3. Proposed Synchronization Model
OpenDEX should initially treat a remote StateSnapshot as a synchronization proposal, rather than automatically treating it as canonical authoritative state.

The receiving node remains responsible for determining whether individual portions of the snapshot are acceptable under local protocol rules.

The intended boundary is:

Remote StateResponse
        ↓
Authenticated sender
        ↓
Synchronization policy
        ↓
Snapshot validation
        ↓
Object-level validation
        ↓
Conflict / freshness evaluation
        ↓
State reconciliation
        ↓
Crash-consistent persistence
        ↓
Commit

A peer therefore does not receive unrestricted authority to replace local state merely by successfully publishing a signed GossipSub message.

4. Synchronization Authority
4.1 Network authentication
The receiving node must establish the authenticated libp2p identity of the peer that transmitted the snapshot.

The authenticated PeerId is the source identity for security decisions.

Application-level fields supplied inside the snapshot must not independently establish authority.

4.2 Authorization scope
The protocol must not assume that an authenticated peer has unrestricted authority over all exchange state.

Synchronization authority must have an explicitly defined scope.

At minimum, the protocol must distinguish:

identity
authority
scope

For example:

Peer A
  ↓
authenticated identity

Peer A
  ↓
permitted synchronization scope

Peer A
  ↓
specific state assertions permitted within that scope

The exact scope remains an architectural decision.

5. Snapshot Authority
A remote snapshot must not be permitted to arbitrarily assert:

ownership of another participant's order;

creation of arbitrary orders;

removal of arbitrary orders;

replacement of active orders;

creation of arbitrary trades;

replacement of existing trades;

resurrection of removed orders;

resurrection of consumed trades;

arbitrary applied-trade history;

arbitrary removed-order history.

Each of these operations must be justified by an explicit protocol rule.

6. Object-Level Validation
A snapshot is a container of state and must not bypass the security rules applied to individual network objects.

Orders
Remote orders must be evaluated against the same fundamental invariants required for remote OrderCreated processing.

The receiver must establish, where applicable:

order identity
order owner
market
side
price
quantity
lifecycle state
immutability rules

An order appearing inside a snapshot must not become trusted solely because it was embedded inside StateSnapshot.

Trades
Remote trades must satisfy the authoritative trade invariants already established for TradeCreated.

Snapshot synchronization must not provide a bypass around:

Trade validation
        ↓
participant consistency
        ↓
market consistency
        ↓
price constraints
        ↓
quantity constraints
        ↓
trade identity rules
        ↓
state rules

7. Removed-State Authority
A snapshot containing removed-order identifiers must not automatically authorize local deletion.

The receiver must establish that the remote state assertion is permitted under the synchronization model.

The invariant is:

Remote removed-order assertion
        ↓
authorization + validation
        ↓
permitted reconciliation

not:

Remote removed-order assertion
        ↓
delete local order

8. Resurrection Protection
State synchronization must preserve the existing protection against resurrection.

An object that has been permanently removed or consumed must not become active merely because an older or conflicting snapshot contains that object.

The required invariant is:

removed / consumed
        ↓
cannot be resurrected by ordinary synchronization

Any legitimate recovery or reconstruction mechanism must be explicitly defined as a separate protocol rule.

9. Freshness and Replay
A valid historical snapshot remains valid data, but validity does not imply freshness.

The protocol must therefore eventually provide a mechanism for distinguishing:

new snapshot

from:

old valid snapshot replayed later

Possible mechanisms include versioning, epochs, monotonic sequence numbers, or another deterministic protocol mechanism.

The exact mechanism is not selected by this document.

Until freshness semantics are established, replayed snapshots remain a security-review concern.

10. Conflicting Snapshots
The protocol must define deterministic behaviour when two valid snapshots disagree.

Examples include:

Peer A → Order X quantity = 5
Peer B → Order X quantity = 2

or:

local → Trade T = Settled
remote → Trade T = Created

The receiving node must not resolve such conflicts through incidental message-arrival order.

A deterministic conflict/reconciliation rule is required.

11. Atomic State Application
Snapshot application must preserve state integrity if validation or persistence fails.

The preferred security boundary is:

receive
  ↓
validate
  ↓
construct candidate state
  ↓
persist safely
  ↓
commit

rather than mutating authoritative state incrementally and discovering an error afterward.

Snapshot application must not leave the local node in a partially updated state.

12. Idempotency
Applying the same accepted snapshot repeatedly should produce the same resulting state.

Therefore:

Snapshot S
Snapshot S
Snapshot S

must not progressively corrupt, duplicate, or otherwise alter protected state beyond the result of the first valid application.

Idempotency does not replace freshness validation.

A stale snapshot may be idempotent and still be unauthorized or unsafe to apply.

13. Security Invariants
The following invariants should become executable requirements.

INV-013-01 — Authenticated identity
The sender used for security decisions must come from the authenticated network identity.

INV-013-02 — Explicit synchronization authority
Authenticated identity alone must not confer unrestricted synchronization authority.

INV-013-03 — Scoped authority
Any synchronization authority must have an explicitly defined scope.

INV-013-04 — Validation before mutation
No protected local state may be mutated before snapshot validation succeeds.

INV-013-05 — Object-level security
Snapshot-contained orders and trades must not bypass their respective semantic and authorization rules.

INV-013-06 — No unauthorized deletion
A remote snapshot must not arbitrarily remove protected local state.

INV-013-07 — No unauthorized resurrection
A remote snapshot must not resurrect protected removed or consumed state.

INV-013-08 — Freshness
The protocol must eventually distinguish current state from replayed historical state.

INV-013-09 — Deterministic conflict resolution
Conflicting valid state must have deterministic reconciliation semantics.

INV-013-10 — Atomic persistence
Failed snapshot persistence must not leave partially committed protected state.

INV-013-11 — Idempotent application
Repeated delivery of an accepted snapshot must not create additional unintended state changes.

14. Required Adversarial Tests
Once the implementation contract is finalized, the following tests should be required.

rejects_snapshot_from_unauthorized_peer

rejects_snapshot_with_unauthorized_order

rejects_snapshot_with_unauthorized_trade

rejects_snapshot_attempting_arbitrary_order_removal

rejects_snapshot_attempting_order_resurrection

rejects_snapshot_attempting_trade_resurrection

rejects_stale_snapshot

handles_conflicting_snapshots_deterministically

rejects_internally_inconsistent_snapshot

does_not_mutate_state_when_snapshot_validation_fails

does_not_partially_commit_when_snapshot_persistence_fails

reapplying_identical_snapshot_is_idempotent

rejects_or_handles_replayed_snapshot_according_to_protocol_rules

These tests should be implemented against the actual multi-node/network path where practical, rather than relying exclusively on isolated unit tests.

15. Relationship to Existing Findings
SEC-012 — Remote OrderCreated / Existing Order Mutation
SEC-012 establishes that order identity and mutation authority require explicit review.

SEC-013 must not provide a synchronization path that bypasses those protections.

SEC-009 — Remote TradeCreated Semantic Validation
Snapshot-provided trades must not bypass the semantic validation established for remote TradeCreated.

SEC-006 — Distributed Replay
Snapshot freshness and replay semantics are directly related to the broader replay-resistance review.

SEC-013 defines the synchronization-specific requirement; SEC-006 remains responsible for the broader distributed replay model.

AUD-003 — Snapshot Persistence Crash Consistency
Snapshot authorization and semantic validation determine whether state may be accepted.

Persistence integrity determines whether accepted state can be committed safely.

These remain related but separate concerns.

16. Current Decision
For the purposes of the security review:

A remote StateSnapshot is not currently considered authoritative merely because it originated from an authenticated peer.

The receiver must establish synchronization authority, validate the snapshot, reconcile it against local state, and persist the resulting state safely before committing the mutation.

The exact mechanism by which synchronization authority, freshness, and conflict resolution are established remains subject to protocol design review.

17. Implementation Rule
No code change should be made solely to close SEC-013 until the authority, scope, freshness, and conflict model has been accepted as the intended OpenDEX protocol behaviour.

Once those decisions are finalized, the implementation should enforce the resulting invariants and the security regression suite should provide executable evidence for each one.

18. Security Boundary
The final intended boundary is:

UNTRUSTED NETWORK INPUT
        ↓
Authenticated Peer
        ↓
Synchronization Authority
        ↓
Snapshot Validation
        ↓
Object-Level Authorization
        ↓
Freshness / Conflict Rules
        ↓
State Reconciliation
        ↓
Crash-Consistent Persistence
        ↓
AUTHORITATIVE LOCAL STATE

This boundary is the basis for the next SEC-013 implementation and adversarial-testing phase.


----------------------------------------------------------------------------------------------------------------

OpenDEX — SEC-013 Synchronization Model Decision Record
Finding: SEC-013 — Remote StateSnapshot Synchronization Authority
Decision: Reconciliation-based synchronization
Status: 🟡 Design decision — implementation pending
Priority: Critical
Category: Authorization / State Integrity / Synchronization

1. Decision
OpenDEX will treat a remote StateSnapshot as a synchronization proposal, not as automatically authoritative local state.

An authenticated peer may provide synchronization data, but the receiving node remains responsible for validating and reconciling that data before protected local state is changed.

The receiving node must not treat:

authenticated peer
        =
canonical state authority

Instead:

authenticated peer
        ↓
permitted synchronization source
        ↓
snapshot proposal
        ↓
validation
        ↓
reconciliation
        ↓
accepted local state

2. Why This Model
The current OpenDEX architecture is distributed and already supports:

duplicate message delivery;

out-of-order delivery;

pending state;

idempotent trade application;

idempotent settlement;

local order ownership;

semantic trade validation;

removed-order protection;

multi-node synchronization.

Automatically treating one authenticated peer's snapshot as canonical would introduce a broad authority boundary that is not currently defined by the protocol.

Reconciliation-based synchronization preserves the distinction between:

Who sent this?
        ↓
What are they proposing?
        ↓
Is the proposal valid?
        ↓
Does it conflict with protected local state?
        ↓
What state may safely be accepted?

3. Authentication
The authenticated libp2p PeerId remains the source of network identity.

Application-level identity fields must not independently establish synchronization authority.

The security model is therefore:

GossipSub message
        ↓
authenticated PeerId
        ↓
synchronization policy

not:

StateSnapshot.sender
        ↓
automatic authority

4. Scope of a Snapshot
A StateSnapshot is considered a proposal concerning state known by the sending peer.

It does not automatically grant the sender authority to:

delete arbitrary local orders;

modify another trader's order;

introduce arbitrary trades;

replace existing trade state;

resurrect removed state;

rewrite local history;

declare arbitrary state authoritative.

Each contained object remains subject to the receiving node's validation and state-integrity rules.

5. Order Reconciliation
Orders supplied by a snapshot must be reconciled against local state.

The receiver must not blindly perform:

remote order
        ↓
upsert_order()
        ↓
authoritative local state

Instead, the reconciliation process must determine whether the remote order:

is new;

already exists and is identical;

represents a permitted state update;

conflicts with local state;

attempts to modify immutable identity fields;

attempts to resurrect a removed order.

An existing active order must not be silently replaced merely because a remote snapshot contains the same UUID.

This requirement is directly related to SEC-012.

6. Trade Reconciliation
Trades supplied by a snapshot must pass the authoritative trade-validation rules.

Snapshot synchronization must not create a validation bypass around the existing remote TradeCreated security boundary.

The receiver must validate, as applicable:

referenced orders;

buyer and seller;

market;

side;

price;

quantity;

remaining quantities;

trade identity;

trade state;

maker/taker semantics;

timestamp invariants.

A snapshot containing an invalid trade must not mutate the order book or trade book.

7. Removed Orders
A remote snapshot's removed_orders collection is treated as a state assertion, not an unconditional deletion command.

The receiver must establish that the removal is consistent with locally known state and the protocol's reconciliation rules.

The following must not occur merely because a remote snapshot contains an identifier:

remote removed_orders
        ↓
delete arbitrary local order

This is a security boundary.

8. Resurrection Protection
Reconciliation must preserve permanent removal/consumption protections.

A snapshot must not resurrect an order that the local node has permanently recorded as removed unless the protocol explicitly defines a separate recovery mechanism.

Likewise, consumed or finalized trades must not be resurrected through an ordinary synchronization snapshot.

Required invariant:

protected historical state
        ↓
ordinary snapshot
        ↓
cannot be resurrected

9. Freshness
Reconciliation-based synchronization requires an explicit freshness mechanism.

The protocol must eventually distinguish:

current proposal

from:

historical valid proposal replayed later

The implementation may use a version, epoch, monotonic sequence, or another deterministic mechanism.

Until the freshness mechanism is implemented, replay handling remains an open part of SEC-013 and SEC-006.

10. Conflict Resolution
Conflicting snapshots must be handled deterministically.

The result must not depend solely on message arrival order.

For example:

Local:
Order X remaining = 5

Remote:
Order X remaining = 2

must have an explicitly defined reconciliation result.

Likewise:

Local:
Trade T = Settled

Remote:
Trade T = Created

must not allow the older state to overwrite protected local state.

The conflict policy must distinguish between:

legitimate newer state;

stale state;

invalid state;

conflicting state;

permanently finalized state.

11. Persistence
Accepted reconciliation results must be committed safely.

The required boundary is:

receive
  ↓
validate
  ↓
reconcile
  ↓
construct candidate state
  ↓
persist safely
  ↓
commit

Snapshot processing must not leave partially applied state if persistence fails.

This remains related to AUD-003.

12. Idempotency
Applying the same accepted snapshot multiple times must not progressively alter state.

For an accepted snapshot S:

apply(S)
apply(S)
apply(S)

must converge on the same state as a single successful application.

Idempotency does not substitute for freshness.

A stale snapshot can be idempotent while still being unsafe to accept.

13. Security Properties
The selected reconciliation model establishes the following properties.

Property 1 — Authentication
The receiver knows which authenticated network peer supplied the proposal.

Property 2 — No implicit authority
Authentication alone does not grant unrestricted authority over local state.

Property 3 — Validation before mutation
No protected state is mutated before the snapshot passes the applicable validation rules.

Property 4 — Object-level protection
Orders and trades cannot bypass their respective authorization and semantic-validation boundaries by being embedded in a snapshot.

Property 5 — No arbitrary deletion
A remote peer cannot use synchronization as an unrestricted deletion mechanism.

Property 6 — No ordinary resurrection
A remote snapshot cannot resurrect protected removed or consumed state.

Property 7 — Deterministic reconciliation
Conflicts are resolved according to explicit protocol rules rather than message-arrival order.

Property 8 — Freshness
The protocol can distinguish current state from replayed historical state.

Property 9 — Atomic persistence
Accepted state is committed without leaving partially applied protected state.

Property 10 — Idempotent convergence
Repeated delivery of the same accepted snapshot converges to the same result.

14. Adversarial Test Plan
The decision should be converted into executable tests before the implementation is considered hardened.

Authentication and authority
rejects_snapshot_from_unauthorized_peer
accepts_snapshot_from_permitted_sync_source

Orders
rejects_snapshot_modifying_foreign_order
rejects_snapshot_replacing_existing_order
rejects_snapshot_removing_arbitrary_order
rejects_snapshot_resurrecting_removed_order
accepts_valid_new_order_under_reconciliation_rules

Trades
rejects_snapshot_with_invalid_trade
rejects_snapshot_with_conflicting_trade
rejects_snapshot_resurrecting_consumed_trade
accepts_valid_trade_under_reconciliation_rules

Freshness and replay
rejects_stale_snapshot
rejects_replayed_snapshot_when_protocol_requires_rejection
repeated_accepted_snapshot_is_idempotent

Conflicts
conflicting_snapshots_have_deterministic_result
older_state_cannot_replace_protected_newer_state

State integrity
invalid_snapshot_does_not_mutate_state
failed_snapshot_persistence_does_not_partially_commit

Multi-node
The critical cases should eventually be exercised through the real libp2p network path rather than only direct calls to apply_snapshot().

15. Relationship to Existing Findings
SEC-009 — Remote TradeCreated Semantic Validation

Snapshot trades must not bypass established trade validation.

SEC-012 — Remote OrderCreated / Existing Order Mutation

Snapshot synchronization must not provide a backdoor around order identity and ownership rules.

SEC-013 — Remote StateSnapshot Synchronization Authority

This decision record defines the intended synchronization model for SEC-013.

SEC-006 — Distributed Replay

Freshness and replay of snapshots remain part of the broader distributed replay model.

AUD-003 — Snapshot Persistence Crash Consistency

Persistence safety remains a separate audit concern.

16. Implementation Boundary
Once the remaining protocol details are finalized, the intended implementation boundary is:

StateResponse
      ↓
authenticated PeerId
      ↓
snapshot structural validation
      ↓
freshness validation
      ↓
order validation
      ↓
trade validation
      ↓
removal validation
      ↓
reconciliation
      ↓
candidate state
      ↓
crash-safe persistence
      ↓
commit

apply_snapshot() must not function as an unrestricted:

remote state
      ↓
replace local state

operation.

17. Decision Status
Decision: Reconciliation-based synchronization.

Implementation: Not yet applied.

SEC-013: Remains open until the reconciliation rules, freshness mechanism, conflict semantics, and required regression tests are implemented and verified.

This decision record therefore defines the intended security direction without prematurely claiming that the implementation already enforces it.

18. Final Security Principle
The synchronization protocol must preserve:

Protocol validity
        ≠
Domain validity
        ≠
Authenticated identity
        ≠
Synchronization authority
        ≠
Freshness
        ≠
Authoritative local state

A remote snapshot is a proposal for reconciliation, not an automatic command to replace local state.
