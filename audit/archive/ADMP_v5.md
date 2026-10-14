> **Archived 2026-10-08**: Superseded by ADMP v6 in `audit/Regulatory_Compliance/regulatory_compliance.md` Section 2.
> This file is preserved for historical reference only.

OpenDex Architecture — v5
1. Purpose
OpenDex is a peer-to-peer exchange system in which nodes maintain a replicated view of orders, trades, balances, and trade state.

The architecture is designed around:

deterministic trade identity

idempotent state application

authenticated actors

conservative snapshot synchronization

persistent order and trade state

recovery from partially observed distributed state

explicit separation between matching, trade recording, settlement, and state persistence

The implementation treats the order book and trade book as replicated state rather than relying on a single central matching authority.

2. Major Components
2.1 OrderBook
The OrderBook maintains:

active orders

applied trade IDs

pending trades

removed-order tombstones

per-resting-order fill sequences

Its responsibilities include:

inserting and updating orders

matching incoming orders

generating deterministic trade IDs

validating trade structure

validating trade identity

validating trades against current order state

applying trades idempotently

preventing stale synchronization data from resurrecting removed orders

retaining trades whose referenced orders are not yet locally available

identifying pending trades when relevant orders arrive (via upsert_order)

The OrderBook stores and identifies pending trades, but the Exchange layer coordinates the actual reconciliation once both referenced orders become available.

2.2 TradeBook
The TradeBook maintains the node's known trades.

Trade IDs are unique within the trade book. Duplicate trade insertion is ignored.

Trade state transitions are managed separately from order matching and execution.

2.3 BalanceBook
The BalanceBook manages balances, order reservations, settlement tracking, and escrow tracking.

It maintains:

balances (available, locked, and escrowed per asset)

order reservations

settled_trades (HashSet<Uuid>) for settlement idempotency

escrowed_trades (HashSet<Uuid>) for escrow funding idempotency

Order reservations are reconciled after execution so that the reservation reflects the remaining quantity of an order.

Buy-side reservations are calculated from the remaining order's required reservation using `quote_amount(price, amount)`, while sell-side reservations track the remaining base asset amount directly. Reconciliation releases the difference between the current and remaining required reservation.

The BalanceBook enforces balance invariants:

- Available, locked, and escrowed balances cannot go negative (uses `checked_sub`/`checked_add`).
- The balance invariant `available + locked + escrowed = total` holds after every operation.
- Order reservations are unique per order ID; double-reservation is prevented.
- Settlement requires sufficient locked balance.
- Escrow operations require sufficient locked or escrowed balance depending on the action.
- All arithmetic uses checked operations to prevent overflow.
- The `escrowed` field uses `#[serde(default)]` for backward compatibility with persisted state from before escrow was added.

2.4 Exchange
Exchange coordinates the OrderBook, TradeBook, and BalanceBook.

It is responsible for:

authenticating order submission actors

authenticating trade-recording actors

coordinating matching and trade creation

validating generated or received trades

recording trades

applying trades to the order book

reconciling balance reservations

completing pending-trade reconciliation

The Exchange layer therefore acts as the transactional coordination layer between order state, trade state, and balance state.

2.5 Persistence
Order, trade, and balance state are persisted so that node restarts do not discard execution history or synchronization safeguards.

Persisted state includes important anti-replay and anti-resurrection information, not merely currently active orders.

3. Order Lifecycle
An order can move through the following conceptual lifecycle:

submitted
    |
    v
validated
    |
    v
active
    |
    +------------------+
    |                  |
    v                  v
partially filled     fully filled
    |                  |
    v                  v
active             removed/tombstoned

An order's remaining amount can decrease through execution.

Snapshot synchronization is monotonic: a remote snapshot may reduce the locally known remaining amount, but it must not increase it.

Immutable order terms must remain consistent across updates.

These include:

trader

market

side

price

A conflicting snapshot is rejected rather than replacing the locally known order.

4. Order Tombstones
Fully removed orders are represented by persistent IDs in removed_orders.

A tombstone prevents a stale order update or snapshot from recreating an order that has already been consumed or removed.

Therefore:

active order
     |
     v
removed
     |
     v
tombstone retained
     |
     X
stale order update cannot resurrect it

Tombstones are part of replicated state and must survive persistence/recovery.

5. Order Matching
Matching is performed by OrderBook::match_order.

The matching process:

identifies compatible opposite-side orders

selects the best available resting order according to price/time-independent deterministic ordering

calculates the fill amount

uses the resting order's price as the execution price

determines maker and taker

determines buyer and seller

assigns a deterministic fill sequence

derives the deterministic trade ID

produces a Trade

Matching uses temporary remaining-quantity state.

The persistent order book is subsequently mutated through apply_trade() rather than directly during the matching calculation.

This separates:

matching decision

from:

state application

6. Deterministic Trade Identity
Every generated trade has a deterministic ID derived from:

buy_order_id
sell_order_id
execution_price
execution_amount
fill_sequence

The current implementation hashes the canonical string representation of these values with SHA-256 and constructs the UUID from the first 16 bytes.

Conceptually:

trade_id =
    SHA256(
        buy_order_id :
        sell_order_id :
        price :
        amount :
        fill_sequence
    )[0..16]

The exact serialization used by the implementation is part of the protocol invariant and must remain stable for interoperating nodes.

7. Fill Sequences
A trade ID cannot rely only on:

buy order
sell order
price
amount

because two legitimate executions can have identical values for all four fields.

For example:

fill #0:
BUY A + SELL B + price 100 + amount 4

fill #1:
BUY A + SELL B + price 100 + amount 4

These are separate executions and must have different trade IDs.

The implementation therefore maintains:

fill_sequences: HashMap<OrderId, u64>

The sequence is associated with the resting order.

When a new match is generated against a resting order:

the current sequence for that resting order is read

that sequence is assigned to the trade

the sequence is incremented

the sequence becomes part of deterministic trade identity

This guarantees distinct deterministic IDs for otherwise identical fills generated through successive executions against the same resting order.

The fill sequence is persisted as part of the OrderBook state.

8. Trade Validation
Trade validation consists of multiple layers.

8.1 Structural Validation
validate_trade_structure() verifies basic invariants, including:

amount is non-zero

price is non-zero

buy and sell orders are different

buyer and seller are non-empty

maker and taker are non-empty

maker is one of the trade participants

taker is one of the trade participants

market base and quote are different

Structural validation protects the node from malformed or obviously invalid trade objects.

8.2 Identity Validation
validate_trade_identity() recomputes the deterministic trade ID from:

buy_order
sell_order
price
amount
fill_sequence

and requires the supplied trade ID to match.

A node must therefore reject a trade whose ID does not correspond to its execution fields.

This prevents a peer from changing the trade identity independently of the execution data.

8.3 Order-Level Validation
Order-dependent validation verifies:

referenced buy order exists

referenced sell order exists

buy order has Side::Buy

sell order has Side::Sell

buyer matches the buy-order trader

seller matches the sell-order trader

both orders belong to the trade's market

execution price does not exceed the buy limit

execution price is not below the sell limit

execution amount does not exceed either order's remaining amount

These checks establish that the execution is compatible with the locally known order state.

9. Trade Application and Idempotency
Trades are applied through OrderBook::apply_trade().

Trade IDs provide the primary idempotency mechanism.

If a trade ID is already present in applied_trades, the trade is not applied again.

Conceptually:

incoming trade
      |
      v
already applied?
   /       \
 yes       no
 |          |
ignore    validate
            |
            v
         apply once

This protects against duplicate network delivery, repeated synchronization, and replay of an already-applied execution.

Applied trade IDs are persisted.

10. Pending Trades
A trade can arrive before the node has both referenced orders.

Such a trade cannot yet be fully applied because order-level validation requires the referenced orders.

Instead, the trade can be retained in:

pending_trades

Pending trades are persisted.

When a relevant order subsequently arrives, upsert_order() identifies pending trades involving that order and returns those for which both referenced orders are now available.

The Exchange layer then:

retrieves the referenced orders

applies the pending trade

reconciles balance reservations

removes the successfully processed pending trade

This allows the replicated state to converge even when network messages arrive in an order different from the order in which the original execution occurred.

11. Pending Trade Safety
Pending status does not bypass validation.

Before a pending trade is applied, the normal trade validation and application rules still apply.

In particular, a pending trade must still have:

valid structure

valid deterministic identity

valid order references

compatible traders

compatible market

compatible price

compatible remaining quantities

A pending trade therefore represents:

temporarily incomplete local knowledge

rather than:

trusted execution

12. Exchange-Level Authorization
Order submission requires:

operation.actor == order.trader

A node therefore cannot submit an order on behalf of another trader through the Exchange API.

Trade recording requires the actor to be one of:

trade.maker
trade.taker

A third party cannot directly record an arbitrary trade through the Exchange interface.

These checks are performed before the trade is accepted into the local trade-processing path.

13. Trade Recording
There are two principal execution paths.

Local Matching
For a locally submitted order:

SubmitOrder
    |
    v
authenticate actor
    |
    v
match order
    |
    v
generate Trade(s)
    |
    v
validate structure
    |
    v
validate deterministic identity
    |
    v
record in TradeBook
    |
    v
apply to OrderBook
    |
    v
reconcile reservations

Remote/Explicit Trade Recording
For an externally received or explicitly recorded trade:

RecordTrade
    |
    v
authenticate maker/taker actor
    |
    v
validate structure
    |
    v
validate deterministic identity
    |
    v
record in TradeBook
    |
    v
apply to OrderBook
    |
    v
reconcile reservations

If a trade already exists in the TradeBook, duplicate insertion is ignored.

14. Reservation Reconciliation
The Exchange captures the original order state before applying a trade because a fully filled order may be removed by apply_trade().

For each affected order, the remaining quantity is calculated as:

remaining = order.amount - trade.amount

using saturating subtraction.

For buy orders, the required reservation is recalculated from the remaining order using `quote_amount(price, amount)` — the quote asset cost of the remaining base asset quantity.

For sell orders, the reservation tracks the remaining base asset amount directly.

Reconciliation releases the difference between the current reservation and the remaining required reservation, ensuring that partially filled orders retain only the reservation needed for their remaining quantity.

This ensures that execution changes both:

order remaining amount

and:

reserved balance

as one coordinated operation.

15. Snapshot Synchronization
Remote snapshots are treated as potentially stale.

For an existing order:

remote amount > local amount

is rejected.

A snapshot therefore cannot increase the locally known remaining quantity.

For immutable order fields:

trader
market
side
price

a conflict is rejected.

For an order whose ID is already in removed_orders, the snapshot is ignored.

This establishes monotonic order state:

local remaining quantity
        |
        | may decrease
        v
smaller remaining quantity

but never:

smaller quantity
        |
        X
larger quantity from stale snapshot

Transactional Snapshot Application
Snapshots are applied using a clone-and-apply pattern to ensure atomic application. The snapshot is applied to cloned copies of both the OrderBook and TradeBook. Only if the application succeeds are the clones persisted and then committed by overwriting the live state. This prevents partial snapshot application from corrupting local state.

16. Trade Synchronization
Remote state responses are validated before being incorporated.

A snapshot must not contain:

duplicate trade IDs

invalid deterministic trade IDs

zero-amount trades

zero-price trades

trades inconsistent with their referenced orders

trades violating order price constraints

trades exceeding available order quantities

invalid applied-trade references

Additional validation rules enforced in code:

- Timestamps must be valid (created_at > 0, updated_at >= created_at).
- Applied trades must reference trades present in the snapshot or locally known.
- Removed orders in snapshots cannot delete locally active orders.
- Trade order references must be resolvable.

Trade IDs are treated as unique execution identities across synchronization.

17. Persistence and Recovery
Persistent state includes the information required to preserve execution semantics across restart.

In particular, the OrderBook persists:

orders
applied_trades
pending_trades
removed_orders
fill_sequences

The BalanceBook persists separately:

balances (available, locked, and escrowed per asset)

order_reservations

settled_trades

escrowed_trades

All persistence uses an atomic write pattern: data is written to a temporary file, the file is synced to disk, and then renamed to overwrite the target file. This prevents partial writes from corrupting state on crash.

This is important because losing any of these independently can create inconsistent behavior.

For example:

losing applied_trades can permit replay

losing removed_orders can permit order resurrection

losing pending_trades can lose executions awaiting synchronization

losing fill_sequences can cause deterministic trade-ID reuse

losing orders can prevent pending trades from being reconciled

losing settled_trades can permit duplicate settlement

losing balances can lose fund state

Therefore these structures are part of the durable state model, not merely caches.

18. Trade State
Trade execution and trade lifecycle state are separate concerns.

The execution record identifies:

what was traded

while TradeState identifies:

where that trade is in its lifecycle

Trade-state transitions are managed by the trade-state subsystem and reference trades by their deterministic trade ID.

This allows trade lifecycle processing such as acceptance, settlement, dispute, or cancellation without changing the identity of the underlying execution.

Trade State Machine Transition Matrix
The following transitions are valid:

- Created → Accepted, Cancelled, Expired
- Accepted → EscrowFunding, Cancelled
- EscrowFunding → EscrowFunded, Disputed, Cancelled
- EscrowFunded → PaymentPending, Disputed
- PaymentPending → PaymentSent, Disputed, Expired
- PaymentSent → PaymentConfirmed, Disputed
- PaymentConfirmed → PayoutPending, Disputed
- PayoutPending → Settled, Disputed
- Disputed → Settled, Cancelled

Happy-Path State Advancement
The trade state subsystem implements recovery for missed intermediate states. If a trade is in an earlier state and receives a state change for a later state, it will walk through the happy path applying only valid transitions. For example, if a trade in `Created` state receives `PaymentSent`, it will advance through: Created → Accepted → EscrowFunding → EscrowFunded → PaymentPending → PaymentSent.

Exceptional states (Cancelled, Expired, Disputed) cannot be inferred this way — they must be explicitly received.

19. Settlement
Settlement uses the trade ID as its idempotency key.

A trade that has already been settled is not settled again.

The settlement subsystem therefore follows the same general principle as order application:

unique trade ID
        +
idempotent processing

Settlement Plan-Then-Commit Pattern
Settlement uses a three-phase approach to prevent partial settlement:

1. Plan: `plan_local_settlement()` creates a `SettlementPlan` describing the balance mutations required.
2. Apply: The plan is applied to a cloned BalanceBook to verify correctness without risking partial mutation.
3. Commit: The cloned BalanceBook is persisted, and only then is the live state updated.

This ensures that if any part of settlement fails, the original balance state remains unchanged.

This prevents duplicate network messages from causing duplicate settlement effects.

20. Escrow

The escrow facility provides a three-state balance model for trade fund protection. Funds move from `locked` into `escrowed` when a trade reaches `EscrowFunded`, and are released or returned as the trade completes or is cancelled.

Three-State Balance Model
Each asset balance maintains three states:

- **Available** — Funds free for new orders or withdrawal.
- **Locked** — Funds reserved for orders or pending escrow.
- **Escrowed** — Funds held in escrow for an active trade.

The invariant `available + locked + escrowed = total` must hold after every balance mutation.

Escrow Operations
The BalanceBook provides three escrow operations:

- **`escrow_lock`** — Moves funds from `locked` to `escrowed`. Triggered when a trade reaches `EscrowFunded`. Requires sufficient locked balance.
- **`escrow_release`** — Moves funds from `escrowed` to `available`. Triggered when a trade reaches `Settled`.
- **`escrow_return`** — Moves funds from `escrowed` to `locked`. Triggered when a disputed trade is `Cancelled`.

All operations use checked arithmetic to prevent overflow and reject operations with insufficient balance.

Escrow Plan-Then-Commit Pattern
Following the settlement pattern, escrow uses a three-phase approach:

1. **Plan**: `plan_local_escrow()` creates an `EscrowPlan` describing the balance mutation.
2. **Apply**: The plan is applied to a cloned BalanceBook to verify correctness.
3. **Commit**: The cloned BalanceBook is persisted, then the live state is updated.

This ensures atomicity: if any part fails, the original state is unchanged.

Dual-Ledger Escrow
Each node mutates only its own BalanceBook. The seller's node moves its own locked funds to escrowed when the trade reaches `EscrowFunded`. The buyer's node does not move funds at that point — it only tracks the trade state. On settlement, both nodes independently release their respective balances.

Idempotent Escrow Funding
Escrow funding is idempotent: applying `EscrowFunded` twice does not double-spend. The BalanceBook maintains an `escrowed_trades` HashSet that records which trades have already been escrow-funded. Duplicate funding attempts are silently skipped.

State Machine Integration
Escrow operations are triggered automatically by trade state transitions in `advance_trade_and_settle()`:

- `EscrowFunded` → Calls `fund_local_escrow()` to move seller's locked base asset to escrowed.
- `Settled` (with prior escrow) → Calls `release_local_escrow()` before normal settlement.
- `Cancelled` (with prior escrow) → Calls `return_local_escrow()` to return funds to seller's locked balance.

Persistence Safety
The `escrowed` field uses `#[serde(default)]` so that persisted balance files from before escrow was added load correctly with `escrowed = 0`. The `escrowed_trades` HashSet also uses `#[serde(default)]` for the same reason.

21. GUI Subsystem

The node includes an optional desktop GUI built with the `iced` framework (v0.14), providing a visual dashboard for monitoring and controlling the P2P node.

Architecture
The GUI follows The Elm Architecture (TEA):

- **State** (`GuiModel`) — Holds all UI state: order book, trades, wallet balances, peer connections, and form inputs.
- **Message** (`GuiMessage`) — Enum representing all UI events: button clicks, input changes, state updates from the node.
- **View** (`gui_view` module) — Renders the UI tree from the current state.

Communication with the Node
The GUI communicates with the node via a channel-based bridge (`gui_bridge`):

- **Commands** (`GuiCommand`) — The GUI sends commands to the node (e.g., submit order, connect peer, advance trade state).
- **Events** (`GuiEvent`) — The node sends events to the GUI (e.g., order book updated, new trade, balance changed).

This separation ensures the GUI runs independently of the node's async runtime while staying synchronized.

View Modules
The GUI is composed of modular view widgets:

- **Order Book** — Displays bid/ask orders with price, amount, and order ID.
- **Trade Form** — Buy/sell order creation with market selector (BTC/XMR default, extensible for future blockchains), side selector, price, and amount inputs. Price and amount labels dynamically reflect the selected market's quote and base assets.
- **Trade History** — Scrollable list of trades with state labels.
- **Wallet** — Displays BTC/XMR balances showing available, locked, and escrowed amounts.
- **Peers** — Peer connection input and connected peer list.
- **Status Bar** — Node PeerId and status message display.

Launch
The GUI is optional and launched via the `--gui` flag:

```
cargo run -p p2p-node -- --gui --port <PORT>
```

Without the flag, the node runs in CLI-only mode.

22. Audit Logging

All node activity is logged via the `tracing` crate and persisted to per-node audit log files for post-incident review and compliance.

Log Configuration
At startup, `init_audit_logging()` creates a 3-layer subscriber:

- **`audit.log`** — Captures all log levels (debug through error) with file/line context.
- **`error.log`** — Warnings and errors only, for quick incident review.
- **`stderr`** — Info+ level for interactive CLI use.

Log Location
Logs are stored per-node at `~/.opendex/node-<port>/audit/`. Each node instance maintains its own log files, preventing log interleaving between nodes.

Log Coverage
Structured logging covers all critical subsystems:

- Escrow operations (fund, release, return) with balance state before and after.
- Settlement operations with plan details.
- Trade state transitions with trade ID and state changes.
- Order book operations (matching, application, reconciliation).
- Network message handling (receipt, validation, rejection).
- Snapshot synchronization (application, conflicts, rejections).
- CLI commands (fund, show balances, order submission).
- Persistence operations (save, load, recovery).

23. Network Message Handling
Incoming network messages must not be treated as inherently authoritative.

Authentication
Before any validation, every gossipsub message verifies the sender identity: the claimed sender must match the libp2p-signed PeerId. Unauthorized messages are rejected before any further processing.

The message-processing path validates received objects before mutating local state.

For trades, the validation chain includes:

message received
      |
      v
authenticate sender (PeerId match)
      |
      v
trade identity
      |
      v
trade structure
      |
      v
referenced orders
      |
      v
execution constraints
      |
      v
local application

Pending Trade State Cache
To prevent unbounded memory growth from state changes for trades that have not yet arrived, the message handler maintains a `pending_trade_states` cache bounded at 10,000 entries. When the cache is full, the oldest entries are evicted. This provides DoS protection against an attacker flooding state changes for non-existent trades.

Unknown trades may be retained or processed through the pending-trade mechanism when the required order state is not yet available.

Known trades are not applied repeatedly.

24. Consistency Model
OpenDex uses eventual state convergence with defensive local validation.

Nodes may temporarily disagree because:

messages can arrive in different orders

an order may arrive after its trade

snapshots can be stale

duplicate messages can be delivered

nodes can restart

The architecture addresses these conditions through:

deterministic trade IDs

fill sequences

idempotent trade application

persistent applied-trade IDs

pending trades

order tombstones

monotonic snapshot updates

deterministic validation rules

persistent execution metadata

The objective is that valid replicated state converges without allowing stale or malformed information to arbitrarily overwrite more advanced local state.

25. Security and Integrity Invariants
The following invariants are considered protocol-level requirements.

Order invariants
Order IDs uniquely identify orders.

An order's immutable terms cannot be changed by snapshots.

Remaining quantity must not increase through synchronization.

Removed orders cannot be resurrected.

Trade invariants
Trade amount must be greater than zero.

Trade price must be greater than zero.

Buy and sell orders must be distinct.

Buyer and seller must correspond to the referenced orders.

Maker and taker must be participants.

Trade market must correspond to both orders.

Trade price must satisfy both limit orders.

Trade amount must not exceed either order's remaining quantity.

Trade ID must deterministically correspond to the execution fields.

Separate fills must have distinct fill sequences and therefore distinct IDs.

Balance invariants
Available, locked, and escrowed balances cannot go negative.

The balance invariant `available + locked + escrowed = total` holds after every operation.

Order reservations are unique per order ID; double-reservation is prevented.

Settlement requires sufficient locked balance.

Escrow operations require sufficient locked or escrowed balance.

All balance arithmetic uses checked operations to prevent overflow.

Escrow funding is idempotent by trade ID.

Processing invariants
Applying the same trade twice has no additional effect.

Duplicate TradeBook insertion does not create another trade.

Settlement is idempotent by trade ID.

Pending trades remain recoverable across restart.

Fill sequences survive restart.

Tombstones survive restart.

26. Test Coverage
The implementation test suite verifies the architecture at multiple levels.

Tests cover, among other areas:

deterministic trade IDs

rejection of invalid trade IDs

identical-sized partial fills receiving distinct IDs

trade validation

duplicate trade handling

applied-trade persistence

pending-trade persistence and recovery

removed-order protection

snapshot validation

remote trade processing

actor authorization

reservation reconciliation

trade state transitions

settlement idempotency

synchronization consistency

escrow operations (lock, release, return)

escrow idempotency

balance invariant with escrowed state

The tests should be considered executable verification of the architectural invariants described above.

The architecture document describes the invariants; individual test names and implementation details remain subject to change.

27. Architectural Data Flow
The primary local execution path is:

                 +----------------+
                 | SubmitOrder    |
                 +-------+--------+
                         |
                         v
                 actor authorization
                         |
                         v
                 +---------------+
                 |   OrderBook   |
                 | match_order() |
                 +-------+-------+
                         |
                  Trade generation
                         |
                         v
              deterministic Trade ID
                         |
                         v
                 trade validation
                         |
                         v
                 +---------------+
                 |   TradeBook   |
                 +-------+-------+
                         |
                         v
                 +---------------+
                 |   OrderBook   |
                 | apply_trade() |
                 +-------+-------+
                         |
                         v
                 reservation
                 reconciliation
                         |
                         v
                   persisted state

Remote execution follows:

             network message
                    |
                    v
              message handler
                    |
                    v
             trade validation
                    |
             +------+------+
             |             |
       orders missing   orders present
             |             |
             v             v
      pending_trades    apply_trade()
             |             |
             +------+------+
                    |
                    v
             reservation
             reconciliation

28. Architectural Principle
The central architectural principle is:

No replicated execution is trusted merely because it was received.

Instead, a trade is accepted only when its identity, structure, participants, market, price, quantity, and referenced order state satisfy the local protocol invariants.

At the same time, the architecture does not require all messages to arrive in a particular order. Pending trades, persistent state, tombstones, deterministic identities, and idempotent application allow nodes to recover and converge despite asynchronous message delivery.

29. Version 5 Changes
Version 5 explicitly documents functionality implemented since Version 4:

- **Escrow Phase 1** — Three-state balance model (available, locked, escrowed) with three escrow operations (escrow_lock, escrow_release, escrow_return), plan-then-commit atomicity, idempotent escrow funding via escrowed_trades HashSet, and automatic triggering from trade state transitions.
- **GUI Subsystem** — Desktop GUI built with iced v0.14 following The Elm Architecture, with channel-based bridge for node communication, modular view widgets (order book, trade form, trade history, wallet, peers, status bar), market selector in the trade form (BTC/XMR default, extensible for future blockchains), and optional launch via --gui flag.
- **Audit Logging** — Persistent per-node audit logging via tracing with 3-layer subscriber (audit.log, error.log, stderr), covering all critical subsystems including escrow, settlement, trade state transitions, and network message handling.
- **Balance invariant extension** — The balance invariant now includes the escrowed state: `available + locked + escrowed = total`.
- **Persistence safety** — The `escrowed` field and `escrowed_trades` HashSet use `#[serde(default)]` for backward compatibility with pre-escrow persisted state.
- **ARCH-004 Resolution** — Trade identity validation (`validate_trade_identity()`) is now called from `OrderBook::apply_trade()` as a safety net, ensuring all three mutation paths (remote TradeCreated, OrderCreated matching, and pending trade reconciliation) verify deterministic trade IDs before mutating state. Two regression tests confirm tampered trade IDs are rejected without state mutation.
- **Fuzzing Infrastructure** — Initialized `cargo-fuzz` workspace under `fuzz/` with 5 fuzz targets: `message` (Message parser), `order` (Order JSON), `trade` (Trade JSON), `trade_state` (state machine transitions), and `state_response` (nested Vec parsing). All targets compile and pass smoke tests with zero crashes.
- **Upstream Framework Upgrade** — Upgraded `libp2p` 0.54.1 → 0.57.0 and `iced` 0.13 → 0.14.0, resolving all 6 transitive dependency vulnerabilities. Post-upgrade dependency audit shows 0 vulnerabilities and 3 low-risk warnings.
- **Integration & Supporting Module Tests** — Added 13 integration tests covering the full message handler pipeline, plus dedicated tests for sync (29), command_handler (48), and gui_bridge (28) modules. Total test count is 191.

Version 4 explicitly documents functionality that has become part of the implemented architecture since earlier versions:

deterministic trade IDs

fill_sequence as part of trade identity

distinct IDs for identical-sized partial fills

separate structural and identity validation

order-dependent trade validation

persisted pending trades

pending-trade reconciliation

persistent removed-order tombstones

monotonic snapshot synchronization

deterministic remote-trade validation

actor authorization at the Exchange layer

reservation reconciliation after execution

persistence of fill sequences

idempotent trade and settlement processing

broader automated security and consistency test coverage

Version 4 also documents implementation-critical patterns previously omitted:

trade state machine transition matrix

happy-path state advancement for missed intermediate states

balance invariants (non-negative balances, unique reservations, checked arithmetic)

settlement plan-then-commit pattern (clone-and-apply atomicity)

transactional snapshot application (clone-and-apply atomicity)

pending trade state cache with 10,000-entry DoS bound

sender authentication via libp2p-signed PeerId

BalanceBook persistence (balances, reservations, settled_trades)

snapshot validation rules (timestamps, applied-trade references, order references)

balance reconciliation details (buy-side quote_amount vs sell-side direct tracking)

BalanceBook settlement tracking via settled_trades HashSet

These are architectural behaviors rather than merely implementation-specific optimizations and should remain stable compatibility considerations for future versions.

30. Fuzzing Infrastructure

The project includes a fuzzing workspace under `fuzz/` managed by `cargo-fuzz` (LLVM libFuzzer). Fuzzing targets the receive-side deserialization boundary and the trade state machine, treating all network-delivered protocol messages as untrusted input.

Five fuzz targets are maintained:

- **message** — Fuzzes `serde_json::from_slice::<Message>`, exercising the outer message parser including all MessageType variants.
- **order** — Fuzzes `serde_json::from_slice::<Order>`, exercising Order JSON payload parsing.
- **trade** — Fuzzes `serde_json::from_slice::<Trade>`, exercising Trade JSON payload parsing.
- **trade_state** — Fuzzes `Trade::transition()` by mapping arbitrary bytes to TradeState enum variants, exercising the state machine's transition guards including invalid transitions and duplicate-state idempotency.
- **state_response** — Fuzzes `serde_json::from_slice::<StateResponse>`, exercising nested `Vec<Order>` and `Vec<Trade>` parsing.

All targets compile in release mode and are smoke-tested regularly. Crash artifacts are preserved under `fuzz/artifacts/` for reproduction and minimization.

Fuzzing process documentation is maintained in:

- `fuzz/FUZZ.md` — Comprehensive fuzzing process guide covering target design, crash triage, corpus management, and session logging.
- `fuzz/INFRASTRUCTURE.md` — Infrastructure setup reference covering prerequisites, workspace configuration, and build behavior.

31. Related Design Documents

The following design documents extend this architecture plan with implementation-specific details:

- **Escrow Design Specification** (`audit/ESCROW_DESIGN.md`) — Comprehensive design for the P2P escrow feature, including the three-state balance model (available, locked, escrowed), state machine integration, cross-node consistency via dual-ledger reconciliation, dispute resolution protocols, timeout mechanisms, atomic escrow operations, and a four-phase implementation plan. The escrow design leverages the architectural patterns documented in this plan: deterministic trade IDs, idempotent operations, plan-then-commit atomicity, and the trade state machine transition matrix.
- **Fuzzing Process Guide** (`fuzz/FUZZ.md`) — Fuzzing methodology, target specifications, crash triage workflow, and completion criteria for the receive-side deserialization boundary.
- **BTC-XMR Financial Transaction Specifications** (`audit/BTC_XMR_financial_transaction/`) — Seven specification documents defining the BTC-XMR atomic swap architecture, including cryptographic adapter design, technical specifications, crate architecture interface, sequence diagrams, fiat-enabled protocol, and product/protocol technical design plan.

32. BTC-XMR Atomic Swap Architecture — 2026-10-06

Milestones 1-3 completed. The BTC-XMR atomic swap system runs alongside the existing P2P fiat trading system using the Hybrid approach (Option C). The existing `p2p-node`, `protocol`, and `common` crates remain fully functional and unchanged.

### Architecture Overview

The BTC-XMR system introduces a new settlement layer that coexists with the existing fiat-based settlement. The key architectural boundary is the **SettlementProtocol trait**, which separates marketplace code from cryptographic settlement details.

```
Marketplace (orders, matching, quotes)
↓
SwapEngine (state machine, persistence, recovery)
↓
SettlementProtocol (cryptographic boundary)
↓
BtcXmrAdapter (DLEQ proofs, adaptor signatures)
↓
BitcoinChain / MoneroChain (blockchain interaction)
```

### Crate Structure (17 Total)

**Legacy crates (3):**
- `common` — Empty placeholder
- `protocol` — P2P fiat trading types (PaymentMethod, FiatCurrency, P2POffer, P2PTrade)
- `p2p-node` — GUI application with libp2p, fiat escrow, orderbook

**New BTC-XMR crates (14):**
- `opendex-types` — Domain types (SwapId, Satoshis, Piconero, Asset, Price)
- `opendex-errors` — Central error hierarchy
- `opendex-protocol-types` — SettlementProtocol trait, SwapState, ProtocolAction
- `opendex-protocol` — Protocol registry and version negotiation
- `opendex-btc-xmr` — Eigenwallet adapter skeleton
- `opendex-bitcoin` — BitcoinChain trait + MockBitcoinChain
- `opendex-monero` — MoneroChain trait + MockMoneroChain
- `opendex-swap` — SwapEngine with persistence, recovery, transport
- `opendex-events` — Event sourcing (append-only event log)
- `opendex-storage` — Repository traits with file-based implementation
- `opendex-watchers` — Blockchain watcher supervisor
- `opendex-recovery` — Deterministic recovery engine
- `opendex-transport` — Async Transport trait, ProtocolEnvelope, replay protection
- `opendex-chain` — Generic Chain trait for Bitcoin/Monero

### Key Architectural Patterns

1. **SettlementProtocol trait** — The core abstraction. Marketplace code interacts only with `ProtocolAction` and `ChainObservation`, never with cryptographic primitives.
2. **Event sourcing** — Every state change is an immutable event. State is reconstructable from the event log.
3. **Repository pattern** — SwapRepository, EventRepository, TransactionIntentRepository with file-based and in-memory implementations.
4. **Read-only watchers** — Blockchain observers report observations; the protocol state machine decides what they mean.
5. **Deterministic recovery** — Same inputs always produce the same RecoveryDecision.
6. **Transport abstraction** — Async Transport trait decouples swap engine from network implementation.
7. **Message replay protection** — MessageStore tracks seen message IDs per swap.

### Milestone Status

| Milestone | Scope | Status |
|---|---|---|
| M1: Core types & protocol | Types, errors, SettlementProtocol, swap state machine | ✅ Done |
| M2: Persistence & recovery | Event sourcing, file storage, watchers, recovery | ✅ Done |
| M3: P2P transport | Transport trait, message envelopes, replay protection | ✅ Done |
| M4: Order book & matching | Maker/taker quotes, order creation, matching | ❌ Not started |
| M5: Public API & GUI | REST/WebSocket API, web frontend | ❌ Not started |
| M6: Fiat subsystem | PaymentAdapter, bonds, disputes | ❌ Not started |

### Verification

- `cargo check --workspace` — 0 errors
- `cargo test --workspace --lib` — 119 tests passed, 0 failed
- Total new code: ~4,694 lines across 14 crates


