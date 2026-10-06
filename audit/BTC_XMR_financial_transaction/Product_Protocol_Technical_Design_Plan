The important thing is to turn the pricing concept into an actual system architecture with clear boundaries between the crypto swap engine, fiat marketplace, settlement, fees, security deposits, reputation, and compliance.
Open Dex
Product, Protocol & Technical Design Plan
Document status: Architecture baseline
Version: 0.1
Primary markets: BTC ↔ XMR, Fiat ↔ BTC/XMR
Initial fiat focus: GBP, with architecture extensible to EUR/USD
Design objective: Build a non-custodial marketplace where liquidity providers determine prices and the protocol determines transparent transaction fees.

1. Executive Summary
Open Dex should be designed as two related but distinct marketplaces:
    1. Crypto Market
        ◦ BTC ↔ XMR
        ◦ Non-custodial atomic-swap settlement
        ◦ Maker/taker order model
        ◦ Low protocol fees
        ◦ Automated settlement wherever technically possible
    2. Fiat Market
        ◦ GBP/EUR/USD ↔ BTC/XMR
        ◦ Peer-to-peer offers
        ◦ Maker-defined exchange prices
        ◦ Payment-method-specific risk controls
        ◦ Security deposits/bonds
        ◦ Reputation and dispute mechanisms
        ◦ Compliance and regulated payment infrastructure separated from the crypto protocol
The fundamental pricing principle is:
Open Dex does not determine the market price. Makers determine the price. Open Dex determines the protocol fee and settlement rules.
This distinction should be embedded into the architecture rather than merely being a UI concept.

2. Core Design Principles
2.1 Non-custodial by default
Open Dex should avoid taking custody of user crypto wherever technically practical.
The system should therefore distinguish:
Protocol-controlled state
from
User-controlled assets.
The protocol should coordinate a transaction rather than become the counterparty holding everyone's funds.
For BTC/XMR, the preferred settlement mechanism is an atomic-swap protocol.
For fiat transactions, true non-custodial settlement depends heavily on the payment method. Bank transfers, cash deposits and payment applications cannot generally be made cryptographically atomic with BTC/XMR.
Therefore the fiat market should be treated as a risk-managed P2P marketplace, not simply as another atomic-swap market.

3. System Architecture
At a high level:
                         OPEN DEX
                            |
             +--------------+--------------+
             |                             |
        CRYPTO MARKET                  FIAT MARKET
             |                             |
        BTC <-> XMR                 GBP/EUR/USD
             |                      <-> BTC/XMR
             |                             |
      Atomic Swap Engine            P2P Offer Engine
             |                             |
      BTC Wallet Layer              Payment Methods
      XMR Wallet Layer              Security Bonds
             |                      Reputation
             |                      Disputes
             |                             |
             +-------------+---------------+
                           |
                     Fee Engine
                           |
                    Risk Engine
                           |
                  Identity / Compliance
                           |
                    Audit / Events
                           |
                     Web / API
The architecture should deliberately prevent the fiat marketplace from becoming tightly coupled to the BTC/XMR swap engine.

4. Major Components
Open Dex should initially be divided into these services/modules:
4.1 Web Application
Responsible for:
    • account creation
    • wallet connection
    • order discovery
    • offer creation
    • trade execution
    • trade status
    • fees
    • reputation
    • disputes
    • transaction history
The UI should never calculate critical financial state independently.
The backend/protocol should provide authoritative trade state.

4.2 API Gateway
Responsibilities:
    • authentication
    • authorization
    • rate limiting
    • API versioning
    • request validation
    • websocket connections
    • market data
Example:
/api/v1/markets
/api/v1/orders
/api/v1/offers
/api/v1/trades
/api/v1/fees
/api/v1/reputation
/api/v1/disputes
/api/v1/payments

4.3 Market Engine
The market engine maintains:
    • crypto orders
    • fiat offers
    • price discovery
    • available liquidity
    • order/offer status
    • matching
Do not make one universal order model.
Instead:
CryptoOrder
FiatOffer
should be separate domain objects.
They can share common primitives, but their settlement logic should remain independent.

5. Crypto Market
5.1 Initial market
Launch with:
BTC/XMR
XMR/BTC
Although these are economically the same market, the UI can support both directions.
Example maker quote:
SELL BTC
Price: 1 BTC = 100 XMR
Amount: 0.50 BTC
Maker fee: 0.10%
The maker controls the quoted price.
Open Dex does not impose a spread.

6. Crypto Fee Model
Initial fee schedule:
Market	Maker	Taker
BTC/XMR	0.10%	0.30%
Fees should be represented explicitly as protocol parameters rather than hard-coded throughout the application.
Example:
fee_schedule
----------------
market = BTC_XMR
maker_bps = 10
taker_bps = 30
version = 1
effective_from = timestamp
Using basis points avoids floating-point financial calculations.
10 bps = 0.10%
30 bps = 0.30%
All monetary calculations should use integer base units.

7. Maker Price vs Protocol Fee
This is one of the most important design decisions.
Do not model:
maker price + hidden spread
as a protocol fee.
Instead:
MARKET PRICE
+
PROTOCOL FEE
+
NETWORK COST
=
TOTAL COST TO USER
Example:
Maker price:
1 BTC = 100 XMR

Maker fee:
0.10%

Network cost:
actual estimated settlement cost

Network buffer:
15%
The UI should show each component separately.
Example:
BTC → XMR

Maker rate             100.000 XMR
Protocol fee             0.100%
Estimated network cost   £X.XX
Network buffer            15%

Estimated received       XX.XXXX XMR
No hidden spread.

8. Network Cost Engine
Network costs should not be fixed percentages.
Create a network-cost service.
NetworkCostService
    |
    +-- BTC estimator
    |
    +-- XMR estimator
    |
    +-- historical estimator
    |
    +-- safety buffer
The estimator should consider:
    • current network conditions
    • transaction size
    • required confirmation policy
    • wallet construction
    • current fee market
    • expected settlement delay
The network buffer can initially be:
15%
but should be configurable.
Do not make the buffer part of protocol revenue.
Any unused network-cost allocation should have an explicit refund/settlement policy.

9. BTC/XMR Settlement
The preferred crypto settlement architecture is:
Maker
  |
  | creates swap
  v
Open Dex
  |
  | coordinates
  v
Atomic Swap Protocol
  |
  +---- BTC chain
  |
  +---- Monero chain
The application layer should never assume that a database transaction equals blockchain settlement.
Instead, define explicit states.
CREATED
  ↓
QUOTED
  ↓
ACCEPTED
  ↓
SWAP_INITIALIZED
  ↓
BTC_LOCKED
  ↓
XMR_LOCKED
  ↓
SECRET_REVEALED
  ↓
SETTLEMENT
  ↓
COMPLETED
Failure paths:
EXPIRED
CANCELLED
REFUNDED
FAILED
DISPUTED
Every state transition should be deterministic and auditable.

10. Atomic Swap State Machine
The swap engine should be built as a state machine rather than a collection of API calls.
Example:
Swap {
    id
    maker
    taker

    base_asset
    quote_asset

    maker_amount
    taker_amount

    maker_fee
    taker_fee

    btc_txid
    xmr_txid

    timeout_at

    state
}
The important rule:
No user interface event should be able to arbitrarily change financial state.
State changes must be caused by validated protocol events.

11. Fiat Market
The fiat marketplace should use a completely different domain model.
Instead of an order book optimized for automated matching, begin with maker offers.
Example:
Offer #12345

SELL BTC
Amount: £500–£5,000

Payment method:
UK bank transfer

Maker price:
£76,000/BTC

Protocol fee:
0.50%

Security bond:
£200

Settlement window:
30 minutes
The maker defines the economics.
Open Dex defines the rules.

12. Fiat Markets
Create separate markets:
GBP/BTC
GBP/XMR

EUR/BTC
EUR/XMR

USD/BTC
USD/XMR
but also maintain payment-method-specific liquidity.
For example:
GBP/BTC
    |
    +-- Bank Transfer
    +-- Cash Deposit
    +-- In-Person Cash
This prevents a high-risk payment method from contaminating the risk model of a lower-risk one.

13. Fiat Pricing
The maker should specify:
reference price
+
maker premium/discount
For example:
Reference BTC price: £75,000

Maker price: £76,000

Maker premium:
1.333%
Open Dex then adds its protocol fee.
The UI should explicitly display:
Reference price
Maker price
Maker premium
Protocol fee
Network fee
Total effective price
This creates transparent price discovery.

14. Fiat Protocol Fees
Initial proposal:
Payment method	Maker	Taker
Bank transfer	0.25%	0.50%
Cash deposit	0.50%	1.00%
In-person cash	0.50%	1.00%
High-risk/reversible	Restricted / higher	Restricted / higher
These should be configuration values rather than application constants.
payment_method_policy
---------------------
method
maker_fee_bps
taker_fee_bps
bond_requirement
settlement_timeout
risk_level
enabled

15. Payment Methods Are Risk Classes
Do not create one object called:
Cash
Instead:
PaymentMethod
    |
    +-- BANK_TRANSFER
    +-- CASH_DEPOSIT
    +-- CASH_IN_PERSON
    +-- PAYMENT_APP
    +-- OTHER
Each payment method should have:
    • risk classification
    • maximum trade size
    • settlement timeout
    • confirmation requirements
    • bond requirement
    • dispute rules
    • geographic restrictions
    • availability status
This becomes one of the most important risk-control mechanisms in the platform.

16. Security Bonds
Fiat trading should support a security-bond system.
Example:
Trade value: £1,000

Maker bond: £200
Taker bond: £200

Total security:
£400
The exact percentages should be configurable.
The bond should not simply be treated as revenue.
It exists to create economic consequences for violating protocol rules.
Possible states:
BOND_REQUIRED
BOND_LOCKED
TRADE_ACTIVE
SETTLEMENT_CONFIRMED
BOND_RELEASE_PENDING
BOND_RELEASED
BOND_SLASHED
DISPUTED

17. Important Bond Design Rule
Do not initially implement arbitrary automated slashing.
Instead:
normal successful trade
        ↓
automatic bond release
For disputes:
trade disputed
        ↓
evidence collection
        ↓
temporary freeze
        ↓
rule-based resolution
        ↓
release / partial slash / full slash
The dispute system should be independently auditable.
Eventually, Open Dex could use a decentralized arbitration mechanism, but that should not be a launch dependency.

18. Reputation System
Every completed fiat trade should update a maker/taker reputation profile.
Example:
Trader:
Alice

Completed trades: 428
Completion rate: 99.1%
Average settlement time: 7m
Disputes: 2
Disputes lost: 0
Account age: 19 months
Avoid reducing reputation to a single star rating.
Use measurable statistics.
Potential reputation score:
completion rate
+
trade history
+
successful volume
+
dispute history
+
response time
+
account age
But reputation should never override hard risk controls.

19. Identity Architecture
Separate:
Protocol identity
from:
Compliance identity
A user might have:
Open Dex ID
Wallet addresses
Reputation profile
while a regulated fiat service may separately maintain:
KYC identity
source-of-funds information
sanctions screening
payment account information
Do not unnecessarily expose sensitive identity information to the public marketplace.

20. Compliance Boundary
This is a critical architectural boundary.
The crypto protocol and the fiat business should be separable.
                 OPEN DEX
                    |
        +-----------+-----------+
        |                       |
   PROTOCOL LAYER          FIAT SERVICES
        |                       |
 BTC/XMR swaps          regulated partners /
                         compliant operator
This makes it possible to operate the crypto infrastructure without assuming that every payment mechanism has identical regulatory requirements.
If Open Dex is operated from or into the UK, this is particularly important. As of October 2026, the FCA says in-scope cryptoasset services require registration under the current MLR regime, while the new FSMA cryptoasset regime is scheduled to begin on 25 October 2027; applications for that regime opened on 30 September 2026. FCA+2
The FCA also explicitly identifies cryptoasset exchange providers, including peer-to-peer providers, within the relevant cryptoasset-service perimeter. FCA
Therefore:
Do not build the fiat architecture first and figure out regulation later.
The legal perimeter should be an input into the system architecture.

21. Compliance Adapter
Create an abstraction:
ComplianceProvider
with methods such as:
checkUser()
checkTransaction()
checkPaymentMethod()
checkJurisdiction()
checkSanctions()
checkLimits()
Possible implementations:
ComplianceProvider
    |
    +-- MockProvider
    +-- InternalProvider
    +-- ExternalProvider
This keeps compliance infrastructure replaceable.

22. Geographic Policy Engine
Create:
JurisdictionPolicy
Example:
UK
    crypto_enabled: true
    fiat_enabled: depends_on_licensing
    btc_enabled: true
    xmr_enabled: policy-dependent

EU
    ...

US
    ...
The system should never assume:
if user can connect wallet
then user can trade.
Instead:
CanTrade(user, asset, payment_method, jurisdiction)
must return an explicit decision.

23. Risk Engine
Create a centralized risk engine.
RiskEngine
    |
    +-- User risk
    +-- Trade risk
    +-- Payment risk
    +-- Asset risk
    +-- Geographic risk
    +-- Velocity risk
    +-- Reputation risk
Output:
ALLOW
ALLOW_WITH_LIMIT
REVIEW
BLOCK
The risk engine should never secretly modify prices.
It should modify:
    • eligibility
    • limits
    • bond requirements
    • settlement requirements
    • review requirements

24. Limits
Every market needs configurable limits.
Example:
New account:
£500/day

Established trader:
£10,000/day

High-reputation maker:
£50,000/day
These are examples only.
Limits should be configurable by:
jurisdiction
payment method
account age
reputation
KYC level
risk score
market
asset

25. Order/Offer Lifecycle
Crypto
CREATE ORDER
      ↓
PUBLISH
      ↓
MATCH
      ↓
ACCEPT
      ↓
SWAP
      ↓
SETTLE
      ↓
COMPLETE
Fiat
CREATE OFFER
      ↓
PUBLISH
      ↓
TAKER ACCEPTS
      ↓
BONDS LOCKED
      ↓
CRYPTO SETTLEMENT INITIATED
      ↓
PAYMENT INSTRUCTIONS
      ↓
FIAT PAYMENT
      ↓
PAYMENT CONFIRMED
      ↓
CRYPTO RELEASE
      ↓
BONDS RELEASED
      ↓
COMPLETE
The exact ordering must be adapted to the payment method.

26. Trade Escrow Architecture
The system should distinguish three concepts:
Escrow
Settlement
Dispute
They are not the same thing.
Escrow
Funds are locked under predefined conditions.
Settlement
The conditions have been satisfied and ownership moves.
Dispute
The normal state machine cannot determine the correct outcome.
This separation will make the system substantially easier to reason about.

27. Event-Driven Architecture
All important actions should generate immutable events.
Example:
OfferCreated
OfferAccepted
BondLocked
SwapCreated
BTCDepositDetected
XMRDepositDetected
PaymentMarkedSent
PaymentConfirmed
SettlementCompleted
BondReleased
DisputeOpened
DisputeResolved
Use an event log as the authoritative history.
The database's current state can then be derived from events.
This will be invaluable for:
    • debugging
    • disputes
    • accounting
    • audits
    • compliance
    • analytics

28. Double-Entry Accounting
Do not rely on transaction tables alone.
Create an internal ledger.
Example:
LedgerAccount
LedgerEntry
Journal
Every protocol fee should produce a ledger event.
Example:
TRADE #1234

Gross trade value       £1,000
Protocol fee               £5
Network allocation         £2
Maker proceeds           £993
The ledger should balance.
For example:
Debit/Credit entries
---------------------
Trader
Maker
Protocol revenue
Network-cost account
Bond account
Never calculate revenue from raw blockchain transactions alone.

29. Protocol Revenue
Revenue should be separately tracked.
ProtocolRevenue
    |
    +-- Crypto maker fees
    +-- Crypto taker fees
    +-- Fiat maker fees
    +-- Fiat taker fees
Bond slashing should not automatically become protocol revenue.
A safer design is:
slashed bond
     ↓
designated resolution account
     ↓
victim compensation / arbitration / reserve
The exact legal treatment should be determined before launch.

30. Treasury Architecture
Create a treasury layer:
Treasury
    |
    +-- Operating revenue
    +-- Network-cost reserve
    +-- Dispute reserve
    +-- Security reserve
Do not mix:
customer funds
with:
company revenue
if the architecture ever handles custody.

31. Wallet Architecture
For BTC:
Bitcoin Wallet Service
For XMR:
Monero Wallet Service
Neither should be directly embedded inside the main API.
Use a wallet abstraction:
WalletProvider
    |
    +-- BitcoinProvider
    +-- MoneroProvider
The application talks to interfaces rather than directly to node RPC implementations.

32. Node Infrastructure
Run dedicated infrastructure for:
Bitcoin node(s)
Monero node(s)
Separate:
production
staging
development
Never let development infrastructure interact with production funds.
Use multiple independent observations for important blockchain events where practical.

33. Blockchain Confirmation Policy
Do not hard-code:
1 confirmation = settled
Instead:
ConfirmationPolicy
Example:
BTC:
required_confirmations = configurable

XMR:
required_confirmations = configurable
The policy should depend on:
    • transaction value
    • network conditions
    • chain-specific behavior
    • risk tolerance

34. API Security
Minimum requirements:
    • hardware-backed/admin MFA
    • signed wallet actions
    • API keys with scopes
    • rate limits
    • replay protection
    • nonce handling
    • idempotency keys
    • CSRF protection
    • strict input validation
    • audit logging
    • secrets management
Financial endpoints should support idempotency.
Example:
POST /trades

Idempotency-Key: abc123
Repeated requests must not create two trades.

35. Smart/Protocol Contract Principle
Where cryptographic contracts are used:
Keep the settlement logic minimal.
Do not put the entire marketplace into an on-chain contract.
The blockchain should enforce the minimum trust-critical rules.
The marketplace can remain off-chain.
This reduces:
    • attack surface
    • transaction costs
    • upgrade complexity
    • operational complexity

36. Database Model
Initial relational model:
users
wallets
assets

markets
crypto_orders
fiat_offers

trades
swap_sessions

payment_methods
payment_instructions
payment_events

bonds
bond_events

fees
fee_schedules

ledger_accounts
ledger_entries
journals

reputation_profiles
reputation_events

disputes
dispute_evidence
dispute_resolutions

risk_decisions
compliance_checks

blockchain_transactions
blockchain_events

audit_events
PostgreSQL would be a sensible initial choice.

37. Core Domain Relationships
User
 |
 +-- Wallet
 |
 +-- Reputation
 |
 +-- ComplianceProfile
 |
 +-- Orders
 |
 +-- Offers
 |
 +-- Trades
Trade:
Trade
 |
 +-- Maker
 +-- Taker
 +-- Market
 +-- Fee
 +-- Settlement
 +-- Bond
 +-- Payment
 +-- BlockchainTransactions
 +-- Events

38. API Design
Markets
GET /markets
GET /markets/{market}
GET /markets/{market}/offers
Crypto orders
POST /crypto/orders
GET /crypto/orders/{id}
DELETE /crypto/orders/{id}
Fiat offers
POST /fiat/offers
GET /fiat/offers/{id}
PATCH /fiat/offers/{id}
DELETE /fiat/offers/{id}
Trades
POST /trades
GET /trades/{id}
POST /trades/{id}/accept
POST /trades/{id}/cancel
Disputes
POST /trades/{id}/dispute
POST /disputes/{id}/evidence
GET /disputes/{id}

39. Pricing API
Every quote should return a complete price breakdown.
Example:
{
  "market_price": 76000,
  "maker_premium_bps": 133,
  "protocol_fee_bps": 50,
  "network_cost": 2.10,
  "network_buffer_bps": 150,
  "effective_price": 76400
}
The exact JSON schema should be finalized later.
The important architectural principle is:
Never return only a single mysterious "price."
The user should be able to understand where the price came from.

40. UI Structure
Home
Buy Crypto
Sell Crypto
Swap BTC/XMR
Crypto
BTC/XMR

Best offers
Order book
Your orders
Swap history
Cash
Buy BTC
Sell BTC
Buy XMR
Sell XMR

Payment method
Currency
Amount
Location
Maker reputation
Settlement time

41. Fiat Offer Card
A fiat offer should display:
Alice
★★★★★

SELL BTC

£76,000/BTC

£500 – £5,000

Bank Transfer

Maker premium: +1.33%
Protocol fee: 0.50%

Completion: 99.1%
428 trades

Settlement: ~10 min
Bond required: £200
This is much more useful than:
BTC £76,000
because the user understands the risk and economics.

42. Maker Dashboard
Makers need substantially more functionality than ordinary takers.
Dashboard:
Liquidity
Active offers
Completed trades
Volume
Revenue
Premium
Completion rate
Disputes
Bond utilization
Payment methods
Allow makers to define:
minimum trade
maximum trade
price
premium
payment method
settlement window
availability
geographic restrictions

43. Maker Pricing Tools
Provide:
Reference price
Maker premium
Effective price
Expected proceeds
Protocol fee
Network cost
Example:
BTC reference:
£75,000

Your premium:
+1.33%

Your price:
£75,997.50

Protocol fee:
0.50%

Expected net proceeds:
...
The maker should never have to manually calculate protocol fees.

44. Market Data
Publish:
best bid
best ask
last trade
24h volume
24h trades
average premium
median settlement time
For fiat markets additionally:
payment method
currency
geographic availability
maker reputation
Do not combine bank-transfer and in-person-cash prices into one misleading order book.

45. Dispute System
Disputes should be structured.
DISPUTE OPENED
       ↓
TRADE FROZEN
       ↓
EVIDENCE REQUESTED
       ↓
EVIDENCE SUBMITTED
       ↓
AUTOMATED RULE CHECK
       ↓
RESOLUTION
       ↓
BOND ACTION
       ↓
TRADE CLOSED
Evidence could include:
    • transaction references
    • blockchain transaction IDs
    • payment references
    • timestamps
    • signed messages
    • platform events
Avoid storing unnecessary personal financial information.

46. Dispute Authority
Initially:
Open Dex controlled arbitration
Later:
multi-party arbitration
Eventually potentially:
decentralized arbitration
Do not attempt decentralized arbitration before the basic trade-state model is proven.

47. Security Model
Threat-model at least:
Users
    • fraudulent maker
    • fraudulent taker
    • collusion
    • Sybil accounts
Blockchain
    • transaction replacement
    • chain reorganization
    • node failure
    • wallet compromise
Application
    • account takeover
    • API abuse
    • replay attacks
    • race conditions
    • database corruption
Fiat
    • payment reversal
    • chargeback
    • forged payment proof
    • compromised bank account
    • social engineering
Marketplace
    • wash trading
    • fake liquidity
    • reputation manipulation
    • price manipulation
    • griefing

48. Sybil Resistance
A user should not be able to create:
Alice1
Alice2
Alice3
Alice4
and immediately manufacture reputation.
Use multiple signals:
account age
trade history
bond history
completed volume
verified identity where required
payment-method history
wallet history
dispute history
Do not make identity verification the only reputation mechanism.

49. Observability
Every production system needs:
metrics
logs
traces
alerts
audit events
Track:
trade success rate
swap failure rate
payment timeout rate
dispute rate
average settlement time
blockchain confirmation time
fee revenue
network-cost variance
bond utilization
API errors
wallet errors
Create alerts for:
unusual withdrawal activity
abnormal trade volume
swap failures
wallet balance anomalies
node disagreement
fee calculation anomalies
database inconsistency

50. Disaster Recovery
Design for:
database failure
node failure
wallet service failure
API failure
cloud-region failure
key compromise
Maintain:
encrypted backups
offline recovery procedures
key rotation procedures
incident response plan
wallet recovery plan
The system should be able to enter:
TRADING PAUSED
without destroying existing trades.

51. Kill Switches
Administrative controls should include:
pause new trades
pause BTC market
pause XMR market
pause fiat market
pause individual payment method
pause individual maker
pause withdrawals/settlement
A kill switch should stop new exposure while preserving the ability to resolve existing trades where possible.

52. Configuration Management
Never deploy fee changes by changing application source code.
Create:
ProtocolConfiguration
with versioning.
Example:
crypto_maker_fee = 10 bps
crypto_taker_fee = 30 bps

bank_maker_fee = 25 bps
bank_taker_fee = 50 bps

cash_maker_fee = 50 bps
cash_taker_fee = 100 bps

network_buffer = 15%
Every trade records the configuration version used.
This prevents historical accounting ambiguity.

53. Fee Calculation
Use:
fee = notional × fee_bps / 10,000
Never use binary floating-point arithmetic for settlement calculations.
Use integer units.
Examples:
BTC → satoshis
XMR → atomic units
GBP → pence
EUR → cents
USD → cents

54. Example Crypto Trade
Maker:
1 BTC = 100 XMR
Taker purchases:
0.10 BTC
Gross:
10 XMR
Maker fee:
0.10%
Taker fee:
0.30%
The trade engine calculates the exact asset-denominated fees and displays them before acceptance.
The maker's 100 XMR/BTC quote remains the maker's market price.

55. Example Fiat Trade
Reference:
BTC = £75,000
Maker chooses:
£76,000
Trade:
£1,000
The UI displays:
Maker price
£76,000/BTC

Maker premium
+1.33%

Protocol fee
0.50%

Network cost
estimated separately

Effective cost
calculated before acceptance
The maker's premium belongs economically to the maker.
The protocol fee belongs to Open Dex.
These must remain separate in accounting.

56. Marketplace Ranking
Do not rank offers purely by price.
A ranking function could consider:
effective price
+
completion rate
+
settlement time
+
reputation
+
payment method
+
trade limits
The user should be able to choose:
Best price
Best reputation
Fastest
Lowest risk

57. MVP Scope
Do not build everything initially.
MVP 1 — Crypto
Build:
BTC/XMR
maker orders
taker acceptance
atomic swap engine
wallet integration
fee engine
transaction history
basic market UI
No fiat.
This proves the core decentralized exchange architecture.

58. MVP 2 — Fiat Marketplace
Add:
GBP/BTC
GBP/XMR

maker offers
bank transfer
reputation
security bonds
trade state machine
disputes
compliance integration
Start with one payment method.
Do not launch bank transfer + cash deposit + in-person + payment apps simultaneously.

59. MVP 3 — Additional Payment Methods
Then add:
EUR
USD

cash deposit
in-person cash
additional payment providers
Each should have its own:
risk model
fee schedule
limits
bond policy
dispute policy

60. Technology Stack
A practical initial stack:
Frontend
Next.js
TypeScript
Backend
TypeScript / Node.js
or:
Rust
for the settlement-critical services.
A good compromise is:
TypeScript
    ↓
API / marketplace / web

Rust
    ↓
cryptographic settlement / wallet-sensitive services
Database
PostgreSQL
Cache
Redis
Event system
Start with:
PostgreSQL transactional outbox
Then move to Kafka/NATS/etc. only when scale requires it.
Infrastructure
Docker
Kubernetes later
Infrastructure-as-code
Do not introduce Kubernetes merely because it is fashionable.

61. Service Architecture
Initial deployment:
                    API
                     |
          +----------+----------+
          |                     |
      Marketplace          Trade Engine
          |                     |
          +----------+----------+
                     |
                PostgreSQL
                     |
              Blockchain Layer
                /          \
             Bitcoin      Monero
As the platform grows:
API Gateway
     |
     +-- Identity Service
     +-- Market Service
     +-- Trade Service
     +-- Fee Service
     +-- Risk Service
     +-- Compliance Service
     +-- Wallet Service
     +-- Payment Service
     +-- Reputation Service
     +-- Dispute Service
     +-- Ledger Service
     +-- Notification Service
Do not start with twelve microservices.
Start with a modular monolith with strict domain boundaries.
Extract services when the boundaries have proven themselves.

62. Recommended Repository Structure
open-dex/

  apps/
    web/
    api/

  packages/
    domain/
    market/
    trades/
    swaps/
    fees/
    ledger/
    risk/
    reputation/
    compliance/
    payments/

  infrastructure/
    bitcoin/
    monero/
    database/
    messaging/

  docs/
    architecture/
    api/
    protocol/
    compliance/
    security/

  tests/
    unit/
    integration/
    e2e/
    adversarial/
The exact language/framework can change.
The domain separation should not.

63. Testing Strategy
You need four layers.
Unit
Test:
fee calculations
state transitions
limits
risk rules
pricing
Integration
Test:
database
Bitcoin node
Monero node
wallets
payment adapters
End-to-end
Test:
maker → taker → trade → settlement
Adversarial
Test:
double acceptance
race conditions
expired trades
malicious maker
malicious taker
node failure
payment reversal
database rollback
API replay

64. Formal Invariants
Define invariants before writing the settlement engine.
Examples:
A completed trade cannot become active again.

A cancelled trade cannot settle.

A bond cannot be released twice.

A protocol fee cannot be charged twice.

A trade cannot have two makers.

A trade cannot settle without satisfying settlement conditions.

A wallet action cannot be replayed.

Ledger entries must always balance.
These should become automated tests.

65. Accounting Invariants
For every trade:
INPUTS
=
OUTPUTS
+
FEES
+
NETWORK COSTS
For the ledger:
TOTAL DEBITS = TOTAL CREDITS
For bonds:
locked + released + slashed
=
total deposited
These checks should run continuously.

66. Security Development Process
Before mainnet launch:
Threat model
      ↓
Unit tests
      ↓
Integration tests
      ↓
Testnet
      ↓
Adversarial testing
      ↓
Independent security audit
      ↓
Limited mainnet beta
      ↓
Monitoring period
      ↓
Full launch
Never treat an audit as a substitute for internal testing.

67. Regulatory Architecture
The regulatory layer should be treated as a first-class product requirement.
For a UK-facing operation, the FCA currently states that certain cryptoasset services must be registered under the MLRs, and its new cryptoasset regime is scheduled to take effect on 25 October 2027. The FCA also says applications for that regime opened on 30 September 2026. FCA+2
The new regime covers activities including operating cryptoasset trading platforms, dealing/arranging and safeguarding, so the exact legal characterization of Open Dex needs to be established before choosing the final operating model. FCA+1
Therefore create a formal:
Regulatory Perimeter Document
before launching fiat functionality.
It should answer:
Who operates Open Dex?
Where is the operator incorporated?
Where are users located?
Who controls the matching engine?
Who controls settlement?
Who holds fiat?
Who holds crypto?
Who communicates promotions?
Who handles disputes?
Who performs KYC?
Who performs sanctions screening?
Which entities are counterparties?
Which entities are service providers?
These answers determine the regulatory architecture.

68. Important Product Decision
I would not attempt to make Open Dex simultaneously:
fully decentralized
+
fiat custodian
+
payment processor
+
arbitrator
+
exchange
That creates an enormous trust and regulatory surface.
Instead:
Open Dex protocol
        |
        +-- decentralized crypto settlement
        |
        +-- P2P marketplace
        |
        +-- reputation/risk infrastructure
        |
        +-- regulated payment/compliance integrations
This is a much more defensible architecture.

69. Revenue Architecture
Target:
Crypto:
10 bps maker
30 bps taker

Fiat:
25–50+ bps protocol fee
Revenue:
Protocol fees
      +
optional infrastructure fees
      +
future premium services
Potential future services:
maker API
market-maker tools
advanced analytics
institutional API
priority liquidity
professional market data
Do not monetize customer deposits or manipulate maker pricing.

70. Core Product Philosophy
The product should communicate one simple idea:
Makers set the price. Open Dex provides the marketplace, settlement infrastructure and rules.
That makes the platform fundamentally different from an exchange that secretly controls the spread.

71. Initial Product Diagram
                         OPEN DEX
                            |
             +--------------+--------------+
             |                             |
             v                             v
       CRYPTO MARKET                  FIAT MARKET
             |                             |
        BTC ↔ XMR                    GBP/EUR/USD
             |                         ↔ BTC/XMR
             |                             |
       Order Book / RFQ                P2P Offers
             |                             |
       Maker Price                  Maker Price
             |                       + Premium
             |                             |
       Protocol Fee                 Protocol Fee
             |                             |
       Network Cost                  Payment Risk
             |                             |
       Atomic Swap                   Security Bond
             |                             |
       Blockchain                    Payment Rail
             |                             |
             +-------------+---------------+
                           |
                           v
                    Trade Engine
                           |
             +-------------+-------------+
             |             |             |
             v             v             v
          Ledger        Risk       Reputation
             |             |             |
             +-------------+-------------+
                           |
                           v
                     Audit / Events

72. Development Roadmap
Phase 0 — Architecture & Legal
Before coding:
[ ] Define operating entity
[ ] Define target jurisdictions
[ ] Obtain specialist legal analysis
[ ] Define custody model
[ ] Define fiat operating model
[ ] Define regulatory perimeter
[ ] Finalize threat model
[ ] Finalize protocol specification

Phase 1 — Crypto Prototype
[ ] Bitcoin node
[ ] Monero node
[ ] Wallet abstraction
[ ] Atomic swap prototype
[ ] Swap state machine
[ ] Fee engine
[ ] PostgreSQL schema
[ ] API
[ ] Basic UI
Target:
Complete BTC/XMR testnet swaps reliably.

Phase 2 — Crypto Beta
[ ] Mainnet integration
[ ] Production wallet architecture
[ ] Security review
[ ] Monitoring
[ ] Ledger
[ ] Fee accounting
[ ] Maker/taker marketplace
[ ] API
[ ] Rate limits
[ ] Incident response
Target:
Small-volume real BTC/XMR marketplace.

Phase 3 — Fiat Architecture
[ ] Regulatory model
[ ] Compliance provider
[ ] Identity model
[ ] Payment-method abstraction
[ ] Fiat offer engine
[ ] Bond engine
[ ] Reputation engine
[ ] Dispute engine
Target:
Fully specify fiat trading before accepting fiat volume.

Phase 4 — GBP/BTC
Launch one payment method.
GBP ↔ BTC
Bank transfer
Only after this works should you add:
GBP ↔ XMR

Phase 5 — Additional Markets
EUR/BTC
EUR/XMR
USD/BTC
USD/XMR
Then additional payment methods.

73. The First 10 Things I Would Build
If development starts tomorrow, the order should be:
    1. Write the BTC/XMR atomic-swap protocol specification.
    2. Define the complete trade state machine.
    3. Build the fee engine using integer/basis-point accounting.
    4. Build the ledger and accounting invariants.
    5. Implement Bitcoin and Monero node adapters.
    6. Build the maker/taker crypto marketplace.
    7. Build testnet end-to-end swaps.
    8. Add security/attack testing.
    9. Only then design the fiat trade state machine.
    10. Run the fiat architecture through specialist legal/regulatory review before implementing payment flows.

74. Definition of Done for Open Dex v1
Open Dex v1 should not be considered complete until:
[ ] BTC/XMR swaps work end-to-end
[ ] Maker pricing works
[ ] Maker/taker fees work
[ ] Network costs are transparent
[ ] Every trade has deterministic state
[ ] Failed swaps recover correctly
[ ] Ledger balances
[ ] Duplicate payments/actions are impossible
[ ] Wallet operations are audited
[ ] Security review completed
[ ] Monitoring operational
[ ] Emergency pause works
[ ] Fiat architecture is legally reviewed
[ ] Fiat payment methods have independent risk policies
[ ] Bonds have deterministic rules
[ ] Disputes have defined procedures
[ ] Reputation cannot be trivially gamed
[ ] Compliance boundaries are documented

75. Final Architecture Decision
The strongest version of Open Dex is not simply a decentralized exchange with another trading pair.
It is:
                  OPEN DEX
                     |
        +------------+------------+
        |                         |
   TRUST-MINIMIZED            RISK-MANAGED
   CRYPTO MARKET              FIAT MARKET
        |                         |
    BTC ↔ XMR                Fiat ↔ Crypto
        |                         |
 Atomic settlement             P2P offers
        |                         |
 Maker pricing               Maker pricing
        |                         |
 Low protocol fee          Higher protocol fee
        |                         |
 Blockchain                Payment methods
        |                         |
        +------------+------------+
                     |
              Common Infrastructure
                     |
       +------+------+------+------+
       |      |      |      |      |
     Ledger  Risk  Events  Reputation
                     |
                 Compliance
The key architectural boundary is:
BTC/XMR should strive for cryptographic settlement.
Fiat trading should strive for transparent, rule-based, risk-managed P2P settlement.
Trying to force fiat into the exact same trust model as BTC/XMR would make the system more complicated and, paradoxically, less decentralized.
The next engineering artifact should therefore be the Open Dex Protocol Specification: the exact data structures, state machines, fee calculations, atomic-swap lifecycle, bond lifecycle, API contracts, and database schema that can be implement against.
A particularly important point for the plan is that the fiat side should be treated as a separate product architecture, not just another pair in the BTC/XMR exchange. The FCA's current UK guidance makes the regulatory perimeter materially relevant to the design, including P2P cryptoasset exchange activity and the forthcoming 2027 regime. FCA+1
If you want to take this from a product document to something developers can actually start coding from, the next document I'd produce is the Open Dex Technical Specification v0.1—with the PostgreSQL schema, API endpoints, TypeScript/Rust interfaces, trade state machines, fee formulas, and BTC/XMR atomic-swap sequence diagrams.

---

## Implementation Status — 2026-10-06

Milestones 1-3 completed. The following items from this specification have been implemented:

- **Crate architecture** — 14 new crates following the spec's proposed structure, using the Hybrid approach (Option C)
- **SettlementProtocol trait** — The core interface defined in the spec is implemented
- **Swap state machine** — Matches the spec's state transitions
- **SwapEngine** — Generic coordinator with persistence, recovery, and transport
- **Event sourcing** — Append-only event log as specified
- **File-based storage** — Repository traits with file-based implementation (PostgreSQL deferred)
- **Blockchain watchers** — Read-only observers as specified
- **Deterministic recovery** — RecoveryEngine with DeterministicRecoveryEngine
- **Transport layer** — Async Transport trait with ProtocolEnvelope
- **Domain types** — SwapId, Satoshis, Piconero, Asset, Price, TradingPair
- **Error hierarchy** — Central error types

Not yet implemented:
- Order book and matching (M4)
- Public API and GUI (M5)
- Fiat subsystem (M6)
- PostgreSQL storage (file-based for M2, PostgreSQL later)
- Real Eigenwallet integration
