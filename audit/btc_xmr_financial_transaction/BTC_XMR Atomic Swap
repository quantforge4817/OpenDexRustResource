Open Dex should not invent its own BTC/XMR cryptographic construction. The COMIT BTC/XMR implementation and the underlying h4sh3d research provide the basis, while the original COMIT implementation is now archived/unmaintained and explicitly warns that its cryptography has not been formally audited. GitHub+1
The protocol below therefore treats the cryptographic primitive as a versioned external protocol component, while specifying exactly how Open Dex should wrap it with its marketplace, state machine, watchers, timeouts, fees, and recovery logic.
Open Dex
Protocol Specification v0.1 — BTC/XMR Atomic Swap
Status: Engineering specification / pre-implementation
Version: 0.1
Scope: BTC ↔ XMR atomic swaps
Settlement model: Non-custodial, cryptographically coordinated cross-chain swap
Primary implementation language: Rust recommended for settlement-critical components
Application layer: TypeScript/Node.js acceptable around the settlement engine

1. Purpose
This specification defines how Open Dex should execute a BTC ↔ XMR atomic swap.
It defines:
    • participant roles
    • protocol phases
    • message flow
    • swap state machine
    • cryptographic-material lifecycle
    • Bitcoin-side settlement
    • Monero-side settlement
    • blockchain monitoring
    • timeout handling
    • refund handling
    • failure recovery
    • database persistence
    • security invariants
    • integration boundaries
It does not define new cryptographic primitives.
Open Dex should integrate a reviewed implementation of an established BTC/XMR atomic-swap construction.
Existing research and implementations use adaptor-signature/key-share techniques because Monero does not provide Bitcoin-style scripting/HTLC functionality. The h4sh3d construction specifically uses cross-curve proofs and adaptor signatures to connect the Bitcoin and Monero sides. GitHub+1

2. Design Objective
The desired property is:
Either both parties obtain the asset they traded for, or each party can recover its original asset according to the protocol's timeout/recovery rules.
More precisely:
Normal execution:

Alice gives BTC
        ↓
Bob gives XMR
        ↓
Alice obtains XMR
        ↓
Bob obtains BTC
Failure execution:
Protocol failure
        ↓
timeout/recovery path
        ↓
Alice recovers BTC
        +
Bob recovers XMR
The protocol must prevent a participant from intentionally obtaining both assets through a protocol violation.

3. Roles
Open Dex should define roles independently from UI terminology.
3.1 Alice
For this specification:
Alice = BTC seller
Alice = XMR buyer
She initially controls:
BTC
and wants:
XMR
3.2 Bob
Bob = XMR seller
Bob = BTC buyer
He initially controls:
XMR
and wants:
BTC
These names are protocol roles, not user identities.

4. Important Directionality Decision
v0.1 should initially implement:
Alice:
BTC → XMR

Bob:
XMR → BTC
The reverse direction should use the same underlying protocol with roles inverted.
Do not implement two independently designed protocols.
There should be:
SwapDirection
with:
BTC_TO_XMR
XMR_TO_BTC
and one underlying settlement implementation.

5. Cryptographic Architecture
The protocol relies on cryptographic relationships between:
Bitcoin secp256k1
and:
Monero Ed25519-based key material
Existing BTC/XMR constructions use cross-curve proofs to establish that related secret material represents the same underlying scalar across the two cryptographic groups. GitHub+1
The Open Dex implementation must therefore contain a dedicated:
CryptoAdapter
rather than scattering curve-specific operations throughout the application.

6. Crypto Adapter
Conceptual interface:
trait BtcXmrSwapCrypto {
    fn generate_swap_secrets(...);
    fn generate_public_commitments(...);
    fn create_dleq_proof(...);
    fn verify_dleq_proof(...);

    fn create_adaptor_signature(...);
    fn verify_adaptor_signature(...);
    fn adapt_signature(...);
    fn extract_secret(...);

    fn derive_monero_spend_material(...);
}
The actual cryptographic operations must come from the selected protocol implementation/library.
The application should never implement:
ECDSA
Schnorr
DLEQ
adaptor signatures
Monero key derivation
itself.

7. Protocol Versioning
Every swap must record:
protocol_name
protocol_version
crypto_library_version
network
Example:
protocol:
    open-dex-btc-xmr

version:
    0.1

crypto implementation:
    implementation identifier

network:
    bitcoin-mainnet
    monero-mainnet
This is essential because cryptographic protocol implementations can change.

8. Swap Identifier
Every swap receives a globally unique identifier:
swap_id
Example:
swap_01J...
The identifier must not itself contain:
    • private keys
    • secrets
    • wallet addresses
    • personally identifying information
It can be used to correlate application events.

9. Swap Commitment
Before execution, both parties must agree on a canonical swap description.
Conceptually:
SwapCommitment {
    swap_id
    protocol_version

    btc_amount
    xmr_amount

    btc_network
    xmr_network

    maker
    taker

    timeout_parameters

    fee_parameters
}
The commitment should be hashed using the cryptographic protocol's prescribed construction.
The purpose is to prevent either party from changing:
amount
asset
participant
timeout
destination
after cryptographic setup.

10. Price and Fee Separation
The atomic swap itself receives the agreed settlement amounts.
For example:
Maker quote:

1 BTC = 100 XMR
Suppose the taker trades:
0.1 BTC
Then the swap amount might be:
BTC:
0.10000000 BTC

XMR:
10 XMR
Protocol fees should be accounted for before the swap amount is committed.
The swap engine must receive exact amounts.
It should never independently reinterpret a market quote.

11. Recommended Economic Model
At the application layer:
gross trade
    ↓
protocol fee
    ↓
network-cost allocation
    ↓
net settlement amounts
    ↓
atomic swap
The atomic swap engine should not calculate business fees.
It receives:
BTC settlement amount
XMR settlement amount
and executes the cryptographic settlement.

12. High-Level Protocol
Conceptually:
Alice                              Bob
  |                                 |
  |--- swap request ---------------->|
  |                                 |
  |<-- protocol parameters ---------|
  |                                 |
  |--- cryptographic material ----->|
  |                                 |
  |<-- cryptographic material ------|
  |                                 |
  |========= BTC lock =============>|
  |                                 |
  |<========== XMR lock =============|
  |                                 |
  |--- BTC redemption -------------->|
  |                                 |
  |<--- secret extraction -----------|
  |                                 |
  |<========== XMR redemption =======|
The actual cryptographic messages must follow the selected reference implementation.

13. Phase 0 — Market Agreement
Before cryptography begins:
Maker creates BTC/XMR order
Example:
Sell:
0.25 BTC

Receive:
25 XMR
The taker accepts.
Open Dex creates:
Trade
SwapSession
Both receive the exact same trade commitment.

14. Phase 1 — Swap Initialization
State:
CREATED
Alice and Bob establish:
swap_id
asset amounts
wallet destinations
protocol version
timeouts
The application verifies:
trade exists
trade not expired
trade not already accepted
participants authorized
amounts unchanged
fees locked
Transition:
CREATED
      ↓
INITIALIZING

15. Phase 2 — Cryptographic Setup
Participants generate the required ephemeral cryptographic material.
Important principle:
Swap secrets must be generated locally by the participant and must never be sent to Open Dex.
Open Dex should coordinate public material only.
Conceptually:
Alice:
secret_A
public_A

Bob:
secret_B
public_B
The actual number and type of secrets depends on the selected BTC/XMR protocol implementation.

16. Secret Handling
Secrets must:
    • exist only in secure process memory where possible
    • never be logged
    • never be sent to the API
    • never be stored in plaintext in PostgreSQL
    • never appear in analytics
    • never appear in error messages
If a secret must be persisted for crash recovery, it must use a dedicated secure key-management design.
Prefer protocol designs where recoverability can be achieved without storing raw secrets centrally.

17. Cross-Curve Proof
The protocol needs to establish the required relationship between Bitcoin and Monero cryptographic material.
This is where the cross-curve proof enters.
Conceptually:
Alice secret
     |
     +----> Bitcoin public relation
     |
     +----> Monero public relation
Alice provides a proof:
Proof_A
Bob verifies:
Verify(Proof_A) == true
and vice versa where required by the protocol.
Existing research specifically identifies this cross-curve equality-of-discrete-log relationship as a critical part of the BTC/XMR construction. GitHub+1

18. Verification Gate
No blockchain funding transaction should occur until all required cryptographic setup has passed verification.
Required:
participants valid
+
amounts match
+
proofs valid
+
destination data valid
+
timeouts valid
Then:
CRYPT_SETUP_VERIFIED
Only after this state may funding begin.

19. Phase 3 — Bitcoin Funding
The BTC side is funded first in the initial protocol orientation.
The exact construction uses Bitcoin-side locking/settlement transactions with the cryptographic conditions established during setup.
The Open Dex application should represent this as:
BTC_FUNDING_PENDING
then:
BTC_FUNDING_BROADCAST
then:
BTC_FUNDING_DETECTED
then:
BTC_FUNDING_CONFIRMED

20. Bitcoin Funding Requirements
Before accepting the funding transaction:
txid valid
amount valid
output structure valid
swap conditions valid
network valid
destination/commitment valid
Do not trust:
"the user says they sent BTC."
Only blockchain evidence advances the state.

21. Bitcoin Watcher
Create a dedicated watcher:
BitcoinWatcher
Responsibilities:
observe transaction
verify transaction
track confirmations
detect replacement/reorg
emit normalized events
Example:
BitcoinFundingSeen
BitcoinFundingConfirmed
BitcoinFundingFinal
BitcoinFundingReorged

22. Confirmation Policy
The swap engine should not define arbitrary confirmation numbers.
Use:
ConfirmationPolicy
Example:
BTC_FUNDING:
minimum confirmations = configurable

BTC_REDEMPTION:
required confirmations = configurable
The policy can later depend on:
trade size
network conditions
risk class

23. Phase 4 — Monero Funding
Once the Bitcoin funding conditions are satisfied:
BTC_FUNDING_CONFIRMED
the Monero side can proceed.
State:
XMR_FUNDING_PENDING
Bob creates the Monero-side funding transaction using the protocol's prescribed construction.

24. Monero Funding
The Monero transaction must be verified for:
amount
recipient/control structure
transaction validity
required protocol conditions
network
confirmation state
Monero does not provide Bitcoin-style scripting/HTLC semantics, which is why the BTC/XMR construction uses cryptographic key relationships rather than simply copying a Bitcoin HTLC design. GitHub

25. Monero Watcher
Create:
MoneroWatcher
Responsibilities:
detect transaction
identify relevant output
verify amount
verify required protocol conditions
track confirmations
detect chain changes
emit events
Events:
XmrFundingSeen
XmrFundingConfirmed
XmrFundingFinal
XmrFundingReorged

26. Settlement Gate
The trade must not proceed to redemption until:
BTC funding confirmed
+
XMR funding confirmed
+
cryptographic conditions verified
Then:
BOTH_LEGS_FUNDED
This is the most important transition in the swap.

27. Phase 5 — Redemption
The normal path now allows the party with the appropriate cryptographic material to redeem the Bitcoin leg.
The Bitcoin redemption transaction contains the cryptographic evidence required by the adaptor-signature construction.
Conceptually:
Bitcoin adaptor signature
        |
        ↓
completed Bitcoin signature
        |
        ↓
Bitcoin redemption
        |
        ↓
witness becomes observable
Adaptor signatures provide the crucial property that completing/publishing the valid signature can reveal the underlying witness. GitHub

28. Witness Extraction
Bob watches the Bitcoin redemption.
Once the valid completed signature is observed:
BitcoinWatcher
       |
       v
Extract witness
       |
       v
Validate witness
       |
       v
Derive Monero redemption capability
The extracted value must pass protocol validation before being used.

29. Never Trust the Application Database for the Secret
This is critical.
The application should derive:
witness_observed
from the Bitcoin-chain evidence.
It should not simply accept:
secret = API.request.secret
The blockchain is the authoritative evidence.

30. Phase 6 — Monero Redemption
Once the necessary witness/key material is obtained:
XMR_REDEMPTION_READY
Bob constructs the Monero redemption transaction according to the selected protocol.
The transaction is broadcast.
State:
XMR_REDEMPTION_BROADCAST
then:
XMR_REDEMPTION_CONFIRMED

31. Completion
The trade becomes:
COMPLETED
only after the required settlement conditions have been met.
Then:
Bond:
released

Ledger:
finalized

Reputation:
updated

Order:
remaining liquidity updated

Trade:
immutable/completed

32. Complete Happy-Path State Machine
CREATED
   |
   v
INITIALIZING
   |
   v
CRYPTO_SETUP
   |
   v
CRYPTO_SETUP_VERIFIED
   |
   v
BTC_FUNDING_PENDING
   |
   v
BTC_FUNDING_BROADCAST
   |
   v
BTC_FUNDING_CONFIRMED
   |
   v
XMR_FUNDING_PENDING
   |
   v
XMR_FUNDING_BROADCAST
   |
   v
XMR_FUNDING_CONFIRMED
   |
   v
BOTH_LEGS_FUNDED
   |
   v
BTC_REDEMPTION_READY
   |
   v
BTC_REDEEMED
   |
   v
WITNESS_EXTRACTED
   |
   v
XMR_REDEMPTION_READY
   |
   v
XMR_REDEEMED
   |
   v
COMPLETED

33. Failure State Machine
Every active state needs explicit failure transitions.
Conceptually:
                         +--> CANCELLED
                         |
CREATED -----------------+
                         |
                         +--> EXPIRED


CRYPTO_SETUP
     |
     +--> SETUP_FAILED
     |
     +--> TIMEOUT


BTC_FUNDING_PENDING
     |
     +--> TIMEOUT
     |
     +--> INVALID_TX


XMR_FUNDING_PENDING
     |
     +--> TIMEOUT
     |
     +--> INVALID_TX


BOTH_LEGS_FUNDED
     |
     +--> REDEMPTION_TIMEOUT
     |
     +--> RECOVERY


RECOVERY
     |
     +--> REFUNDED
     |
     +--> RESOLUTION_REQUIRED

34. Timeout Design
There should be multiple timeout stages.
Do not use one global:
timeout = 1 hour
Instead:
T_setup
T_btc_funding
T_xmr_funding
T_redemption
T_recovery
The exact values should be determined by the selected cryptographic protocol and blockchain confirmation characteristics.

35. Timeout Rule
A timeout should never be calculated from:
server wall clock alone
For blockchain-sensitive actions, use:
current chain height
+
protocol-defined relative/absolute conditions
where the underlying protocol permits it.
Store both:
application deadline
blockchain deadline
where relevant.

36. Clock Safety
Use:
server time
for:
    • quote expiration
    • API sessions
    • application reservations
Use:
blockchain height/time semantics
for:
    • chain settlement conditions
    • protocol refunds
    • Bitcoin timelocks
Do not confuse the two.

37. Recovery Principle
Recovery is not:
admin presses refund
Recovery must be a deterministic protocol action.
Example:
BTC funding exists
XMR funding never occurred
        ↓
Bitcoin recovery path
        ↓
Alice recovers BTC
Another:
Both legs funded
Alice never redeems
        ↓
protocol timeout
        ↓
Bob's recovery path
        ↓
Alice cannot obtain Bob's XMR
The exact recovery semantics must follow the chosen underlying swap protocol.

38. Recovery State Machine
ACTIVE
  |
  v
TIMEOUT_DETECTED
  |
  v
RECOVERY_ELIGIBLE
  |
  v
RECOVERY_TRANSACTION_CREATED
  |
  v
RECOVERY_BROADCAST
  |
  v
RECOVERY_CONFIRMED
  |
  v
REFUNDED

39. Automated Recovery Worker
Create:
SwapRecoveryWorker
It periodically evaluates:
active swaps
against:
protocol state
blockchain state
timeout state
It must be safe to run repeatedly.
Example:
run worker
run worker
run worker
must not produce three recovery transactions.
Use:
idempotency
+
database locks
+
transaction identifiers

40. Crash Recovery
Suppose Open Dex crashes:
BTC funded
XMR funded
server crashes
When it restarts:
load active swaps
        ↓
query Bitcoin
query Monero
        ↓
reconstruct protocol state
        ↓
resume
The system must never assume:
database state = blockchain state
Blockchain state must be reconciled.

41. Swap Recovery Algorithm
On startup:
for each non-terminal swap:

    load persisted state

    query Bitcoin state

    query Monero state

    verify cryptographic evidence

    compare against expected state

    if valid:
        resume

    if inconsistent:
        freeze swap

        create security alert

        require deterministic recovery path
Never blindly continue after detecting an impossible state.

42. Impossible State
Examples:
BTC redeemed
but no valid BTC transaction exists
or:
XMR redeemed
but protocol never reached BOTH_LEGS_FUNDED
or:
database says completed
but blockchain settlement is absent
These should trigger:
PROTOCOL_INCONSISTENCY
and an operational alert.

43. Database Swap Record
Recommended:
swap_sessions
-------------
id
trade_id

protocol
protocol_version

role
direction

btc_amount
xmr_amount

btc_address_data
xmr_address_data

btc_funding_txid
xmr_funding_txid

btc_redeem_txid
xmr_redeem_txid

state

state_version

setup_deadline
btc_deadline
xmr_deadline
redemption_deadline

created_at
updated_at
completed_at
Sensitive cryptographic material should not be stored here unless the protocol explicitly requires it and a secure storage design has been approved.

44. Swap Events
Create an append-only event stream:
SwapCreated
SwapInitialized
CryptoSetupStarted
CryptoSetupVerified

BtcFundingRequested
BtcFundingSeen
BtcFundingConfirmed

XmrFundingRequested
XmrFundingSeen
XmrFundingConfirmed

BothLegsFunded

BtcRedemptionDetected
WitnessExtracted

XmrRedemptionDetected

SwapCompleted

TimeoutDetected
RecoveryStarted
RecoveryCompleted
SwapFailed

45. State Version
Every state transition increments:
state_version
Example:
state_version = 17
A transition request must specify the expected version.
This protects against concurrent workers.

46. Example
Two workers see:
BOTH_LEGS_FUNDED
Worker A attempts:
→ BTC_REDEMPTION_READY
Worker B attempts the same.
Only one succeeds because:
expected_state_version = 14
After A:
state_version = 15
B fails safely.

47. Blockchain Watcher Architecture
                 BLOCKCHAIN
                     |
          +----------+----------+
          |                     |
      Bitcoin                 Monero
       Node                    Node
          |                     |
          v                     v
   BitcoinWatcher         MoneroWatcher
          |                     |
          +----------+----------+
                     |
               Event Normalizer
                     |
                     v
                Swap Engine

48. Watcher Requirements
Watchers must be:
idempotent
restartable
reorg-aware
rate-limited
observable
They must be able to replay historical blocks/events to reconstruct state.

49. Reorganization Handling
A transaction that was previously observed as confirmed can become invalidated by a chain reorganization.
Therefore:
CONFIRMED
must not necessarily mean:
irreversible
Use:
FINAL
for the protocol's stronger settlement threshold.
If a relevant transaction is reorged:
transaction status
    ↓
REORGED
and the swap state machine evaluates the appropriate recovery path.

50. Wallet Separation
The swap engine should communicate with:
BitcoinWallet
MoneroWallet
through interfaces.
Example:
trait BitcoinWallet {
    fn create_funding_transaction(...);
    fn create_redemption_transaction(...);
    fn broadcast(...);
}

trait MoneroWallet {
    fn prepare_funding(...);
    fn prepare_redemption(...);
    fn broadcast(...);
}
Actual implementation should use well-tested wallet libraries/RPC interfaces.

51. Wallet Authorization
The application must distinguish:
prepare transaction
from:
sign transaction
and:
broadcast transaction
This allows future deployment models where signing is isolated.

52. Recommended Signing Architecture
Swap Engine
     |
     | unsigned transaction / signing request
     v
Signing Boundary
     |
     v
Secure Wallet
     |
     v
Signed transaction
     |
     v
Swap Engine
     |
     v
Blockchain
The swap engine should not automatically have unrestricted wallet authority.

53. Mainnet Wallet Security
For production:
hot wallet
should contain only the liquidity necessary for active trading.
Do not store the entirety of a market maker's treasury in the hot wallet.
Use:
operational liquidity
+
cold reserve
where applicable.

54. Liquidity Management
A maker should have:
available BTC
available XMR
reserved BTC
reserved XMR
Example:
BTC:
10 BTC available
2 BTC reserved

XMR:
1,000 XMR available
250 XMR reserved
An offer cannot exceed available liquidity.

55. Liquidity Reservation
When a maker creates an order:
order amount
does not necessarily mean:
funds permanently locked
Instead:
available
    ↓
reserved when matched
    ↓
released on cancel/failure
    ↓
consumed on completion
This is an application-level reservation.
Actual blockchain locking occurs during the swap.

56. Maker Failure
If the maker disappears before funding:
trade timeout
The trade should:
cancel
release reservation
record failed completion
update reputation
No protocol funds should be lost merely because a maker stopped responding before the blockchain settlement phase.

57. Taker Failure
If the taker accepts but never performs the required protocol action:
timeout
Then:
trade expires
reservation released
reputation updated
If blockchain funds have already been committed, the underlying swap recovery mechanism takes precedence.

58. Fee Immutability
Once a trade enters:
ACCEPTED
its fees are frozen.
Changing:
current fee schedule
must not change an existing trade.
The trade stores:
maker_fee
taker_fee
network_allocation
fee_schedule_version

59. Network Fee Handling
Bitcoin transaction fees and Monero transaction fees are not protocol profit.
Track them separately:
protocol_fee
network_fee
network_buffer
Example:
Protocol:
£2.50

Estimated network:
£0.80

Buffer:
£0.12
The exact settlement accounting should be defined before launch.

60. Network Cost Estimator
NetworkFeeEstimator
       |
       +-- Bitcoin
       |
       +-- Monero
Inputs:
transaction type
estimated size
current fee conditions
priority
Outputs:
estimated fee
confidence
timestamp
Example:
{
    estimated: 8200 sats,
    confidence: HIGH,
    timestamp: ...
}

61. No Silent Fee Changes
If network conditions change substantially between quote and execution:
quote
   ↓
network conditions changed
   ↓
fee outside allowed tolerance
the trade should either:
reprice before commitment
or:
abort safely
Never silently deduct additional funds.

62. Protocol Messages
The exact message schema should follow the underlying swap protocol, but Open Dex should wrap messages in a transport envelope:
{
  "protocol": "open-dex-btc-xmr",
  "version": 1,
  "swapId": "...",
  "messageId": "...",
  "sequence": 7,
  "type": "...",
  "payload": "...",
  "createdAt": "...",
  "signature": "..."
}

63. Message Properties
Every message should provide:
message_id
swap_id
sequence
protocol_version
message_type
payload
authentication
Messages must be:
authenticated
ordered where required
replay-resistant
idempotent

64. Transport
The marketplace API and the cryptographic peer protocol should be conceptually separate.
Open Dex API
    |
    +-- marketplace
    +-- trade coordination

Swap Peer Protocol
    |
    +-- cryptographic messages
    +-- settlement coordination
This allows the underlying swap protocol eventually to operate independently of the Open Dex web application.

65. Peer Communication
The initial implementation can use an Open Dex relay:
Alice
  |
  v
Open Dex Relay
  |
  v
Bob
But the protocol messages should be designed so that later they can support:
Alice
  |
  +-----------> Bob
directly.
Open Dex should not become a cryptographic trusted third party merely because it relays messages.

66. Message Security
Transport encryption:
TLS
is not sufficient by itself.
Important protocol messages should also have cryptographic authentication where required.
This protects against:
relay compromise
message modification
replay

67. Replay Protection
Each message includes:
swap_id
message_id
sequence
The receiver maintains:
last_processed_sequence
Duplicate message:
same message_id
must return the existing result rather than executing the operation twice.

68. Swap Authentication
A participant should prove control over the identity/wallet credentials required by the selected protocol.
Open Dex account authentication should not be treated as equivalent to cryptographic control of the swap.

69. Privacy
The swap protocol should minimize unnecessary metadata.
Do not put:
user email
IP address
account ID
KYC ID
payment information
into blockchain transactions or cryptographic commitments unless absolutely required.

70. On-Chain Linkability
One major advantage of adaptor-signature-based approaches is avoiding a simple shared HTLC preimage that visibly links both chains.
Existing research specifically identifies cross-chain linkability as a motivation for scriptless/adaptor-signature constructions. GitHub
Open Dex should therefore avoid adding application-specific identifiers to blockchain transactions merely for analytics.
Use off-chain identifiers instead.

71. Logging Rule
Never log:
private key
secret scalar
seed
mnemonic
adaptor witness
raw signing material
Even at:
DEBUG
TRACE
levels.

72. Secure Debugging
For development:
secret:
<redacted>
Use deterministic test fixtures only in isolated test environments.
Production secrets must never be copied into bug reports.

73. Test Vectors
Before implementation is considered complete, maintain protocol test vectors containing:
known keys
known commitments
known proofs
known adaptor signatures
known transactions
known expected extracted witness
known expected final state
These should be version-controlled.
Do not include production secrets.

74. Property Testing
Test cryptographic invariants such as:
valid proof verifies
modified proof fails
modified amount fails
modified participant fails
modified commitment fails
invalid adaptor signature fails
wrong witness fails
extracted witness reconstructs expected relationship

75. State-Machine Property Testing
Generate random sequences of:
accept
cancel
timeout
fund
confirm
reorg
redeem
restart
and verify:
no invalid state is reachable
no asset can be claimed twice
no recovery path creates funds
no trade completes without settlement evidence

76. Adversarial Test Matrix
At minimum:
Scenario	Expected result
Maker disappears before funding	Safe cancellation
Taker disappears before funding	Safe cancellation
BTC funding never arrives	Swap expires
BTC funding underfunded	Swap rejected/recovery
XMR funding never arrives	BTC recovery
XMR funding underfunded	Recovery
Both legs funded	Redemption proceeds
BTC redemption delayed	Protocol timeout
Bitcoin reorg	State recalculated
Monero reorg	State recalculated
Duplicate message	No duplicate action
Duplicate transaction event	No duplicate settlement
Server crash	Swap resumes
Database rollback	Reconciliation detects inconsistency
Malformed proof	Setup rejected
Wrong secret	Redemption rejected

77. Security Invariants
The implementation must preserve these invariants.
Invariant 1
A participant cannot redeem the counterparty's asset without satisfying the protocol's cryptographic conditions.
Invariant 2
Publishing the required Bitcoin redemption evidence must reveal the information required by the counterparty to complete the Monero side.
Invariant 3
A timeout cannot create a state where both parties can independently claim both assets.
Invariant 4
A failed swap must have a defined recovery path.
Invariant 5
Application state cannot authorize settlement without blockchain evidence.
Invariant 6
A completed trade cannot return to an active state.

78. Economic Invariant
The swap must preserve:
BTC supplied
=
BTC redeemed
+
BTC recovered
+
explicit network fees
and:
XMR supplied
=
XMR redeemed
+
XMR recovered
+
explicit network fees
within the exact accounting rules of the implementation.

79. No Minting Principle
The application must never be able to create the appearance of settlement merely by modifying:
trade.status
ledger
A ledger entry claiming a blockchain transfer must reference a verifiable blockchain transaction.

80. Settlement Evidence
Every completed swap should contain:
BTC funding txid
BTC redemption txid
XMR funding transaction identifier
XMR redemption transaction identifier
confirmation evidence
protocol version
This makes the trade independently auditable.

81. Completion Certificate
After successful settlement, Open Dex can generate an internal:
SwapCompletionCertificate
containing:
swap_id
trade_id
protocol_version

btc_amount
xmr_amount

btc_funding_txid
btc_redeem_txid

xmr_funding_reference
xmr_redeem_reference

completed_at
This is useful for:
    • user history
    • accounting
    • dispute resolution
    • audits

82. Protocol Boundary
The following belongs inside the atomic swap protocol:
cryptographic setup
proof verification
funding conditions
redemption conditions
witness extraction
timeout/recovery conditions
blockchain evidence
The following belongs outside:
maker price
taker price
market matching
protocol revenue
user reputation
UI
analytics
KYC
payment methods
This separation should be maintained.

83. Open Dex Settlement Adapter
Architecture:
                 Trade Engine
                      |
                      v
             BtcXmrSwapAdapter
                      |
             +--------+--------+
             |                 |
             v                 v
       Protocol Engine     Blockchain
             |              Watchers
             v                 |
        Crypto Library         |
             |                 |
             +--------+--------+
                      |
                      v
                 Settlement
The BtcXmrSwapAdapter is the critical boundary.

84. Adapter Interface
Conceptually:
trait AtomicSwap {
    fn initialize(
        trade: TradeTerms
    ) -> Result<SwapSession>;

    fn prepare(
        session: &SwapSession
    ) -> Result<ProtocolMessage>;

    fn handle_message(
        session: &mut SwapSession,
        message: ProtocolMessage
    ) -> Result<ProtocolAction>;

    fn observe_chain_event(
        session: &mut SwapSession,
        event: BlockchainEvent
    ) -> Result<ProtocolAction>;

    fn recovery_action(
        session: &SwapSession
    ) -> Result<ProtocolAction>;
}
The exact API will change during implementation.

85. Protocol Engine Must Be Deterministic
Given:
current state
+
valid message
+
blockchain evidence
the protocol engine should produce:
next state
+
required action
It should not depend on:
random application behavior
except where cryptographic randomness is explicitly required.

86. Reference Implementation Strategy
The first engineering task should be a protocol implementation review, not coding.
Evaluate the currently maintained BTC/XMR implementation options and their cryptographic dependencies.
The historical COMIT implementation is useful reference material, but its repository now explicitly says it is unmaintained and points users toward Eigenwallet/core. GitHub
The Open Dex team should therefore evaluate:
protocol correctness
+
current maintenance
+
cryptographic review
+
library maturity
+
test coverage
+
network compatibility
+
licensing
before selecting a production dependency.

87. Reference Materials
The historical COMIT implementation demonstrates a practical architecture involving:
CLI
+
automated swap backend
+
Bitcoin wallet
+
Monero wallet
+
blockchain watchers
+
persistent swap state
and describes automatic resume/recovery behavior. GitHub
The archived PoC also demonstrates integration testing with Bitcoin and Monero nodes and wallet RPC infrastructure. GitHub
These are useful architectural references, but they should not be treated as a current security endorsement.

88. Open Dex Protocol Version 0.1
The initial protocol identifier should be:
OPENDEX-BTC-XMR-1
It means:
OPENDEX
    protocol family

BTC-XMR
    asset pair

1
    protocol version
Future incompatible changes become:
OPENDEX-BTC-XMR-2

89. Network Identifiers
Never infer network from environment alone.
Every swap explicitly contains:
bitcoin_network
monero_network
Examples:
bitcoin-mainnet
monero-mainnet

bitcoin-testnet
monero-stagenet
Development swaps must fail if a mainnet asset is accidentally sent to a test environment.

90. Network Safety
A swap must reject:
Bitcoin testnet
vs
Bitcoin mainnet
mismatches.
Likewise:
Monero stagenet
vs
Monero mainnet
mismatches.
The network identifier must be included in the signed/committed swap parameters where required.

91. Amount Safety
The following must be committed:
BTC amount
XMR amount
Changing either after cryptographic setup invalidates the setup.
Therefore:
quote
 ↓
trade accepted
 ↓
amounts immutable
 ↓
crypto setup
Never:
crypto setup
 ↓
change price

92. Destination Safety
The final recipient destinations must be committed before the relevant cryptographic setup.
A malicious participant must not be able to substitute:
their address
after the other party has locked funds.

93. Trade Expiration
Quotes should expire before swap setup.
Example:
Market quote:
expires in 60 seconds
Once accepted:
swap deadlines
take over.
Do not use the market quote expiration as the swap recovery deadline.

94. Protocol Timeout Hierarchy
QUOTE_TIMEOUT
       |
       v
SETUP_TIMEOUT
       |
       v
BTC_FUNDING_TIMEOUT
       |
       v
XMR_FUNDING_TIMEOUT
       |
       v
REDEMPTION_TIMEOUT
       |
       v
RECOVERY_TIMEOUT
The actual order and semantics must be adapted to the chosen reference protocol.

95. Emergency Pause
Open Dex must be able to:
pause new swaps
without disabling:
existing swap monitoring
recovery
refund
This is essential.
A pause should mean:
NO_NEW_EXPOSURE
not:
IGNORE_ACTIVE_SWAPS

96. Emergency Protocol
If a serious vulnerability is discovered:
1. Pause new swaps.
2. Continue monitoring active swaps.
3. Determine affected protocol versions.
4. Prevent new affected sessions.
5. Execute safe recovery paths.
6. Preserve evidence.
7. Notify affected operators/users as appropriate.
8. Patch.
9. Test.
10. Re-enable only after review.

97. Upgrade Policy
Never silently change the cryptographic protocol.
A protocol upgrade requires:
new protocol version
+
new test vectors
+
new security review
+
compatibility decision
Existing swaps continue under their original protocol version.

98. What v0.1 Does Not Specify
This document intentionally does not freeze:
exact cryptographic equations
exact adaptor-signature implementation
exact cross-curve DLEQ implementation
exact Bitcoin transaction templates
exact Monero transaction construction
exact timeout values
exact confirmation counts
Those must be taken from the selected, reviewed protocol implementation rather than invented by Open Dex.
This is a deliberate safety boundary.

99. Implementation Sequence
The engineering work should now proceed in this order:
1. Select reference BTC/XMR protocol.
        ↓
2. Audit its cryptographic dependencies.
        ↓
3. Confirm licensing.
        ↓
4. Build isolated protocol harness.
        ↓
5. Run existing test vectors.
        ↓
6. Run Bitcoin + Monero local networks.
        ↓
7. Complete one full swap.
        ↓
8. Kill/restart every participant at every stage.
        ↓
9. Test timeout/recovery paths.
        ↓
10. Integrate Open Dex trade state machine.
        ↓
11. Integrate fee/ledger system.
        ↓
12. Add marketplace.
        ↓
13. Security review.
        ↓
14. Testnet beta.
        ↓
15. Mainnet limited release.

100. First Prototype
The first prototype should be deliberately small.
Alice Wallet
      |
      |
Open Dex Swap Harness
      |
      |
Bob Wallet
No:
order book
fiat
reputation
payment methods
mobile app
yet.
The harness should prove:
BTC funded
        ↓
XMR funded
        ↓
BTC redeemed
        ↓
witness extracted
        ↓
XMR redeemed
        ↓
both sides settled

101. Second Prototype
Then introduce Open Dex:
Maker
  |
  v
Order Book
  |
  v
Taker
  |
  v
Trade Engine
  |
  v
BTC/XMR Swap Adapter
  |
  v
Atomic Swap Protocol
The marketplace should have no ability to modify the cryptographic settlement rules.

102. Third Prototype
Introduce failure testing:
Maker disappears
Taker disappears
Node dies
Server dies
Network disconnects
Transaction delayed
Blockchain reorg
Invalid message
Duplicate message
Invalid proof
The system should automatically recover or enter a clearly defined safe state.

103. Production Readiness Gate
Do not move to real money until:
[ ] Protocol implementation selected
[ ] Cryptographic dependencies reviewed
[ ] Test vectors pass
[ ] Local-chain swaps pass
[ ] Testnet swaps pass
[ ] Failure paths pass
[ ] Restart tests pass
[ ] Reorg tests pass
[ ] Accounting tests pass
[ ] State-machine tests pass
[ ] Security review complete
[ ] Wallet architecture reviewed
[ ] Operational monitoring complete
[ ] Emergency recovery tested

104. Final Protocol Principle
The Open Dex application should never be the thing that makes a BTC/XMR swap atomic.
The cryptographic protocol and the two blockchains provide that property.
Open Dex provides:
price discovery
+
counterparty coordination
+
protocol messaging
+
state tracking
+
blockchain observation
+
fees
+
accounting
+
reputation
+
risk controls
The fundamental architecture is therefore:
                 OPEN DEX
                    |
              Trade Agreement
                    |
                    v
            BTC/XMR Swap Adapter
                    |
                    v
          Cryptographic Protocol
             /              \
            /                \
           v                  v
       BITCOIN              MONERO
       settlement           settlement
           \                  /
            \                /
             +------v-------+
                    |
               Proof of
               settlement
                    |
                    v
               Trade Complete

105. The One Rule That Matters Most
Never make the database the source of truth for blockchain ownership.
The database records what Open Dex believes the protocol state to be.
The blockchains provide the external evidence.
The cryptographic protocol determines what that evidence means.
The state machine determines what Open Dex is allowed to do next.
That four-layer separation is the core of the BTC/XMR implementation.
End of Open Dex Protocol Specification v0.1
The next engineering artifact
The next document should be much more concrete: Open Dex BTC/XMR Swap — Sequence Diagrams & Transaction Specification v0.1.
That one should take the protocol above and draw the actual sequence for Alice → Open Dex → Bob → Bitcoin → Monero, including every message and state transition, and then define the interfaces around the selected reference implementation. That is where we should decide exactly how the Rust BtcXmrSwapAdapter, Bitcoin watcher, Monero watcher, wallet interfaces, and recovery worker fit together.
One important constraint from the research is worth carrying forward: the historical COMIT implementation is now archived and explicitly unmaintained, so I would use it as reference material rather than simply fork it and call that the Open Dex protocol. GitHub The current Eigenwallet protocol repository also describes itself as based on the relevant BTC/XMR research, which makes it worth evaluating as part of the dependency-selection step. GitHub

---

## Implementation Status — 2026-10-06

Milestones 1-3 completed. The following items from this specification have been implemented:

- **Swap state machine** — 9 states (Created → Negotiating → ProtocolSetup → Funding → Funded → Redeeming → Completed, plus Recovering/Refunded/Failed/WaitingForPeer) with validated transitions in `opendex-protocol-types`
- **SettlementProtocol trait** — The core interface separating marketplace from cryptographic settlement
- **BtcXmrAdapter** — Skeleton implementation in `opendex-btc-xmr` (stub logic, no real DLEQ proofs)
- **Bitcoin/Monero chain traits** — With mock implementations for testing
- **SwapEngine** — Generic coordinator with persistence, recovery, and transport
- **Event sourcing** — Append-only event log for all state changes
- **File-based storage** — Atomic JSON writes for swaps, events, transaction intents
- **Blockchain watchers** — Read-only observers with supervisor coordination
- **Deterministic recovery** — Same inputs always produce same RecoveryDecision
- **Transport layer** — Async Transport trait with ProtocolEnvelope and replay protection

Not yet implemented:
- Real Eigenwallet DLEQ proofs and adaptor signatures
- Real blockchain RPC integration
- Real P2P transport (libp2p)
- Order book and matching
