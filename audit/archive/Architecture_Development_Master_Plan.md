

OpenDEX—Refactoring_Architecture_Development_Master_Plan

Version: 2.0
Date: 26 August 2026
Current phase: Architectural simplification & security-oriented refactoring
Current file: crates/p2p-node/src/message_handler.rs

1. Mission

The objective is to evolve OpenDEX into a modular, comprehensible, auditable and security-conscious P2P exchange implementation.

The project should avoid reproducing the original monolithic architecture as functionality is migrated.

The guiding principle is:

Simple code, narrow responsibilities, explicit boundaries, minimal authority and predictable state transitions.

The purpose of the refactoring is therefore not simply code organisation.

We want to make the eventual system:

easier to understand
easier to test
easier to maintain
easier to audit
harder to misuse
less coupled
less exposed to unexpected interactions
easier to reason about from a security perspective
2. Primary Architectural Objectives
   2.1 Eliminate monolithic structures

No single file or function should become the "place where everything happens."

In particular, we want to avoid structures such as:

Network event
↓
decode message
↓
validate message
↓
modify order book
↓
modify balances
↓
settle trade
↓
update peer state
↓
publish response
↓
log everything


inside one handler.

Instead:

             Network
                │
                ▼
        Message Boundary
                │
                ▼
          Message Router
                │
        ┌───────┼────────┐
        ▼       ▼        ▼
      Orders  Trades   State Sync
        │       │        │
        └───────┼────────┘
                ▼
          Domain State
                │
        ┌───────┴────────┐
        ▼                ▼
     Balances        Settlement


Each layer should have a clearly defined responsibility.

3. Security Architecture Objective

Security should be considered during refactoring, not postponed until the final audit.

Our goal is to reduce the effective attack surface by reducing unnecessary complexity and authority.

We will aim for:

Complexity ↓

Coupling ↓

Mutable state access ↓

Implicit behaviour ↓

Duplicate validation ↓

Duplicate state transitions ↓

Unused code ↓

while increasing:

Auditability ↑

Testability ↑

Explicit validation ↑

Isolation ↑

Traceability ↑

4. Security Principle

The central principle for the architecture is:

Every component should have one clear responsibility and the minimum authority required to perform it.

For example, a network message handler should not automatically have unrestricted responsibility for balances, settlement, order matching and peer discovery.

Instead, it should delegate.

This creates security boundaries that can eventually be audited individually.

5. Migration Status
   Phase 1 — Core Architecture Migration

Status: 🟢 Functionally completed

The original networking implementation has been moved away from the monolithic implementation.

Completed
P2P node executable operational
libp2p networking operational
Peer identity generation
Listening
Direct peer dialing
Connection establishment
Identify
Kademlia
Gossipsub
OpenDEX topic subscription
OpenDEX message exchange
State requests
State responses
Trade synchronisation
Trade deduplication
Removed-order synchronisation
Multi-peer discovery
Networking separated into network.rs
CLI separated into cli.rs
Command processing separated into command_handler.rs
Message processing separated into message_handler.rs
Settlement separated into settlement.rs
Verified behaviour

The current node logs demonstrate successful:

Connection
↓
Identify
↓
Kademlia
↓
Gossipsub
↓
OpenDEX message
↓
State request
↓
State response
↓
Trade synchronisation
↓
Duplicate detection


Therefore the migration has achieved functional P2P parity for the tested paths.

6. Phase 2 — Complexity Audit

Status: 🟡 CURRENT PHASE

This is now the primary development activity.

We will inspect every source file systematically rather than allowing complexity to accumulate.

Audit categories

Every significant function/block will be classified as:

🟢 KEEP — already simple and appropriately located
🟡 SIMPLIFY — correct but unnecessarily complicated
🟠 EXTRACT — contains multiple responsibilities
🔴 REMOVE — dead, obsolete, duplicated or unnecessary
🔵 MOVE — valid logic but belongs in another module
7. Audit Order

We will proceed methodically:

message_handler.rs ← CURRENT
network.rs
command_handler.rs
settlement.rs
balances.rs
cli.rs
main.rs
Supporting/domain modules
Legacy files
Cross-module architecture review

We will not move to the next file simply because the previous one compiles.

The previous file must first pass its complexity and responsibility audit.

8. message_handler.rs — First Target

Status: 🟡 Pending detailed audit

This file is currently the most important target because it sits directly between the network and application layers.

We will examine:

Network event handling
SwarmEvent

Identify processing
Identify
↓
Peer information
↓
Address handling
↓
Kademlia

Gossipsub processing
Gossipsub
↓
OpenDEX message
↓
Decode
↓
Dispatch

State synchronisation
StateRequest
StateResponse

Orders
Order creation
Order updates
Order removal

Trades
Trade creation
Trade updates
Trade deduplication

Settlement
message_handler
↓
settlement
↓
balances


We will specifically check whether the handler has more authority than it requires.

9. Message Boundary Security

One of the most important architectural boundaries will be the transition from:

UNTRUSTED NETWORK INPUT


to:

TRUSTED INTERNAL DOMAIN STATE


The desired model is:

Network
│
▼
Raw Message
│
▼
Decode
│
▼
Validate
│
▼
Convert to Domain Command
│
▼
Domain Logic


We should avoid allowing raw network data to flow directly into balance or settlement operations.

This will become particularly important during the eventual security audit.

10. Domain Boundary

Core exchange logic should eventually be independent of libp2p wherever practical.

Ideally:

libp2p
│
▼
transport/message layer
│
▼
OpenDEX domain operation
│
▼
state


rather than:

domain logic
↔
libp2p
↔
Kademlia
↔
Gossipsub


This separation allows domain logic to be tested without needing an actual network.

11. State Mutation Principle

State-changing operations should be explicit.

We want to be able to identify exactly where:

orders are created
orders are removed
trades are created
trades are updated
balances change
locked funds change
settlement occurs

A security auditor should eventually be able to follow:

Incoming message
↓
Validation
↓
Authorisation / ownership check
↓
State transition
↓
Result


without tracing through a large event handler.

12. Error Handling Objective

We will simplify error paths wherever possible.

Avoid deeply nested structures such as:

if let Ok(...) = ...
if let Some(...) = ...
match ...
...


when a clearer early-return or helper function expresses the same logic.

The goal is:

validate
↓
reject early if invalid
↓
perform operation
↓
return result


This also reduces the number of paths that need security testing.

13. Logging Objective

Logging should help us understand the system without becoming part of its logic.

We will distinguish between:

operational logging
debugging
protocol events
errors
security-relevant events

Logging should not determine whether an operation succeeds.

14. Dead & Legacy Code

Current compiler warning:

debit_available
transfer_locked


are currently unused.

Status: 🟡 Review

We will determine whether they are:

required future functionality
incomplete migration remnants
obsolete
incorrectly disconnected from the current architecture

We will not blindly remove them.

Likewise:

main.rs.save
main.rs.save.1


will be reviewed before removal.

The objective is eventually to eliminate unnecessary executable/reference code from the project so that the security audit does not have to reason about obsolete implementations.

15. Refactoring Rules
    Rule 1 — Preserve behaviour

No unnecessary protocol or business-logic changes during the complexity pass.

Rule 2 — Small changes

One logical refactoring at a time.

Rule 3 — Compile frequently

After meaningful changes:

cargo check

Rule 4 — Test frequently
cargo test

Rule 5 — Verify P2P after network changes

Run the multi-node test whenever networking behaviour is touched.

Rule 6 — Don't create abstraction for abstraction's sake

More functions and more modules do not automatically mean better architecture.

Rule 7 — Prefer explicit code

Especially around:

validation
state transitions
balances
settlement
peer identity
message handling
Rule 8 — Minimise authority

A function should receive the state and capabilities it actually needs rather than an unnecessarily broad collection of mutable objects.

16. Phase 3 — Cross-Module Architecture Audit

Status: ⏳ Pending

After each individual file has been reviewed, we'll examine the architecture as a whole.

Questions include:

Are module responsibilities clear?
Are dependencies flowing in the correct direction?
Is networking leaking into domain logic?
Is domain logic leaking into networking?
Are there unnecessary shared mutable references?
Are there duplicated validation paths?
Are there multiple ways to mutate the same state?
Are there unnecessary public functions?
Can APIs between modules be narrowed?
Are there unnecessary dependencies?
Are there unnecessary trust relationships?
17. Phase 4 — Attack Surface Reduction

Status: ⏳ Pending architecture audit

Once the code is simplified, we perform a dedicated attack-surface review.

Areas will include:

Network surface
listening interfaces
peer connections
inbound messages
malformed messages
connection lifecycle
discovery
Gossipsub behaviour
Message surface
deserialisation
validation
message size
unexpected message types
replay considerations
duplicate messages
state-sync manipulation
State surface
order mutations
trade mutations
balance mutations
locked funds
settlement
Identity surface
PeerId handling
sender identity
ownership assumptions
peer announcements
Resource surface
memory growth
unbounded collections
repeated messages
repeated state synchronisation
connection exhaustion
computationally expensive operations
18. Phase 5 — Security Audit Preparation

Before a formal security review, we want to be able to produce a simple map:

                 ┌──────────────┐
                 │   Network    │
                 └──────┬───────┘
                        │
                 Untrusted Input
                        │
                        ▼
                 ┌──────────────┐
                 │   Messages   │
                 └──────┬───────┘
                        │
                     Validate
                        │
                        ▼
                 ┌──────────────┐
                 │    Domain    │
                 └──────┬───────┘
                        │
                Explicit State
                  Transitions
                        │
              ┌─────────┴─────────┐
              ▼                   ▼
         ┌─────────┐        ┌──────────┐
         │Balances │        │Settlement│
         └─────────┘        └──────────┘


The simpler this map becomes, the easier the final audit becomes.

19. Testing Strategy

Every architectural change should maintain a regression baseline.

Build
cargo check

Tests
cargo test

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
settlement
balances
20. Definition of Done

The refactoring phase is not complete merely because:

cargo check


passes.

A module is considered complete when:

responsibilities are clear
complex blocks have been reviewed
unnecessary nesting has been addressed
duplicated logic has been considered
unnecessary authority has been reduced
dead code has been assessed
error paths are understandable
security-sensitive state changes are identifiable
tests pass
behaviour remains unchanged unless an intentional bug fix was made
21. Current Project Position
    OpenDEX
    │
    ▼
    Architecture Migration
    │
    ▼
    ┌───────────────────┐
    │ FUNCTIONAL ✓      │
    │                   │
    │ P2P               │
    │ Identify          │
    │ Kademlia          │
    │ Gossipsub         │
    │ State Sync        │
    │ Trade Sync        │
    └─────────┬─────────┘
    │
    ▼
    ╔══════════════════════════╗
    ║ COMPLEXITY AUDIT         ║
    ║                          ║
    ║ message_handler.rs  ← NOW║
    ╚════════════╤═════════════╝
    │
    ▼
    network.rs
    │
    ▼
    command_handler.rs
    │
    ▼
    settlement.rs
    │
    ▼
    balances.rs
    │
    ▼
    main.rs
    │
    ▼
    Cross-module architecture
    │
    ▼
    Attack-surface reduction
    │
    ▼
    Security audit preparation
    │
    ▼
    Formal security review
    │
    ▼
    Further development

22. The Core Objective

The most important change in this version of the plan is that security is no longer a final-stage activity.

We're designing the architecture now so that the eventual security work is tractable.

Our target is:

A small number of clearly defined components, each with a narrow responsibility, explicit inputs and outputs, minimal authority, controlled state mutation, and easily traceable security boundaries.

That gives us a much better foundation than simply getting the exchange functionality working and then trying to untangle a second monolith later.

Immediate next action: message_handler.rs — complete structural/complexity audit before making changes.