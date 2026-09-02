OpenDEX — Architecture Development Master Plan
“The old v2.0 can be retained in Git history...”
Version: 3.0
Date: 2 September 2026
Status: Active — Architectural Redesign & Simplification
Supersedes: Version 2.0

1. Mission

The objective is to evolve OpenDEX into a modular, comprehensible, auditable and security-conscious P2P exchange implementation.

The project must avoid reproducing the original monolithic architecture as functionality is migrated or redesigned.

The guiding principle remains:

Simple code, narrow responsibilities, explicit boundaries, minimal authority and predictable state transitions.

The purpose of the architecture work is therefore not simply code organisation.

The eventual system should be:

easier to understand
easier to test
easier to maintain
easier to audit
harder to misuse
less coupled
less exposed to unexpected interactions
easier to reason about from a security perspective

This document is the single architectural and development plan for OpenDEX.

2. Governing Architectural Principles

These principles govern all future development.

2.1 Simplicity

Prefer the simplest design that correctly expresses the required behaviour.

Additional abstraction, indirection or modules must have a clear purpose.

We will not create abstractions merely because they appear architecturally sophisticated.

2.2 Narrow Responsibilities

Each component should have one clear primary responsibility.

A component should not simultaneously become responsible for:

networking
message decoding
order matching
trade state
balances
settlement
persistence
peer management

unless there is a compelling architectural reason.

2.3 Minimal Authority

Every function and module should receive only the state and capabilities it actually requires.

A network message handler should not automatically have unrestricted authority over financial state.

A balance component should not need to understand libp2p.

A domain operation should not need to understand Gossipsub.

2.4 Explicit Boundaries

The boundaries between:

network
messages
validation
authorisation
domain logic
state
persistence
publication

must be explicit.

Raw network input must not flow directly into sensitive state mutation.

2.5 Predictable State Transitions

Security-sensitive state changes must be explicit and traceable.

An auditor should be able to follow:

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


without tracing through a large event handler.

2.6 Preserve Behaviour Unless Intentionally Changed

The redesign is permitted to replace the implementation.

It is not permitted to accidentally change protocol or business behaviour.

Where behaviour is intentionally changed to correct a bug or security weakness, the change must be documented and tested.

3. Why the Architecture Is Being Redesigned

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
trade creation
    ↓
trade state
    ↓
settlement
    ↓
consume_locked()
    ↓
balances


and:

order cancellation
    ↓
remove_order()
    ↓
unlock_for_order()
    ↓
balances


This creates several architectural concerns:

financial state mutations are distributed
order and trade state have overlapping responsibilities
settlement is triggered from message-processing paths
multiple event paths can reach related state transitions
persistence boundaries are not sufficiently centralised
handlers have more authority than their names suggest
correctness depends on interactions between modules
security auditing becomes unnecessarily difficult

The redesign therefore changes the objective from:

Refactor the existing architecture file by file

to:

Define a simpler state and authority model, then migrate the implementation toward it.

4. Architectural Direction

The target architecture is based on a strict separation between network concerns and domain concerns.

The conceptual architecture is:

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
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
          Orders          Trades        State Sync
             │              │              │
             └──────────────┼──────────────┘
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


This is a target architectural model rather than a requirement that every box become a separate Rust module.

The implementation should follow the responsibilities, not the diagram mechanically.

5. Trust Boundary

The most important security boundary is the transition from:

UNTRUSTED NETWORK INPUT


to:

TRUSTED INTERNAL DOMAIN OPERATION


The intended flow is:

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


Raw network structures must not directly control balances or settlement.

6. Domain Independence

Core exchange logic should be independent of libp2p wherever practical.

The preferred relationship is:

libp2p
   ↓
transport
   ↓
message boundary
   ↓
domain operation
   ↓
state


rather than:

domain logic
   ↔
libp2p
   ↔
Gossipsub
   ↔
Kademlia


This allows core exchange behaviour to be tested without requiring a running P2P network.

7. State Ownership

The redesign will explicitly define ownership of mutable state.

The following questions must have one clear answer:

Who creates orders?
Who removes orders?
Who matches orders?
Who creates trades?
Who changes trade state?
Who locks funds?
Who unlocks funds?
Who consumes locked funds?
Who credits balances?
Who performs settlement?
Who persists state?
Who publishes resulting protocol events?

There should not be multiple unrelated paths capable of performing the same sensitive mutation.

8. Order State

Orders represent trading intent.

Order-related responsibilities should include:

order validation
order ownership
order lifecycle
order matching
remaining quantity
removal/cancellation
order-book state

The order system should not directly contain network-specific behaviour.

The order system should not be responsible for interpreting arbitrary network messages.

9. Trade State

Trades represent the result of a successful match.

Trade responsibilities should include:

trade identity
participating orders
buyer
seller
market
price
amount
lifecycle state
valid state transitions
trade idempotency

Trade state transitions must be explicit.

A trade should not become settled merely because a particular message was received.

The domain must determine whether the transition is valid.

10. Balance Architecture

Balances represent financial state and therefore require a particularly narrow authority boundary.

A balance system should explicitly distinguish:

available funds
       │
       ▼
locked funds
       │
       ├── unlock
       │
       └── consume during settlement


Balance mutations must be explicit.

The following operations require particular scrutiny:

credit
debit
lock
unlock
consume locked funds
settlement transfers

The redesign must prevent unrelated network or order-book code from directly manipulating balances.

11. Settlement Architecture

Settlement is a financial state transition, not merely a consequence of receiving a network message.

The intended conceptual operation is:

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


Settlement must be:

deterministic
idempotent
explicitly authorised
validated before mutation
protected against arithmetic overflow
protected against insufficient funds
independent of network transport

The implementation must avoid partial financial transitions.

12. Atomicity Principle

A financial operation must not leave the balance system partially modified if a later part of the same operation fails.

The preferred model is:

validate everything
       ↓
perform complete transition
       ↓
persist result


rather than:

mutate
   ↓
try something else
   ↓
discover failure
   ↓
attempt recovery


Where persistence itself can fail after in-memory mutation, the architecture must explicitly determine the appropriate recovery or transactional strategy.

This issue must be addressed during the balance/settlement redesign rather than hidden inside handlers.

13. Idempotency

Distributed systems inevitably receive:

duplicate messages
delayed messages
reordered messages
repeated state notifications
duplicate trade announcements

Idempotency must therefore be a domain property wherever appropriate.

Trade settlement in particular must guarantee:

same trade
     ↓
first valid settlement → applied
     ↓
duplicate settlement → no second financial mutation


Idempotency should not depend solely on a particular network handler behaving correctly.

14. Message Architecture

Network messages should be treated as untrusted input.

The message layer should be responsible for:

decoding
message-size considerations
message-type validation
sender authentication
basic structural validation
routing

It should not become the primary owner of domain state.

The desired model is:

Message
   ↓
Validate
   ↓
Authenticate
   ↓
Create domain operation
   ↓
Domain

15. Authorisation

Authentication and authorisation are separate concerns.

Knowing who sent a message does not automatically mean that the sender is authorised to perform the requested operation.

For operations involving orders or trades, ownership and participation must be explicitly checked.

The system should be able to answer:

Who sent this?
        ↓
What operation are they requesting?
        ↓
Are they authorised to perform it?
        ↓
What state may they change?

16. Network Architecture

Networking should remain responsible for network concerns.

Existing functionality includes:

libp2p
peer identity
listening
direct dialing
Identify
Kademlia
Gossipsub
topic subscription
connection lifecycle
network event processing

These capabilities should remain operational while being progressively isolated from domain logic.

Networking should not directly implement:

settlement rules
balance rules
trade lifecycle rules
financial validation
core matching semantics
17. Message Router

The message router should become a thin boundary.

Conceptually:

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


The router should not become another monolith.

If routing logic begins accumulating domain behaviour, that behaviour must be moved into the appropriate domain component.

18. State Synchronisation

State synchronisation is a separate concern from ordinary domain operations.

State snapshots must be:

structurally validated
bounded
checked for consistency
applied through controlled state operations

A remote snapshot must never automatically grant authority to mutate protected financial state.

Particular attention will be given to:

stale snapshots
malicious snapshots
conflicting state
oversized state
repeated synchronisation
trade/order inconsistencies
19. Persistence

Persistence should have a clearly defined responsibility.

The system must distinguish between:

domain state


and:

storage representation


Persistence should not silently contain business logic.

The redesign will evaluate whether:

each state component should persist independently
a unified state store is simpler
operations require transactional persistence
crash recovery needs additional mechanisms
corrupted state can be detected and rejected

The simplest safe approach should be preferred.

20. Module Direction

The exact final module structure will be determined during implementation.

A possible target structure is:

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


This is illustrative only.

We will not create modules simply to match this diagram.

If two responsibilities are genuinely simpler together, they may remain together.

If a component becomes too authoritative or complex, it should be split.

The principle is responsibility, not file count.

21. API Narrowing

Public interfaces should be reduced wherever practical.

A function should receive:

the exact state it needs
the exact command it needs
the minimum capability required

Avoid passing broad collections of mutable state when a narrower interface is possible.

For example, prefer:

settle(trade, balances)


over a function that receives:

network
order_book
trade_book
balance_book
pending_states
swarm
topic


when most of those objects are irrelevant to settlement.

22. Error Handling

Errors should be explicit and predictable.

Preferred pattern:

validate
   ↓
reject invalid input early
   ↓
perform operation
   ↓
return result


Avoid unnecessarily deep nesting.

Errors must not be hidden by logging.

Logging should describe an error; it should not determine whether an operation succeeds.

23. Logging

Logging will be separated conceptually into:

operational events
debugging information
protocol events
errors
security-relevant events

Logging must never become part of the correctness mechanism.

The system must not rely on:

"we logged that settlement happened"


as evidence that settlement actually occurred.

State determines truth.

24. Legacy and Dead Code

Legacy implementations will be removed where they are no longer required.

Known examples requiring review include:

main.rs.save
main.rs.save.1


and previously identified unused functionality such as:

debit_available
transfer_locked


Legacy code must not remain simply because it might be useful someday.

Before removal, determine whether it is:

required
part of an incomplete migration
obsolete
duplicated
incorrectly disconnected

Once confirmed obsolete, it should be removed.

The final security review should not have to reason about abandoned implementations.

25. Migration Strategy

The redesign will proceed in controlled stages.

Phase 1 — Architecture Definition

Define:

state ownership
trust boundaries
domain operations
mutation authority
invariants
module responsibilities
persistence boundaries

No unnecessary implementation changes should be made before these are understood.

Phase 2 — Domain Core

Build or restructure the core exchange operations independently of libp2p.

Priority areas:

orders
trades
balances
settlement
state transitions

The domain core must be testable without a running network.

Phase 3 — State Mutation Consolidation

Reduce the number of places capable of changing sensitive state.

Particular attention:

orders
trades
balances
locked funds
settlement


Each important mutation should have an identifiable owner.

Phase 4 — Message Boundary Migration

Move network message handling toward:

decode
  ↓
validate
  ↓
authenticate
  ↓
authorise
  ↓
domain command


Network handlers should become progressively thinner.

Phase 5 — Network Integration

Reconnect the simplified domain core to:

libp2p
Identify
Kademlia
Gossipsub
peer discovery
message publication

The domain must remain independent of these mechanisms.

Phase 6 — Persistence Integration

Establish clear persistence boundaries.

Verify:

successful operations persist
failed operations do not create inconsistent state
restart behaviour is correct
duplicate operations remain safe
corrupted data is handled safely
Phase 7 — Legacy Removal

Remove:

obsolete handlers
duplicate state mutation paths
migration remnants
unused APIs
obsolete files

Only after the replacement behaviour has been verified.

26. Development Rules
Rule 1 — Preserve behaviour

Do not introduce unrelated protocol or business changes during architectural work.

Rule 2 — Small logical changes

Each change should have one clear purpose.

Rule 3 — Compile frequently

After meaningful changes:

cargo check

Rule 4 — Test frequently

Run:

cargo test

Rule 5 — Test domain logic independently

Financial and state-transition logic should have tests that do not require networking.

Rule 6 — Verify P2P after network changes

Whenever networking behaviour changes, perform multi-node verification.

Rule 7 — Do not abstract for abstraction's sake

More modules do not automatically mean better architecture.

Rule 8 — Prefer explicit code

Especially around:

validation
authorisation
state transitions
balances
settlement
peer identity
message handling
Rule 9 — Minimise authority

A function should receive only the capabilities it needs.

Rule 10 — One owner for sensitive mutation

Avoid multiple independent paths capable of changing the same protected state.

Rule 11 — Domain logic must be network-independent

Where practical, domain operations must be testable without libp2p.

Rule 12 — Security is part of implementation

Do not postpone security considerations until the final audit.

27. Testing Strategy

Every architectural change must maintain a regression baseline.

Build
cargo check

Unit tests
cargo test


Tests should cover:

order creation
order validation
order matching
order cancellation
trade creation
trade state transitions
balance credit
balance debit
locking
unlocking
locked-fund consumption
settlement
settlement idempotency
arithmetic overflow
insufficient balances
invalid state transitions
authorisation failures
P2P integration

At minimum:

Node A
  ↕
Node B
  ↕
Node C


Verify:

connectivity
Identify
Kademlia
Gossipsub
order propagation
trade propagation
state request
state response
duplicate handling
trade lifecycle
settlement
balances
restart/recovery where applicable
28. Security Review Areas

After the architectural migration, conduct a dedicated attack-surface review.

Network surface

Review:

listening interfaces
inbound connections
peer connections
connection lifecycle
discovery
Identify
Kademlia
Gossipsub
resource exhaustion
Message surface

Review:

deserialisation
malformed messages
oversized messages
unexpected message types
duplicate messages
replay
sender identity
authorisation
state synchronisation
State surface

Review:

order mutation
trade mutation
balance mutation
locked funds
settlement
persistence
state transitions
Resource surface

Review:

unbounded collections
pending messages
pending trades
pending state changes
repeated synchronisation
computationally expensive operations
connection exhaustion
29. Security Invariants

The following invariants should become explicit testable properties.

Financial invariants

Funds cannot be spent twice.

Locked funds cannot exceed the funds legitimately locked.

A settlement cannot create funds from nothing.

A settlement cannot be applied twice.

A failed operation must not produce a partial financial transition.

Ownership invariants

A peer cannot modify another peer's protected state without authorisation.

Trade participants must be authenticated.

State-changing messages must be associated with an authorised participant where required.

State invariants

Invalid trade transitions are rejected.

Duplicate operations are harmless where the protocol requires idempotency.

Remote messages cannot directly bypass domain validation.

Architectural invariants

Network code does not directly implement financial rules.

Domain logic does not depend unnecessarily on libp2p.

Sensitive state has clearly defined mutation owners.

30. Definition of Done

The redesign is not complete merely because:

cargo check


passes.

The architecture is considered complete when:

responsibilities are clear
state ownership is explicit
trust boundaries are clear
complex blocks have been reviewed
unnecessary nesting has been addressed
duplicated logic has been removed or justified
unnecessary authority has been reduced
dead code has been assessed
financial mutations have clear owners
settlement is explicit and idempotent
error paths are understandable
security-sensitive state changes are identifiable
domain logic can be tested independently
persistence behaviour is understood
P2P behaviour remains functional
tests pass
behaviour remains unchanged unless an intentional bug/security fix was made
obsolete architecture has been removed
31. Current Project Position

OpenDEX has already achieved substantial functional P2P migration.

Existing capabilities include:

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


The project now enters:

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

32. Immediate Development Plan

The immediate objective is not to continue patching message_handler.rs.

The immediate objective is to design the replacement state and authority model.

The first implementation questions are:

What state exists?
Who owns each state component?
What operations may mutate it?
What validation is required before mutation?
What authorisation is required?
What operations must be atomic?
What must be idempotent?
What must be persisted?
What events need to be published?
What code should have no knowledge of networking?

Only after these questions are answered should implementation migration begin.

33. Architectural Success Criteria

The final architecture should make the following flow easy to understand:

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


A security auditor should be able to trace a sensitive operation through this sequence without having to reconstruct behaviour from unrelated event handlers.

34. Core Objective

The most important objective of this version of the plan is:

Do not merely make the existing architecture smaller. Build the simplest architecture that gives each responsibility a clear owner and each sensitive state transition an explicit boundary.

The project should favour:

less code
less coupling
less authority
less implicit behaviour
less duplicated state mutation


while increasing:

clarity
testability
auditability
isolation
traceability
predictability


The redesign is successful when the architecture itself makes incorrect behaviour difficult to express.

35. Final Architectural Principle

OpenDEX should ultimately consist of a small number of clearly defined components with:

narrow responsibilities
explicit inputs and outputs
minimal authority
controlled state mutation
predictable transitions
clear trust boundaries
independently testable domain logic

The architecture must be designed so that security is not something added after development.

Security should emerge from the structure of the system itself.

This should replace the existing audit/Architecture_Development_Master_Plan.md rather than sit beside it. The old v2.0 can be retained in Git history, but v3.0 becomes the document we work from going forward.


Ah 😂 yes — the plan of action itself should be documented, but we don't need another heavyweight architectural document.

I'd add a short section to audit/Architecture_Redesign.md called "Implementation / Migration Plan". That way the redesign document contains both the target architecture and the controlled sequence for getting there.

I'd make the sequence explicit:

Freeze the current implementation

Establish cargo check / cargo test baseline.
Record current behaviour that must be preserved.
Do not modify the old architecture yet.

Map the domain

Orders
Order matching
Trades
Trade states
Balances
Locked funds
Settlement
Persistence

Define state ownership

Decide exactly which component owns each mutable state.
Define the only permitted mutation paths.
Eliminate broad &mut access wherever possible.

Define the new module boundaries

Domain logic independent of libp2p.
Network layer handles transport.
Message layer handles decoding/validation/routing.
Domain services perform operations.
Persistence has a defined boundary.

Redesign balances + settlement

This deserves special attention because the current implementation has coupling between orders, locks, trades and settlement.
Define the invariants first.
Then implement the new model.

Implement the domain layer

Build and test it without networking.
Make state transitions explicit.
Add focused unit tests.

Build the message boundary

Raw network message
Decode
Validate
Authorise
Convert to domain operation
Execute
Produce result/event

Put P2P on top

libp2p should transport messages rather than contain exchange logic.
Reconnect Gossipsub, Identify, Kademlia and synchronisation to the new boundaries.

Migrate persistence

Make loading/saving explicit.
Ensure persistence cannot accidentally become another state-mutation mechanism.

Integration testing

Single-node domain tests
Two-node P2P
Three-node P2P
Order propagation
Matching
Trade propagation
State synchronisation
Duplicate/replay handling
Settlement
Balance invariants

Remove the old architecture

Only after the replacement has demonstrated functional parity.
Remove .save legacy implementations and obsolete APIs.
Re-run the full regression suite.

Architecture/security review

Review the final dependency direction.
Review authority boundaries.
Review every state mutation.
Review network trust boundaries.
Then move toward the formal security audit.
One important addition

I'd explicitly put this rule into the migration plan:

Do not migrate code simply because it already exists. Reimplement behaviour where the existing implementation conflicts with the target architecture.

That's particularly important for the settlement/balance problems we're looking at. We don't want to spend the redesign carrying old assumptions into new modules.

So yes: document the plan, but keep it inside Architecture_Redesign.md. That gives us one document describing what we're building and how we're going to get there, while the Master Plan remains the higher-level set of principles we must obey.
