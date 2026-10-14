Open Dex Technical Specification v0.1
This is the implementation-level companion to the architecture plan. It is deliberately written so a developer can turn it into tickets, database migrations, API contracts, and test cases.
Open Dex — Technical Specification v0.1
Status: Engineering baseline
Version: 0.1
Primary implementation target: BTC/XMR
Secondary architecture: Fiat ↔ BTC/XMR P2P marketplace
Design priority: Correct settlement before scale

1. Technical Goals
Open Dex v0.1 must establish five reliable foundations:
    1. A deterministic marketplace.
    2. A deterministic trade state machine.
    3. Non-custodial BTC/XMR settlement where technically possible.
    4. Exact fee and accounting calculations.
    5. An architecture capable of adding fiat markets without contaminating the crypto settlement layer.
The system should be built as a modular monolith first, with clean domain boundaries that can later become independent services.

2. Technology Baseline
Recommended initial stack:
Frontend:
    Next.js
    TypeScript

Backend:
    TypeScript
    Node.js

Database:
    PostgreSQL

Cache / ephemeral state:
    Redis

Blockchain:
    Bitcoin Core
    Monero node

Infrastructure:
    Docker
    Linux

Testing:
    Vitest/Jest
    Playwright
    integration test environment
For cryptographic primitives and security-sensitive settlement code, use mature audited libraries rather than implementing cryptography yourself.
A future Rust service can be introduced for settlement-critical components if testing demonstrates a need.

3. Repository Structure
open-dex/
│
├── apps/
│   ├── web/
│   └── api/
│
├── packages/
│   ├── domain/
│   ├── markets/
│   ├── orders/
│   ├── offers/
│   ├── trades/
│   ├── swaps/
│   ├── fees/
│   ├── ledger/
│   ├── wallets/
│   ├── risk/
│   ├── reputation/
│   ├── payments/
│   ├── disputes/
│   └── compliance/
│
├── infrastructure/
│   ├── bitcoin/
│   ├── monero/
│   ├── postgres/
│   ├── redis/
│   └── monitoring/
│
├── migrations/
│
├── tests/
│   ├── unit/
│   ├── integration/
│   ├── e2e/
│   └── adversarial/
│
└── docs/
    ├── architecture/
    ├── protocol/
    ├── api/
    ├── security/
    └── compliance/
The directory structure is less important than maintaining the domain boundaries.

4. Domain Model
The fundamental entities are:
User
Wallet
Market
CryptoOrder
FiatOffer
Trade
SwapSession
Payment
Bond
FeeSchedule
LedgerAccount
LedgerEntry
ReputationProfile
Dispute
RiskDecision
ComplianceCheck
BlockchainTransaction
AuditEvent

5. User
User
----
id
status
created_at
updated_at
Possible statuses:
ACTIVE
SUSPENDED
RESTRICTED
CLOSED
Do not put unnecessary KYC information directly into the public user record.

6. Wallet
Wallet
------
id
user_id
asset
address
address_type
status
created_at
Assets:
BTC
XMR
Potential future:
ETH
USDT
USDC
but these should not affect the BTC/XMR architecture.

7. Market
Market
------
id
base_asset
quote_asset
market_type
status
created_at
Examples:
BTC_XMR
GBP_BTC
GBP_XMR
Market types:
CRYPTO
FIAT
Statuses:
ACTIVE
PAUSED
DISABLED

8. Crypto Order
CryptoOrder
-----------
id
user_id
market_id
side
order_type
price
amount
remaining_amount
maker_fee_bps
status
created_at
expires_at
Side:
BUY
SELL
Order type initially:
LIMIT
Avoid implementing market orders in v0.1.
Limit orders make the settlement relationship easier to reason about.

9. Fiat Offer
Fiat offers should remain a separate entity.
FiatOffer
---------
id
user_id
market_id
currency
asset
side
price
min_amount
max_amount
payment_method_id
maker_fee_bps
taker_fee_bps
bond_requirement
settlement_timeout
status
created_at
expires_at
This is intentionally different from CryptoOrder.

10. Payment Method
PaymentMethod
-------------
id
name
currency
risk_class
reversible
status
Examples:
BANK_TRANSFER
CASH_DEPOSIT
CASH_IN_PERSON
Additional fields should be added only when necessary.

11. Trade
Trade is the central aggregate.
Trade
-----
id
market_id
maker_user_id
taker_user_id

maker_order_id
fiat_offer_id

base_asset
quote_asset

maker_amount
taker_amount

maker_price

maker_fee
taker_fee

network_cost
network_buffer

status

created_at
accepted_at
completed_at
A trade must preserve the exact economic terms that were agreed.
Do not recalculate historical trade prices from current market data.

12. Trade Status
Use a finite state machine.
CREATED
ACCEPTED
SETTLEMENT_PENDING
SETTLEMENT_ACTIVE
COMPLETED
CANCELLED
EXPIRED
FAILED
DISPUTED
REFUNDED
Not every trade can transition into every state.

13. Valid Trade Transitions
CREATED
  |
  +--> CANCELLED
  |
  +--> EXPIRED
  |
  +--> ACCEPTED
          |
          +--> CANCELLED
          |
          +--> SETTLEMENT_PENDING
                    |
                    +--> FAILED
                    |
                    +--> SETTLEMENT_ACTIVE
                              |
                              +--> COMPLETED
                              |
                              +--> DISPUTED
                              |
                              +--> REFUNDED
Transitions must be validated centrally.

14. State Machine Rule
Never allow code such as:
trade.status = "COMPLETED"
throughout the application.
Instead:
tradeStateMachine.transition(
    trade,
    TradeEvent.SETTLEMENT_CONFIRMED
)
The state machine verifies:
    • current state
    • event
    • permissions
    • required evidence
    • resulting state
This is one of the most important architectural rules.

15. Swap Session
A crypto trade creates a swap session.
SwapSession
-----------
id
trade_id

protocol_version

maker_asset
taker_asset

maker_amount
taker_amount

maker_hashlock
taker_hashlock

maker_timeout
taker_timeout

btc_txid
xmr_txid

secret_hash
secret_revealed_at

status

created_at
updated_at
The exact fields will depend on the selected BTC/XMR atomic-swap implementation.
Do not invent a custom cryptographic protocol for production.
Use a well-reviewed protocol/library and document its assumptions.

16. Atomic Swap Lifecycle
Conceptually:
Maker
  |
  | Create quote
  v
Open Dex
  |
  | Taker accepts
  v
Swap session created
  |
  v
Cryptographic conditions established
  |
  v
BTC/XMR lock transactions
  |
  v
Blockchain monitoring
  |
  v
Secret revelation / settlement
  |
  v
Finality checks
  |
  v
Trade completed
The application database coordinates the process.
The blockchains provide the settlement evidence.

17. Blockchain Event Model
Never make the API directly infer settlement from a single RPC call.
Create normalized blockchain events.
BlockchainEvent
---------------
id
chain
transaction_id
block_height
event_type
asset
amount
address
confirmations
observed_at
Events might include:
TRANSACTION_SEEN
TRANSACTION_CONFIRMED
TRANSACTION_FINAL
TRANSACTION_FAILED

18. Blockchain Transaction
BlockchainTransaction
---------------------
id
chain
txid
trade_id
wallet_id
direction
asset
amount
fee
block_height
confirmation_count
status
first_seen_at
confirmed_at
Status:
SEEN
CONFIRMING
CONFIRMED
FINAL
FAILED
REORGED

19. Confirmation Policy
Create a configurable policy:
ConfirmationPolicy
------------------
chain
minimum_confirmations
finality_policy
max_reorg_depth
Do not hard-code confirmation assumptions into the swap engine.

20. Fee Model
Store fees as basis points.
FeeSchedule
-----------
id
market_id
maker_fee_bps
taker_fee_bps
network_buffer_bps
effective_from
effective_until
version
Initial crypto configuration:
BTC/XMR

Maker: 10 bps
Taker: 30 bps
Network buffer: 1500 bps
The 1500 bps value here represents the proposed 15% network-cost buffer and should be applied to the estimated network cost, not the trade notional.
That distinction is critical.

21. Fee Calculation
Protocol fee:
protocol_fee =
    notional × fee_bps / 10,000
Network allocation:
network_allocation =
    estimated_network_cost × 1.15
The network buffer must not accidentally become:
trade_value × 15%
It applies to the network-cost estimate.

22. Currency Representation
Never store:
0.1 BTC
as a floating-point database value.
Use integer atomic units.
Examples:
BTC:
satoshis

XMR:
atomic units

GBP:
pence

EUR:
cents

USD:
cents
Example:
100,000,000 satoshis = 1 BTC
The exact decimal conversion belongs in the presentation layer.

23. Money Type
Create a shared domain type:
type Money = {
  asset: Asset;
  atomicAmount: bigint;
};
Never use JavaScript number for settlement calculations.
Use:
bigint
or a properly tested arbitrary-precision decimal implementation where decimal calculations are unavoidable.

24. Price Representation
Represent prices as rational quantities rather than floating-point numbers.
Conceptually:
Price
-----
base_amount
quote_amount
For example:
1 BTC = 100 XMR
becomes:
base_amount  = 1 BTC
quote_amount = 100 XMR
This avoids floating-point rounding errors.

25. Matching Engine
The crypto marketplace can use a conventional price/time-priority model.
For a BUY:
highest price first
then earliest order
For a SELL:
lowest price first
then earliest order
However, do not allow the matching engine to assume that a matched order is automatically settled.
Matching produces:
MATCH_FOUND
which creates a trade.
Settlement is a separate process.

26. Matching Transaction
The following should occur atomically at the database level:
1. Lock maker order.
2. Verify remaining amount.
3. Verify order status.
4. Reserve matching amount.
5. Create trade.
6. Update remaining order amount.
7. Emit MatchCreated event.
8. Commit transaction.
This prevents two takers from consuming the same liquidity.

27. Idempotency
All financial POST endpoints require idempotency.
Example:
POST /v1/trades
Idempotency-Key: 01HXYZ...
If the same request is submitted twice:
first request:
trade created

second request:
existing trade returned
Never create two financial operations because of a network retry.

28. API Authentication
Use short-lived access tokens plus refresh/session controls.
Sensitive operations should require stronger authentication.
Examples:
create offer
cancel order
accept trade
change withdrawal address
initiate settlement
For wallet-sensitive actions, use signed authorization where appropriate.

29. REST API
Initial endpoints:
GET    /v1/markets
GET    /v1/markets/:id

GET    /v1/orders
POST   /v1/orders
GET    /v1/orders/:id
DELETE /v1/orders/:id

GET    /v1/offers
POST   /v1/offers
GET    /v1/offers/:id
PATCH  /v1/offers/:id
DELETE /v1/offers/:id

POST   /v1/trades
GET    /v1/trades/:id
POST   /v1/trades/:id/accept
POST   /v1/trades/:id/cancel

GET    /v1/swaps/:id
GET    /v1/swaps/:id/events

GET    /v1/fees
GET    /v1/reputation/:userId

30. WebSocket API
Use WebSockets for market information.
Channels:
market.BTC_XMR.orderbook
market.BTC_XMR.trades
market.GBP_BTC.offers
trade.{tradeId}
Private trade channels should require authentication.

31. API Response Principle
Financial responses should expose the calculation.
Example:
{
  "tradeId": "...",
  "price": {
    "baseAmount": "100000000",
    "quoteAmount": "1000000000000"
  },
  "fees": {
    "maker": "...",
    "taker": "...",
    "network": "...",
    "networkBuffer": "..."
  },
  "total": "...",
  "currency": "XMR"
}
The user should be able to independently understand how the final amount was produced.

32. Ledger
The ledger is a separate subsystem.
Core tables:
ledger_accounts
ledger_journals
ledger_entries
Example accounts:
USER_ASSET
PROTOCOL_REVENUE
NETWORK_RESERVE
BOND_LIABILITY
DISPUTE_RESERVE

33. Ledger Rule
Every financial operation creates a journal.
A journal must balance.
sum(debits) = sum(credits)
No exceptions.
If the application detects an imbalance:
TRADING MUST PAUSE
until the inconsistency is resolved.

34. Trade Accounting Example
Suppose:
Trade:
£1,000

Protocol fee:
£5

Network allocation:
£2
The ledger should represent the economic movements explicitly.
Do not simply write:
trade.revenue = 5
The ledger must show where the £5 came from and where it went.

35. Fiat Bond Model
For a fiat trade:
Trade
 |
 +-- Maker bond
 |
 +-- Taker bond
Bond:
Bond
----
id
trade_id
user_id
asset
amount
status
locked_at
released_at
slashed_at
Statuses:
REQUIRED
LOCKED
RELEASED
PARTIALLY_SLASHED
FULLY_SLASHED
FROZEN

36. Bond State Machine
REQUIRED
   ↓
LOCKED
   |
   +--> RELEASED
   |
   +--> FROZEN
          |
          +--> RELEASED
          |
          +--> PARTIALLY_SLASHED
          |
          +--> FULLY_SLASHED
No bond can return to LOCKED after RELEASED.

37. Fiat Payment Model
Payment
-------
id
trade_id
payment_method_id
payer_id
payee_id
currency
amount
reference
status
created_at
sent_at
confirmed_at
Statuses:
PENDING
INSTRUCTIONS_ISSUED
MARKED_SENT
CONFIRMATION_PENDING
CONFIRMED
FAILED
DISPUTED
REVERSED
A payment marked SENT is not necessarily a payment that has settled.

38. Payment Evidence
PaymentEvidence
---------------
id
payment_id
submitted_by
type
reference
created_at
Evidence should be minimized and access-controlled.
Do not create an unnecessarily large repository of sensitive financial documents.

39. Fiat Trade State Machine
Initial design:
OFFER_CREATED
      ↓
ACCEPTED
      ↓
BONDS_LOCKED
      ↓
CRYPTO_SETTLEMENT_READY
      ↓
PAYMENT_PENDING
      ↓
PAYMENT_MARKED_SENT
      ↓
PAYMENT_CONFIRMED
      ↓
SETTLEMENT_COMPLETE
      ↓
BONDS_RELEASED
Failure paths:
CANCELLED
EXPIRED
DISPUTED
REFUNDED
The exact crypto-release ordering depends on the payment mechanism and custody/escrow architecture.

40. Important Fiat Architecture Constraint
A bank transfer cannot be made genuinely atomic with Bitcoin or Monero merely by putting both events in the same database transaction.
Therefore:
database transaction
≠
fiat settlement
≠
blockchain settlement
The protocol must explicitly manage the gap between those systems.
This is why the fiat side requires a separate risk/dispute architecture.

41. Risk Engine Interface
interface RiskEngine {
  evaluateTrade(input: TradeRiskInput): Promise<RiskDecision>;
}
Result:
type RiskDecision =
  | { action: "ALLOW" }
  | { action: "ALLOW_WITH_LIMIT"; limit: bigint }
  | { action: "REVIEW"; reasonCodes: string[] }
  | { action: "BLOCK"; reasonCodes: string[] };
Risk decisions should be persisted.

42. Risk Inputs
Potential inputs:
user age
account history
trade size
payment method
market
currency
jurisdiction
completion rate
dispute history
velocity
bond size
Avoid opaque risk decisions where possible.
Store reason codes.
Example:
HIGH_VALUE
NEW_ACCOUNT
HIGH_RISK_PAYMENT_METHOD
LIMIT_EXCEEDED

43. Reputation
ReputationProfile
-----------------
user_id
completed_trades
failed_trades
disputes
disputes_lost
volume
average_settlement_time
completion_rate
created_at
Reputation should be derived from immutable trade events where possible.
Do not let users manually edit their reputation.

44. Reputation Event
ReputationEvent
---------------
id
user_id
trade_id
event_type
value
created_at
Examples:
TRADE_COMPLETED
TRADE_CANCELLED
DISPUTE_LOST
DISPUTE_WON
LATE_SETTLEMENT

45. Dispute Model
Dispute
-------
id
trade_id
opened_by
reason
status
resolution
created_at
resolved_at
Statuses:
OPEN
EVIDENCE_PENDING
UNDER_REVIEW
RESOLVED
ESCALATED

46. Audit Log
Every security-sensitive action creates an immutable audit event.
AuditEvent
----------
id
actor_id
action
entity_type
entity_id
metadata
ip_hash
created_at
Examples:
LOGIN
ORDER_CREATED
ORDER_CANCELLED
TRADE_ACCEPTED
WALLET_CHANGED
DISPUTE_OPENED
ADMIN_ACTION
Do not store unnecessary raw personal data in audit logs.

47. Event Outbox
Use the transactional outbox pattern.
When a database transaction changes financial state:
BEGIN
    update trade
    insert ledger entry
    insert audit event
    insert outbox event
COMMIT
A worker then publishes:
TradeCompleted
to subscribers.
This prevents the database and event system from getting out of sync.

48. Event Types
Initial event catalogue:
UserCreated
WalletAdded

OrderCreated
OrderCancelled
OrderMatched

OfferCreated
OfferCancelled
OfferAccepted

TradeCreated
TradeAccepted
TradeCancelled
TradeExpired

SwapCreated
SwapFundingDetected
SwapConfirmed
SwapCompleted
SwapFailed

PaymentCreated
PaymentMarkedSent
PaymentConfirmed
PaymentReversed

BondLocked
BondReleased
BondFrozen
BondSlashed

DisputeOpened
DisputeResolved

LedgerJournalCreated
RiskDecisionCreated
ComplianceCheckCompleted

49. Concurrency Control
Financial state must use database-level locking where necessary.
For example:
SELECT *
FROM crypto_orders
WHERE id = $1
FOR UPDATE;
This prevents simultaneous requests from consuming the same liquidity.
Optimistic versioning can also be used:
version = 7
Update only if:
version = 7
then increment to:
version = 8

50. Order Reservation
When an order is matched:
available amount
must be separated from:
reserved amount
Example:
Order:
1 BTC total

Available:
0.6 BTC

Reserved:
0.4 BTC
The maker must not be able to cancel or modify the reserved portion.

51. Quote Expiration
Every executable quote should have an expiration.
Example:
quote expires in 30 seconds
or for maker offers:
offer expires in 30 minutes
This prevents stale pricing.
The exact duration is market-dependent.

52. Price Protection
Add limits against obviously erroneous prices.
Example:
reference price:
£75,000

maker price:
£750,000
The system should flag the quote rather than silently publish it.
Possible result:
PRICE_OUTSIDE_ALLOWED_RANGE
This is not price control; it is an error/abuse safeguard.

53. Oracle / Reference Price
The reference price is informational for fiat pricing and risk checks.
It should not automatically determine maker price.
Architecture:
ReferencePriceProvider
        |
        +-- Provider A
        +-- Provider B
        +-- Provider C
Use multiple sources where appropriate.
Store:
price
timestamp
source
Do not allow a stale reference price to silently remain authoritative.

54. Fiat Offer Example
{
  "market": "GBP_BTC",
  "side": "SELL",
  "minAmount": "50000",
  "maxAmount": "500000",
  "price": {
    "currency": "GBP",
    "baseAtomicAmount": "100000000",
    "quoteAtomicAmount": "7600000"
  },
  "paymentMethod": "BANK_TRANSFER",
  "makerFeeBps": 25,
  "takerFeeBps": 50,
  "bondAmount": "20000"
}
Here:
50,000 pence = £500
500,000 pence = £5,000
The API should always document units explicitly.

55. Authentication vs Wallet Ownership
A platform account does not automatically prove control of a wallet.
Where wallet ownership matters, support signed messages or equivalent cryptographic proof.
Conceptually:
User
 |
 | signs challenge
 v
Wallet
 |
 v
Open Dex verifies signature
Never ask users to give Open Dex private keys.

56. Private Key Architecture
Preferred model:
Open Dex
    |
    X
    |
Private key
The platform should not require users to provide private keys.
If the platform itself ever controls operational keys, isolate them in dedicated secure infrastructure.
Production signing keys should not live in:
source code
.env files
developer laptops
plain database fields

57. Admin Architecture
Administrative actions must be separate from ordinary user APIs.
Admin API
    |
    +-- pause market
    +-- freeze account
    +-- modify configuration
    +-- resolve dispute
    +-- emergency actions
Every admin action must produce:
AuditEvent
and require strong authentication.

58. Configuration Versioning
Every trade records:
fee_schedule_version
risk_policy_version
settlement_policy_version
This makes historical behavior reproducible.
Example:
Trade #123

Fee version: 4
Risk version: 7
Settlement version: 3

59. API Versioning
Use:
/v1/
from day one.
Never silently change financial semantics under an existing API version.
Breaking changes require:
/v2/

60. Error Model
Standardize errors.
Example:
{
  "error": {
    "code": "ORDER_ALREADY_FILLED",
    "message": "The requested liquidity is no longer available.",
    "requestId": "..."
  }
}
Financial errors should be machine-readable.

61. Important Error Codes
INSUFFICIENT_LIQUIDITY
ORDER_EXPIRED
ORDER_ALREADY_FILLED
TRADE_ALREADY_ACCEPTED
TRADE_NOT_CANCELLABLE
INVALID_STATE_TRANSITION
FEE_CHANGED
QUOTE_EXPIRED
PAYMENT_TIMEOUT
SWAP_TIMEOUT
CONFIRMATION_REQUIRED
RISK_REVIEW_REQUIRED
COMPLIANCE_RESTRICTION
MARKET_PAUSED

62. Testnet Environment
Create a completely isolated environment:
open-dex-dev
open-dex-staging
open-dex-production
Each environment has independent:
database
wallets
blockchain nodes
API credentials
signing keys
Production credentials must never be usable against staging.

63. Integration Testing
A complete BTC/XMR test should simulate:
Maker creates order
       ↓
Taker accepts
       ↓
Trade created
       ↓
Swap initialized
       ↓
Funding transaction detected
       ↓
Required confirmations
       ↓
Settlement
       ↓
Trade completed
       ↓
Ledger balanced
       ↓
Reputation updated
Then test failure at every stage.

64. Failure Testing
Inject:
Bitcoin node unavailable
Monero node unavailable
database unavailable
Redis unavailable
websocket disconnect
transaction delayed
transaction reorg
maker disappears
taker disappears
quote expires
duplicate API request
duplicate blockchain event
The system must recover without double-settling.

65. Security Testing
Before mainnet:
API penetration test
authentication review
authorization review
wallet security review
state-machine review
database review
cryptographic review
economic attack review
The economic attack review is particularly important.
Ask:
"How can someone make money by causing the protocol to behave incorrectly?"

66. Economic Attack Scenarios
Test:
wash trading
Sybil reputation farming
quote manipulation
fee manipulation
bond griefing
order sniping
settlement griefing
dispute abuse
payment reversal
fake payment evidence

67. Monitoring
Core metrics:
active_orders
active_offers
trades_per_minute
trade_volume
swap_success_rate
swap_failure_rate
average_settlement_time
payment_timeout_rate
dispute_rate
bond_locked
bond_slashed
protocol_revenue
network_cost_variance
Alerts should trigger on abnormal changes.

68. Financial Reconciliation
Run automated reconciliation jobs.
Daily:
blockchain balances
+
ledger balances
+
trade records
must reconcile according to the custody model.
For every discrepancy:
RECONCILIATION_FAILURE
should be generated.

69. Fiat Reconciliation
Where fiat infrastructure is integrated:
bank/payment-provider records
        ↕
Open Dex payment records
        ↕
Open Dex ledger
These should reconcile independently.
Never assume that:
payment provider says complete
automatically means:
trade state = complete
The settlement rules determine when the trade transitions.

70. MVP Cut Line
Do not put these into v0.1:
[ ] Multiple fiat currencies
[ ] Multiple payment providers
[ ] Mobile apps
[ ] Advanced derivatives
[ ] Lending
[ ] Staking
[ ] Automated market making
[ ] Token launch
[ ] Governance token
[ ] Cross-chain routing
The initial product should prove:
BTC/XMR
+
maker pricing
+
non-custodial settlement
+
correct fees
+
correct accounting

71. Development Ticket Breakdown
Epic 1 — Foundation
DEX-001 Repository
DEX-002 CI/CD
DEX-003 PostgreSQL
DEX-004 Database migrations
DEX-005 Configuration
DEX-006 Logging
DEX-007 Error framework
Epic 2 — Identity
DEX-010 User model
DEX-011 Authentication
DEX-012 Sessions
DEX-013 Wallet registration
DEX-014 Signed wallet verification
Epic 3 — Markets
DEX-020 Market model
DEX-021 Crypto orders
DEX-022 Order validation
DEX-023 Matching engine
DEX-024 Order cancellation
DEX-025 Order websocket
Epic 4 — Trades
DEX-030 Trade model
DEX-031 Trade state machine
DEX-032 Idempotency
DEX-033 Trade API
DEX-034 Trade events
Epic 5 — BTC/XMR
DEX-040 Bitcoin node adapter
DEX-041 Monero node adapter
DEX-042 Blockchain event listener
DEX-043 Confirmation engine
DEX-044 Swap session
DEX-045 Atomic swap integration
DEX-046 Swap timeout handling
DEX-047 Refund/failure handling
Epic 6 — Fees
DEX-050 Fee schedule
DEX-051 Fee calculator
DEX-052 Network cost estimator
DEX-053 Network buffer
DEX-054 Fee ledger
Epic 7 — Accounting
DEX-060 Ledger
DEX-061 Journals
DEX-062 Reconciliation
DEX-063 Revenue reporting

72. Fiat Development Tickets
Only after the crypto engine is stable:
DEX-100 Fiat market model
DEX-101 Payment method model
DEX-102 Fiat offer engine
DEX-103 Bond model
DEX-104 Bond state machine
DEX-105 Payment state machine
DEX-106 Reputation
DEX-107 Disputes
DEX-108 Risk engine
DEX-109 Compliance integration
DEX-110 Fiat reconciliation

73. Definition of Done — Crypto MVP
The crypto MVP is complete when:
[ ] User can connect/register wallet
[ ] Maker can create BTC/XMR order
[ ] Taker can accept order
[ ] Matching is race-safe
[ ] Trade state machine is enforced
[ ] Fees are calculated exactly
[ ] Network cost is estimated
[ ] Swap is initialized
[ ] Blockchain events are detected
[ ] Required confirmations are enforced
[ ] Swap completes
[ ] Failed swap follows recovery path
[ ] Ledger balances
[ ] Duplicate requests are safe
[ ] Full audit trail exists
[ ] Testnet adversarial tests pass

74. Definition of Done — Fiat MVP
[ ] Fiat offer creation
[ ] Payment-method selection
[ ] Maker pricing
[ ] Protocol fee
[ ] Bond requirement
[ ] Bond lock
[ ] Payment state machine
[ ] Trade timeout
[ ] Dispute creation
[ ] Evidence handling
[ ] Reputation updates
[ ] Risk decision
[ ] Compliance decision
[ ] Fiat reconciliation
[ ] Admin controls
[ ] Full audit trail

75. Golden Rule
The entire Open Dex codebase should obey this rule:
The UI never decides whether money moved.
The UI displays state.
The API requests state transitions.
The trade engine validates state transitions.
The settlement layer provides evidence.
The ledger records the economic consequences.
The blockchain/payment infrastructure provides external settlement evidence.
That separation is what keeps the architecture sane.

76. Final Technical Architecture
                         USER
                          |
                          v
                    WEB APPLICATION
                          |
                          v
                     API GATEWAY
                          |
       +------------------+------------------+
       |                  |                  |
       v                  v                  v
   MARKET ENGINE      TRADE ENGINE       USER/RISK
       |                  |                  |
       |                  v                  |
       |             STATE MACHINE           |
       |                  |                  |
       |          +-------+-------+          |
       |          |               |          |
       v          v               v          v
   ORDERS      SWAP ENGINE    PAYMENT      RISK
                  |              ENGINE     ENGINE
                  |                 |
          +-------+-------+         |
          |               |         |
       BITCOIN          MONERO    FIAT
          |               |         |
          +-------+-------+---------+
                  |
                  v
             EVENT / OUTBOX
                  |
        +---------+---------+
        |         |         |
        v         v         v
      LEDGER   AUDIT     REPUTATION
        |
        v
   RECONCILIATION

77. The First Engineering Milestone
The first milestone should not be "build the DEX UI."
It should be:
Two test wallets can complete a BTC/XMR trade through the Open Dex state machine, with the exact agreed price, protocol fees, network-cost accounting, blockchain verification, failure recovery, and balanced ledger.
Once that works reliably, the UI is comparatively straightforward.
The fiat marketplace should then be layered onto the same trade/event/ledger infrastructure while retaining its own payment, bond, risk and dispute state machines.

78. Engineering Priority Order
The build sequence is therefore:
1. Domain model
        ↓
2. State machines
        ↓
3. Ledger
        ↓
4. Fee engine
        ↓
5. Blockchain adapters
        ↓
6. BTC/XMR settlement
        ↓
7. Matching engine
        ↓
8. API
        ↓
9. Web UI
        ↓
10. Security testing
        ↓
11. Fiat domain
        ↓
12. Bonds
        ↓
13. Payment integrations
        ↓
14. Risk/compliance
        ↓
15. Fiat launch
This order is intentional.
Building the frontend or fiat payment integrations before the settlement and accounting primitives are correct will create technical debt in the most sensitive parts of the system.

79. Version 0.1 Architectural Contract
The following should be considered non-negotiable until explicitly changed through an architecture review:
1. BTC/XMR and fiat are separate market domains.

2. Makers determine market prices.

3. Protocol fees are explicit and separate from maker spreads.

4. Financial amounts use integer atomic units.

5. Trade state transitions are controlled by state machines.

6. Blockchain settlement is not inferred from database state.

7. Every financial action is represented in the ledger.

8. Every important state transition produces an event.

9. Financial POST requests are idempotent.

10. User funds and protocol revenue are separate concepts.

11. Fiat payment methods have independent risk policies.

12. Bonds are liabilities/security mechanisms, not automatic revenue.

13. Compliance is an architectural boundary, not a frontend feature.

14. Cryptographic primitives are not custom-built.

15. Production launch requires independent security review.

16. The system can pause new trading without destroying existing trade state.
End of Technical Specification v0.1
One thing I would not lock down yet is the exact BTC/XMR cryptographic protocol implementation. That deserves its own specification and security review rather than being casually embedded into the application design.
The next useful artifact is therefore Open Dex Protocol Specification v0.1 — BTC/XMR Atomic Swap, which should go one level deeper into the actual swap sequence: who generates what, what gets signed, what gets locked, timeout/refund paths, blockchain watchers, secret revelation, failure recovery, and the exact state transitions.

---

## Implementation Status — 2026-10-06

Milestones 1-3 completed. The following items from this specification have been implemented:

- **Crate architecture** — 14 new crates created following the Hybrid approach (Option C), coexisting with 3 legacy crates
- **SettlementProtocol trait** — The core abstraction separating marketplace from cryptographic settlement
- **Swap state machine** — 9 states with validated transitions
- **SwapEngine** — Generic coordinator with persistence, recovery, and transport
- **Event sourcing** — Append-only event log for full state reconstruction
- **File-based storage** — Atomic JSON writes for swaps, events, transaction intents
- **Blockchain watchers** — Read-only observers with supervisor coordination
- **Deterministic recovery** — Same inputs always produce same RecoveryDecision
- **Transport layer** — Async Transport trait with ProtocolEnvelope and replay protection
- **Domain types** — SwapId, Satoshis, Piconero, Asset (with Fiat), Price, TradingPair
- **Error hierarchy** — Central error types with thiserror derive
- **Mock infrastructure** — MockBitcoinChain, MockMoneroChain, MockWatcher, MockTransport

Not yet implemented:
- Real Eigenwallet integration
- Real blockchain RPC
- Real P2P transport
- Order book and matching (M4)
- Public API and GUI (M5)
- Fiat subsystem (M6)
