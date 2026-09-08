 # OpenDEX — Architecture Development Master Plan

 > “The old v2.0 can be retained in Git history…”

 **Version:** 3.0\
 **Date:** 6 September 2026\
 **Status:** Active — Architectural Redesign & Simplification\
 **Supersedes:** Version 2.0

---

 ## 1\. Mission

 The objective is to evolve OpenDEX into a modular, comprehensible, auditable and security-conscious P2P exchange implementation.

 The project must avoid reproducing the original monolithic architecture as functionality is migrated or redesigned.

 The guiding principle remains:

 > **Simple code, narrow responsibilities, explicit boundaries, minimal authority and predictable state transitions.**

 The purpose of the architecture work is therefore not simply code organisation.

 The eventual system should be:

 - easier to understand
- easier to test
- easier to maintain
- easier to audit
- harder to misuse
- less coupled
- less exposed to unexpected interactions
- easier to reason about from a security perspective

 This document is the single architectural and development plan for OpenDEX.

---

 ## 2\. Governing Architectural Principles

 These principles govern all future development.

 ### 2.1 Simplicity

 Prefer the simplest design that correctly expresses the required behaviour.

 Additional abstraction, indirection or modules must have a clear purpose.

 We will not create abstractions merely because they appear architecturally sophisticated.

 ### 2.2 Narrow Responsibilities

 Each component should have one clear primary responsibility.

 A component should not simultaneously become responsible for:

 - networking
- message decoding
- order matching
- trade state
- balances
- settlement
- persistence
- peer management

 unless there is a compelling architectural reason.

 ### 2.3 Minimal Authority

 Every function and module should receive only the state and capabilities it actually requires.

 A network message handler should not automatically have unrestricted authority over financial state.

 A balance component should not need to understand libp2p.

 A domain operation should not need to understand Gossipsub.

 ### 2.4 Explicit Boundaries

 The boundaries between:

 - network
- messages
- validation
- authorisation
- domain logic
- state
- persistence
- publication

 must be explicit.

 Raw network input must not flow directly into sensitive state mutation.

 ### 2.5 Predictable State Transitions

 Security-sensitive state changes must be explicit and traceable.

 An auditor should be able to follow:

```
Input
  ↓
Decode
  ↓
Validate
  ↓
Authorise
  ↓
Domain operation
  ↓
State transition
  ↓
Persistence
  ↓
Event/publication
```

 without tracing through a large event handler.

 ### 2.6 Preserve Behaviour Unless Intentionally Changed

 The redesign is permitted to replace the implementation.

 It is not permitted to accidentally change protocol or business behaviour.

 Where behaviour is intentionally changed to correct a bug or security weakness, the change must be documented and tested.

---

 ## 3\. Why the Architecture Is Being Redesigned
 
  The existing modularisation successfully removed much of the original networking monolith.
 
  However, further inspection has demonstrated that simply continuing a file-by-file complexity refactor is not necessarily the simplest or safest path.
 
  Important state transitions remain distributed across multiple components.
 
  Examples include:
 
command_handler
↓
lock_for_order()
↓
order_book

↓
match_order()
↓
Exchange
↓
trade creation
↓
trade creation
↓
trade state
↓
settlement
↓
consume_locked()
↓
balances

The redesign therefore changes the objective from:

> Refactor the existing architecture file by file

to:

> **Define a simpler state and authority model, then migrate the implementation toward it.**

+### 3.1 Implementation Progress
+
+The redesign has now established several concrete boundaries:
+
+- order matching has been extracted behind the `Exchange` abstraction rather than remaining solely as an `OrderBook` concern
+- persistence helpers have been separated into `persistence.rs`
+- balance reservations are explicitly associated with order IDs
+- existing order, trade, settlement and security behaviour remains covered by tests
+
+These changes are incremental architectural progress rather than completion of the target architecture.
+
+The current verification checkpoint is:
+
+```
+cargo fmt --check → pass
+cargo test       → 96 passed, 0 failed
+```
+
+The test suite continues to cover authentication, authorisation, idempotency,
+distributed ordering/convergence, persistence, settlement and arithmetic
+failure cases.
+
+One non-fatal compiler warning remains for `BalanceBook::order_reservation`.
+Its current consumer is an integration test, so the production target reports
+the method as unused. This is not currently considered an architectural
+blocker.
+
---

 ## 4\. Architectural Direction

 The target architecture is based on a strict separation between network concerns and domain concerns.

 The conceptual architecture is:

```
                          NETWORK
                             │
                             ▼
                  ┌─────────────────────┐
                  │  Message Boundary   │
                  │                     │
                  │ decode              │
                  │ basic validation    │
                  │ authentication      │
                  └──────────┬──────────┘
                             │
                             ▼
                  ┌─────────────────────┐
                  │   Message Router    │
                  └──────────┬──────────┘
                             │
                 ┌───────────┼───────────┐
                 ▼           ▼           ▼
              Orders       Trades     State Sync
                 │           │           │
                 └───────────┼───────────┘
                             ▼
                  ┌─────────────────────┐
                  │    Domain Layer     │
                  │                     │
                  │ validation          │
                  │ authorisation       │
                  │ state transitions   │
                  └──────────┬──────────┘
                             │
                  ┌──────────┴──────────┐
                  ▼                     ▼
           ┌─────────────┐       ┌─────────────┐
           │    State    │       │  Balances   │
           │             │       │             │
           │ orders      │       │ available   │
           │ trades      │       │ locked      │
           │ lifecycle   │       │ mutations   │
           └─────────────┘       └──────┬──────┘
                                        │
                                        ▼
                                 ┌─────────────┐
                                 │ Settlement  │
                                 └─────────────┘
                                        │
                                        ▼
                                   Persistence
```

 This is a target architectural model rather than a requirement that every box become a separate Rust module.

 The implementation should follow responsibilities, not the diagram mechanically.

---

 ## 5\. Trust Boundary

 The most important security boundary is the transition from:

```
UNTRUSTED NETWORK INPUT
```

 to:

```
TRUSTED INTERNAL DOMAIN OPERATION
```

 The intended flow is:

```
Network
   ↓
Raw Message
   ↓
Decode
   ↓
Validate
   ↓
Authenticate sender
   ↓
Authorise operation
   ↓
Convert to domain operation
   ↓
Domain logic
   ↓
Controlled state mutation
```

 Raw network structures must not directly control balances or settlement.

---

 ## 6\. Domain Independence

 Core exchange logic should be independent of libp2p wherever practical.

 The preferred relationship is:

```
libp2p
   ↓
transport
   ↓
message boundary
   ↓
domain operation
   ↓
state
```

 rather than:

```
domain logic
    ↕
libp2p
    ↕
Gossipsub
    ↕
Kademlia
```

 This allows core exchange behaviour to be tested without requiring a running P2P network.

---

 ## 7\. State Ownership

 The redesign will explicitly define ownership of mutable state.

 The following questions must have one clear answer:

 - Who creates orders?
- Who removes orders?
- Who matches orders?
- Who creates trades?
- Who changes trade state?
- Who locks funds?
- Who unlocks funds?
- Who consumes locked funds?
- Who credits balances?
- Who performs settlement?
- Who persists state?
- Who publishes resulting protocol events?

 There should not be multiple unrelated paths capable of performing the same sensitive mutation.

 Each sensitive state component must have:

 - a clearly identified owner
- defined mutation operations
- defined validation requirements
- defined authorisation requirements
- defined persistence behaviour
- defined event/publication behaviour where applicable

 Broad mutable access should be avoided wherever practical.

---

 ## 8\. Order State

 Orders represent trading intent.

 Order-related responsibilities should include:

- order validation
- order ownership
- order lifecycle
- order matching, delegated to the exchange/matching boundary
- remaining quantity
- removal/cancellation
- order-book state

 The order system should not directly contain network-specific behaviour.

 The order system should not be responsible for interpreting arbitrary network messages.

---

 ## 9\. Trade State

 Trades represent the result of a successful match.

 Trade responsibilities should include:

 - trade identity
- participating orders
- buyer
- seller
- market
- price
- amount
- lifecycle state
- valid state transitions
- trade idempotency

 Trade state transitions must be explicit.

 A trade should not become settled merely because a particular message was received.

 The domain must determine whether the transition is valid.

---

 ## 10\. Balance Architecture

 Balances represent financial state and therefore require a particularly narrow authority boundary.

 A balance system should explicitly distinguish:

```
available funds
       │
       ▼
locked funds
       │
       ├── unlock
       │
       └── consume during settlement
```

 Balance mutations must be explicit.

 The following operations require particular scrutiny:

 - credit
- debit
- lock
- unlock
- consume locked funds
- settlement transfers

 The redesign must prevent unrelated network or order-book code from directly manipulating balances.

---

 ## 11\. Settlement Architecture

 Settlement is a financial state transition, not merely a consequence of receiving a network message.

 The intended conceptual operation is:

```
Validated Trade
      ↓
Authorised Participants
      ↓
Settlement Preconditions
      ↓
Atomic Financial Transition
      ↓
Buyer:
    locked quote decreases
    available base increases

Seller:
    locked base decreases
    available quote increases
      ↓
Trade marked/recorded as settled
      ↓
Persist
```

 Settlement must be:

 - deterministic
- idempotent
- explicitly authorised
- validated before mutation
- protected against arithmetic overflow
- protected against insufficient funds
- independent of network transport

 The implementation must avoid partial financial transitions.

---

 ## 12\. Atomicity Principle

 A financial operation must not leave the balance system partially modified if a later part of the same operation fails.

 The preferred model is:

```
validate everything
       ↓
perform complete transition
       ↓
persist result
```

 rather than:

```
mutate
   ↓
try something else
   ↓
discover failure
   ↓
attempt recovery
```

 Where persistence itself can fail after in-memory mutation, the architecture must explicitly determine the appropriate recovery or transactional strategy.

 This issue must be addressed during the balance/settlement redesign rather than hidden inside handlers.

---

 ## 13\. Idempotency

 Distributed systems inevitably receive:

 - duplicate messages
- delayed messages
- reordered messages
- repeated state notifications
- duplicate trade announcements

 Idempotency must therefore be a domain property wherever appropriate.

 Trade settlement in particular must guarantee:

```
same trade
    ↓
first valid settlement → applied
    ↓
duplicate settlement → no second financial mutation
```

 Idempotency should not depend solely on a particular network handler behaving correctly.

---

 ## 14\. Message Architecture

 Network messages should be treated as untrusted input.

 The message layer should be responsible for:

 - decoding
- message-size considerations
- message-type validation
- sender authentication
- basic structural validation
- routing

 It should not become the primary owner of domain state.

 The desired model is:

```
Message
   ↓
Validate
   ↓
Authenticate
   ↓
Create domain operation
   ↓
Domain
```

---

 ## 15\. Authorisation

 Authentication and authorisation are separate concerns.

 Knowing who sent a message does not automatically mean that the sender is authorised to perform the requested operation.

 For operations involving orders or trades, ownership and participation must be explicitly checked.

 The system should be able to answer:

```
Who sent this?
       ↓
What operation are they requesting?
       ↓
Are they authorised to perform it?
       ↓
What state may they change?
```

---

 ## 16\. Network Architecture

 Networking should remain responsible for network concerns.

 Existing functionality includes:

 - libp2p
- peer identity
- listening
- direct dialing
- Identify
- Kademlia
- Gossipsub
- topic subscription
- connection lifecycle
- network event processing

 These capabilities should remain operational while being progressively isolated from domain logic.

 Networking should not directly implement:

 - settlement rules
- balance rules
- trade lifecycle rules
- financial validation
- core matching semantics

---

 ## 17\. Message Router

 The message router should become a thin boundary.

 Conceptually:

```
incoming message
       ↓
decode
       ↓
authenticate
       ↓
route
       ├── order operation
       ├── trade operation
       ├── state synchronisation
       └── network operation
```

 The router should not become another monolith.

 If routing logic begins accumulating domain behaviour, that behaviour must be moved into the appropriate domain component.

---

 ## 18\. State Synchronisation

 State synchronisation is a separate concern from ordinary domain operations.

 State snapshots must be:

 - structurally validated
- bounded
- checked for consistency
- applied through controlled state operations

 A remote snapshot must never automatically grant authority to mutate protected financial state.

 Particular attention will be given to:

 - stale snapshots
- malicious snapshots
- conflicting state
- oversized state
- repeated synchronisation
- trade/order inconsistencies

---

 ## 19\. Persistence

 Persistence should have a clearly defined responsibility.

 The system must distinguish between:

```
domain state
```

 and:

```
storage representation
```

 Persistence should not silently contain business logic.

 The redesign will evaluate whether:

 - each state component should persist independently
- a unified state store is simpler
- operations require transactional persistence
- crash recovery needs additional mechanisms
- corrupted state can be detected and rejected

 The simplest safe approach should be preferred.

---

 ## 20\. Module Direction

 The exact final module structure will be determined during implementation.

 A possible target structure is:

```
crates/p2p-node/src/

    main.rs
    network.rs
    messages.rs
    router.rs

    domain/
        orders.rs
        trades.rs
        balances.rs
        settlement.rs
        state.rs

    persistence.rs
```

 This is illustrative only.

 We will not create modules simply to match this diagram.

 If two responsibilities are genuinely simpler together, they may remain together.

 If a component becomes too authoritative or complex, it should be split.

 The principle is responsibility, not file count.

---

 ## 21\. API Narrowing

 Public interfaces should be reduced wherever practical.

 A function should receive:

 - the exact state it needs
- the exact command it needs
- the minimum capability required

 Avoid passing broad collections of mutable state when a narrower interface is possible.

 For example, prefer:

```
settle(trade, balances)
```

 over a function that receives:

```
network
order_book
trade_book
balance_book
pending_states
swarm
topic
```

 when most of those objects are irrelevant to settlement.

---

 ## 22\. Error Handling

 Errors should be explicit and predictable.

 Preferred pattern:

```
validate
   ↓
reject invalid input early
   ↓
perform operation
   ↓
return result
```

 Avoid unnecessarily deep nesting.

 Errors must not be hidden by logging.

 Logging should describe an error; it should not determine whether an operation succeeds.

---

 ## 23\. Logging

 Logging will be separated conceptually into:

 - operational events
- debugging information
- protocol events
- errors
- security-relevant events

 Logging must never become part of the correctness mechanism.

 The system must not rely on:

```
"we logged that settlement happened"
```

 as evidence that settlement actually occurred.

 State determines truth.

---

 ## 24\. Legacy and Dead Code

 Legacy implementations will be removed where they are no longer required.

 Known examples requiring review include:

```
main.rs.save
main.rs.save.1
```

 and previously identified unused functionality such as:

```
debit_available
transfer_locked
```

 Legacy code must not remain simply because it might be useful someday.

 Before removal, determine whether it is:

 - required
- part of an incomplete migration
- obsolete
- duplicated
- incorrectly disconnected

 Once confirmed obsolete, it should be removed.

 The final security review should not have to reason about abandoned implementations.

---

 # 25\. Implementation / Migration Plan

 The redesign will proceed in controlled stages.

 The purpose of this migration is **not** to move existing code into new files mechanically.

 The target architecture must be established first, and existing behaviour should be migrated only where it is compatible with the new state and authority model.

 Where the existing implementation conflicts with the target architecture, behaviour should be deliberately reimplemented rather than carrying the old architectural assumptions into new modules.

 ### Phase 1 — Freeze the Current Implementation

 Establish a regression baseline before beginning architectural migration.

 Run:

```
cargo check
cargo test
```

 Record:

 - current build status
- current test status
- known protocol behaviour
- known business behaviour
- existing P2P behaviour
- known architectural or security issues

 Do not modify the old architecture simply to make migration easier.

 The existing implementation remains the behavioural reference until replacement behaviour has been verified.

 ### Phase 2 — Map the Domain

 Identify the actual domain state and operations before deciding the final module structure.

 The initial domain map must include:

 - orders
- order matching
- trades
- trade lifecycle
- balances
- locked funds
- settlement
- persistence
- state synchronisation

 For each state component, determine:

 - what state exists
- who creates it
- who owns it
- who may mutate it
- what operations may mutate it
- what validation is required
- what authorisation is required
- whether the operation must be atomic
- whether the operation must be idempotent
- whether the state must be persisted
- what events result from the operation

 This domain map is authoritative for subsequent implementation work.

 ### Phase 3 — Define State Ownership

 Define exactly one primary owner for each sensitive mutable state component.

 The implementation must identify the permitted mutation paths for:

 - orders
- order-book state
- trades
- trade lifecycle
- available balances
- locked balances
- settlement state
- synchronisation state

 Broad mutable access should be reduced wherever practical.

 Each sensitive mutation should have an identifiable owner and a small, explicit API.

 ### Phase 4 — Define the New Module Boundaries

 Establish the dependency direction before migrating implementation code.

 The intended relationship is:

```
libp2p
    ↓
network transport
    ↓
message boundary
    ↓
authentication / authorisation
    ↓
domain command
    ↓
domain operation
    ↓
state mutation
    ↓
persistence
    ↓
event publication
```

 The domain must not depend unnecessarily on:

 - libp2p
- Gossipsub
- Kademlia
- network event types
- transport-specific behaviour

 The network layer should transport messages rather than contain exchange logic.

 The message layer should decode, structurally validate, authenticate, authorise and route messages rather than own financial state.

 The persistence layer should store domain state rather than silently become another business-logic layer.

 The exact Rust module structure will be determined by these responsibilities rather than imposed mechanically.

 ### Phase 5 — Redesign Balances and Settlement

 Balances and settlement receive special attention because the current implementation has coupling between:

 - orders
- locked funds
- trades
- balances
- settlement

 Define the financial invariants before implementing the replacement.

 At minimum, establish and test:

 - available funds cannot become negative
- locked funds cannot exceed legitimately available funds
- funds cannot be spent twice
- settlement cannot create funds from nothing
- settlement cannot be applied twice
- failed operations cannot leave partial financial state
- arithmetic overflow is rejected
- insufficient funds are rejected

 The conceptual flow is:

```
validated trade
      ↓
authorised participants
      ↓
settlement preconditions
      ↓
complete financial transition
      ↓
trade marked settled
      ↓
persist
```

 Settlement must be deterministic, explicitly authorised, validated before mutation, idempotent and independent of network transport.

 **Do not migrate existing settlement code merely because it already exists.**

 If the existing implementation conflicts with these requirements, reimplement the behaviour within the new model.

 ### Phase 6 — Implement the Domain Core

 Build or restructure the core exchange operations independently of libp2p.

 The domain core should provide explicit operations for the required behaviour, including as applicable:

```
create order
cancel order
match orders
create trade
transition trade
lock funds
unlock funds
consume locked funds
settle trade
```

 The exact API should be determined by the state-ownership model.

 The domain core must be testable without a running P2P network.

 State transitions must be explicit and invalid transitions must be rejected.

 Focused unit tests should be added as each operation is implemented.

 ### Phase 7 — Build the Message Boundary

 Move network message handling toward the following sequence:

```
raw network message
        ↓
decode
        ↓
structural validation
        ↓
authenticate sender
        ↓
authorise operation
        ↓
convert to domain command
        ↓
execute domain operation
        ↓
produce result/event
```

 Network handlers should become progressively thinner.

 Raw network structures must not directly control balances, trades or settlement.

 Authentication must remain separate from authorisation.

 Knowing the identity of the sender does not by itself grant permission to perform the requested operation.

 ### Phase 8 — Reconnect P2P Networking

 Reconnect the simplified domain core to the existing P2P functionality.

 This includes:

 - libp2p
- peer identity
- Identify
- Kademlia
- Gossipsub
- peer discovery
- topic subscription
- connection lifecycle
- message publication
- state requests
- state responses
- trade synchronisation

 The domain must remain independent of these mechanisms.

 P2P components should act as transport and synchronisation infrastructure around the domain rather than become alternative owners of domain state.

 ### Phase 9 — Integrate Persistence

 Establish explicit persistence boundaries around the new state model.

 Determine:

 - what state is persisted
- when state is persisted
- whether operations require transactional persistence
- how failed persistence is handled
- how restart/recovery works
- how corrupted state is detected
- whether duplicate operations remain safe after restart

 The architecture must explicitly address the case where an in-memory state transition succeeds but persistence fails.

 Persistence must not become an alternative state-mutation mechanism outside the domain's authority model.

 ### Phase 10 — Integration and Regression Testing

 Verify the replacement architecture progressively.

 Begin with:

```
single-node domain tests
```

 Then verify:

```
Node A
   ↕
Node B
```

 and subsequently:

```
Node A
   ↕
Node B
   ↕
Node C
```

 Test at minimum:

 - connectivity
- peer identity
- Identify
- Kademlia
- Gossipsub
- order propagation
- order matching
- trade creation
- trade propagation
- state requests
- state responses
- state synchronisation
- duplicate handling
- replay handling
- trade lifecycle
- settlement
- balance invariants
- restart/recovery where applicable

 The replacement must demonstrate functional parity before the old implementation is removed, except where a deliberate bug or security correction has been documented and tested.

 ### Phase 11 — Remove the Old Architecture

 Only after the replacement behaviour has been verified should obsolete implementation paths be removed.

 Review and remove, where confirmed obsolete:

 - legacy handlers
- duplicate state mutation paths
- migration remnants
- unused APIs
- `.save` legacy implementations
- obsolete files
- disconnected or duplicated functionality

 Before removal, determine whether each legacy component is:

 - required
- part of an incomplete migration
- obsolete
- duplicated
- incorrectly disconnected

 The final security review should not have to reason about abandoned implementations.

 ### Phase 12 — Architecture and Security Review

 After migration, perform a cross-module architecture review.

 Review:

 - dependency direction
- state ownership
- mutation authority
- trust boundaries
- authentication
- authorisation
- every financial mutation
- every trade-state transition
- persistence boundaries
- network-to-domain boundaries
- duplicate and replay handling
- resource limits
- error paths
- remaining legacy code

 Confirm that the final architecture satisfies the security invariants defined in this document.

 Only after this review should the project proceed toward the formal security audit.

---

 ## 26\. Development Rules

 ### Rule 1 — Preserve Behaviour

 Do not introduce unrelated protocol or business changes during architectural work.

 ### Rule 2 — Small Logical Changes

 Each change should have one clear purpose.

 ### Rule 3 — Compile Frequently

 After meaningful changes:

```
cargo check
```

 ### Rule 4 — Test Frequently

 Run:

```
cargo test
```

 ### Rule 5 — Test Domain Logic Independently

 Financial and state-transition logic should have tests that do not require networking.

 ### Rule 6 — Verify P2P After Network Changes

 Whenever networking behaviour changes, perform multi-node verification.

 ### Rule 7 — Do Not Abstract for Abstraction's Sake

 More modules do not automatically mean better architecture.

 ### Rule 8 — Prefer Explicit Code

 Especially around:

 - validation
- authorisation
- state transitions
- balances
- settlement
- peer identity
- message handling

 ### Rule 9 — Minimise Authority

 A function should receive only the capabilities it needs.

 ### Rule 10 — One Owner for Sensitive Mutation

 Avoid multiple independent paths capable of changing the same protected state.

 ### Rule 11 — Domain Logic Must Be Network-Independent

 Where practical, domain operations must be testable without libp2p.

 ### Rule 12 — Security Is Part of Implementation

 Do not postpone security considerations until the final audit.

---

 ## 27\. Testing Strategy

 Every architectural change must maintain a regression baseline.

 ### Build

```
cargo check
```

 ### Unit Tests

```
cargo test
```

 Tests should cover:

 - order creation
- order validation
- order matching
- order cancellation
- trade creation
- trade state transitions
- balance credit
- balance debit
- locking
- unlocking
- locked-fund consumption
- settlement
- settlement idempotency
- arithmetic overflow
- insufficient balances
- invalid state transitions
- authorisation failures
- P2P integration

 ### Multi-Node Testing

 At minimum:

```
Node A
  ↕
Node B
  ↕
Node C
```

 Verify:

 - connectivity
- Identify
- Kademlia
- Gossipsub
- order propagation
- trade propagation
- state request
- state response
- duplicate handling
- trade lifecycle
- settlement
- balances
- restart/recovery where applicable

---

 ## 28\. Security Review Areas

 After the architectural migration, conduct a dedicated attack-surface review.

 ### Network Surface

 Review:

 - listening interfaces
- inbound connections
- peer connections
- connection lifecycle
- discovery
- Identify
- Kademlia
- Gossipsub
- resource exhaustion

 ### Message Surface

 Review:

 - deserialisation
- malformed messages
- oversized messages
- unexpected message types
- duplicate messages
- replay
- sender identity
- authorisation
- state synchronisation

 ### State Surface

 Review:

 - order mutation
- trade mutation
- balance mutation
- locked funds
- settlement
- persistence
- state transitions

 ### Resource Surface

 Review:

 - unbounded collections
- pending messages
- pending trades
- pending state changes
- repeated synchronisation
- computationally expensive operations
- connection exhaustion

---

 ## 29\. Security Invariants

 The following invariants should become explicit testable properties.

 ### Financial Invariants

 - Funds cannot be spent twice.
- Locked funds cannot exceed the funds legitimately locked.
- A settlement cannot create funds from nothing.
- A settlement cannot be applied twice.
- A failed operation must not produce a partial financial transition.

 ### Ownership Invariants

 - A peer cannot modify another peer's protected state without authorisation.
- Trade participants must be authenticated.
- State-changing messages must be associated with an authorised participant where required.

 ### State Invariants

 - Invalid trade transitions are rejected.
- Duplicate operations are harmless where the protocol requires idempotency.
- Remote messages cannot directly bypass domain validation.

 ### Architectural Invariants

 - Network code does not directly implement financial rules.
- Domain logic does not depend unnecessarily on libp2p.
- Sensitive state has clearly defined mutation owners.

---

 ## 30\. Definition of Done

 The redesign is not complete merely because:

```
cargo check
```

 passes.

 The architecture is considered complete when:

 - responsibilities are clear
- state ownership is explicit
- trust boundaries are clear
- complex blocks have been reviewed
- unnecessary nesting has been addressed
- duplicated logic has been removed or justified
- unnecessary authority has been reduced
- dead code has been assessed
- financial mutations have clear owners
- settlement is explicit and idempotent
- error paths are understandable
- security-sensitive state changes are identifiable
- domain logic can be tested independently
- persistence behaviour is understood
- P2P behaviour remains functional
- tests pass
- behaviour remains unchanged unless an intentional bug/security fix was made
- obsolete architecture has been removed

---

 ## 31\. Current Project Position

 OpenDEX has already achieved substantial functional P2P migration.

 Existing capabilities include:

```
OpenDEX
   │
   ▼
P2P Node
   │
   ├── libp2p
   ├── Peer identity
   ├── Identify
   ├── Kademlia
   ├── Gossipsub
   ├── OpenDEX messaging
   ├── State requests
   ├── State responses
   ├── Trade synchronisation
   ├── Trade deduplication
   └── Removed-order synchronisation
```

 The project now enters:

```
ARCHITECTURAL REDESIGN
        │
        ▼
STATE OWNERSHIP
        │
        ▼
DOMAIN CORE
        │
        ▼
BALANCES / SETTLEMENT
        │
        ▼
MESSAGE BOUNDARY
        │
        ▼
NETWORK INTEGRATION
        │
        ▼
PERSISTENCE
        │
        ▼
LEGACY REMOVAL
        │
        ▼
CROSS-MODULE AUDIT
        │
        ▼
ATTACK-SURFACE REDUCTION
        │
        ▼
SECURITY AUDIT PREPARATION
        │
        ▼
FORMAL SECURITY REVIEW
```

---

 ## 32\. Immediate Development Plan

 The immediate objective is **not** to continue patching `message_handler.rs`.

 The immediate objective is to design and implement the replacement state and authority model.

 The first implementation questions are:

 - What state exists?
- Who owns each state component?
- What operations may mutate it?
- What validation is required before mutation?
- What authorisation is required?
- What operations must be atomic?
- What must be idempotent?
- What must be persisted?
- What events need to be published?
- What code should have no knowledge of networking?

 These questions are addressed through the implementation/migration phases defined in Section 25.

 Implementation should proceed according to those phases rather than through uncontrolled file-by-file refactoring.

---

 ## 33\. Architectural Success Criteria

 The final architecture should make the following flow easy to understand:

```
UNTRUSTED NETWORK INPUT
          │
          ▼
       DECODE
          │
          ▼
      VALIDATE
          │
          ▼
     AUTHENTICATE
          │
          ▼
      AUTHORISE
          │
          ▼
    DOMAIN OPERATION
          │
          ▼
  EXPLICIT STATE CHANGE
          │
      ┌───┴────┐
      ▼        ▼
   DOMAIN   BALANCES
      │        │
      └───┬────┘
          ▼
       PERSIST
          │
          ▼
    PUBLISH EVENT
```

 A security auditor should be able to trace a sensitive operation through this sequence without having to reconstruct behaviour from unrelated event handlers.

---

 ## 34\. Core Objective

 The most important objective of this version of the plan is:

 > **Do not merely make the existing architecture smaller. Build the simplest architecture that gives each responsibility a clear owner and each sensitive state transition an explicit boundary.**

 The project should favour:

 - less code
- less coupling
- less authority
- less implicit behaviour
- less duplicated state mutation

 while increasing:

 - clarity
- testability
- auditability
- isolation
- traceability
- predictability

 The redesign is successful when the architecture itself makes incorrect behaviour difficult to express.

---

 ## 35\. Final Architectural Principle

 OpenDEX should ultimately consist of a small number of clearly defined components with:

 - narrow responsibilities
- explicit inputs and outputs
- minimal authority
- controlled state mutation
- predictable transitions
- clear trust boundaries
- independently testable domain logic

 The architecture must be designed so that security is not something added after development.

 > **Security should emerge from the structure of the system itself.**

 This document replaces the existing `audit/Architecture_Development_Master_Plan.md` as the active architectural development plan.

 The old v2.0 may be retained in Git history, but v3.0 becomes the document we work from going forward.

 There must be **one authoritative migration plan**, contained in Section 25. No duplicated or appended migration-plan material should be retained outside that section.

 ### One important thing before you save it

 I made the **document-cleanup fixes**, but I deliberately did **not** pretend that every architectural statement has been verified against the actual Rust codebase. For example, claims such as:

 - “OpenDEX has already achieved substantial functional P2P migration”
- `debit_available` being unused
- `transfer_locked` being unused
- the exact current responsibilities of `message_handler.rs`
- the current persistence behaviour

 are **codebase-specific claims** that should be checked against the repository.

 So this is the **clean/fixed architecture document**, not yet a code-verified audit of whether the architecture accurately describes the current implementation.

 The next sensible step is **Phase 1**: inspect the actual Rust code and establish the baseline before changing anything.
## Architecture Verification Status

The architecture described in this document is a target architecture.
The following inventory records which architectural boundaries have been
verified against the current repository implementation.

| Architecture claim | Verification status |
|---|---|
| Separate protocol crate | Verified |
| Separate P2P/network layer | Verified |
| Dedicated message handler | Verified |
| Dedicated command handler | Verified |
| Dedicated OrderBook | Verified |
| Dedicated TradeBook | Verified |
| Dedicated balances | Verified — including per-order balance reservations |
| Dedicated settlement | Verified |
| Dedicated synchronization | Verified |
| Matching owned by OrderBook | Verified |
| Persistence exists | Verified — including persisted order reservation state |
| Centralized persistence abstraction | Not present |
| Fully separated authorization layer | Partially verified |
| Synchronization authority model | Not yet established |
| Complete remote semantic validation | Not yet established |
| Target v3.0 architecture fully implemented | Not verified |

Boundary	Current implementation	Verified
|---|---|
Network authentication	libp2p PeerId	✅
Claimed sender binding	msg.sender == PeerId	✅
Order authorization	Owner check	✅
Cancellation authorization	Owner check	✅
Trade authorization	Participant check	✅
Trade-state authorization	Participant check	✅
Snapshot semantic validation	validate_state_snapshot()	✅
Snapshot authority	No check identified	⚠️
Snapshot mutation	apply_snapshot()	Need to inspect
Verification is based on the repository state at the Phase 1 architecture
baseline. "Verified" means the corresponding boundary or responsibility
was identified in the current Rust implementation; it does not mean that
the implementation has been judged architecturally optimal or security
complete.
|---|---|
current checkpoint.

The target architecture must not be treated as an implementation claim.
Remaining gaps are to be resolved through the architecture verification
and migration process.
Mutation surface has been mapped.
Order, trade, balance, and snapshot mutation paths have been identified.
Authentication/authorization checks have been located.
Idempotency/tombstone mechanisms have been identified.
Snapshot application is transactional.
Existing tests cover a substantial portion of these invariants.
Distributed ordering/convergence remains under verification.
Failure consistency across order → trade → balance → persistence remains under verification.

### Persistence Verification

Repository inspection confirms that persistence is implemented, but it is
not centralized behind a persistence abstraction.

Direct filesystem access currently exists in:

- `balances.rs` — `BalanceBook::load()` / `save()`
- `orderbook.rs` — `OrderBook::load_orders()` / `save_orders()`
- `trades.rs` — `TradeBook::load_trades()` / `save_trades()`
- `main.rs` — node identity loading and creation

The state stores serialize directly with `serde_json` and write directly with
`std::fs`. Command and settlement paths also invoke `BalanceBook::save()`
directly.

Therefore:

| Finding | Status |
|---|---|
| Persistence exists | Verified |
| Persistent balances include order reservations | Verified |
| Persistence is centralized behind one abstraction | Not present |
| Filesystem access is isolated from domain state stores | Not present |
| Atomic/transactional persistence across order/trade/balance state | Not established |

This confirms the existing architectural assessment that a centralized
persistence abstraction is not currently present. The current implementation
should not be described as having a unified persistence layer.

|---|---|

17. Message Boundary → Domain Operation
The next architectural boundary is the separation between network message handling and exchange-domain operations.

The current message_handler performs several distinct responsibilities:

decode protocol messages
authenticate the claimed sender
authorise operations
invoke exchange behaviour
mutate order and trade state
trigger settlement
publish resulting messages
coordinate persistence
This is functional, but it gives the message handler more authority than its architectural role requires.

The redesign will progressively change the message handler from a domain orchestrator into a network-to-domain adapter.

The intended flow is:

                    NETWORK
                       │
                       ▼
              ┌─────────────────┐
              │ Message Handler │
              │                 │
              │ decode          │
              │ authenticate    │
              │ basic validate  │
              │ authorise       │
              └────────┬────────┘
                       │
                       ▼
              ┌─────────────────┐
              │ Domain Operation│
              │                 │
              │ explicit input  │
              │ explicit actor  │
              │ explicit intent │
              └────────┬────────┘
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
       Exchange     TradeBook    Settlement
          │            │            │
          └────────────┼────────────┘
                       ▼
                  Domain State

The message handler should not interpret a valid network message as permission to perform arbitrary state mutation.

Instead, it should establish the trusted facts required by the domain operation:

raw message
    ↓
decode
    ↓
authenticate sender
    ↓
validate message
    ↓
authorise sender for operation
    ↓
construct domain operation
    ↓
execute domain operation

17.1 Domain Operations
A domain operation represents an explicit request to change exchange state.

The operation should contain only the information required to perform the requested action.

Examples include:

SubmitOrder
CancelOrder
RecordTrade
AdvanceTradeState
ApplyStateSnapshot

These names describe intent rather than transport.

A domain operation must not require knowledge of:

libp2p
Gossipsub
peer connections
network events
message encoding
message propagation
This keeps the core exchange behaviour testable without a running network.

17.2 Remote Order Submission
The first migration target will be remote order submission.

The current conceptual path is:

Gossipsub
    ↓
handle_gossipsub_message
    ↓
decode OrderCreated
    ↓
authenticate sender
    ↓
authorise order owner
    ↓
handle_order_created
    ↓
Exchange::submit_order
    ↓
OrderBook / TradeBook
    ↓
publish TradeCreated

The target path is:

Gossipsub
    ↓
message_handler
    ↓
decode + authenticate + authorise
    ↓
SubmitOrder
    ↓
Exchange
    ↓
domain state

Publication of resulting protocol messages remains a transport/application concern and should not be embedded into the core exchange matching logic.

This boundary will be introduced without changing matching semantics.

17.3 Explicit Actor Context
Where an operation depends on the identity of the requester, that identity must be explicit.

For example:

SubmitOrder {
    actor
    order
}

rather than relying on the domain operation to infer identity from network state.

The message boundary establishes:

authenticated peer
        ↓
authenticated actor

The domain operation then performs any additional domain-level ownership or participation checks required by the operation.

This preserves the distinction between:

Authentication:
    Who sent the message?

Authorisation:
    Is that actor permitted to perform this operation?

Domain validation:
    Is the requested operation valid?

17.4 Authority Reduction
The migration must reduce the authority held by message_handler.rs.

In particular, the message handler should progressively stop directly coordinating:

order matching
trade lifecycle transitions
settlement
balance mutations
Those responsibilities should be reached through explicit domain operations.

The handler may continue to provide the necessary dependencies while the boundary is being migrated, but new domain behaviour should not be added directly to the handler.

17.5 Publication Boundary
Domain operations may produce results that require protocol publication.

The preferred model is:

Domain operation
      ↓
Domain result
      ↓
Application/network layer
      ↓
Protocol message
      ↓
Gossipsub

rather than:

Domain operation
      ↓
libp2p
      ↓
Gossipsub

This keeps domain logic independent from network transport.

For example, an order submission may produce:

SubmitOrder
    ↓
Exchange
    ↓
TradeCreated result(s)

The application layer can then convert those results into protocol messages and publish them.

17.6 Incremental Migration Rule
This boundary will be introduced incrementally.

Each migration step must:

preserve existing protocol behaviour;
preserve existing security checks;
preserve idempotency;
preserve persistence semantics;
maintain or improve test coverage;
leave the architecture document consistent with the implementation.
The migration should not introduce a large framework or generic command architecture unless the existing operations demonstrate a concrete need for one.

The simplest useful domain boundary is preferred.

17.7 Initial Migration Sequence
The planned sequence is:

1. Document Message Boundary → Domain Operation       ← current step
2. Introduce remote SubmitOrder operation
3. Move remote order processing behind Exchange
4. Separate TradeCreated processing
5. Separate TradeStateChanged processing
6. Separate settlement triggering
7. Separate state synchronisation
8. Reduce HandlerContext authority
9. Re-evaluate remaining message-handler responsibilities

Each step should be independently testable and should result in a reviewable commit.

17.8 Architectural Success Criteria
The boundary will be considered successful when:

network message parsing is clearly separated from domain intent;
domain operations can be tested without libp2p;
authentication occurs before domain execution;
authorisation is explicit;
message_handler.rs no longer owns core exchange behaviour;
settlement and balance mutations are reached through explicit domain operations;
protocol publication remains outside core domain logic;
the architecture document accurately describes the implemented system.
The goal is not to eliminate message_handler.rs.

The goal is to ensure that its responsibility is narrow and auditable:

receive
  ↓
decode
  ↓
authenticate
  ↓
authorise
  ↓
translate
  ↓
dispatch

rather than:

receive
  ↓
decode
  ↓
authenticate
  ↓
validate
  ↓
match
  ↓
create trade
  ↓
advance trade
  ↓
settle balances
  ↓
persist
  ↓
publish

This boundary therefore represents the next stage of reducing authority and making the exchange domain independently understandable and auditable.

### SubmitOrder Migration

The Message Boundary → Domain Operation migration has begun.

Order submission is represented by an explicit domain operation:

SubmitOrder {
    actor,
    order
}

Both local command submission and remote OrderCreated handling
construct the same domain operation rather than directly invoking
OrderBook matching semantics.

The message boundary remains responsible for:

- decoding
- sender authentication
- message-specific authorisation checks

The domain operation is responsible for enforcing actor/order ownership
before mutating exchange state.

converting the authenticated request into a domain operation
The Exchange domain operation is responsible for:

validating the submitter against the order owner
order submission semantics
matching
trade creation
order/trade state mutation
The domain operation currently enforces the invariant:

SubmitOrder.actor == SubmitOrder.order.trader

This provides a second, domain-level authority boundary rather than
relying exclusively on the network message handler to enforce order
ownership.

The migration does not yet imply that all financial or lifecycle
authority has been removed from handlers. Balance locking,
publication, persistence and other surrounding responsibilities remain
outside SubmitOrder and will be migrated deliberately in subsequent
architectural steps.

The current implementation has been verified with the existing test
suite, including security, order-book, trade and settlement tests.
No protocol behaviour was intentionally changed by this migration.
