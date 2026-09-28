OpenDex Architecture — v4
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

2.2 TradeBook
The TradeBook maintains the node's known trades.

Trade IDs are unique within the trade book. Duplicate trade insertion is ignored.

Trade state transitions are managed separately from order matching and execution.

2.3 BalanceBook
The BalanceBook manages balances and order reservations.

Order reservations are reconciled after execution so that the reservation reflects the remaining quantity of an order.

Buy-side reservations are calculated from the remaining order's required reservation, while sell-side reservations track the remaining asset amount.

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

For buy orders, the required reservation is recalculated from the remaining order.

For sell orders, the reservation tracks the remaining sell quantity.

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

Trade IDs are treated as unique execution identities across synchronization.

17. Persistence and Recovery
Persistent state includes the information required to preserve execution semantics across restart.

In particular, the OrderBook persists:

orders
applied_trades
pending_trades
removed_orders
fill_sequences

This is important because losing any of these independently can create inconsistent behavior.

For example:

losing applied_trades can permit replay

losing removed_orders can permit order resurrection

losing pending_trades can lose executions awaiting synchronization

losing fill_sequences can cause deterministic trade-ID reuse

losing orders can prevent pending trades from being reconciled

Therefore these structures are part of the durable state model, not merely caches.

18. Trade State
Trade execution and trade lifecycle state are separate concerns.

The execution record identifies:

what was traded

while TradeState identifies:

where that trade is in its lifecycle

Trade-state transitions are managed by the trade-state subsystem and reference trades by their deterministic trade ID.

This allows trade lifecycle processing such as acceptance, settlement, dispute, or cancellation without changing the identity of the underlying execution.

19. Settlement
Settlement uses the trade ID as its idempotency key.

A trade that has already been settled is not settled again.

The settlement subsystem therefore follows the same general principle as order application:

unique trade ID
        +
idempotent processing

This prevents duplicate network messages from causing duplicate settlement effects.

20. Network Message Handling
Incoming network messages must not be treated as inherently authoritative.

The message-processing path validates received objects before mutating local state.

For trades, the validation chain includes:

message received
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

Unknown trades may be retained or processed through the pending-trade mechanism when the required order state is not yet available.

Known trades are not applied repeatedly.

21. Consistency Model
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

22. Security and Integrity Invariants
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

Processing invariants
Applying the same trade twice has no additional effect.

Duplicate TradeBook insertion does not create another trade.

Settlement is idempotent by trade ID.

Pending trades remain recoverable across restart.

Fill sequences survive restart.

Tombstones survive restart.

23. Test Coverage
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

The tests should be considered executable verification of the architectural invariants described above.

The architecture document describes the invariants; individual test names and implementation details remain subject to change.

24. Architectural Data Flow
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

25. Architectural Principle
The central architectural principle is:

No replicated execution is trusted merely because it was received.

Instead, a trade is accepted only when its identity, structure, participants, market, price, quantity, and referenced order state satisfy the local protocol invariants.

At the same time, the architecture does not require all messages to arrive in a particular order. Pending trades, persistent state, tombstones, deterministic identities, and idempotent application allow nodes to recover and converge despite asynchronous message delivery.

26. Version 4 Changes
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

These are architectural behaviors rather than merely implementation-specific optimizations and should remain stable compatibility considerations for future versions.


