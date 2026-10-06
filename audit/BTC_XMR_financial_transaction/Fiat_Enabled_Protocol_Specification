

Open Dex should implement a unified trading protocol with multiple settlement backends: cryptographic atomic settlement for BTC/XMR and verified payment settlement for fiat.
Fiat cannot be made cryptographically atomic in the same way BTC/XMR can. The fiat side therefore needs payment-method adapters, payment verification, settlement conditions, disputes/recovery, identity/compliance hooks, and—depending on the operating model—regulated payment/custody infrastructure.
That distinction is particularly important for a UK deployment. The FCA currently treats exchanging or arranging the exchange of cryptoassets for money as an in-scope cryptoasset service, and its new UK cryptoasset regime begins on 25 October 2027. FCA+1 So I would build the software with a compliance/payment-provider boundary, rather than hard-coding a particular bank or assuming the P2P layer itself can legally handle fiat.
Open Dex — Fiat-Enabled Protocol Specification v0.1
Open Dex
Fiat-Enabled Trading & Settlement Protocol Specification
Version: 0.1
Status: Architecture baseline
Primary markets:
    • BTC/XMR
    • Fiat/BTC
    • Fiat/XMR
Initial fiat currencies:
    • GBP
    • EUR
    • USD
Initial cryptoassets:
    • BTC
    • XMR

1. Purpose
Open Dex is a peer-to-peer marketplace allowing participants to discover, negotiate, and settle trades without requiring a conventional centralized exchange order-matching and custody architecture.
The protocol supports two fundamentally different settlement classes:
Crypto Settlement
    BTC ↔ XMR
        │
        └── cryptographic atomic swap

Fiat Settlement
    GBP/EUR/USD ↔ BTC/XMR
        │
        └── verified payment settlement
The marketplace layer must treat both as implementations of a common settlement interface.

2. Core Architecture
                         OPEN DEX
                            │
                     MARKETPLACE CORE
                            │
             ┌──────────────┴──────────────┐
             │                             │
           ORDER                         TRADE
             │                             │
             └──────────────┬──────────────┘
                            │
                         SETTLEMENT
                            │
              ┌─────────────┴─────────────┐
              │                           │
       CRYPTO SETTLEMENT             FIAT SETTLEMENT
              │                           │
          BTC ↔ XMR                 GBP/EUR/USD
              │                           │
        Atomic Swap              Payment Method
              │                           │
        Eigenwallet              Payment Adapter
              │                           │
        BTC / Monero             Bank / Cash / PSP
The marketplace does not need to know whether a trade is:
BTC → XMR
GBP → BTC
BTC → GBP
EUR → XMR
XMR → USD
It only knows:
asset offered
asset requested
amount
price
settlement method
settlement state

3. Asset Model
The asset model becomes:
pub enum Asset {
    Bitcoin,
    Monero,

    Fiat(FiatCurrency),
}
Currencies:
pub enum FiatCurrency {
    GBP,
    EUR,
    USD,
}
Future currencies can be added without changing the trade engine.

4. Settlement Classification
Every trade explicitly declares its settlement class.
pub enum SettlementClass {
    CryptoAtomic,
    FiatPayment,
}
Examples:
BTC/XMR
    → CryptoAtomic

GBP/BTC
    → FiatPayment

EUR/XMR
    → FiatPayment

5. Trading Pair
pub struct TradingPair {
    pub base: Asset,
    pub quote: Asset,
}
Examples:
BTC/XMR
GBP/BTC
GBP/XMR
EUR/BTC
EUR/XMR
USD/BTC
USD/XMR

6. Order
An order contains:
pub struct Order {
    pub id: OrderId,

    pub maker: ParticipantId,

    pub pair: TradingPair,

    pub side: OrderSide,

    pub amount: Amount,

    pub price: Price,

    pub settlement: SettlementMethod,

    pub expires_at: Timestamp,
}
The critical new field is:
settlement: SettlementMethod

7. Settlement Method
pub enum SettlementMethod {
    BtcXmrAtomic,

    BankTransfer {
        currency: FiatCurrency,
    },

    CashDeposit {
        currency: FiatCurrency,
    },

    CashInPerson {
        currency: FiatCurrency,
    },
}
Additional methods should be adapters rather than changes to the marketplace engine.

8. Payment Methods
Fiat is not one homogeneous market.
The protocol therefore represents:
Fiat
 │
 ├── Bank Transfer
 │
 ├── Cash Deposit
 │
 ├── In-Person Cash
 │
 └── Future Payment Provider
Each method has its own:
    • settlement time
    • verification mechanism
    • reversal risk
    • fraud risk
    • limits
    • dispute rules
    • identity requirements
    • security-bond requirements

9. Maker Pricing
Open Dex does not require makers to use a fixed exchange rate.
The maker publishes:
reference price
+
maker premium/discount
Example:
Reference BTC price:

£75,000

Maker A:

£75,750

Maker B:

£76,000

Maker C:

£77,250
The market determines which offer is attractive.

10. Protocol Fee
The protocol fee is independent of the maker's price.
Example:
Maker price:
£76,000/BTC

Protocol fee:
0.50%

Network cost:
actual estimated BTC cost
The maker's premium belongs to the maker.
The protocol fee belongs to Open Dex according to the platform's fee policy.

11. Fee Model
Initial target:
BTC/XMR

Maker:
0.10%

Taker:
0.30%
Fiat:
Bank transfer:
0.25–0.50%

Cash deposit:
0.50–1.00%

In-person cash:
0.50–1.00%
These are configuration values rather than hard-coded protocol constants.
pub struct FeeSchedule {
    pub maker_bps: u32,
    pub taker_bps: u32,
}
100 basis points = 1%.

12. Effective Price
For fiat markets:
Effective Price =
Maker Price
+ Protocol Fee
+ Network Cost
+ Payment Cost
The UI/API should expose these separately.
Do not hide the protocol fee inside the maker's price.

13. Trade
Once an order is matched:
pub struct Trade {
    pub id: TradeId,

    pub maker: ParticipantId,
    pub taker: ParticipantId,

    pub pair: TradingPair,

    pub base_amount: Amount,
    pub quote_amount: Amount,

    pub settlement: SettlementMethod,

    pub fee: FeeBreakdown,

    pub state: TradeState,
}

14. Trade State
The generic state machine becomes:
pub enum TradeState {
    Created,
    Negotiating,
    Accepted,

    Funding,

    PaymentPending,
    PaymentDetected,
    PaymentConfirmed,

    CryptoSettlement,

    Releasing,

    Completed,

    Disputed,
    Recovering,

    Cancelled,
    Failed,
}
Not every state applies to every trade.
For example:
BTC/XMR:

Created
→ Accepted
→ CryptoSettlement
→ Completed
Fiat:
Created
→ Accepted
→ Funding
→ PaymentPending
→ PaymentDetected
→ PaymentConfirmed
→ Releasing
→ Completed

15. Settlement Trait
The central interface becomes:
pub trait SettlementProtocol {
    type State;
    type Error;

    fn initialize(
        &self,
        trade: &Trade,
    ) -> Result<Self::State, Self::Error>;

    fn handle_event(
        &self,
        state: &mut Self::State,
        event: SettlementEvent,
    ) -> Result<Vec<SettlementAction>, Self::Error>;

    fn recover(
        &self,
        state: &mut Self::State,
    ) -> Result<Vec<SettlementAction>, Self::Error>;
}

16. Settlement Events
pub enum SettlementEvent {
    ParticipantAccepted,

    FundingSubmitted,

    PaymentSubmitted,

    PaymentDetected,

    PaymentConfirmed,

    CryptoFundingDetected,

    CryptoRedeemed,

    Timeout,

    DisputeOpened,

    DisputeResolved,

    CancellationRequested,
}

17. Settlement Actions
pub enum SettlementAction {
    RequestPayment,

    VerifyPayment,

    LockCrypto,

    ReleaseCrypto,

    RefundCrypto,

    OpenDispute,

    NotifyParticipant,

    CompleteTrade,

    RecoverTrade,
}
This gives the marketplace a common language.

18. Fiat Settlement Architecture
The fiat side should look like:
                  FIAT TRADE
                      │
                      ▼
             Fiat Settlement Engine
                      │
          ┌───────────┴───────────┐
          │                       │
     Payment Adapter         Crypto Wallet
          │                       │
          ▼                       ▼
    Bank / PSP / Cash          BTC / XMR
          │
          ▼
   Payment Verification
          │
          ▼
    Settlement Decision
          │
          ▼
     Crypto Release

19. Payment Adapter
The core protocol must not directly implement:
bank API
payment-provider API
cash-deposit API
Instead:
pub trait PaymentAdapter {
    type Payment;
    type Error;

    fn create_payment(
        &self,
        request: PaymentRequest,
    ) -> Result<Self::Payment, Self::Error>;

    fn check_status(
        &self,
        payment: &Self::Payment,
    ) -> Result<PaymentStatus, Self::Error>;

    fn verify(
        &self,
        payment: &Self::Payment,
    ) -> Result<VerifiedPayment, Self::Error>;
}

20. Payment Status
pub enum PaymentStatus {
    Created,
    Pending,
    Detected,
    Confirmed,
    Failed,
    Reversed,
    Unknown,
}
Unknown is mandatory.
A payment-provider timeout must never be interpreted automatically as:
payment failed

21. Payment Verification
A fiat payment should require a verification result.
pub struct VerifiedPayment {
    pub payment_id: PaymentId,

    pub currency: FiatCurrency,

    pub amount: FiatAmount,

    pub sender_reference: String,

    pub recipient_reference: String,

    pub verified_at: Timestamp,

    pub status: PaymentStatus,
}
The system must verify:
currency
amount
recipient
payment reference
status
before releasing crypto.

22. No Screenshot-Based Settlement
A participant submitting:
"I sent the money."

is not sufficient.
Likewise:
"I have a bank screenshot."
is not a cryptographic payment proof.
The settlement engine should distinguish:
ParticipantClaim
from:
VerifiedPayment
Only the latter can trigger automatic release.

23. Bank Transfer Flow
Example:
Maker
  │
  │ offers BTC
  ▼
Order Book
  │
  │
Taker accepts
  │
  ▼
Trade Created
  │
  ▼
Crypto Reserved
  │
  ▼
Payment Instructions
  │
  ▼
Taker sends GBP
  │
  ▼
Payment Adapter
  │
  ▼
Bank/PSP verification
  │
  ▼
Payment Confirmed
  │
  ▼
Release BTC
  │
  ▼
Trade Complete

24. Cash Deposit Flow
Maker
  │
  ▼
Offer BTC
  │
  ▼
Taker accepts
  │
  ▼
Trade created
  │
  ▼
Cash instructions
  │
  ▼
Taker deposits cash
  │
  ▼
Payment provider / designated verifier
  │
  ▼
Deposit verified
  │
  ▼
BTC released
Cash deposits require a specific verification mechanism.
The protocol should not assume that a deposit exists merely because the taker uploads a receipt.

25. In-Person Cash
This is fundamentally different.
Maker
     \
      \ meet
       \
        Taker
The protocol can manage:
meeting location
time window
trade amount
identity requirements
security bond
confirmation
timeout
dispute
but it cannot cryptographically prove that physical cash changed hands.
Therefore the protocol needs an explicit:
PaymentAttestation
mechanism.

26. Payment Attestation
pub struct PaymentAttestation {
    pub trade_id: TradeId,

    pub payer: ParticipantId,
    pub payee: ParticipantId,

    pub amount: FiatAmount,

    pub method: SettlementMethod,

    pub confirmed_by: AttestationParty,

    pub timestamp: Timestamp,
}
Possible attestation models:
Both participants
Payment provider
Designated verifier
Regulated intermediary

27. Security Bond
Fiat trades may require:
pub struct SecurityBond {
    pub participant: ParticipantId,
    pub asset: Asset,
    pub amount: Amount,
}
Example:
Trade:

£1,000 BTC

Maker bond:
£200 equivalent

Taker bond:
£200 equivalent
The exact bond policy must be configurable.

28. Bond States
pub enum BondState {
    Required,
    Locked,
    Released,
    PartiallySlashed,
    Slashed,
    Refunded,
}

29. Critical Rule
A bond must not be silently confiscated.
Every slash must have:
trade ID
reason
rule violated
evidence
decision
timestamp
appeal/dispute state
This is essential for an auditable marketplace.

30. Dispute System
Fiat settlement introduces disputes.
Payment claimed
       │
       ▼
Verification
       │
   ┌───┴───┐
   │       │
verified  disputed
   │       │
   ▼       ▼
release  dispute
           │
           ▼
       resolution

31. Dispute
pub struct Dispute {
    pub id: DisputeId,

    pub trade_id: TradeId,

    pub opened_by: ParticipantId,

    pub reason: DisputeReason,

    pub evidence: Vec<Evidence>,

    pub state: DisputeState,
}

32. Dispute Reasons
pub enum DisputeReason {
    PaymentNotReceived,
    IncorrectAmount,
    PaymentReversed,
    WrongPaymentReference,
    CounterpartyNoShow,
    PaymentProviderFailure,
    SuspectedFraud,
    Other,
}

33. Dispute State
pub enum DisputeState {
    Open,
    EvidenceCollection,
    UnderReview,
    Resolved,
    Appealed,
    Closed,
}

34. Evidence
Evidence should be stored as references rather than putting arbitrary large files into the trade state.
pub struct Evidence {
    pub id: EvidenceId,

    pub kind: EvidenceType,

    pub content_hash: Hash,

    pub storage_reference: String,

    pub submitted_by: ParticipantId,

    pub timestamp: Timestamp,
}

35. Reputation
Fiat introduces a marketplace reputation layer.
A participant can have:
completed trades
successful trades
disputes
cancelled trades
payment reversals
bond slashes
average settlement time
But reputation must never become an irreversible financial blacklist.

36. Payment Method Profile
Each maker should advertise supported payment methods.
pub struct PaymentMethodProfile {
    pub method: SettlementMethod,

    pub min_trade: FiatAmount,

    pub max_trade: FiatAmount,

    pub settlement_window: Duration,

    pub required_bond_bps: u32,
}

37. Example Maker Offer
SELL BTC

Price:
£76,000 / BTC

Payment:
UK bank transfer

Minimum:
£100

Maximum:
£5,000

Settlement:
0–30 minutes

Protocol fee:
0.50%

Maker premium:
1.33%
The maker controls the offer price.

38. Market Discovery
A taker should be able to filter by:
asset
currency
payment method
amount
maker reputation
settlement time
price
bond requirement
Example:
BUY £1,000 BTC

Payment:
Bank transfer

Results:

Maker A
£75,900
98.9% completion

Maker B
£76,100
99.7% completion

Maker C
£77,000
100% completion

39. Crypto and Fiat Books Remain Separate
Internally:
MarketBook<TradingPair>
can be generic.
But settlement capabilities remain distinct.
BTC/XMR
    SettlementClass::CryptoAtomic

GBP/BTC
    SettlementClass::FiatPayment
Do not create a single giant settlement implementation.

40. Rust Crate Architecture
Given your existing repository, I would modify the current tree rather than replacing it.
Target:
crates/
├── common/
│
├── protocol/
│   └── src/
│       ├── settlement/
│       │   ├── mod.rs
│       │   ├── traits.rs
│       │   ├── events.rs
│       │   ├── actions.rs
│       │   ├── registry.rs
│       │   ├── fiat.rs
│       │   └── crypto.rs
│       │
│       └── ...
│
├── p2p-node/
│
├── btc-xmr/
│
└── fiat/
    └── src/
        ├── lib.rs
        ├── adapter.rs
        ├── payment.rs
        ├── bank.rs
        ├── cash.rs
        ├── verification.rs
        ├── bond.rs
        ├── dispute.rs
        └── error.rs

41. protocol Owns the Interfaces
The protocol crate owns:
SettlementProtocol
PaymentAdapter
SettlementEvent
SettlementAction
PaymentStatus
Dispute
Bond
It does not own a particular bank or payment provider.

42. fiat Owns Implementations
The fiat crate owns adapters.
Eventually:
fiat/
├── bank-transfer/
├── cash-deposit/
├── cash-in-person/
└── provider/
But initially these can live behind one crate.

43. Payment Provider Boundary
Production architecture:
Open Dex
    │
    ▼
Fiat Adapter
    │
    ▼
Payment Service Provider
    │
    ▼
Banking System
Open Dex should not store users' online banking passwords or directly imitate a banking application.

44. Custody Model
This specification supports two possible deployment models.
Model A — External regulated settlement provider
Open Dex
    │
    ▼
regulated provider
    │
    ▼
bank
This is the preferred initial production architecture.
Model B — Open Dex-operated fiat custody
Open Dex
    │
    ▼
Open Dex-controlled fiat accounts
    │
    ▼
banking system
This is substantially more operationally and legally complex.
The software should therefore abstract the payment provider so either model can be supported.

45. Compliance Boundary
The technical architecture must include:
pub trait ComplianceProvider {
    fn check_participant(
        &self,
        participant: &Participant,
    ) -> Result<ComplianceDecision, ComplianceError>;

    fn check_trade(
        &self,
        trade: &Trade,
    ) -> Result<ComplianceDecision, ComplianceError>;
}
Possible result:
pub enum ComplianceDecision {
    Approved,
    ReviewRequired,
    Rejected,
}
This does not encode a particular country's law.
It creates a place for the applicable compliance system.

46. Why This Boundary Is Necessary
For UK operations, the FCA currently says that businesses providing certain cryptoasset services—including exchange of cryptoassets for money or money for cryptoassets—can fall within its registration regime, and the future regime introduces broader authorisation requirements. FCA+1
The FCA also expects firms applying under the current regime to have documented AML/CTF/CPF frameworks, customer risk assessment, transaction monitoring, SAR procedures, and related controls. FCA
Therefore:
protocol
    ≠
compliance policy
Instead:
protocol
    │
    ▼
compliance interface
    │
    ▼
deployment-specific compliance system

47. Participant Model
Participants become:
pub struct Participant {
    pub id: ParticipantId,

    pub identity: ParticipantIdentity,

    pub capabilities: ParticipantCapabilities,

    pub payment_methods: Vec<PaymentMethodProfile>,
}
The actual sensitive identity information should not be broadcast over the P2P network.

48. Public Identity vs Compliance Identity
Separate:
P2P identity
from:
regulated identity
The P2P network may need:
participant ID
public key
reputation
trade history
while the compliance provider may hold:
legal identity
address
verification information
risk classification

49. P2P Privacy Rule
Never put this into ordinary P2P trade messages:
passport
date of birth
full bank account details
proof of address
KYC documents
The trade should carry references and authorization states instead.

50. Fiat Trade Message
A P2P offer can contain:
pub struct FiatOffer {
    pub trade_id: TradeId,

    pub currency: FiatCurrency,

    pub crypto_asset: Asset,

    pub amount: FiatAmount,

    pub price: Price,

    pub payment_method: PaymentMethodId,

    pub settlement_window: Duration,

    pub bond_requirement: BondRequirement,
}
No banking credentials.

51. Payment Instructions
Payment instructions should be generated securely.
pub struct PaymentInstructions {
    pub trade_id: TradeId,

    pub payment_id: PaymentId,

    pub currency: FiatCurrency,

    pub amount: FiatAmount,

    pub reference: PaymentReference,

    pub recipient: PaymentRecipient,
}
Sensitive recipient information should be delivered only to the relevant participant through the authenticated channel.

52. Payment Reference
Every fiat trade gets a unique reference:
OPENDEX-7F3A91
The reference helps the payment adapter reconcile incoming funds.

53. Idempotency
Every external payment operation needs an idempotency key.
pub struct PaymentOperation {
    pub idempotency_key: String,
    pub trade_id: TradeId,
}
A retry must not accidentally create two payments.

54. Fiat Transaction State Machine
             CREATED
                │
                ▼
            ACCEPTED
                │
                ▼
          CRYPTO_RESERVED
                │
                ▼
        PAYMENT_REQUESTED
                │
                ▼
         PAYMENT_PENDING
                │
          ┌─────┴─────┐
          │           │
          ▼           ▼
       DETECTED     FAILED
          │
          ▼
      CONFIRMING
          │
          ▼
       CONFIRMED
          │
          ▼
      CRYPTO_RELEASE
          │
          ▼
        COMPLETE

55. Reversal State
Payment reversal must be explicitly represented.
COMPLETE
   │
   │ payment reversed
   ▼
REVERSAL_DETECTED
   │
   ▼
DISPUTE
Never silently turn a completed trade back into pending.
A completed trade followed by a payment reversal is a distinct security event.

56. Timeouts
Every fiat trade has:
payment deadline
confirmation deadline
release deadline
dispute deadline
Example:
pub struct SettlementDeadlines {
    pub payment_deadline: Timestamp,
    pub confirmation_deadline: Timestamp,
    pub dispute_deadline: Timestamp,
}

57. Timeout Behaviour
If payment is not received:
payment_deadline
      ↓
cancel/recover
      ↓
release reserved crypto
      ↓
bond handling
If payment is detected but not confirmed:
confirmation_deadline
      ↓
hold
      ↓
provider reconciliation
Do not automatically release crypto merely because a timeout occurred.

58. Crypto Locking
For fiat trades, the crypto seller needs a reservation/escrow mechanism.
Conceptually:
Maker BTC
    │
    ▼
Reserved for trade
    │
    ├── payment confirmed → release
    │
    └── timeout/dispute → recovery
The exact custody mechanism depends on whether the seller's BTC is:
non-custodially locked
provider-custodied
escrowed
or otherwise controlled
This is one of the areas that requires a dedicated security design before mainnet deployment.

59. Fiat Does Not Use the BTC/XMR Atomic Swap
This is critical.
Do not implement:
GBP
 ↓
atomic swap
 ↓
BTC
as though GBP were a blockchain asset.
Instead:
GBP
 ↓
payment rail
 ↓
verification
 ↓
settlement decision
 ↓
BTC release

60. Unified Trade Interface
The marketplace still gets one interface:
pub trait TradeSettlement {
    fn start(
        &mut self,
        trade: &Trade,
    ) -> Result<(), SettlementError>;

    fn handle_event(
        &mut self,
        event: SettlementEvent,
    ) -> Result<Vec<SettlementAction>, SettlementError>;

    fn state(&self) -> SettlementState;
}
Implementations:
BtcXmrSettlement
FiatSettlement

61. Resulting Architecture
                         TRADE
                           │
                           ▼
                    SETTLEMENT ENGINE
                           │
             ┌─────────────┴─────────────┐
             │                           │
             ▼                           ▼
      CryptoSettlement             FiatSettlement
             │                           │
             ▼                           ▼
       BTC/XMR Adapter             Payment Adapter
             │                           │
             ▼                    ┌──────┴──────┐
        Eigenwallet               │             │
             │                  Bank          Cash
         ┌───┴───┐
         ▼       ▼
        BTC     XMR

62. What v0.1 Actually Implements
The first fiat milestone should implement:
✓ Fiat asset model
✓ Fiat order
✓ Fiat trade
✓ Payment method abstraction
✓ Payment state machine
✓ Payment verification interface
✓ Security bond model
✓ Dispute model
✓ Timeout/recovery model
✓ Compliance interface
✓ Fiat settlement adapter
✓ Persistence model
✓ P2P fiat offer messages
It should initially use:
MockPaymentProvider
rather than a real bank.

63. First Real Payment Method
After the mock implementation passes testing, add:
BankTransferAdapter
for the chosen jurisdiction/provider.
Do not implement every payment method simultaneously.

64. Recommended Development Sequence
Phase 1
Domain model
     ↓
FiatAsset
FiatOrder
FiatTrade
Phase 2
Settlement abstraction
     ↓
PaymentAdapter
SettlementProtocol
Phase 3
Mock payment provider
     ↓
payment lifecycle tests
Phase 4
Security bonds
     ↓
dispute engine
     ↓
recovery
Phase 5
P2P integration
     ↓
fiat offers
     ↓
trade negotiation
Phase 6
Compliance interface
     ↓
identity provider
     ↓
transaction monitoring
Phase 7
Real payment provider
Phase 8
BTC/XMR integration

65. Existing Open Dex Repository
Your existing structure should therefore evolve toward:
OpenDexRustSource/
│
├── apps/
│
├── audit/
│
├── crates/
│   ├── common/
│   │
│   ├── protocol/
│   │   └── src/
│   │       ├── settlement/
│   │       │   ├── mod.rs
│   │       │   ├── traits.rs
│   │       │   ├── events.rs
│   │       │   ├── actions.rs
│   │       │   ├── fiat.rs
│   │       │   └── crypto.rs
│   │       │
│   │       └── ...
│   │
│   ├── p2p-node/
│   │
│   ├── btc-xmr/
│   │
│   └── fiat/
│       └── src/
│           ├── adapter.rs
│           ├── payment.rs
│           ├── verification.rs
│           ├── bond.rs
│           ├── dispute.rs
│           ├── recovery.rs
│           └── ...
│
├── fuzz/
│
├── tests/
│   ├── fiat/
│   ├── btc_xmr/
│   └── protocol/
│
├── docs/
│   ├── architecture/
│   ├── fiat/
│   ├── btc-xmr/
│   └── security/
│
├── orders.json
└── trades.json

66. The Most Important Design Rule
The final system should not be:
Open Dex
   ├── crypto DEX
   └── separate fiat exchange
It should be:
                       OPEN DEX
                          │
                       MARKET
                          │
                        TRADE
                          │
                     SETTLEMENT
                          │
             ┌────────────┴────────────┐
             │                         │
          CRYPTO                      FIAT
             │                         │
        BTC/XMR                  GBP/EUR/USD
             │                         │
       Atomic protocol          Payment protocol
             │                         │
       Eigenwallet              Payment adapter
That means fiat is a first-class asset class in Open Dex, while its settlement mechanism remains fundamentally different from BTC/XMR.

67. Security Boundary
The architecture must assume:
Blockchain settlement
        ≠
Bank settlement
        ≠
Cash settlement
Each has different trust assumptions.
The protocol therefore records the settlement method explicitly and never pretends that a fiat payment has the same cryptographic guarantees as an on-chain transaction.

68. Regulatory Boundary
The production implementation must also keep the regulatory layer explicit.
For a UK deployment, this is not optional architecture. The FCA says firms providing in-scope cryptoasset services in the UK need registration under the current MLR framework, while the new FSMA cryptoasset regime introduces authorisation requirements from 25 October 2027. FCA+1
The FATF framework likewise expects jurisdictions to supervise relevant virtual-asset service providers and expects applicable providers to implement measures such as customer due diligence, record keeping and suspicious-transaction controls. FATF+1
Therefore the protocol should have:
             Open Dex
                │
       ┌────────┴────────┐
       │                 │
 Settlement           Compliance
       │                 │
       ▼                 ▼
Payment/BTC/XMR     KYC/AML/Risk
rather than mixing compliance logic directly into the atomic-swap code.

69. Definition of Fiat v0.1
Fiat support is considered architecturally complete when Open Dex can execute this simulated lifecycle:
Maker
  │
  │ creates GBP/BTC offer
  ▼
Order Book
  │
  │ taker accepts
  ▼
Trade
  │
  ▼
Compliance check
  │
  ▼
Security bonds
  │
  ▼
BTC reserved
  │
  ▼
GBP payment instructions
  │
  ▼
Taker submits payment
  │
  ▼
Payment adapter
  │
  ▼
Payment detected
  │
  ▼
Payment verified
  │
  ▼
BTC released
  │
  ▼
Trade completed
  │
  ▼
Bonds released
And the failure path:
Payment not received
        │
        ▼
      timeout
        │
        ▼
     recovery
        │
        ▼
BTC reservation released
        │
        ▼
bond decision
And the dispute path:
Payment claimed
      │
      ▼
verification failure
      │
      ▼
   dispute
      │
      ▼
evidence
      │
      ▼
resolution
      │
 ┌────┴────┐
 ▼         ▼
release   refund/recovery

70. Final Architecture Principle
The Open Dex protocol therefore becomes:
                OPEN DEX PROTOCOL
                       │
       ┌───────────────┴────────────────┐
       │                                │
   MARKET PROTOCOL                SETTLEMENT PROTOCOL
       │                                │
 Orders / Trades                ┌───────┴────────┐
 Matching / Pricing             │                │
 Fees / Reputation          CRYPTO             FIAT
                              │                  │
                         BTC ↔ XMR         Fiat ↔ Crypto
                              │                  │
                        Atomic Swap        Payment Settlement
                              │                  │
                         Eigenwallet       Payment Adapter
This is the version I would build against.
The next step should therefore be to modify your actual existing Rust tree around this specification—not create a parallel architecture. In particular, we should inspect crates/common, crates/protocol, and crates/p2p-node and determine exactly where FiatAsset, PaymentMethod, SettlementProtocol, PaymentAdapter, Bond, and Dispute belong before writing the implementation.
We should not make the first real fiat integration directly with a bank. Build the complete fiat state machine against a MockPaymentProvider first, including payment detection, confirmation, timeout, reversal, bonds and disputes. Then put the regulated/payment-provider boundary behind that same interface. That keeps the Open Dex core reusable while acknowledging that UK crypto/fiat activity has regulatory requirements that need to be designed alongside the production deployment. FCA+1


---

## Implementation Status — 2026-10-06

Milestones 1-3 completed for the BTC-XMR atomic swap path. The fiat subsystem (M6) has not yet been implemented.

Implemented items relevant to fiat:
- **Asset enum extended with Fiat** — `opendex-types` includes `Asset::Fiat(FiatCurrency)` variant
- **SettlementProtocol trait** — The generic trait can eventually be implemented for fiat settlement
- **Swap state machine** — The state machine is settlement-agnostic and could support fiat paths

Not yet implemented:
- **PaymentAdapter trait** — Not yet created
- **Fiat trade state machine** — Not yet implemented
- **Bond system** — Not yet implemented
- **Dispute resolution** — Not yet implemented
- **MockPaymentProvider** — Not yet implemented
- **Payment detection, confirmation, timeout, reversal** — Not yet implemented

The existing `protocol/p2p.rs` already contains P2P fiat trading types (PaymentMethod, FiatCurrency, P2POffer, P2PTrade) that predate the BTC-XMR work. These may be migrated to use the SettlementProtocol trait in a future milestone.
