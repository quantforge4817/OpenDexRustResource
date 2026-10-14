Open Dex
Rust Crate Architecture & Interface Specification v0.1
Status: Engineering specification
Version: 0.1
Target language: Rust
Initial settlement pair: BTC/XMR
Settlement implementation: Eigenwallet BTC/XMR protocol adapter
Architecture: Modular, non-custodial, protocol-adapter based

1. Purpose
This document defines the Rust workspace architecture for Open Dex.
The objective is to create a marketplace platform in which:
Open Dex marketplace
        |
        v
Swap Engine
        |
        v
Protocol Adapter
        |
        v
BTC/XMR Atomic Swap
is separated into independently testable layers.
The most important architectural rule is:
Open Dex owns the marketplace; the atomic-swap protocol owns cryptographic settlement.
The marketplace must not become a second implementation of the BTC/XMR cryptographic protocol.

2. Architectural Objectives
Open Dex must provide:
    • non-custodial settlement
    • maker-defined pricing
    • protocol-defined fees
    • atomic BTC/XMR settlement
    • replaceable transport
    • replaceable blockchain backends
    • replaceable wallet implementations
    • versioned settlement protocols
    • deterministic recovery
    • crash-safe state management
    • future support for additional trading pairs
The architecture should allow:
BTC/XMR
   |
   +--> Eigenwallet adapter

Future pair
   |
   +--> Another protocol adapter
without changing the order book or marketplace engine.

3. High-Level Architecture
                         OPEN DEX
                            |
             +--------------+--------------+
             |                             |
             v                             v
       Marketplace                     Identity
          Core                         / Reputation
             |
             v
       Matching Engine
             |
             v
         Trade Engine
             |
             v
         Swap Engine
             |
             v
      Protocol Registry
             |
      +------+------+
      |             |
      v             v
 BTC/XMR v0.1    Future Protocols
      |
      v
 Eigenwallet Adapter
      |
 +----+---------+
 |              |
 v              v
Bitcoin       Monero
Adapter       Adapter
 |              |
 v              v
Wallets       Wallets

4. Workspace Structure
The initial Cargo workspace should be:
opendex/
│
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── justfile
│
├── crates/
│   │
│   ├── opendex-types/
│   │
│   ├── opendex-errors/
│   │
│   ├── opendex-events/
│   │
│   ├── opendex-protocol/
│   │
│   ├── opendex-protocol-types/
│   │
│   ├── opendex-btc-xmr/
│   │
│   ├── opendex-bitcoin/
│   │
│   ├── opendex-monero/
│   │
│   ├── opendex-wallet/
│   │
│   ├── opendex-chain/
│   │
│   ├── opendex-transport/
│   │
│   ├── opendex-swap/
│   │
│   ├── opendex-market/
│   │
│   ├── opendex-matching/
│   │
│   ├── opendex-storage/
│   │
│   ├── opendex-watchers/
│   │
│   ├── opendex-recovery/
│   │
│   ├── opendex-api/
│   │
│   └── opendex-node/
│
├── integration-tests/
│
└── docs/

5. Dependency Direction
Dependencies must flow downward.
                    opendex-node
                         |
             +-----------+-----------+
             |                       |
       opendex-api             opendex-market
                                     |
                               opendex-matching
                                     |
                                opendex-swap
                                     |
                            opendex-protocol
                                     |
                       +-------------+-------------+
                       |                           |
              opendex-btc-xmr              future adapters
                       |
              +--------+--------+
              |                 |
       opendex-bitcoin   opendex-monero
Lower layers must never import higher layers.
For example:
opendex-bitcoin
must not depend on:
opendex-market

6. Core Principle
The atomic-swap implementation should be usable without the marketplace.
This must compile conceptually:
let swap = BtcXmrSwap::new(...);

let action = swap.next_action(...);
without requiring:
database
order book
HTTP server
GUI
user account
That makes the cryptographic settlement layer independently auditable.

7. opendex-types
This crate contains fundamental domain types.
Examples:
pub struct SwapId(Uuid);

pub struct TradeId(Uuid);

pub struct OrderId(Uuid);

pub struct ParticipantId(Uuid);

pub struct ProtocolVersion {
    pub family: String,
    pub version: u16,
}

8. Money Types
Never represent monetary values with:
f64
Use integer-based types.
For Bitcoin:
pub struct Satoshis(pub u64);
For Monero:
pub struct Piconero(pub u64);
For fiat:
pub struct FiatAmount {
    pub currency: Currency,
    pub minor_units: i64,
}

9. Currency
pub enum Asset {
    Btc,
    Xmr,
    Fiat(FiatCurrency),
}
Example:
pub enum FiatCurrency {
    GBP,
    EUR,
    USD,
}
Adding a currency must not require modifying settlement logic.

10. Price
Prices should use fixed-point/integer representation.
Do not use:
f64
for executable orders.
Example:
pub struct Price {
    pub base: Asset,
    pub quote: Asset,
    pub numerator: u128,
    pub denominator: u128,
}

11. Amount + Price
An order should contain:
asset
quantity
price
rather than:
floating point total
The final settlement amount must be deterministically calculated.

12. opendex-errors
Central error definitions should be separated from implementation crates.
pub enum OpenDexError {
    InvalidRequest,
    InvalidState,
    Protocol(ProtocolError),
    Wallet(WalletError),
    Chain(ChainError),
    Storage(StorageError),
    Transport(TransportError),
    Security(SecurityError),
}
Lower-level errors should retain their source information.

13. opendex-events
Everything important happening in Open Dex becomes an event.
Example:
pub enum Event {
    OrderCreated(OrderCreated),
    OrderMatched(OrderMatched),
    TradeCreated(TradeCreated),

    SwapCreated(SwapCreated),
    SwapStateChanged(SwapStateChanged),

    FundingDetected(FundingDetected),
    RedemptionDetected(RedemptionDetected),

    RefundDetected(RefundDetected),

    SwapCompleted(SwapCompleted),
    SwapFailed(SwapFailed),
}

14. Event Sourcing Principle
The event log should allow reconstruction of:
order
trade
swap
transaction intent
without trusting a volatile in-memory process.

15. Event Immutability
Events are append-only.
Do not update:
SwapCompleted
to:
SwapFailed
Instead append:
SwapFailureDetected
if a later inconsistency occurs.
The state projection decides what that means.

16. opendex-protocol-types
This crate defines generic settlement abstractions.
pub struct ProtocolId(pub String);

pub struct ProtocolVersion {
    pub id: ProtocolId,
    pub major: u16,
    pub minor: u16,
}

17. Generic Protocol Trait
pub trait SettlementProtocol {
    type State;
    type Message;
    type TransactionPlan;
    type Error;

    fn initialize(
        &self,
        context: SwapContext,
    ) -> Result<Self::State, Self::Error>;

    fn process_message(
        &self,
        state: &mut Self::State,
        message: Self::Message,
    ) -> Result<ProtocolAction, Self::Error>;

    fn observe_chain(
        &self,
        state: &mut Self::State,
        observation: ChainObservation,
    ) -> Result<ProtocolAction, Self::Error>;
}

18. Why This Trait Exists
The marketplace should not know whether settlement uses:
atomic swap
HTLC
adaptor signature
multisig
future protocol
It only sees:
SettlementProtocol

19. Protocol Actions
pub enum ProtocolAction {
    SendMessage(ProtocolMessage),
    BuildTransaction(TransactionIntent),
    Wait(WaitCondition),
    Complete,
    Recover(RecoveryAction),
    Fail(ProtocolFailure),
}

20. opendex-protocol
This is the protocol orchestration layer.
It owns:
protocol registry
protocol version negotiation
protocol capability discovery
protocol lifecycle
It does not own:
order matching
pricing
UI

21. Protocol Registry
pub trait ProtocolRegistry {
    fn resolve(
        &self,
        pair: TradingPair,
        version: ProtocolVersion,
    ) -> Result<Box<dyn SettlementProtocol>, RegistryError>;
}

22. Registry Example
BTC/XMR
   |
   v
OPENDEX-BTC-XMR-1
   |
   v
EigenwalletAdapter
Future:
BTC/XXX
   |
   v
OPENDEX-BTC-XXX-1
   |
   v
OtherAdapter

23. opendex-btc-xmr
This is the most security-sensitive Open Dex crate.
It is the compatibility boundary around Eigenwallet's BTC/XMR implementation.
Its responsibility:
Open Dex types
       |
       v
Eigenwallet protocol
       |
       v
Open Dex protocol interface

24. Upstream Dependency Policy
Do not depend on an unpinned branch.
Bad:
git = "https://github.com/eigenwallet/core"
branch = "master"
Preferred:
git = "https://github.com/eigenwallet/core"
rev = "<APPROVED_COMMIT>"
or, preferably where the architecture permits:
version = "<APPROVED_RELEASE>"
with the exact lockfile committed.
The current Eigenwallet repository is actively changing; its 2026 changelog shows security-sensitive changes including cross-curve proof fixes and Bitcoin output-selection fixes. GitHub

25. Upstream Compatibility Record
Create:
docs/protocols/eigenwallet-btc-xmr.md
containing:
protocol:
  id: OPENDEX-BTC-XMR-1

upstream:
  project: eigenwallet
  repository: eigenwallet/core
  revision: "<commit>"
  release: "<release>"

approved:
  date: "<date>"
  reviewer: "<review>"

26. Do Not Fork Immediately
Open Dex should initially:
depend
  |
  v
adapt
  |
  v
test
rather than:
copy
  |
  v
modify
  |
  v
forget upstream
If a fork becomes necessary later, the fork should receive its own protocol identity.

27. Adapter Boundary
pub struct EigenwalletBtcXmrAdapter {
    protocol_version: ProtocolVersion,
    config: AdapterConfig,
}
Expose only Open Dex-compatible operations.

28. Adapter API
pub trait BtcXmrAtomicSwap {
    fn create_swap(
        &self,
        context: SwapContext,
    ) -> Result<SwapState, SwapError>;

    fn handle_message(
        &self,
        state: &mut SwapState,
        message: PeerMessage,
    ) -> Result<Vec<SwapAction>, SwapError>;

    fn observe(
        &self,
        state: &mut SwapState,
        observation: ChainObservation,
    ) -> Result<Vec<SwapAction>, SwapError>;

    fn recover(
        &self,
        state: &SwapState,
    ) -> Result<RecoveryPlan, SwapError>;
}

29. opendex-swap
This crate owns the generic swap lifecycle.
It should not know cryptographic details.
It knows:
swap ID
trade ID
participants
settlement amounts
protocol version
state
timeouts
events

30. Swap State
pub enum SwapState {
    Created,
    Negotiating,
    ProtocolSetup,
    Funding,
    Funded,
    Redeeming,
    Completed,
    Recovering,
    Refunded,
    Failed,
}
The underlying protocol state is stored separately.

31. Generic Swap Record
pub struct Swap {
    pub id: SwapId,
    pub trade_id: TradeId,

    pub maker: ParticipantId,
    pub taker: ParticipantId,

    pub sell_asset: Asset,
    pub buy_asset: Asset,

    pub sell_amount: Amount,
    pub buy_amount: Amount,

    pub protocol: ProtocolVersion,

    pub state: SwapState,

    pub created_at: Timestamp,
}

32. Marketplace vs Swap
An important distinction:
Order
means:
"I am willing to trade."
Trade
means:
"These two orders matched."
Swap
means:
"These two parties are now executing settlement."
Therefore:
Order != Trade != Swap

33. opendex-market
Marketplace responsibilities:
orders
quotes
maker/taker roles
fees
pricing
minimum quantities
maximum quantities
availability
No cryptography.

34. Maker Order
pub struct MakerOrder {
    pub id: OrderId,
    pub maker: ParticipantId,

    pub pair: TradingPair,

    pub side: Side,

    pub quantity: Amount,
    pub price: Price,

    pub fee_policy: FeePolicy,

    pub payment_method: Option<PaymentMethod>,
}

35. Maker Price
The maker controls:
price
Open Dex controls:
protocol fee
This preserves the architecture we established earlier.

36. Fee Policy
pub struct FeePolicy {
    pub maker_fee_bps: u32,
    pub taker_fee_bps: u32,
}
For BTC/XMR v0.1:
maker = 10 bps
taker = 30 bps
But these should be configuration, not cryptographic constants.

37. Cash Market Future
The same marketplace model can later represent:
GBP/BTC
GBP/XMR
EUR/BTC
EUR/XMR
with:
PaymentMethod
and:
SettlementProvider
However, fiat settlement should not be added to the BTC/XMR protocol crate.

38. opendex-matching
The matching engine determines:
maker
taker
price
quantity
It does not determine:
Bitcoin transaction
Monero transaction
timelock
cryptographic proof

39. Matching Output
pub struct Match {
    pub maker_order: OrderId,
    pub taker_order: OrderId,

    pub maker: ParticipantId,
    pub taker: ParticipantId,

    pub pair: TradingPair,

    pub price: Price,
    pub quantity: Amount,
}

40. Trade Creation
Order A
   +
Order B
   |
   v
Match
   |
   v
Trade
   |
   v
Swap
This distinction will become extremely valuable when we add multiple settlement mechanisms.

41. opendex-wallet
Wallet access is an explicit security boundary.
pub trait Wallet {
    type Address;
    type Transaction;
    type Error;

    fn addresses(&self) -> Result<Vec<Self::Address>, Self::Error>;

    fn balance(&self) -> Result<Balance, Self::Error>;

    fn sign(
        &self,
        request: SigningRequest,
    ) -> Result<SignedTransaction, Self::Error>;
}

42. Wallet Must Not Trust the Swap Engine
Before signing, the wallet should validate:
network
recipient
amount
transaction type
swap ID
protocol commitment
where the underlying wallet architecture supports those checks.
The goal is:
compromised server
        |
        X
        |
   cannot arbitrarily
   drain wallet

43. Bitcoin Wallet
opendex-bitcoin
       |
       v
BitcoinWallet
Responsibilities:
UTXO discovery
fee estimation
transaction construction
transaction signing
broadcast
confirmation

44. Bitcoin Chain Interface
pub trait BitcoinChain {
    fn transaction(
        &self,
        txid: Txid,
    ) -> Result<Option<BitcoinTransaction>, ChainError>;

    fn tip(&self) -> Result<BlockTip, ChainError>;

    fn broadcast(
        &self,
        tx: RawTransaction,
    ) -> Result<Txid, ChainError>;

    fn confirmations(
        &self,
        txid: Txid,
    ) -> Result<u32, ChainError>;
}

45. Bitcoin Backend
The chain interface allows:
Bitcoin Core RPC
Electrum
Esplora
other backend
without changing the swap engine.

46. Monero Chain Interface
pub trait MoneroChain {
    fn transaction(
        &self,
        txid: MoneroTxid,
    ) -> Result<Option<MoneroTransaction>, ChainError>;

    fn height(&self) -> Result<BlockHeight, ChainError>;

    fn broadcast(
        &self,
        tx: RawMoneroTransaction,
    ) -> Result<MoneroTxid, ChainError>;
}

47. Monero Wallet
The Monero wallet implementation should remain isolated.
opendex-monero
       |
       v
MoneroWallet
The application should not directly manipulate raw Monero private spend keys.

48. opendex-chain
This crate defines generic chain abstractions.
pub trait Chain {
    type Tx;
    type TxId;
    type BlockHeight;

    fn tip(&self) -> Result<Self::BlockHeight, ChainError>;
}
Bitcoin- and Monero-specific behavior remains in their respective crates.

49. opendex-transport
Transport must be independent from protocol state.
pub trait Transport {
    type Message;
    type Error;

    async fn send(
        &self,
        peer: PeerId,
        message: Self::Message,
    ) -> Result<(), Self::Error>;

    async fn receive(
        &self,
    ) -> Result<InboundMessage<Self::Message>, Self::Error>;
}

50. Transport Implementations
Potential implementations:
Direct P2P
Tor
libp2p
relay
Hermes-compatible channel
The current Eigenwallet implementation already has libp2p relay functionality in development and Hermes support for blockchain-mediated messaging. GitHub+1
Open Dex should therefore avoid coupling its state machine to a single network transport.

51. Message Envelope
pub struct ProtocolEnvelope {
    pub protocol: ProtocolVersion,
    pub swap_id: SwapId,
    pub message_id: MessageId,
    pub sender: ParticipantId,
    pub payload: Vec<u8>,
}

52. Message Replay Protection
Every message requires:
message_id
swap_id
protocol_version
sequence / nonce
The receiver maintains:
seen_message_ids
or equivalent protocol-level replay protection.

53. opendex-watchers
Watchers monitor external state.
Responsibilities:
Bitcoin funding
Bitcoin redemption
Bitcoin refund
Monero funding
Monero redemption
Monero refund
confirmations
reorganizations

54. Watcher Architecture
                 Watcher Supervisor
                        |
          +-------------+-------------+
          |                           |
          v                           v
   Bitcoin Watcher             Monero Watcher
          |                           |
          v                           v
   Chain Backend                Chain Backend
          |                           |
          +-------------+-------------+
                        |
                        v
                   Event Bus

55. Watchers Are Read-Only
A watcher should not decide:
"send money"
It reports:
"this transaction exists"
The protocol state machine decides what that means.

56. opendex-recovery
Recovery is important enough to be its own crate.
pub trait RecoveryEngine {
    fn evaluate(
        &self,
        state: &SwapState,
        chain: &ChainState,
    ) -> Result<RecoveryDecision, RecoveryError>;
}

57. Recovery Decisions
pub enum RecoveryDecision {
    Wait,
    Redeem,
    Refund,
    Punish,
    Abort,
}

58. Recovery Must Be Deterministic
Given:
same protocol state
+
same blockchain state
+
same time
the recovery engine must return:
same decision
This makes recovery testable and prevents operator discretion from becoming part of the settlement mechanism.

59. opendex-storage
Recommended initial database:
PostgreSQL
The storage layer exposes repositories rather than raw SQL to the rest of the application.

60. Storage Interfaces
pub trait SwapRepository {
    fn create(&self, swap: &Swap) -> Result<(), StorageError>;

    fn get(
        &self,
        id: SwapId,
    ) -> Result<Option<Swap>, StorageError>;

    fn update_state(
        &self,
        id: SwapId,
        state: SwapState,
    ) -> Result<(), StorageError>;
}

61. Event Repository
pub trait EventRepository {
    fn append(
        &self,
        event: Event,
    ) -> Result<EventId, StorageError>;

    fn load_after(
        &self,
        sequence: u64,
    ) -> Result<Vec<Event>, StorageError>;
}

62. Transaction Intent Repository
pub trait TransactionIntentRepository {
    fn create(
        &self,
        intent: TransactionIntent,
    ) -> Result<(), StorageError>;

    fn find(
        &self,
        swap_id: SwapId,
        tx_type: TransactionType,
    ) -> Result<Option<TransactionIntent>, StorageError>;
}
This prevents duplicate publication after process restarts.

63. Idempotency
Every external action must have an idempotency key.
Examples:
swap_id + action_type
or:
transaction_intent_id
The system should be able to safely repeat:
check
construct
publish
observe
operations.

64. Crash Recovery
Startup sequence:
Open Dex starts
       |
       v
Load incomplete swaps
       |
       v
Load transaction intents
       |
       v
Query chains
       |
       v
Reconcile
       |
       v
Resume state machines
Never assume:
process died = transaction did not happen

65. Reconciliation
The reconciler compares:
database state
against:
blockchain state
Example:
DB:
XmrRedeemConstructed

Chain:
XMR redeem transaction already exists
Result:
XmrRedeemPublished
rather than constructing a second transaction.
This is especially important because the upstream Eigenwallet implementation has added explicit handling for already-on-chain Monero redeem/refund transactions after restart. GitHub

66. opendex-api
The API exposes the marketplace and swap state.
It should not expose cryptographic secrets.
Example endpoints:
GET  /markets
GET  /orders
POST /orders
DELETE /orders/:id

POST /trades
GET  /trades/:id

GET  /swaps/:id
POST /swaps/:id/cancel

67. API Does Not Control Settlement
The API may request:
cancel
but the protocol decides whether cancellation is actually possible.
Bad:
POST /swap/cancel
=> immediately refund funds
Correct:
POST /swap/cancel
       |
       v
protocol checks eligibility
       |
       v
RecoveryDecision

68. opendex-node
This is the application composition layer.
It wires together:
database
market
matching
swap engine
protocol registry
wallets
chain backends
watchers
transport
API
No business logic should live here.

69. Application Dependency Injection
Conceptually:
let bitcoin = BitcoinBackend::new(...);
let monero = MoneroBackend::new(...);

let protocol = EigenwalletBtcXmrAdapter::new(
    bitcoin.clone(),
    monero.clone(),
);

let swap_engine = SwapEngine::new(
    protocol_registry,
    wallet_manager,
    watcher_manager,
);

let market = Market::new(
    matching_engine,
    swap_engine,
);

70. The Order Book Never Imports Eigenwallet
This is a hard architectural rule.
The dependency graph should never become:
market
  |
  +--> eigenwallet
Instead:
market
  |
  v
swap
  |
  v
protocol
  |
  v
btc-xmr adapter
  |
  v
eigenwallet

71. Protocol Capability Model
Future protocols may have different capabilities.
pub struct ProtocolCapabilities {
    pub atomic: bool,
    pub requires_interactive_messages: bool,
    pub supports_recovery: bool,
    pub supports_offline_continuation: bool,
    pub supported_assets: Vec<Asset>,
}

72. Example
BTC/XMR:
atomic = true
recovery = true
offline_continuation = protocol-dependent
A future protocol might be:
atomic = true
interactive = false
The marketplace does not need to care.

73. Pair Registry
pub struct TradingPair {
    pub base: Asset,
    pub quote: Asset,
}
Registry:
BTC/XMR
GBP/BTC
GBP/XMR
EUR/BTC
EUR/XMR
However:
TradingPair
does not imply:
SettlementProtocol

74. Settlement Routing
Trading Pair
     |
     v
Settlement Resolver
     |
     +---- BTC/XMR ----> Atomic Swap
     |
     +---- GBP/BTC ----> Fiat Provider
     |
     +---- GBP/XMR ----> Fiat Provider
This is how the cash market can eventually be added without contaminating the BTC/XMR cryptographic architecture.

75. Fiat Separation
When fiat is added:
opendex-fiat/
should be a separate subsystem.
It should handle:
payment method
KYC/AML integration
payment provider
bank transfer
cash settlement
chargeback risk
disputes
security deposits
The BTC/XMR protocol crate should remain unaware of all of this.

76. Security Boundary Diagram
                  UNTRUSTED
                     |
          +----------+----------+
          |                     |
       Internet              Counterparty
          |                     |
          +----------+----------+
                     |
                  API/P2P
                     |
                     v
             VALIDATION LAYER
                     |
                     v
             STATE MACHINE
                     |
             +-------+-------+
             |               |
             v               v
          PROTOCOL         WALLET
             |               |
             v               v
          CHAIN --------> CHAIN

77. Trust Assumptions
Open Dex should assume:
API server may be compromised
database may be corrupted
relay may be malicious
counterparty may be malicious
blockchain RPC may lie
peer may send malformed data
process may crash
network may disappear
Security-critical decisions must therefore be independently validated.

78. RPC Trust
Never blindly trust:
"transaction confirmed"
from an RPC response.
The watcher should verify the transaction against the relevant chain backend and protocol expectations.

79. Multiple Backend Strategy
For production:
Bitcoin:
    primary backend
    secondary backend

Monero:
    primary backend
    secondary backend
Disagreement should result in:
CHAIN_STATE_UNKNOWN
rather than guessing.

80. Protocol Version Negotiation
At swap setup:
maker supported protocols
        +
taker supported protocols
        |
        v
common protocol
Example:
Maker:
OPENDEX-BTC-XMR-1

Taker:
OPENDEX-BTC-XMR-1
OPENDEX-BTC-XMR-2

=> select version 1
Only approved versions can be selected.

81. No Silent Downgrade
If the participants cannot agree:
NO_COMMON_PROTOCOL
The swap must not automatically fall back to an older protocol unless downgrade policy explicitly allows it.

82. Protocol Negotiation Message
pub struct ProtocolOffer {
    pub swap_id: SwapId,
    pub supported: Vec<ProtocolVersion>,
}
Response:
pub struct ProtocolSelection {
    pub swap_id: SwapId,
    pub selected: ProtocolVersion,
}
Both sides must authenticate/bind the selection to the swap.

83. Configuration
Separate:
protocol configuration
from:
market configuration
Example:
[protocols.btc_xmr]
enabled = true
version = "OPENDEX-BTC-XMR-1"

[market.btc_xmr]
maker_fee_bps = 10
taker_fee_bps = 30

84. Mainnet Configuration
Never allow:
network = "mainnet"
to be selected dynamically from user input.
The node configuration determines:
network
protocol version
approved commit

85. Testnet Configuration
The exact same architecture must support:
Bitcoin testnet
Monero testnet/stagenet
without changing protocol code.

86. Development Modes
Three modes:
Development
Integration
Production
Development may use:
regtest
local Monero network
mock wallet
mock transport

87. Mock Chain
For unit testing:
pub struct MockBitcoinChain;
pub struct MockMoneroChain;
They implement the production chain interfaces.

88. Simulated Swap
A full integration test should be capable of:
create maker
      |
create taker
      |
match order
      |
initialize swap
      |
fund BTC
      |
fund XMR
      |
redeem
      |
extract witness
      |
complete
without real mainnet funds.

89. Failure Simulation
The same test harness should simulate:
maker disappears
taker disappears
message delayed
message duplicated
message reordered
RPC unavailable
transaction delayed
transaction rejected
process crashes
database restart
chain reorg

90. State Machine Testing
For every state:
valid transitions
invalid transitions
recovery transitions
must be explicitly tested.
Example:
Created
  |
  v
Negotiating
  |
  v
ProtocolSetup
  |
  v
Funding
But:
Created
  |
  X
Completed
must be impossible.

91. Typestate Consideration
For cryptographic transaction construction, Rust typestate may be useful.
Example:
UnsignedTransaction
      |
      v
VerifiedTransaction
      |
      v
SignedTransaction
      |
      v
BroadcastableTransaction
This prevents accidentally passing an unsigned or unchecked transaction into a broadcast function.

92. Broadcast API
Prefer:
fn broadcast(
    tx: BroadcastableTransaction
) -> Result<TxId, BroadcastError>;
over:
fn broadcast(
    raw_bytes: Vec<u8>
)
The type itself should communicate that validation has occurred.

93. Secret Types
Use distinct types:
pub struct SecretScalar(...);
pub struct Witness(...);
pub struct PrivateKey(...);
pub struct EncryptedSecret(...);
Do not use:
Vec<u8>
everywhere.
Strong types reduce accidental secret misuse.

94. Serialization
All wire structures should use explicit versioned serialization.
For example:
#[derive(Serialize, Deserialize)]
struct ProtocolMessageV1 {
    version: u16,
    ...
}
Changing the cryptographic meaning of a field requires a protocol version.

95. API Serialization vs Protocol Serialization
Do not use the same serialization automatically for:
HTTP API
and:
cryptographic protocol
They have different requirements.
API:
JSON
may be appropriate.
Cryptographic protocol:
canonical binary encoding
is preferred.

96. Database Serialization
Database schema is a third concern.
Therefore:
API JSON
!=
Protocol encoding
!=
Database representation
Each has its own compatibility rules.

97. Security Logging
Structured logs should contain:
timestamp
swap_id
trade_id
event
protocol
state
txid
Never:
private key
seed
witness
secret
raw signed message

98. Metrics
Production metrics:
swap_created_total
swap_completed_total
swap_failed_total
swap_refunded_total

swap_duration_seconds

btc_funding_confirmations
xmr_funding_confirmations

protocol_error_total
chain_error_total

broadcast_failure_total

99. Do Not Log User Identity to Blockchain Observers
The blockchain watcher should know:
swap_id
transaction
but should not require:
real-world identity
This keeps settlement infrastructure modular.

100. Security Monitoring
Alert on:
unusual refund rate
unusual failed swaps
unexpected protocol errors
chain backend disagreement
repeated malformed proofs
repeated invalid messages
abnormal peer behavior

101. Anti-DoS Boundary
The marketplace must rate-limit:
order creation
swap initialization
protocol messages
peer connections
before expensive cryptographic operations where possible.

102. Cryptographic Work Limits
Never allow an unauthenticated peer to trigger unlimited:
DLEQ verification
signature verification
transaction parsing
without resource controls.

103. Peer Reputation
Peer reputation can exist at the marketplace layer:
successful swaps
failed swaps
timeouts
malicious behavior
But reputation must not alter the cryptographic rules.

104. Security Bond
The earlier security-bond model can eventually sit above the swap:
Order
 |
 +--> maker bond
 |
 +--> taker bond
 |
 v
Trade
 |
 v
Atomic Swap
The bond mechanism should never modify the underlying BTC/XMR cryptographic construction.

105. Anti-Spam Deposit Compatibility
Current Eigenwallet releases include an anti-spam deposit concept in its refund policy. Open Dex should treat such upstream protocol features as protocol-specific behavior, not assume they are a universal Open Dex bond mechanism. GitHub
This distinction is important.
Open Dex bond
      !=
Eigenwallet protocol deposit
They may eventually interact, but they should remain separate concepts.

106. Repository Layout
The complete repository should eventually look like:
opendex/
│
├── crates/
│   ├── opendex-types/
│   ├── opendex-errors/
│   ├── opendex-events/
│   ├── opendex-protocol-types/
│   ├── opendex-protocol/
│   ├── opendex-btc-xmr/
│   ├── opendex-bitcoin/
│   ├── opendex-monero/
│   ├── opendex-wallet/
│   ├── opendex-chain/
│   ├── opendex-transport/
│   ├── opendex-swap/
│   ├── opendex-market/
│   ├── opendex-matching/
│   ├── opendex-storage/
│   ├── opendex-watchers/
│   ├── opendex-recovery/
│   ├── opendex-api/
│   └── opendex-node/
│
├── integration-tests/
│
├── fixtures/
│   ├── btc-xmr/
│   └── protocols/
│
├── docs/
│   ├── architecture/
│   ├── protocols/
│   └── security/
│
└── Cargo.toml

107. Initial MVP Crates
Do not build every crate before running a swap.
The first executable milestone only needs:
opendex-types
opendex-errors
opendex-protocol-types
opendex-protocol
opendex-btc-xmr
opendex-bitcoin
opendex-monero
opendex-wallet
opendex-chain
opendex-swap
Then add:
storage
watchers
transport
market
matching
api
incrementally.

108. Milestone 1
"One BTC/XMR Swap"
Build:
Maker
  |
  v
Taker
  |
  v
Swap Engine
  |
  v
Eigenwallet Adapter
  |
  +--> Bitcoin
  |
  +--> Monero
No order book.
No public marketplace.
No fiat.
No GUI.

109. Milestone 1 Acceptance Test
Two local participants must be able to:
1. negotiate protocol
2. create swap
3. exchange protocol messages
4. fund required chains
5. detect funding
6. redeem
7. extract required witness
8. complete
and the entire process must be restartable.

110. Milestone 2
Add:
persistent state
event log
watchers
reconciliation
recovery
The acceptance criterion becomes:
Kill either process at any point and restart it without losing the swap state.

111. Milestone 3
Add:
P2P transport
peer discovery
authenticated messaging
The swap should no longer require local process-to-process coordination.

112. Milestone 4
Add:
maker service
quotes
order creation
matching
trade creation
Now Open Dex becomes a marketplace.

113. Milestone 5
Add:
public API
web frontend
account/reputation layer
The UI comes last because it should consume the already-stable domain/API model.

114. Milestone 6
Add:
fiat subsystem
payment methods
compliance architecture
cash-market risk controls
This remains a separate subsystem.

115. Testing Pyramid
                 E2E
                /   \
             Chain Integration
              /       \
        Protocol Integration
            /           \
       Unit + Property + Fuzz
The majority of tests should remain below the E2E layer.

116. Unit Test Requirements
Every crate must have:
happy path
invalid input
boundary conditions
error propagation
serialization tests
Cryptographic crates additionally require:
negative vectors
malformed input
cross-version rejection

117. Integration Test Requirements
At minimum:
successful swap
maker timeout
taker timeout
BTC funding failure
XMR funding failure
BTC redemption
XMR redemption
BTC refund
XMR refund
restart recovery
duplicate message
replayed message
chain reorg

118. Fuzzing
Initial fuzz targets:
protocol messages
Bitcoin transactions
Monero transaction metadata
proofs
signatures
state transitions
serialization

119. Dependency Security
Run:
cargo audit
cargo deny
cargo clippy
cargo fmt --check
in CI.
Pin cryptographic dependencies where practical.

120. Reproducible Builds
Production binaries should be built from:
locked Cargo.lock
pinned Rust toolchain
pinned upstream protocol revision
Build metadata should record:
git commit
protocol version
Rust version
dependency lock hash

121. Supply Chain
For the Eigenwallet dependency:
upstream source
      |
      v
reviewed revision
      |
      v
Cargo.lock
      |
      v
Open Dex release
A dependency update should require review.

122. Updating Eigenwallet
When upstream releases a new version:
new release
    |
    v
security diff
    |
    v
protocol diff
    |
    v
transaction diff
    |
    v
test vectors
    |
    v
Open Dex review
    |
    v
approve
Do not simply run:
cargo update
and deploy.

123. Why This Matters
Eigenwallet is actively maintained and currently has a large Rust monorepo with thousands of commits. Its recent history demonstrates that seemingly small releases can modify important settlement behavior. GitHub+1
Therefore Open Dex needs its own compatibility discipline.

124. Protocol Approval Record
Every production protocol should have:
Protocol ID
Upstream repository
Upstream revision
Audit status
Test status
Supported networks
Approval date
Approvers
Known limitations

125. Security Review Record
Example:
protocol: OPENDEX-BTC-XMR-1

upstream:
  repository: eigenwallet/core
  revision: "<commit>"

review:
  status: approved
  reviewer: "<reviewer>"
  date: "<date>"

tests:
  unit: passing
  integration: passing
  fuzzing: passing
  recovery: passing

networks:
  bitcoin: mainnet
  monero: mainnet

126. The Critical Boundary
The most important interface in the entire project is:
trait SettlementProtocol
Everything above it should be marketplace-neutral.
Everything below it should be settlement-specific.
That means:
                MARKETPLACE
                     |
                     |
              SettlementProtocol
                     |
        +------------+------------+
        |                         |
      BTC/XMR                   Future
        |
 Eigenwallet Adapter
        |
 Bitcoin + Monero

127. Future Pair Example
Suppose Open Dex eventually adds:
BTC/ETH
The architecture should become:
BTC/XMR
    |
    +--> Eigenwallet adapter

BTC/ETH
    |
    +--> ETH atomic-swap adapter

BTC/XYZ
    |
    +--> XYZ adapter
The order book remains unchanged.

128. Future Settlement Example
Suppose a future protocol supports:
BTC
   |
   v
Lightning
The marketplace should only need:
SettlementProtocol
It should not require a rewrite.

129. Marketplace Core Therefore Becomes Protocol-Agnostic
The ideal dependency is:
Open Dex
   |
   +--> Asset Registry
   |
   +--> Order Book
   |
   +--> Matching
   |
   +--> Trade Engine
   |
   +--> Settlement Resolver
                     |
                     +--> Protocol A
                     +--> Protocol B
                     +--> Protocol C
This is the long-term architecture.

130. First Concrete Repository Target
The first code we should actually write is:
opendex/
├── Cargo.toml
└── crates/
    ├── opendex-types/
    ├── opendex-errors/
    ├── opendex-protocol-types/
    ├── opendex-protocol/
    ├── opendex-btc-xmr/
    ├── opendex-bitcoin/
    ├── opendex-monero/
    ├── opendex-wallet/
    ├── opendex-chain/
    └── opendex-swap/
Then implement the following vertical slice:
SwapContext
      |
      v
ProtocolRegistry
      |
      v
EigenwalletBtcXmrAdapter
      |
      v
Bitcoin/Monero interfaces
      |
      v
Mock chains
      |
      v
Successful simulated swap

131. First Code Deliverables
The first implementation PR should contain:
[ ] Cargo workspace
[ ] opendex-types
[ ] opendex-errors
[ ] protocol traits
[ ] protocol registry
[ ] BTC/XMR adapter skeleton
[ ] Bitcoin chain trait
[ ] Monero chain trait
[ ] wallet trait
[ ] swap state machine
[ ] mock chain implementations
[ ] one end-to-end simulated swap test
It should not contain:
[ ] web frontend
[ ] order book UI
[ ] fiat
[ ] token
[ ] governance
[ ] production mainnet deployment

132. Definition of Done
The architecture is v0.1 complete when:
Open Dex can create a Trade
        |
        v
create a Swap
        |
        v
select OPENDEX-BTC-XMR-1
        |
        v
invoke Eigenwallet adapter
        |
        v
construct protocol actions
        |
        v
observe mock Bitcoin/Monero chains
        |
        v
reach Completed or Recoverable state
without any marketplace crate knowing the internal details of DLEQ, adaptor signatures, Monero key shares, or Bitcoin transaction construction.

133. Final Architecture
                         OPEN DEX NODE
                              |
            +-----------------+-----------------+
            |                 |                 |
            v                 v                 v
        MARKET            MATCHING           API
            |                 |
            +--------+--------+
                     |
                     v
                  TRADE
                     |
                     v
                   SWAP
                     |
                     v
             PROTOCOL REGISTRY
                     |
                     v
          OPENDEX-BTC-XMR-1
                     |
                     v
          EIGENWALLET ADAPTER
                     |
             +-------+-------+
             |               |
             v               v
          BITCOIN          MONERO
          ADAPTER          ADAPTER
             |               |
             v               v
          WALLET           WALLET
             |               |
             +-------+-------+
                     |
                     v
                 CHAINS
The resulting principle is simple:
Open Dex decides who trades, at what price, for what fee, and when a trade exists. The settlement protocol decides how the assets move safely.
That separation is the rail we should build the rest of Open Dex on.
What I would build next
The natural next document is now Open Dex Repository Bootstrap & Rust Interface Skeleton v0.1. That one can get much more concrete: actual Cargo.toml workspace layout, crate dependencies, module trees, Rust trait definitions, state enums, database tables, and the first MockBitcoinChain / MockMoneroChain test harness.
The key is that we now have enough architecture to start writing code without prematurely coupling the exchange to Eigenwallet's internals. Eigenwallet's own repository is already structured as a Rust monorepo with swap, maker/taker, GUI, and supporting crates, so we'll borrow the useful separation while keeping Open Dex's marketplace layer completely independent. GitHub
Eigenwallet Core repository
Eigenwallet protocol repository

---

## Implementation Status — 2026-10-06

Milestones 1-3 completed. The following items from this specification have been implemented:

- **SettlementProtocol trait** — Implemented in `opendex-protocol-types` with `initialize()`, `process_message()`, `observe_chain()` methods
- **Protocol registry** — Implemented in `opendex-protocol` with version negotiation
- **BtcXmrAdapter skeleton** — Implemented in `opendex-btc-xmr` (stub logic, no real Eigenwallet integration)
- **BitcoinChain / MoneroChain traits** — Implemented in `opendex-bitcoin` and `opendex-monero` with mock implementations
- **Generic Chain trait** — Implemented in `opendex-chain`
- **Swap state machine** — Implemented in `opendex-protocol-types` with 9 states and validated transitions
- **SwapEngine** — Implemented in `opendex-swap` with persistence, recovery, and transport integration
- **Event sourcing** — Implemented in `opendex-events` with append-only event log
- **File-based storage** — Implemented in `opendex-storage` with atomic writes
- **Blockchain watchers** — Implemented in `opendex-watchers` with supervisor coordination
- **Deterministic recovery** — Implemented in `opendex-recovery`
- **Transport abstraction** — Implemented in `opendex-transport` with async Transport trait
- **Message replay protection** — Implemented via `MessageStore` in `opendex-transport`
- **Domain types** — Implemented in `opendex-types` (SwapId, Satoshis, Piconero, Asset, Price, etc.)
- **Error hierarchy** — Implemented in `opendex-errors`

Not yet implemented:
- Real Eigenwallet integration (DLEQ proofs, adaptor signatures)
- Real Bitcoin/Monero chain RPC
- Real libp2p transport integration
- Order book and matching (M4)
- Public API and GUI (M5)
- Fiat subsystem (M6)
