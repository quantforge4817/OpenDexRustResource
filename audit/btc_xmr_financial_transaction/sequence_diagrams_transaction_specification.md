Open Dex
BTC/XMR Swap — Sequence Diagrams & Transaction Specification v0.1
Status: Engineering specification
Version: 0.1
Pair: BTC ↔ XMR
Primary settlement direction: BTC → XMR
Counter-direction: XMR → BTC using the same protocol abstraction where supported
Settlement: Non-custodial atomic swap
Implementation target: Rust settlement core + Open Dex application services

1. Purpose
This document translates the Open Dex BTC/XMR protocol into an implementation-level sequence.
It defines:
    • participants
    • protocol messages
    • transaction lifecycle
    • blockchain observations
    • state transitions
    • wallet boundaries
    • transaction verification
    • timeout handling
    • recovery
    • restart behavior
    • sequence diagrams
    • API boundaries
    • implementation milestones
It does not invent a new BTC/XMR atomic-swap cryptographic construction.
Open Dex should use a reviewed implementation of the established BTC/XMR protocol family.
The underlying research describes a BTC/XMR construction based on cross-curve equality-of-discrete-log proofs and Bitcoin adaptor signatures rather than ordinary HTLCs. GitHub

2. System Model
The complete system is:
                         OPEN DEX
                            |
             +--------------+--------------+
             |                             |
        Marketplace                  Swap Coordinator
             |                             |
        Order Book                        |
             |                             |
             +-------------+---------------+
                           |
                    BTC/XMR Swap Engine
                           |
          +----------------+----------------+
          |                                 |
          v                                 v
    Bitcoin Adapter                    Monero Adapter
          |                                 |
          v                                 v
    Bitcoin Wallet                      Monero Wallet
          |                                 |
          v                                 v
    Bitcoin Network                     Monero Network
The central rule is:
Open Dex coordinates the swap; it does not become the custodian of the user's funds.

3. Participants
For the remainder of this document:
Alice = BTC seller / XMR buyer
Bob   = XMR seller / BTC buyer
The marketplace terminology is:
Maker = Bob
Taker = Alice
for the initial BTC → XMR flow.

4. Initial Trade
Example:
Maker:
Bob

Sells:
100 XMR

Receives:
1 BTC

Maker price:
1 BTC = 100 XMR
Suppose Open Dex charges:
Maker fee:
0.10%

Taker fee:
0.30%
The exact fee calculation occurs before the swap is committed.
The atomic-swap engine receives only the final immutable settlement amounts.

5. Separation of Responsibilities
Marketplace
Responsible for:
orders
quotes
matching
fees
maker reputation
liquidity
trade creation
Swap Engine
Responsible for:
protocol state
cryptographic messages
swap deadlines
transaction coordination
recovery
Bitcoin Adapter
Responsible for:
Bitcoin transaction creation
Bitcoin transaction inspection
Bitcoin broadcasting
Bitcoin confirmation tracking
Monero Adapter
Responsible for:
Monero transaction creation
Monero transaction inspection
Monero broadcasting
Monero confirmation tracking
Blockchain Watchers
Responsible for:
chain observation
confirmation tracking
reorg detection
transaction evidence

6. The Critical Boundary
The following boundary must never be blurred:
                APPLICATION
                    |
                    v
             "Swap is ready"
                    |
                    v
              PROTOCOL ENGINE
                    |
                    v
        "Cryptographic conditions"
                    |
                    v
              BLOCKCHAIN
                    |
                    v
           "Settlement evidence"
The application cannot declare a blockchain transaction successful.
The blockchain observation layer must establish that fact.

7. High-Level Happy Path
Alice                       Open Dex                       Bob
  |                            |                            |
  |---- accept order --------->|                            |
  |                            |---- trade created -------->|
  |                            |                            |
  |<--- swap parameters -------|---- swap parameters ------>|
  |                            |                            |
  |========= crypto setup ================================>|
  |<======== crypto setup =================================|
  |                            |                            |
  |--------- BTC funding ------>                            |
  |                            |                            |
  |                            |<---- XMR funding ----------|
  |                            |                            |
  |--------- BTC redemption --->                            |
  |                            |                            |
  |                            |<---- witness observed -----|
  |                            |                            |
  |                            |<---- XMR redemption -------|
  |                            |                            |
  |<----------- complete ------|------------- complete -----|
Open Dex is a coordinator/observer.
It should not be necessary for Open Dex itself to possess either asset.

8. Detailed Sequence
Phase A — Order Creation
Bob creates:
Order {
    pair: BTC_XMR
    side: SELL_XMR
    xmr_amount: 100
    btc_amount: 1
    price: 100 XMR/BTC
}
Open Dex assigns:
order_id
The order becomes:
OPEN

9. Order Reservation
Alice accepts the order.
Open Dex atomically performs:
OPEN
  ↓
RESERVED
The maker's available liquidity becomes:
available XMR
    ↓
reserved XMR
No blockchain funds have necessarily moved yet.

10. Trade Creation
Open Dex creates:
trade_id
swap_id
Example:
trade_id = TRD-...
swap_id  = SWP-...
The two identifiers should remain separate.
trade_id
is the marketplace object.
swap_id
is the settlement protocol object.

11. Trade Commitment
Open Dex creates the canonical settlement terms:
SwapTerms {
    protocol_version
    bitcoin_network
    monero_network

    btc_amount
    xmr_amount

    maker
    taker

    btc_destination
    xmr_destination

    timeout_parameters
}
A canonical serialization is hashed.
Conceptually:
commitment =
    HASH(
        canonical(SwapTerms)
    )

12. Commitment Rule
Once cryptographic setup begins:
SwapTerms
are immutable.
Changing:
BTC amount
XMR amount
destination
participant
network
protocol version
timeout
must invalidate the existing cryptographic setup.

13. Phase B — Swap Initialization
State:
CREATED
Alice receives:
swap_id
protocol_version
commitment
Bob receives the same.
Both independently verify:
commitment
amounts
participants
network
destination

14. Sequence — Initialization
Alice                    Open Dex                    Bob
  |                         |                         |
  |--- accept order ------->|                         |
  |                         |--- create trade ------->|
  |                         |                         |
  |<--- swap terms ---------|--- swap terms --------->|
  |                         |                         |
  |--- verify terms --------------------------------->|
  |<-------------------------------- verify terms -----|
The peer-to-peer protocol may eventually bypass Open Dex for these messages.

15. Phase C — Cryptographic Setup
Both participants generate protocol-specific ephemeral material.
Conceptually:
Alice:
secret_A
public_A

Bob:
secret_B
public_B
The actual protocol may use several secrets/commitments.
Open Dex does not prescribe the number.

16. Secret Boundary
The following must never be transmitted to Open Dex:
private scalar
secret witness
seed
wallet private key
Monero spend secret
Bitcoin private key
Open Dex receives only protocol material that the underlying construction permits to be shared.

17. Cryptographic Message Flow
Conceptually:
Alice                              Bob
  |                                  |
  |--- protocol-init --------------->|
  |                                  |
  |<-- protocol-response ------------|
  |                                  |
  |--- proof/commitment ------------>|
  |                                  |
  |<-- proof/commitment -------------|
  |                                  |
  |=== local verification ===========|
Every message is associated with:
swap_id
sequence
message_id
protocol_version

18. Cryptographic Verification Gate
Before funding:
if:
    commitments valid
    AND proofs valid
    AND amounts match
    AND destinations match
    AND protocol version supported
then:
CRYPTO_SETUP_VERIFIED
Otherwise:
SETUP_FAILED
No funding should proceed after a failed verification.

19. Phase D — Bitcoin Funding
Alice creates the Bitcoin funding transaction according to the selected BTC/XMR protocol.
Conceptually:
Alice wallet
     |
     v
Create funding transaction
     |
     v
Sign
     |
     v
Broadcast
Open Dex does not create an arbitrary Bitcoin transaction template.
The transaction construction comes from the underlying protocol adapter.

20. Bitcoin Funding Transaction Object
Open Dex records:
BitcoinFunding {
    swap_id

    txid

    amount

    output_reference

    observed_height

    confirmation_count

    status
}
The actual output structure is protocol-specific.

21. Bitcoin Funding Sequence
Alice             Bitcoin             Open Dex              Bob
  |                  |                    |                  |
  |-- broadcast ---->|                    |                  |
  |                  |                    |                  |
  |                  |-- mempool -------->|                  |
  |                  |                    |-- notify ------->|
  |                  |                    |                  |
  |                  |-- block ---------->|                  |
  |                  |                    |                  |
  |                  |-- confirmations -->|                  |
  |                  |                    |                  |
  |                  |                    |-- BTC funded --->|

22. Bitcoin Watcher
The Bitcoin watcher consumes:
new transaction
new block
confirmation update
reorg
It emits normalized events:
BitcoinFundingSeen
BitcoinFundingConfirmed
BitcoinFundingFinal
BitcoinFundingReorged

23. Funding Verification
A transaction is not accepted simply because the txid exists.
The watcher must verify:
correct network
correct transaction
correct amount
correct output
correct protocol conditions
Then:
BTC_FUNDING_CONFIRMED

24. Bitcoin Reorg
Suppose:
BTC_FUNDING_CONFIRMED
was previously emitted.
Then a reorg removes the transaction from the canonical chain.
The watcher emits:
BitcoinFundingReorged
The swap engine transitions back to the appropriate non-settled state.
It must not continue as though the original funding still existed.

25. Phase E — Monero Funding
After the Bitcoin-side protocol condition is met:
BTC_FUNDING_CONFIRMED
Bob prepares the Monero side.
Bob wallet
    |
    v
create XMR funding
    |
    v
sign
    |
    v
broadcast

26. Monero Funding Observation
The Monero watcher monitors the relevant transaction/output.
Conceptual events:
XmrFundingSeen
XmrFundingConfirmed
XmrFundingFinal
XmrFundingReorged
The actual detection logic must use the Monero transaction model required by the selected swap implementation.

27. Both Legs Funded
Once both conditions are satisfied:
BTC_FUNDING_FINAL
+
XMR_FUNDING_FINAL
the swap becomes:
BOTH_LEGS_FUNDED
This is the primary settlement checkpoint.

28. Sequence — Funding Complete
Alice                 Open Dex                 Bob
  |                       |                     |
  |--- BTC funding ------>|                     |
  |                       |                     |
  |                       |---- BTC verified -->|
  |                       |                     |
  |                       |<--- XMR funding ---|
  |                       |                     |
  |                       |---- XMR verified -->|
  |                       |                     |
  |                       |                     |
  |                       | BOTH_LEGS_FUNDED   |

29. Phase F — Bitcoin Redemption
The next action is determined by the selected atomic-swap protocol.
The redemption transaction must satisfy the cryptographic conditions established during setup.
Conceptually:
adaptor signature
       |
       v
completed signature
       |
       v
Bitcoin redemption transaction
       |
       v
Bitcoin network

30. Bitcoin Redemption Observation
The counterparty watches the Bitcoin blockchain.
When the completed signature appears on-chain, the relevant cryptographic witness can be extracted according to the underlying protocol.
Conceptually:
Bitcoin redemption
        |
        v
signature evidence
        |
        v
extract witness
        |
        v
verify witness

31. Witness Extraction Boundary
The witness extraction logic belongs inside:
BtcXmrSwapCryptoAdapter
not:
TradeService
The trade service should receive a high-level result:
WitnessExtracted
not manipulate cryptographic scalars itself.

32. Witness Validation
Extracted material must be checked against the expected cryptographic commitment.
Conceptually:
extracted_witness
        |
        v
Verify(witness, commitment)
        |
    +---+---+
    |       |
  valid   invalid
    |       |
    v       v
continue  security failure
An invalid witness is a protocol-level fault.

33. Phase G — Monero Redemption
After valid witness extraction:
XMR_REDEMPTION_READY
Bob constructs the Monero redemption transaction.
Bob wallet
    |
    v
derive required redemption material
    |
    v
construct transaction
    |
    v
sign
    |
    v
broadcast

34. Monero Redemption
The Monero transaction is observed by the Monero watcher.
XmrRedemptionSeen
        ↓
XmrRedemptionConfirmed
        ↓
XmrRedemptionFinal
Once the final condition is satisfied:
COMPLETED

35. Complete Happy Path
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
BTC_FUNDING_CONFIRMED
   |
   v
XMR_FUNDING_PENDING
   |
   v
XMR_FUNDING_CONFIRMED
   |
   v
BOTH_LEGS_FUNDED
   |
   v
BTC_REDEMPTION
   |
   v
BTC_REDEEMED
   |
   v
WITNESS_EXTRACTED
   |
   v
XMR_REDEMPTION
   |
   v
XMR_REDEEMED
   |
   v
COMPLETED

36. Important Correction to the State Model
BTC_REDEEMED and XMR_REDEEMED should not mean merely:
transaction broadcast
They should mean the protocol's required observation threshold has been reached.
Therefore internally distinguish:
BROADCAST
SEEN
CONFIRMED
FINAL
This prevents premature state transitions.

37. Transaction State Model
Every blockchain transaction should have:
UNKNOWN
    |
    v
CREATED
    |
    v
BROADCAST
    |
    v
SEEN
    |
    v
CONFIRMED
    |
    v
FINAL
Possible failure branch:
CONFIRMED
    |
    v
REORGED
    |
    v
REVALIDATE

38. Transaction Record
Bitcoin:
btc_transactions
-----------------
id
swap_id
type
txid
raw_tx_hash
amount
network
observed_height
confirmation_count
status
first_seen_at
finalized_at
Monero:
xmr_transactions
-----------------
id
swap_id
type
tx_hash
amount
network
observed_height
confirmation_count
status
first_seen_at
finalized_at
Do not store private keys in these tables.

39. Transaction Types
Bitcoin:
FUNDING
REDEMPTION
RECOVERY
Monero:
FUNDING
REDEMPTION
RECOVERY
Future versions can add:
FEE_ADJUSTMENT
if the protocol ever requires it.

40. Transaction Identity
A transaction record must be uniquely constrained.
Bitcoin:
(network, txid)
Monero:
(network, tx_hash)
A transaction observed multiple times must update the same record.

41. Blockchain Event Model
Normalize different node APIs into:
enum BlockchainEvent {
    TransactionSeen,
    TransactionConfirmed,
    TransactionFinal,
    TransactionReorged,
    BlockAdvanced,
}
Chain-specific metadata remains inside the adapter.

42. Why This Matters
The swap engine should not contain:
Bitcoin RPC logic
Monero RPC logic
It should receive:
BlockchainEvent
This gives us:
swap engine
that can be tested without running real nodes.

43. Bitcoin Adapter
Recommended interface:
trait BitcoinAdapter {

    fn inspect_transaction(
        &self,
        txid: &str
    ) -> Result<BtcTransactionEvidence>;

    fn create_funding(
        &self,
        request: FundingRequest
    ) -> Result<UnsignedTransaction>;

    fn create_redemption(
        &self,
        request: RedemptionRequest
    ) -> Result<UnsignedTransaction>;

    fn create_recovery(
        &self,
        request: RecoveryRequest
    ) -> Result<UnsignedTransaction>;

    fn broadcast(
        &self,
        tx: SignedTransaction
    ) -> Result<TxId>;
}

44. Monero Adapter
Conceptually:
trait MoneroAdapter {

    fn inspect_transaction(
        &self,
        tx_hash: &str
    ) -> Result<XmrTransactionEvidence>;

    fn create_funding(
        &self,
        request: FundingRequest
    ) -> Result<UnsignedTransaction>;

    fn create_redemption(
        &self,
        request: RedemptionRequest
    ) -> Result<UnsignedTransaction>;

    fn create_recovery(
        &self,
        request: RecoveryRequest
    ) -> Result<UnsignedTransaction>;

    fn broadcast(
        &self,
        tx: SignedTransaction
    ) -> Result<TxHash>;
}
Exact operations depend on the selected protocol implementation.

45. Wallet Boundary
The adapter should not directly expose:
private keys
to the swap engine.
Instead:
Swap Engine
     |
     v
Transaction Builder
     |
     v
Signing Interface
     |
     v
Wallet

46. Signing Interface
trait TransactionSigner {
    fn sign(
        &self,
        request: SigningRequest
    ) -> Result<SignedTransaction>;
}
The signing implementation can later be:
local wallet
hardware wallet
remote signer
HSM
isolated process
without rewriting the swap state machine.

47. Sequence — Signing
Swap Engine
     |
     | unsigned transaction
     v
Transaction Builder
     |
     | signing request
     v
Wallet / Signer
     |
     | signed transaction
     v
Swap Engine
     |
     | broadcast
     v
Blockchain

48. Recovery Sequence
Consider:
BTC funded
XMR funding never arrives
The state becomes:
BTC_FUNDING_CONFIRMED
        |
        v
XMR_FUNDING_TIMEOUT
The recovery worker checks:
Is BTC recovery now permitted?
If yes:
create recovery transaction
        |
        v
sign
        |
        v
broadcast
        |
        v
confirm
        |
        v
BTC_RECOVERED

49. Recovery Sequence Diagram
Alice             Open Dex             Bitcoin
  |                   |                   |
  |--- BTC funded --->|                   |
  |                   |                   |
  |                   | XMR timeout       |
  |                   |                   |
  |                   | recovery allowed  |
  |                   |                   |
  |                   |-- create refund ->|
  |                   |                   |
  |                   |-- broadcast ----->|
  |                   |                   |
  |<-- recovered -----|<-- confirmed -----|
The actual refund/recovery conditions are determined by the selected BTC/XMR protocol.

50. Important Recovery Rule
Open Dex must never invent a recovery transaction because the application believes a trade has timed out.
The application asks the protocol adapter:
CanRecover(state, blockchain_state)?
The adapter returns:
YES
or:
NO
and, if yes:
RecoveryAction

51. Recovery Interface
trait SwapRecovery {

    fn evaluate(
        &self,
        swap: &SwapState,
        chain: &ChainState
    ) -> RecoveryDecision;

}
Example:
enum RecoveryDecision {
    NotEligible,
    WaitUntil(BlockHeight),
    Recover(RecoveryAction),
    AlreadyRecovered,
    ProtocolFailure,
}

52. Recovery Must Be Idempotent
If five workers call:
recover(swap_id)
simultaneously:
only one recovery transaction
may be generated/broadcast.
Use:
database lock
+
state version
+
unique recovery transaction constraint

53. Database Concurrency
Every swap mutation should use:
UPDATE swap_sessions
SET state = ...
WHERE id = ?
AND state_version = ?
If:
rows_affected == 0
another worker won the race.
Reload state.

54. Restart Sequence
Suppose the server dies:
BOTH_LEGS_FUNDED
After restart:
Open Dex
   |
   v
load active swaps
   |
   v
query Bitcoin
   |
   v
query Monero
   |
   v
reconstruct state
   |
   v
resume
The system should not depend on in-memory state.

55. Restart Invariant
At startup:
database state
is treated as:
hypothesis
Blockchain evidence is then queried.
The reconstructed state becomes authoritative.

56. Reconciliation Worker
Create:
SwapReconciliationWorker
Responsibilities:
find active swaps
query relevant chains
compare transaction state
repair stale observations
detect impossible states
resume eligible swaps
Run:
continuously
+
on startup
+
after node reconnect

57. Reconciliation Example
Database:
BTC_FUNDING_CONFIRMED
Blockchain:
transaction absent from canonical chain
Result:
state != database
The worker must not mark the trade completed.
It moves the swap into:
RECONCILIATION_REQUIRED
or the appropriate protocol state.

58. Impossible-State Handling
Example:
Database:
COMPLETED

Blockchain:
BTC redemption missing
XMR redemption missing
Do not simply rewrite:
COMPLETED → FAILED
Instead create:
PROTOCOL_INCONSISTENCY
with an incident record.

59. Incident Record
protocol_incidents
-------------------
id
swap_id
incident_type
severity
detected_at
description
evidence
resolved_at
resolution
This becomes invaluable during production incidents.

60. Message Sequence Numbers
Every swap message:
message_id
sequence
Example:
1 INIT
2 INIT_ACK
3 CRYPTO_COMMITMENT
4 CRYPTO_PROOF
5 FUNDING_READY
6 FUNDING_CONFIRMED
...
The actual protocol messages should follow the selected upstream implementation.
These numbers are Open Dex transport sequencing, not cryptographic protocol steps unless explicitly mapped.

61. Message Idempotency
If:
message_id = abc123
arrives twice:
first:
process

second:
return previous result
Never:
execute twice

62. Message Storage
swap_messages
-------------
message_id
swap_id
sequence
direction
message_type
payload_hash
created_at
processed_at
status
Do not necessarily store sensitive payload contents.
Store encrypted payloads only when necessary for recovery/debugging.

63. Message Authentication
The transport layer should provide:
authenticated peer
where appropriate.
The underlying swap protocol remains the authority for cryptographic settlement.
Do not assume:
TLS identity = wallet ownership

64. Transport Failure
If Alice disconnects:
swap continues
provided the protocol can continue without the live connection.
The swap should transition to:
WAITING_FOR_PEER
rather than immediately:
FAILED

65. Peer Reconnection
When Alice reconnects:
GET swap state
       |
       v
last known sequence
       |
       v
replay missing messages
       |
       v
resume
This is much safer than attempting to restart the entire protocol.

66. Open Dex Relay
v0.1 can use:
Alice
 |
 v
Open Dex Relay
 |
 v
Bob
The relay should be intentionally dumb.
It should:
route messages
authenticate sessions
enforce size limits
rate-limit
store temporary delivery state
It should not:
create private keys
sign transactions
modify protocol messages

67. Future Direct Mode
The same message model should support:
Alice
 |
 +---------------------> Bob
without changing the underlying swap state machine.
Therefore:
Transport
must be an abstraction.

68. Transport Interface
trait SwapTransport {

    fn send(
        &self,
        peer: PeerId,
        message: ProtocolMessage
    ) -> Result<()>;

    fn receive(
        &self
    ) -> Result<ProtocolMessage>;

}
Potential implementations:
OpenDexRelay
DirectPeer
Tor
libp2p
future decentralized transport

69. Exact Transaction Lifecycle
Every blockchain transaction follows:
REQUESTED
   |
   v
CREATED
   |
   v
SIGNED
   |
   v
BROADCAST
   |
   v
SEEN
   |
   v
CONFIRMED
   |
   v
FINAL
Failure:
CREATED
   |
   +--> SIGNING_FAILED

BROADCAST
   |
   +--> REJECTED

CONFIRMED
   |
   +--> REORGED

70. Transaction Ownership
Each transaction has:
created_by
which may be:
ALICE
BOB
PROTOCOL_RECOVERY
This is application metadata.
It does not imply custody.

71. Transaction Intent
Before creating a transaction, persist:
TransactionIntent
Example:
{
  "swapId": "...",
  "chain": "bitcoin",
  "type": "funding",
  "amount": "...",
  "protocolVersion": "...",
  "createdAt": "..."
}
Then create the actual transaction.
This helps recover from crashes occurring between:
transaction created
and:
database updated

72. Broadcast Crash Problem
Consider:
1. create transaction
2. broadcast succeeds
3. server crashes
4. database says "not broadcast"
On restart:
query transaction
If it exists:
recover transaction record
Do not create a second transaction automatically.

73. Transaction Intent Reconciliation
At startup:
for each pending transaction intent:

    search known txid

    if found:
        attach txid

    else:
        determine whether creation can safely repeat
Never assume broadcast failed merely because the RPC call timed out.

74. Bitcoin Fee Replacement
v0.1 should treat fee replacement as disabled unless the selected protocol explicitly supports it safely.
The historical COMIT implementation warned against bumping fees during swaps because it can have unpredictable consequences. GitHub
Therefore:
default:
NO FEE BUMP
until a specific protocol-safe replacement strategy has been reviewed.

75. Monero Transaction Replacement
Likewise, transaction replacement behavior should be protocol-specific.
Do not introduce generic:
replace transaction
logic into the swap engine.

76. Confirmation Policy
Configuration:
bitcoin:
  funding_confirmations: configurable
  redemption_confirmations: configurable

monero:
  funding_confirmations: configurable
  redemption_confirmations: configurable
The values must ultimately be selected based on:
asset
trade size
risk
protocol requirements
network behavior
They should not be hard-coded into business logic.

77. Finality Model
Use three terms:
SEEN
CONFIRMED
FINAL
Meaning:
SEEN:
transaction detected

CONFIRMED:
meets minimum confirmation requirement

FINAL:
meets the protocol's settlement threshold
This gives operations room to change policies without changing the state machine.

78. Swap-Level Finality
A swap becomes:
COMPLETED
only when the final settlement condition is satisfied.
Not merely because:
BTC transaction exists
or:
XMR transaction exists

79. Accounting Sequence
When the swap completes:
blockchain settlement
        |
        v
verified
        |
        v
trade ledger finalized
        |
        v
fee ledger finalized
        |
        v
maker liquidity updated
        |
        v
order updated
Blockchain verification happens before accounting finalization.

80. Fee Ledger
Record:
protocol_fee
network_fee
network_buffer
maker_amount
taker_amount
separately.
Example:
trade:
1 BTC ↔ 100 XMR

protocol fee:
0.003 BTC equivalent

network:
actual BTC fee
actual XMR fee
Do not merge protocol revenue and network fees.

81. Failed Trade Accounting
If a trade fails before settlement:
protocol fee:
0

maker reservation:
released

taker reservation:
released
If a protocol-specific fee becomes unavoidable, it must be explicitly defined in the fee policy.

82. Reputation Events
A failed trade should distinguish:
USER_ABORT
TIMEOUT
NETWORK_FAILURE
PROTOCOL_FAILURE
BLOCKCHAIN_REORG
COUNTERPARTY_FAILURE
Do not automatically punish a participant for:
Bitcoin network congestion
Monero node outage
Open Dex outage

83. Maker Reputation
Future reputation scoring can use:
successful swaps
timeouts
invalid messages
funding failures
recovery events
availability
But reputation must remain outside the atomic-swap protocol.

84. State Machine — Complete
                         +----------------+
                         |                |
                         v                |
CREATED -> INITIALIZING -> CRYPTO_SETUP   |
                         |                |
                         v                |
                 CRYPTO_SETUP_VERIFIED    |
                         |                |
                         v                |
                BTC_FUNDING_PENDING       |
                         |                |
                         v                |
                BTC_FUNDING_CONFIRMED     |
                         |                |
                         v                |
                XMR_FUNDING_PENDING       |
                         |                |
                         v                |
                XMR_FUNDING_CONFIRMED     |
                         |                |
                         v                |
                  BOTH_LEGS_FUNDED        |
                         |                |
                         v                |
                  BTC_REDEMPTION          |
                         |                |
                         v                |
                    BTC_REDEEMED          |
                         |                |
                         v                |
                 WITNESS_EXTRACTED        |
                         |                |
                         v                |
                  XMR_REDEMPTION          |
                         |                |
                         v                |
                    XMR_REDEEMED          |
                         |                |
                         v                |
                     COMPLETED <-----------+
Failure paths:
any active state
       |
       +--> TIMEOUT
       |
       +--> PROTOCOL_FAILURE
       |
       +--> RECOVERY
       |
       +--> REFUNDED

85. State Transition Rules
Every transition requires:
current_state
+
event
+
protocol validation
Example:
BTC_FUNDING_PENDING
+
BitcoinFundingConfirmed
=
BTC_FUNDING_CONFIRMED
But:
BTC_FUNDING_PENDING
+
XmrFundingConfirmed
must be rejected.

86. Invalid Event Handling
Invalid event:
current state does not permit event
results in:
EVENT_REJECTED
not:
state mutation
The event can still be logged for diagnostics.

87. State Transition Table
Current state	Event	Next state
CREATED	initialize	INITIALIZING
INITIALIZING	setup-started	CRYPTO_SETUP
CRYPTO_SETUP	proofs-valid	CRYPTO_SETUP_VERIFIED
CRYPTO_SETUP	invalid-proof	SETUP_FAILED
CRYPTO_SETUP_VERIFIED	BTC funding seen	BTC_FUNDING_PENDING
BTC_FUNDING_PENDING	funding final	BTC_FUNDING_CONFIRMED
BTC_FUNDING_PENDING	timeout	RECOVERY
BTC_FUNDING_CONFIRMED	XMR funding started	XMR_FUNDING_PENDING
XMR_FUNDING_PENDING	funding final	XMR_FUNDING_CONFIRMED
XMR_FUNDING_PENDING	timeout	RECOVERY
XMR_FUNDING_CONFIRMED	both funded	BOTH_LEGS_FUNDED
BOTH_LEGS_FUNDED	BTC redemption	BTC_REDEEMED
BTC_REDEEMED	witness valid	WITNESS_EXTRACTED
WITNESS_EXTRACTED	XMR redemption	XMR_REDEEMED
XMR_REDEEMED	final	COMPLETED
any recoverable state	recovery complete	REFUNDED
The exact legal transitions should be encoded rather than inferred dynamically.

88. Sequence Diagram — Full Swap
Alice            Open Dex             Bob          Bitcoin       Monero
 |                  |                  |              |             |
 |-- accept ------->|                  |              |             |
 |                  |-- trade -------->|              |             |
 |<-- terms --------|-- terms -------->|              |             |
 |                  |                  |              |             |
 |==== crypto setup ==================>|              |             |
 |<=== crypto setup ===================|              |             |
 |                  |                  |              |             |
 |                  |                  |              |             |
 |-- BTC funding ------------------------------------>|             |
 |                  |                  |              |             |
 |                  |<-- BTC evidence ----------------|             |
 |                  |----------------->|              |             |
 |                  |                  |              |             |
 |                  |                  |-- XMR funding ------------>|
 |                  |                  |              |             |
 |                  |<-- XMR evidence -----------------------------|
 |                  |                  |              |             |
 |                  | BOTH LEGS FUNDED |              |             |
 |                  |                  |              |             |
 |-- BTC redeem ------------------------------------->|             |
 |                  |                  |              |             |
 |                  |<-- redemption evidence --------|             |
 |                  |                  |              |             |
 |                  |-- witness --------------------->|             |
 |                  |                  |              |             |
 |                  |                  |-- XMR redeem ------------->|
 |                  |                  |              |             |
 |                  |<------------- completion ---------------------|
 |<-- complete -----|----------------->|              |             |

89. Sequence Diagram — Timeout
Alice            Open Dex             Bob          Bitcoin       Monero
 |                  |                  |              |             |
 |--- BTC fund ------------------------------------->|             |
 |                  |<-- confirmed ------------------|             |
 |                  |                  |              |             |
 |                  |--- wait -------->|              |             |
 |                  |                  |              |             |
 |                  |                  |       XMR funding absent  |
 |                  |                  |              |             |
 |                  | timeout detected |              |             |
 |                  |                  |              |             |
 |                  | recovery check ------------------------------|
 |                  |                  |              |             |
 |                  | recovery allowed |              |             |
 |                  |                  |              |             |
 |                  |-- recovery tx ---------------->|             |
 |                  |<-- confirmed ------------------|             |
 |                  |                  |              |             |
 |<-- BTC recovered |                  |              |             |

90. Sequence Diagram — Server Crash
Alice              Open Dex               Bitcoin
  |                    |                     |
  |---- funding ------>|                     |
  |                    |---- broadcast ----->|
  |                    |                     |
  |                    |<---- accepted -----|
  |                    X                     |
  |                  CRASH                   |
  |                    |                     |
  |                    | restart             |
  |                    |                     |
  |                    |---- query --------->|
  |                    |<---- transaction ---|
  |                    |                     |
  |                    | reconcile           |
  |                    |                     |
  |                    | resume              |

91. Sequence Diagram — Bitcoin Reorg
Bitcoin                Watcher                Swap Engine
   |                       |                       |
   |--- tx confirmed ----->|                       |
   |                       |--- confirmed ------->|
   |                       |                       |
   |--- chain reorg ------>|                       |
   |                       |--- reorged --------->|
   |                       |                       |
   |                       |--- revalidate ------>|
   |                       |                       |
   |                       |<-- current state ----|
   |                       |                       |
The swap engine then follows the protocol-defined path.

92. Sequence Diagram — Duplicate Message
Alice              Relay              Bob
  |                  |                 |
  |-- msg #17 ------>|---------------->|
  |                  |                 |
  |-- msg #17 ------>|---------------->|
  |                  |                 |
  |                  |       duplicate |
  |                  |                 |
  |                  |<-- cached result|
  |<-----------------|                 |
The second message must not execute the protocol action again.

93. Sequence Diagram — Duplicate Transaction Event
Bitcoin             Watcher             Swap Engine
   |                   |                     |
   |--- tx ----------- >|                    |
   |                   |--- seen ---------->|
   |                   |                     |
   |--- same tx ------>|                     |
   |                   |--- seen ---------->|
   |                   |                     |
   |                   |             idempotent
Only one state transition occurs.

94. Protocol Event Log
For every swap maintain an immutable event log:
1 SwapCreated
2 TermsCommitted
3 CryptoSetupStarted
4 CryptoSetupVerified
5 BtcFundingSeen
6 BtcFundingConfirmed
7 XmrFundingSeen
8 XmrFundingConfirmed
9 BothLegsFunded
10 BtcRedemptionSeen
11 WitnessExtracted
12 XmrRedemptionSeen
13 SwapCompleted
This becomes the audit trail.

95. Event Sourcing vs State Storage
Use both:
current state
+
append-only events
The state is optimized for operations.
The event log is optimized for:
audit
debugging
reconciliation
reconstruction

96. Transaction Specification — BTC Funding
Logical specification:
Transaction:
BTC_FUNDING

Creator:
Alice

Purpose:
commit Alice's BTC to the atomic-swap protocol.

Required:
protocol-defined funding output/conditions

Amount:
exact committed BTC amount

Network:
Bitcoin network specified by swap

Fee:
Bitcoin miner fee

Broadcast:
Bitcoin network

Evidence:
txid + required output evidence + confirmations
The exact script/witness construction is delegated to the selected upstream protocol implementation.

97. Transaction Specification — XMR Funding
Logical specification:
Transaction:
XMR_FUNDING

Creator:
Bob

Purpose:
commit Bob's XMR to the atomic-swap protocol.

Amount:
exact committed XMR amount

Network:
Monero network specified by swap

Fee:
Monero network fee

Broadcast:
Monero network

Evidence:
transaction hash + protocol-required output evidence + confirmations

98. Transaction Specification — BTC Redemption
Transaction:
BTC_REDEMPTION

Creator:
protocol participant permitted by swap state

Purpose:
claim BTC according to atomic-swap conditions

Requirements:
valid protocol signature
valid adaptor completion
correct funding reference
correct destination

Evidence:
Bitcoin transaction
+
observable cryptographic witness

99. Transaction Specification — XMR Redemption
Transaction:
XMR_REDEMPTION

Creator:
protocol participant possessing required derived material

Purpose:
claim XMR according to atomic-swap construction

Requirements:
valid protocol-specific signing material
valid funding reference
correct destination

Evidence:
Monero transaction
+
required confirmations

100. Transaction Specification — Recovery
Transaction:
RECOVERY

Creator:
participant authorized by protocol

Purpose:
recover originally supplied asset after protocol failure/timeout

Requirements:
protocol-defined timeout/recovery condition

Evidence:
on-chain transaction
+
required finality
Open Dex does not get a privileged emergency refund key.

101. No Admin Refund
There must be no:
POST /admin/refund
that can arbitrarily transfer swap funds.
An administrator may:
pause new swaps
inspect state
disable a maker
disable a protocol version
but cannot bypass the cryptographic settlement protocol.

102. API — Create Swap
Conceptual:
POST /v1/swaps
Request:
{
  "tradeId": "TRD...",
  "protocol": "OPENDEX-BTC-XMR-1",
  "btcAmount": "...",
  "xmrAmount": "...",
  "btcNetwork": "mainnet",
  "moneroNetwork": "mainnet"
}
Response:
{
  "swapId": "SWP...",
  "state": "INITIALIZING"
}

103. API — Get Swap
GET /v1/swaps/{swapId}
Response:
{
  "swapId": "SWP...",
  "state": "BOTH_LEGS_FUNDED",
  "btc": {
    "funding": {
      "status": "FINAL"
    }
  },
  "xmr": {
    "funding": {
      "status": "FINAL"
    }
  }
}
Never return private cryptographic material.

104. API — Swap Events
GET /v1/swaps/{swapId}/events
Returns:
{
  "events": [
    {
      "sequence": 1,
      "type": "SwapCreated"
    },
    {
      "sequence": 2,
      "type": "CryptoSetupVerified"
    }
  ]
}

105. API — Transaction Evidence
GET /v1/swaps/{swapId}/transactions
Returns public settlement evidence:
{
  "bitcoin": {
    "fundingTxid": "...",
    "redemptionTxid": "..."
  },
  "monero": {
    "fundingTxid": "...",
    "redemptionTxid": "..."
  }
}

106. API — Recovery Status
GET /v1/swaps/{swapId}/recovery
Response:
{
  "eligible": false,
  "reason": "waiting_for_timeout",
  "nextEvaluation": "..."
}
The API should not allow the client to force recovery.

107. Worker Architecture
Recommended workers:
SwapCoordinatorWorker
BitcoinWatcher
MoneroWatcher
SwapReconciliationWorker
SwapRecoveryWorker
MessageDeliveryWorker
FeeEstimatorWorker

108. Worker Ownership
SwapCoordinatorWorker
    |
    +-- state transitions

BitcoinWatcher
    |
    +-- Bitcoin observations

MoneroWatcher
    |
    +-- Monero observations

RecoveryWorker
    |
    +-- protocol recovery

ReconciliationWorker
    |
    +-- state verification
Each worker should have one clear responsibility.

109. Queue Architecture
Events can flow through:
Bitcoin Node
      |
      v
Bitcoin Watcher
      |
      v
Event Queue
      |
      v
Swap Engine
      |
      v
State DB
and:
Monero Node
      |
      v
Monero Watcher
      |
      v
Event Queue
      |
      v
Swap Engine

110. Queue Guarantees
Assume:
at-least-once delivery
rather than assuming exactly-once delivery.
Therefore every consumer must be idempotent.

111. Security Boundaries
Recommended deployment:
                    Internet
                       |
                 API Gateway
                       |
              +--------+--------+
              |                 |
          Marketplace      Swap Relay
              |                 |
              +--------+--------+
                       |
                 Swap Engine
                       |
          +------------+------------+
          |                         |
    Bitcoin Adapter            Monero Adapter
          |                         |
      BTC node                  XMR node
Wallet/signing infrastructure should be further isolated.

112. Node Isolation
The API server should not need unrestricted RPC access to wallets.
Prefer:
API
 |
 v
Swap Engine
 |
 v
Wallet Service
 |
 v
Wallet
with narrow permissions.

113. Monitoring
Track:
active swaps
swaps completed
swaps failed
timeouts
recovery attempts
recovery failures
Bitcoin watcher lag
Monero watcher lag
node connectivity
message delivery latency

114. Critical Alerts
Alert immediately on:
protocol inconsistency
unexpected redemption
invalid witness
repeated recovery failure
wallet signing failure
chain reorg affecting active swaps
watcher stalled
database/event divergence

115. Metrics
Recommended:
swap_started_total
swap_completed_total
swap_failed_total

swap_duration_seconds

btc_funding_latency
xmr_funding_latency

btc_confirmation_latency
xmr_confirmation_latency

recovery_total
recovery_success_total
recovery_failure_total

protocol_error_total

116. Security Testing
Before mainnet:
fuzz protocol messages
fuzz transaction parsers
fuzz state transitions
fuzz chain events
fuzz recovery decisions
Especially test:
malformed transaction
wrong amount
wrong network
wrong participant
wrong commitment
wrong witness
duplicate event
duplicate transaction
reorg

117. Integration Environment
The first complete integration environment should contain:
Bitcoin regtest
Monero local/stagenet environment
Open Dex API
Swap Engine
Alice wallet
Bob wallet
Bitcoin watcher
Monero watcher
PostgreSQL
message broker

118. Automated Integration Test
Test:
1. Create maker order.
2. Accept order.
3. Create swap.
4. Complete crypto setup.
5. Fund BTC.
6. Confirm BTC.
7. Fund XMR.
8. Confirm XMR.
9. Redeem BTC.
10. Extract witness.
11. Redeem XMR.
12. Confirm XMR.
13. Mark trade completed.
14. Release reservations.
15. Finalize accounting.

119. Crash Matrix
Kill the swap engine at:
after trade creation
after crypto setup
after BTC transaction creation
after BTC broadcast
after BTC confirmation
after XMR transaction creation
after XMR broadcast
after XMR confirmation
after BTC redemption
after witness extraction
after XMR redemption
Every test must resume correctly.

120. Reorg Matrix
Test reorgs after:
BTC seen
BTC confirmed
BTC final
XMR seen
XMR confirmed
XMR final
Verify that no invalid state transition occurs.

121. Duplicate Matrix
Send duplicate:
crypto message
funding event
confirmation event
redemption event
recovery event
Expected:
one logical state transition

122. Failure Matrix
Test:
Alice offline
Bob offline
Bitcoin node offline
Monero node offline
Open Dex database offline
message queue offline
wallet offline
network partition
transaction broadcast timeout
transaction rejected
The system must distinguish:
temporary infrastructure failure
from:
protocol failure

123. Protocol Adapter Rule
The swap engine should know:
what state it is in
The protocol adapter should know:
what cryptographic action is valid
The blockchain adapter should know:
what happened on-chain
The wallet should know:
how to securely sign
This separation is the foundation of the architecture.

124. Recommended Rust Workspace
Conceptually:
opendex/
├── crates/
│   ├── protocol-types/
│   ├── swap-engine/
│   ├── btc-xmr-adapter/
│   ├── bitcoin-adapter/
│   ├── monero-adapter/
│   ├── wallet-interface/
│   ├── state-machine/
│   ├── blockchain-events/
│   ├── transport/
│   ├── recovery/
│   └── test-fixtures/
│
├── services/
│   ├── api/
│   ├── watcher-bitcoin/
│   ├── watcher-monero/
│   ├── swap-coordinator/
│   └── recovery-worker/
│
└── tests/
    ├── integration/
    ├── state-machine/
    ├── protocol/
    └── adversarial/

125. Recommended Dependency Boundary
Do not allow:
API
to import:
cryptographic implementation internals
Instead:
API
 ↓
Swap Engine
 ↓
AtomicSwap trait
 ↓
BtcXmrAdapter
 ↓
Selected protocol implementation
This means the underlying implementation can later be upgraded.

126. Upstream Protocol Selection
Before writing the cryptographic adapter, evaluate the current Eigenwallet implementation and its protocol code.
The current repository describes itself as a Monero-Bitcoin DEX based on atomic swaps and contains maker/taker swap components. GitHub
Its project documentation says the implementation has been operating for years and has processed substantial swap volume, although those claims should be treated as project claims rather than independent security certification. GitHub
Open Dex should therefore investigate:
eigenwallet/core
as the first integration candidate.

127. Important Compatibility Decision
Do not assume:
COMIT protocol
=
current Eigenwallet protocol
=
Open Dex protocol
The archived COMIT repository explicitly warns that the later fork introduced network-level breaking changes and is incompatible with peers using the archived implementation. GitHub
Therefore Open Dex must pin:
protocol version
+
implementation version
+
network version

128. Protocol Registry
Create:
protocol_registry
Example:
{
  "OPENDEX-BTC-XMR-1": {
    "implementation": "selected-upstream",
    "version": "...",
    "bitcoinNetworks": ["mainnet", "testnet"],
    "moneroNetworks": ["mainnet", "stagenet"],
    "enabled": true
  }
}

129. Protocol Disable Switch
The registry should support:
enabled = false
This prevents new swaps while allowing existing swaps to finish/recover.

130. What We Should NOT Build Yet
Do not build:
fiat rails
cash disputes
bond slashing
complex reputation
DAO governance
token
cross-chain routing
multi-hop swaps
until the BTC/XMR atomic swap is independently reliable.
The atomic settlement engine is the foundation.

131. v0.1 Definition of Done
Open Dex BTC/XMR v0.1 is complete when:
[ ] Maker can create BTC/XMR offer
[ ] Taker can accept
[ ] Swap terms become immutable
[ ] Cryptographic setup completes
[ ] BTC funding is detected
[ ] BTC funding is verified
[ ] XMR funding is detected
[ ] XMR funding is verified
[ ] Both legs reach finality
[ ] BTC redemption completes
[ ] Witness is extracted/validated
[ ] XMR redemption completes
[ ] Trade becomes completed
[ ] Fees finalize
[ ] Liquidity reservations release
And importantly:
[ ] Server restart works
[ ] Duplicate messages work
[ ] Duplicate chain events work
[ ] Bitcoin reorg handling works
[ ] Monero reorg handling works
[ ] Recovery works
[ ] Timeout handling works
[ ] Wallet failures are recoverable

132. The Architectural Picture
The complete Open Dex BTC/XMR system should ultimately look like:
                         OPEN DEX
                            |
             +--------------+--------------+
             |                             |
          ORDER BOOK                   TRADE ENGINE
             |                             |
             +-------------+---------------+
                           |
                     SWAP SESSION
                           |
                  +--------+--------+
                  |                 |
             PROTOCOL ENGINE    EVENT LOG
                  |
             +----+----+
             |         |
             v         v
           BTC         XMR
         ADAPTER     ADAPTER
             |         |
          WALLET     WALLET
             |         |
          BITCOIN    MONERO
             |         |
          WATCHER    WATCHER
             |         |
             +----+----+
                  |
             CHAIN EVENTS
                  |
                  v
             SWAP ENGINE
                  |
             STATE MACHINE
                  |
          +-------+-------+
          |               |
       COMPLETE        RECOVERY

133. Core Engineering Principle
The Open Dex implementation should make it difficult—not merely discouraged—for developers to bypass the protocol.
The architecture should make the safe path the easiest path:
Transaction
    ↓
Protocol Adapter
    ↓
Validation
    ↓
Signing
    ↓
Broadcast
    ↓
Blockchain Evidence
    ↓
State Transition
rather than:
API
 ↓
"send BTC"
 ↓
database = completed

134. Final Specification Boundary
The next document after this one should be:
Open Dex BTC/XMR — Cryptographic Adapter & Transaction Construction Specification v0.1
That document should contain the actual protocol-specific cryptographic interface:
secret generation
public commitments
DLEQ/equality proofs
adaptor signatures
Bitcoin transaction templates
Monero transaction construction
witness extraction
refund/recovery construction
That is intentionally not invented in this document.
The current Eigenwallet codebase is the most relevant implementation candidate to inspect next because it is the maintained successor to the archived COMIT implementation. GitHub+1

135. Implementation Order
The recommended build order is now:
PHASE 1
Protocol adapter harness
        ↓
PHASE 2
Local BTC/XMR integration
        ↓
PHASE 3
Swap state machine
        ↓
PHASE 4
Blockchain watchers
        ↓
PHASE 5
Wallet/signing boundary
        ↓
PHASE 6
Recovery engine
        ↓
PHASE 7
Open Dex order book
        ↓
PHASE 8
Fee/accounting engine
        ↓
PHASE 9
Security testing
        ↓
PHASE 10
Testnet
        ↓
PHASE 11
Limited mainnet
The critical insight is that the order book is not the hard part. The difficult and security-sensitive component is the bridge between the marketplace state machine and the BTC/XMR atomic-swap protocol.
That bridge should therefore be treated as a separately testable product:
             Open Dex
                 |
                 v
       +-------------------+
       | BTC/XMR Adapter   |
       +-------------------+
                 |
        +--------+--------+
        |                 |
        v                 v
     Bitcoin            Monero
If that adapter is correct, the rest of Open Dex can evolve around it without changing the fundamental settlement mechanism.
End of Open Dex BTC/XMR Swap — Sequence Diagrams & Transaction Specification v0.1
This gives us the rails for the implementation. The next step I would take is the cryptographic adapter spec, because that is where we stop talking in abstract terms and pin down the actual BTC transaction structure, Monero transaction flow, adaptor-signature/witness lifecycle, and exact interfaces to the maintained upstream implementation.

---

## Implementation Status — 2026-10-06

Milestones 1-3 completed. The following items from this specification have been implemented:

- **Swap state machine** — All states from the sequence diagrams implemented: Created → Negotiating → ProtocolSetup → Funding → Funded → Redeeming → Completed, plus Recovering/Refunded/Failed/WaitingForPeer
- **SettlementProtocol trait** — `process_message()` and `observe_chain()` methods implemented
- **BtcXmrAdapter** — Skeleton with message types defined (stub logic, no real cryptographic operations)
- **BitcoinChain / MoneroChain traits** — With mock implementations for testing
- **SwapEngine** — Coordinates swaps with persistence, recovery, and transport
- **Blockchain watchers** — BitcoinWatcher and MoneroWatcher poll chains and emit ChainObservation
- **Transport layer** — ProtocolEnvelope carries swap_id, message_id, sender, sequence for routing
- **Message replay protection** — MessageStore tracks seen message IDs per swap
- **Event sourcing** — All state transitions recorded as immutable events

Not yet implemented:
- Real DLEQ proof exchange in protocol messages
- Real adaptor signature creation and completion
- Real Bitcoin/Monero transaction broadcasting
- Witness extraction from on-chain signatures
- Real blockchain watchers (mock watchers used for testing)
