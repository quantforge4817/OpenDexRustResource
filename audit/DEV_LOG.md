OpenDEX — Development Log
Purpose
This log records the development history of OpenDEX at a milestone and decision level.
The opening section establishes the project's architectural direction and development principles. Individual entries are intentionally concise and diary-like. Detailed bugs, security findings, and deferred technical issues are tracked separately so that this document remains readable as the project grows.
The development log records what changed, why it changed, and the resulting checkpoint.
Detailed issue tracking belongs in:
• BUG_TRACKER.md — bugs, technical issues, regressions, and deferred fixes
• SECURITY_LOG.md — security findings, threat-model concerns, mitigations, and audit preparation

Development Principles
Simplify first. Secure second. Audit third.
OpenDEX should favour:
• clear responsibility boundaries;
• small, understandable operations;
• explicit state transitions;
• deterministic distributed behaviour;
• minimal unnecessary coupling;
• testable domain logic;
• conservative handling of remote input;
• a small effective attack surface.
A large module is not automatically a problem. Code should only be extracted when doing so creates a meaningful responsibility boundary.
Refactoring must preserve behaviour unless a behavioural change is explicitly the objective.

Milestones
P2P Migration
Status: 🟢 Functionally Complete
The original P2P migration established the libp2p networking foundation, GossipSub messaging, peer discovery, protocol message handling, and distributed event processing.
The networking layer now has genuine multi-node integration coverage rather than relying solely on isolated unit tests.

Architectural Simplification & Audit Preparation
Status: 🟡 Active
The current phase is focused on simplifying and hardening the implementation before the final audit phase.
The intended architectural direction remains:
main.rs
│
│ coordination
▼
message_handler.rs
│
│ protocol interpretation
▼
small, explicit operations
│
├── sync.rs
├── orderbook.rs
├── trades.rs
├── trade_state.rs
├── balances.rs
└── settlement.rs
The exact structure may evolve.
The important requirement is that responsibilities remain clear and security-sensitive behaviour is concentrated in identifiable components.

Development Entries
27 August 2026 — Networking & Command Handling
Status: 🟢 GREEN
Refactored the p2p-node networking and command-handling structure.
Moved CLI command processing out of main.rs and established clearer separation between networking, protocol handling, and command handling.
Added genuine two-node GossipSub integration testing.
The network test now waits for actual subscription events and polls both swarms while waiting for message delivery rather than relying on arbitrary delays.
Checkpoint: networking and command-handling refactor integrated successfully.

27 August 2026 — GossipSub Event Boundary
Status: 🟢 GREEN
Simplified the swarm-event path by separating GossipSub handling from the broader swarm event dispatcher.
The resulting flow is:
SwarmEvent
↓
Behaviour event
↓
GossipSub
↓
GossipSub message handler
↓
Protocol handling
The obsolete inline GossipSub handling was removed.
Validation remained green.

27 August 2026 — Dead Code Cleanup
Status: 🟢 GREEN
Removed the unused debit_available() balance operation.
The remaining transfer_locked() warning was deliberately left alone because the operation represents a legitimate settlement primitive and is covered by the test suite.
The warning is deferred for later architectural review rather than removed solely to silence the compiler.

28 August 2026 — Protocol Dispatch Simplification
Status: 🟢 GREEN
Flattened the protocol-decoding path in handle_gossipsub_message().
The previous nested structure was simplified to:
Receive GossipSub data
↓
Decode envelope
↓
Reject invalid envelope
↓
Dispatch MessageType
↓
Decode payload
↓
Reject invalid payload
↓
Call domain handler
No new catch-all module was introduced.
No domain behaviour was intentionally changed.
The existing order, trade, settlement, balance, persistence, and synchronization paths were preserved.

28 August 2026 — Distributed Trade Application
Status: 🟢 GREEN
Fixed and verified distributed TradeCreated processing.
Trades received before their corresponding orders are retained as pending and applied when the required orders arrive.
Trade application remains idempotent.
Out-of-order distributed events are covered by regression tests.
TradeStateChanged events received before TradeCreated are also retained and applied when the trade becomes available.

28 August 2026 — Settlement Accounting
Status: 🟢 GREEN
Fixed local and partial trade settlement accounting.
Settlement now:
• applies only to local participants;
• consumes the correct locked asset;
• credits the correct received asset;
• consumes only the quantity actually traded;
• rejects quote-amount overflow;
• remains idempotent;
• validates required conditions before balance mutation.
Partial-fill behaviour is covered by regression tests.

28 August 2026 — Trade State CLI
Status: 🟢 GREEN
Added the trade-state CLI command:
trade-state <trade-id> <state>
The command uses the existing trade-state transition logic rather than bypassing the state machine.
When a trade reaches Settled, the existing local settlement path is invoked.
Successful state changes are published as TradeStateChanged messages over GossipSub.
The command therefore connects:
CLI
↓
trade state machine
↓
local settlement
↓
TradeStateChanged
↓
P2P propagation
Commit: 153d59d — add trade state CLI command

29 August 2026 — Peer Authentication & Authorization Hardening
Status: 🟢 GREEN
Implemented the first dedicated security-hardening pass over remote message authorization.
Incoming GossipSub messages now establish and verify the authenticated libp2p peer identity before message-specific authorization is applied.
Implemented protections include:
• binding OpenDexMessage.sender to the authenticated PeerId;
• rejecting forged or mismatched sender identities before state mutation;
• requiring order creation identity to match the authenticated sender;
• restricting order cancellation to the authenticated order owner;
• restricting trade-state changes to authenticated trade participants;
• retaining authenticated sender identity for pending trade-state changes;
• revalidating pending state changes when the corresponding trade becomes available;
• preventing unauthorized state transitions from mutating existing trade state;
• preventing previously removed orders from being resurrected through synchronization;
• preventing stale synchronization from rolling trade state backward.
This work addresses the previously identified application-level identity spoofing risk.
The related issue is tracked as BUG-SEC-001 and is now resolved.

29 August 2026 — Security Regression Suite
Status: 🟢 GREEN
Established the dedicated security regression test module:
crates/p2p-node/tests/security.rs
The placeholder security tests were converted into executable regression coverage.
The suite covers:
• forged sender rejection;
• authenticated sender acceptance;
• order ownership validation;
• unauthorized order cancellation;
• buyer trade-state authorization;
• seller trade-state authorization;
• unauthorized trade-state rejection;
• forged-message rejection without state mutation;
• impersonated order rejection without state mutation;
• unauthorized cancellation rejection;
• stale state synchronization protection;
• removed-order resurrection protection;
• stale trade-state synchronization protection.
The security suite now provides executable protection for the authenticated identity and authorization boundary.

30 August 2026 — Security Hardening Verification Checkpoint
Status: 🟢 GREEN
Completed the security-hardening checkpoint at commit d79be81 on branch peer-dial.
The completed work establishes a clean baseline for authenticated peer identity and authorization.
Verification completed successfully:
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
Results:
• Workspace tests: 56 passed / 0 failed
• Security tests: 14 passed / 0 failed
• Clippy: clean with -D warnings
• Formatting: clean
• Network integration test: passed
• Order-book tests: passed
• Trade tests: passed
• Settlement tests: passed
• Balance tests: passed
• Protocol tests: passed
This checkpoint closes the previously identified application-level sender-authentication issue.
The security model is now explicitly separated into:
Protocol validity
≠
Domain validity
≠
Authorization
≠
Settlement authority
Authenticated origin is established, but semantic validation of remotely supplied trade data remains incomplete.

30 August 2026 — Trade/Order Consistency Review
Status: 🟡 Active Review
The current TradeCreated path was reviewed after completion of sender authentication.
The review established that authenticated origin does not prove that a remotely supplied Trade is semantically consistent with the referenced orders.
The outstanding validation boundary includes:
• order side;
• trader identity;
• market;
• trade amount;
• remaining order quantities;
• trade price;
• maker/taker relationships;
• deterministic trade identity where independently verifiable;
• contradictory order-book mutations.
This work is tracked as ARCH-004 in the Bug & Issue Tracker and as the corresponding remote trade semantic-validation concern in the Security Log.
No resolution has been recorded yet.
The next implementation task should establish the authoritative invariants and architectural owner for this validation before changing OrderBook::apply_trade().

Historical Development Session — Trade State Handler / Pending State Cleanup
Completed
• Moved PendingTradeState ownership into message_handler.rs.
• Made PendingTradeState public so it can be used by main.rs.
• Added the trade-state message sender to handle_trade_state_changed.
• Updated trade handler tests to pass the sender when processing TradeStateChanged messages.
• Preserved pending trade-state handling for out-of-order delivery.
• Preserved idempotent settlement behaviour for duplicate Settled messages.
• Ran cargo fmt.
• Ran cargo test -p p2p-node.
Historical Test Result
• Balance tests: 6/6 passed
• Network tests: 1/1 passed
• Orderbook tests: 16/16 passed
• Settlement tests: 6/6 passed
• Trade tests: 11/11 passed
• Doc tests: 0/0 passed
• Total: 40 tests passed, 0 failed
Remaining Warning
BalanceBook::transfer_locked() remains unused and produces a dead_code warning.
No action was taken because the method may be needed by subsequent settlement/trade work.
Status: Clean checkpoint — all p2p-node tests passing.

Current Checkpoint
Date: 30 August 2026
Branch: peer-dial
Commit: d79be81
Status: 🟢 GREEN baseline / 🟡 Security architecture work active
The current implementation provides:
• P2P networking;
• GossipSub message propagation;
• authenticated remote peer identity;
• order creation and cancellation;
• order ownership authorization;
• order matching;
• distributed order convergence;
• distributed trade creation;
• pending/out-of-order event handling;
• trade-state progression;
• trade participant authorization;
• local settlement;
• partial-fill settlement;
• settlement idempotency;
• checked settlement arithmetic;
• persistent order/trade state;
• CLI trade-state control;
• synchronization protections;
• two-node network integration testing;
• dedicated security regression coverage.
Current Verification Baseline
cargo fmt --all                         PASS
cargo test --workspace                  PASS
cargo clippy --workspace --all-targets \
--all-features -- -D warnings         PASS

Workspace tests:     56 passed / 0 failed
Security tests:      14 passed / 0 failed
The repository currently has a clean reproducible verification baseline.

Current Architectural Position
P2P Migration
↓
 Functionally Complete
↓
Architectural Simplification
↓
 GossipSub event boundary simplified
↓
 Protocol dispatch simplified
↓
 Distributed trade application verified
↓
 Settlement accounting verified
↓
 Trade-state CLI connected
↓
 Authenticated peer identity established
↓
 Remote authorization regression coverage
↓
 Trade/order semantic validation
↓
 Trade application ownership review
↓
 Per-order balance reservation review
↓
 Synchronization/persistence review
↓
 Replay and resource-exhaustion review
↓
 Adversarial multi-node testing
↓
Security hardening
↓
Final audit preparation
↓
Final audit
The next phase should concentrate on correctness and security boundaries rather than cosmetic decomposition.
In particular, the relationship between:
Remote TradeCreated
↓
trade semantic validation
↓
OrderBook::apply_trade()
↓
trade/order state
↓
balance reservation
↓
settlement
requires explicit invariants before the final audit phase.

Current Development Position
The authenticated peer boundary is now implemented and regression-tested.
The next major development objective is not additional sender-authentication work.
The immediate objective is to establish the semantic invariants for remotely supplied trades and ensure that every trade-application path enforces those invariants before state mutation.
Architectural refactoring should follow the invariant definition rather than precede it.
The project should continue to follow:
Simplify first. Secure second. Audit third.












Development Log Update
Date: 2026-08-31
Branch: peer-dial
Milestone: Pending Trade-State Hardening
Implemented and verified improvements to distributed trade-state handling.
Completed
• Added happy-path trade-state identification.
• Improved pending trade-state replacement logic.
• Added support for later valid states when intermediate messages are missed.
• Preserved the original pending state for duplicate messages.
• Preserved authenticated sender information with pending state changes.
• Added authorization checks when delayed state changes are applied.
• Prevented exceptional trade states from being incorrectly ordered using the happy-path ranking.
• Added regression and security tests covering the new behavior.
• Resolved PendingTradeState visibility required by integration tests.
• Formatted the project with cargo fmt.
Verification
cargo test -p p2p-node
Passed.
cargo test --workspace
Passed.
cargo check
Passed.
git diff --check
Passed.
Test Coverage
Current p2p-node test suite covers:
• Balance locking and unlocking
• CLI parsing
• Peer networking
• Order matching
• Distributed order convergence
• Trade persistence
• Trade-state transitions
• Pending trade-state progression
• Unauthorized state-change rejection
• Settlement
• Settlement idempotency
• Overflow protection
• Message authentication
• Order ownership
• Cancellation authorization
Git Checkpoint
Changes committed:
2c846ac Harden pending trade state handling
Checkpoint created:
checkpoint-pending-trade-state
Working tree verified clean.
Next Step
Continue development from the clean checkpoint-pending-trade-state baseline.
OpenDEX — Development Log
DL-2026-08-31-001 — message_handler.rs Complexity Audit
Date: 31 August 2026
Phase: Phase 2 — Complexity Audit
Current Target: crates/p2p-node/src/message_handler.rs
Status: 🟡 Audit in progress
Work Completed
The current message_handler.rs implementation has been reviewed alongside:
• sync.rs
• orderbook.rs
• trade_state.rs
• trades.rs
The review confirms that the previous pending-trade-state hardening has been committed and checkpointed.
Architectural Findings
message_handler.rs currently performs several distinct responsibilities:
Swarm events
↓
Identify handling
↓
Kademlia interaction

Gossipsub events
↓
Message decoding
↓
Sender authentication
↓
Message dispatch
↓
Domain mutation
↓
Persistence
↓
Network publication
The file therefore remains a significant architectural boundary and should continue to be treated as the primary refactoring target.
Current Classification
🟢 KEEP
• Explicit sender identity check.
• Explicit order ownership checks.
• Explicit trade participant checks.
• Delegation of settlement to settlement.rs.
• Delegation of state transitions to trade_state.rs.
• Delegation of snapshot creation/application to sync.rs.
🟡 SIMPLIFY
• Repeated JSON decoding/error-handling patterns.
• Repeated message construction.
• Repeated persistence calls.
• Some verbose dispatch paths.
🟠 EXTRACT
Potential future extraction targets:
Gossipsub message decoding
Message authentication/validation
Order message handling
Trade message handling
State synchronisation handling
Swarm/Identify handling
Gossipsub event handling
These are candidates only. No extraction should occur until responsibility boundaries are confirmed.
🔵 MOVE
The following areas may eventually belong outside the main message handler:
• transport-specific event handling
• message envelope validation
• state synchronisation policy
• potentially network publication orchestration
The exact destination should be determined during the cross-module audit rather than creating abstractions prematurely.
🔴 REMOVE
No confirmed dead code has been identified in message_handler.rs during this pass.
Important Architecture Observation
The current handler has a relatively broad HandlerContext:
swarm
topic
local_peer_id
order_book
trade_book
balance_book
pending_trade_states
This gives the handler substantial authority across networking, orders, trades, balances and pending state.
This directly relates to Master Plan §4:
Every component should have one clear responsibility and the minimum authority required to perform it.
Reducing this authority is therefore a major candidate for the ongoing Phase 2 refactoring.
Current Decision
Do not begin another large refactor yet.
Complete the structural audit first.
The next investigation should focus on the boundary:
StateResponse
↓
StateSnapshot
↓
apply_snapshot()
↓
OrderBook / TradeBook
This is both an architectural complexity boundary and a security-sensitive trust boundary.
Next Planned Action
Complete the message_handler.rs audit and document the intended responsibility boundaries before extracting or moving code.
After the audit:
1. Identify the smallest safe refactoring.
2. Make one logical change.
3. Run cargo check.
4. Run cargo test.
5. Verify P2P behaviour if networking is affected.
6. Commit the change.
7. Update the development and security logs.

