OpenDEX Source-Level Security Audit Findings
Audit type: Source-level security and accounting review
Project: OpenDEX
Status: In progress
Date: 2026-08-31

1. Audit Scope
   The following source files are included in this review:

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
Order matching and trade creation
Trade validation and application
Trade state transitions
Balance accounting and settlement
State synchronization
Idempotency and replay protection
Integer overflow/underflow
Persistence and consistency
Trust boundaries between protocol messages and local accounting
Physical infrastructure, production deployment, host security, key custody, and live penetration testing are outside this source-level review and are intended for independent external audit.

2. Executive Summary
   The reviewed code contains several positive defensive mechanisms, including:

Explicit trade-state transition validation
Idempotent trade application
Idempotent settlement tracking
Checked arithmetic in settlement
Trade validation against referenced orders
Temporary state copies during snapshot processing
Protection against applying removed orders
Deterministic trade identification
Validation of trade price and quantity against orders
However, several areas require attention before the implementation should be considered production-ready.

The highest-priority issue is the trust boundary around settlement. settle_local_trade() performs balance transfers based on fields contained in a Trade, but the function itself does not validate the trade against the originating orders. The security impact depends on whether every possible path to settlement is guaranteed to perform order-level validation first.

3. Findings
   AUD-001 — Settlement Trusts Trade Data Without Local Validation
   Severity: HIGH / potentially CRITICAL
   Status: Open
   Release impact: Block production release until the call path is verified

Location
crates/p2p-node/src/settlement.rs

Description
settle_local_trade() directly uses the following fields from the supplied Trade:

buyer
seller
market
price
amount
The settlement function validates the local locked balance and checks arithmetic safety, but it does not itself validate the trade against the referenced buy and sell orders.

The project does contain OrderBook::validate_trade() in:

crates/p2p-node/src/orderbook.rs

That validation checks:

referenced buy order exists
referenced sell order exists
buy order is actually a Buy order
sell order is actually a Sell order
buyer matches the buy-order trader
seller matches the sell-order trader
market matches both orders
trade price satisfies both limit orders
trade amount does not exceed either order
Security concern
Settlement is the final accounting boundary. If an attacker can cause an arbitrary or modified Trade to reach settle_local_trade() without first passing validate_trade(), the attacker may be able to influence the asset, counterparty, price, or quantity used for a balance transfer.

Required verification
Trace every path that can invoke settlement and establish that:

untrusted P2P message
↓
trade parsing
↓
trade/order validation
↓
state validation
↓
settlement
↓
balance mutation

No untrusted network input should be able to bypass the validation boundary.

Recommended remediation
Prefer making the settlement boundary itself require validated trade context.

At minimum, ensure:

OrderBook::validate_trade(trade)

has necessarily succeeded before settlement.

Ideally, settlement should receive a locally validated/authorized trade rather than relying on callers to remember to perform validation.

Auditor note
This is currently a validation-boundary finding, not a confirmed exploit. Confirmation requires tracing the message/event dispatch path.

AUD-002 — Settlement Is Not Fully Transactional With Persistence
Severity: HIGH
Status: Open
Release impact: Fix before production

Location
crates/p2p-node/src/settlement.rs

Description
Settlement mutates the live BalanceBook before calling:

balance_book.save()

The sequence is effectively:

consume locked balance
↓
credit balance
↓
record settled trade
↓
save

If persistence fails after the in-memory mutations have occurred, the function returns an error while the supplied balance book has already been modified.

Risk
This creates a potential divergence between:

live in-memory accounting
persisted accounting
settlement idempotency records
A crash or restart at the wrong point can make recovery more difficult and may result in inconsistent accounting state.

Recommended remediation
Use a transactional pattern similar to the snapshot implementation:

clone BalanceBook
↓
validate
↓
mutate clone
↓
persist clone
↓
commit clone to live state

For production, an atomic database transaction or equivalent durable transaction mechanism would be preferable.

AUD-003 — Snapshot Persistence Is Not Atomic Across Order and Trade Books
Severity: MEDIUM / HIGH
Status: Open
Release impact: Fix before production

Location
crates/p2p-node/src/sync.rs

Description
apply_snapshot() correctly works on temporary copies:

next_order_book
next_trade_book

This is a strong design choice.

However, persistence occurs as two separate operations:

next_order_book.save_orders()
next_trade_book.save_trades()

If the first succeeds and the second fails, the filesystem can contain a newer order state alongside an older trade state.

Risk
After restart, the node may load internally inconsistent persisted state.

Recommended remediation
Use a coordinated persistence mechanism:

database transaction
atomic combined state file
staged temporary files followed by coordinated commit
equivalent crash-safe persistence strategy
The in-memory transactional behavior should be retained.

AUD-004 — Deterministic 128-Bit Trade Identifier
Severity: LOW
Status: Informational / Design Review

Location
crates/p2p-node/src/orderbook.rs

Description
Trade IDs are generated by hashing:

buy_order
sell_order
price
amount

with SHA-256 and taking the first 16 bytes.

This produces a deterministic 128-bit identifier.

Assessment
There is no immediate vulnerability identified.

The deterministic property can be useful for idempotency because identical trade parameters produce the same identifier.

However, this should be an intentional protocol property rather than an accidental implementation detail.

Recommendation
Document the intended uniqueness and replay semantics of trade IDs.

AUD-005 — Floating-Point Order Conversion Requires Stronger Input Validation
Severity: MEDIUM
Status: Open
Release impact: Fix

Location
crates/p2p-node/src/order_factory.rs

Description
Orders are constructed using:

price * PRICE_SCALE
amount * AMOUNT_SCALE

followed by conversion to u64.

The public factory accepts f64 values but does not explicitly reject:

negative values
NaN
positive infinity
negative infinity
values outside the representable integer range
problematic rounding cases
Risk
Financial quantities should not depend on implicit floating-point conversion behavior.

Recommended remediation
Validate all input before conversion:

finite
> 0
within representable range

Preferably, financial/accounting APIs should use integer scaled units internally and perform decimal parsing explicitly at the application boundary.

AUD-006 — Trade State Ranking Can Be Misinterpreted
Severity: LOW
Status: Open / Hardening

Location
crates/p2p-node/src/trade_state.rs

Description
Exceptional states:

Cancelled
Expired
Disputed

are assigned rank 0, alongside the beginning of the normal lifecycle.

The implementation of advance_trade_to_state() correctly avoids blindly using this ranking for exceptional transitions.

Assessment
The current implementation appears intentionally defensive.

The risk is that another component may incorrectly interpret trade_state_rank() as a complete ordering of all possible states.

Recommended remediation
Either make the ranking function private or explicitly document it as:

Happy-path progression rank only; not a general ordering of trade states.

4. Positive Security Controls Observed
   The review identified several good defensive measures.

Trade validation
OrderBook::validate_trade() performs multiple consistency checks before applying a trade.

This is a strong control against malformed or inconsistent trades.

Trade idempotency
apply_trade() maintains applied_trades and refuses to apply the same trade twice.

Settlement idempotency
settle_local_trade() maintains settled_trades and avoids settling the same trade twice.

Checked arithmetic
Settlement uses checked multiplication/addition before balance mutation.

This is important for preventing integer overflow from corrupting accounting.

Transactional snapshot processing
apply_snapshot() operates on cloned books and only replaces the live books after processing and persistence succeed.

This is a good pattern and should be reused for settlement.

Protocol state validation
Trade::transition() delegates transition authorization to TradeState::can_transition_to().

The state machine therefore does not simply accept arbitrary state jumps.

Removed-order protection
Orders recorded in removed_orders cannot simply be reintroduced through snapshot processing.

5. Preliminary Severity Summary
   ID	Severity	Status
   AUD-001	HIGH / potentially CRITICAL	Open
   AUD-002	HIGH	Open
   AUD-003	MEDIUM/HIGH	Open
   AUD-004	LOW	Informational
   AUD-005	MEDIUM	Open
   AUD-006	LOW	Open

6. Audit Status
   Source-level audit: In progress

Critical findings: None confirmed at this stage

High findings: 2 open

Medium findings: 2 open

Low findings: 2 open

External audit: Planned

7. External Auditor Handoff
   The external auditor should independently verify:

The complete network-to-settlement call path.
That untrusted peers cannot directly cause balance settlement.
Trade authenticity and authorization.
Persistence/crash recovery behavior.
Replay and duplicate-message handling.
Cross-node state consistency.
Host and network security.
Production configuration and deployment security.
Key management and custody.
Live penetration and adversarial testing.
The external auditor should treat AUD-001 as the first priority because settlement is the ultimate financial accounting boundary.
Source-Level Finding 1 — Untrusted StateResponse Accepted as Authoritative
Location: crates/p2p-node/src/message_handler.rs

Finding:
A StateResponse from any authenticated gossipsub peer can pass structural validation and be applied directly to local state as an authoritative snapshot. The message path verifies that the claimed sender matches the authenticated propagation_source, but it does not establish that the peer is authorized to provide authoritative synchronization state.

Evidence:

handle_gossipsub_message() authenticates the message sender at lines 522–530 by comparing msg.sender with the authenticated peer ID.
MessageType::StateResponse parses and validates the supplied snapshot at lines 609–623.
validate_state_snapshot() explicitly permits remote removals and states that the snapshot is authoritative about removals at lines 234–239.
handle_state_response() immediately passes the validated snapshot to apply_snapshot() at lines 265–271.
There is no additional authorization, trust relationship, request/response binding, snapshot version/ordering check, or equivalent provenance check in this path.
Invariant to establish:
Authentication of a peer must not, by itself, imply authorization to overwrite or remove locally maintained order/trade state. If StateResponse is intended to be authoritative, the protocol needs an explicit invariant defining which peers/messages are authorized to make authoritative state changes. If arbitrary peers are intentionally allowed to synchronize state, the snapshot must instead be treated as non-authoritative/mergeable.

Current status: Finding recorded. No implementation change made.
