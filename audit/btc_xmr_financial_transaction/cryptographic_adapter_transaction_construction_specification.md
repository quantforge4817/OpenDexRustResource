Open Dex
BTC/XMR Cryptographic Adapter & Transaction Construction Specification v0.1
Status: Engineering specification
Version: 0.1
Pair: BTC ↔ XMR
Initial supported direction: BTC → XMR
Protocol family: Eigenwallet / COMIT / h4sh3d BTC-XMR atomic swap
Implementation language: Rust
Security classification: Cryptographic settlement core

1. Purpose
This document defines the cryptographic boundary between Open Dex and the BTC/XMR atomic-swap implementation.
The purpose is not to design new cryptography.
Open Dex should integrate a reviewed implementation of the established BTC/XMR atomic-swap construction and place a stable adapter around it.
The published construction uses Bitcoin-side adaptor signatures and cross-curve discrete-log-equality proofs connecting secp256k1 and Ed25519/Monero-side secrets. The underlying protocol uses Bitcoin timelocks and a Monero shared-spend-key construction rather than requiring Monero-side script/timelock functionality. GitHub+1

2. Design Principle
The architecture is:
                    OPEN DEX
                       |
                       v
              +----------------+
              |  Swap Engine   |
              +----------------+
                       |
                       v
              +----------------+
              | AtomicSwap API |
              +----------------+
                       |
                       v
              +----------------+
              | BTC/XMR Adapter|
              +----------------+
                 /          \
                /            \
               v              v
        Bitcoin Protocol   Monero Protocol
               |              |
               v              v
        Bitcoin Wallet    Monero Wallet
The Swap Engine knows:
swap state
participants
amounts
deadlines
events
The cryptographic adapter knows:
keys
commitments
proofs
adaptor signatures
transaction construction
transaction verification
witness extraction
refund/punish conditions

3. Non-Goals
v0.1 does not define:
new cryptographic primitives
new signature schemes
new DLEQ construction
new Monero consensus rules
new Bitcoin consensus rules
new custody mechanisms
fiat settlement
DAO governance
token economics

4. Reference Implementation
The first implementation target should be the maintained Eigenwallet codebase rather than the archived COMIT repository.
The current Eigenwallet repository describes itself as a Monero-Bitcoin DEX implementing atomic swaps and contains separate maker/taker swap components. GitHub
The old COMIT repository is explicitly marked unmaintained and directs users toward Eigenwallet. It also warns that the fork introduced network-level breaking changes. GitHub
Therefore Open Dex must pin an exact upstream protocol version.

5. Protocol Registry
Open Dex defines its own protocol identifier:
OPENDEX-BTC-XMR-1
The registry maps this to:
protocol ID
upstream implementation
upstream commit/tag
network compatibility
transaction format
message format
Example:
protocol:
  id: OPENDEX-BTC-XMR-1

  implementation:
    project: eigenwallet
    repository: core
    revision: <PINNED_COMMIT>

  bitcoin:
    network: mainnet

  monero:
    network: mainnet

  enabled: true
Never deploy:
"latest"
as a production cryptographic dependency.

6. Why the Pin Matters
The Eigenwallet project has continued making protocol changes during 2026.
For example, recent releases include changes to:
    • cross-curve equality-proof cryptography
    • Bitcoin output selection
    • Monero redeem/refund handling
    • Bitcoin punish transactions
    • timelocks
    • communication protocols
    • recovery behavior
The changelog records a cross-curve proof fix in version 4.13.0 and significant refund/punish changes in 4.0.0. GitHub
Therefore:
Open Dex protocol version
        !=
Git repository master
They must be explicitly mapped.

7. Cryptographic Actors
For this document:
Alice = BTC provider / XMR recipient

Bob = XMR provider / BTC recipient
In the Open Dex marketplace:
Maker = Bob
Taker = Alice

8. Cryptographic Goal
At the end of a successful swap:
Alice owns XMR
Bob owns BTC
At no point should either participant be able to obtain the counterparty's asset without satisfying the protocol's corresponding cryptographic condition.
The protocol must also provide a recovery path when the swap fails.

9. Core Cryptographic Relationship
The central mechanism links secrets represented across:
secp256k1
and:
Ed25519 / Monero
using a cross-curve discrete-log-equality proof.
Conceptually:
Bitcoin:
P_BTC = x * G_BTC

Monero:
P_XMR = x * G_XMR
The participant proves that the same secret scalar x relates the two public points.
The implementation must use the exact protocol-specific proof construction rather than a generic textbook DLEQ implementation.
The published research explicitly describes the need for equality-of-discrete-log proofs across the two groups. GitHub+1

10. Secret Scalar Domain
The shared secret must be valid in the relevant scalar domains.
The implementation must enforce the range and subgroup rules required by the selected protocol.
Do not implement:
secret % curve_order
blindly.
The protocol's exact scalar conversion and range requirements must be inherited from the reviewed implementation.

11. Secret Material Classes
Separate:
long-term wallet keys
from:
swap ephemeral secrets
and:
protocol-derived secrets
Never reuse wallet keys as ephemeral swap secrets unless the protocol explicitly requires it.

12. Secret Storage
Ephemeral secrets must be:
encrypted at rest
zeroized after use where possible
excluded from logs
excluded from telemetry
excluded from API responses
A database record should contain:
encrypted_secret
rather than:
raw_secret

13. Key Hierarchy
Conceptually:
Wallet Seed
    |
    +--> Bitcoin Wallet Keys
    |
    +--> Monero Wallet Keys
Separately:
Swap Session
    |
    +--> ephemeral BTC material
    |
    +--> ephemeral XMR material
    |
    +--> protocol witness material
The two trees should not be accidentally conflated.

14. Cryptographic Adapter
Open Dex exposes:
trait AtomicSwapProtocol {
    type Init;
    type PeerMessage;
    type FundingRequest;
    type RedemptionRequest;
    type RecoveryRequest;

    fn initialize(
        &self,
        context: SwapContext
    ) -> Result<Self::Init, ProtocolError>;

    fn process_message(
        &self,
        state: &mut ProtocolState,
        message: Self::PeerMessage
    ) -> Result<ProtocolEvent, ProtocolError>;

    fn verify_funding(
        &self,
        evidence: FundingEvidence
    ) -> Result<(), ProtocolError>;

    fn build_redemption(
        &self,
        request: Self::RedemptionRequest
    ) -> Result<TransactionPlan, ProtocolError>;

    fn extract_witness(
        &self,
        evidence: RedemptionEvidence
    ) -> Result<Witness, ProtocolError>;

    fn build_recovery(
        &self,
        request: Self::RecoveryRequest
    ) -> Result<TransactionPlan, ProtocolError>;
}

15. Important Interface Rule
The application should never call:
generate_adaptor_signature(...)
directly.
Instead:
Swap Engine
     |
     v
AtomicSwapProtocol
     |
     v
BTC/XMR implementation
This prevents application code from depending on cryptographic internals.

16. Protocol Context
struct SwapContext {
    swap_id: SwapId,

    btc_amount: Amount,
    xmr_amount: Amount,

    bitcoin_network: BitcoinNetwork,
    monero_network: MoneroNetwork,

    maker: ParticipantId,
    taker: ParticipantId,

    btc_redeem_address: BitcoinAddress,
    xmr_receive_address: MoneroAddress,

    protocol_version: ProtocolVersion,
}
The context becomes immutable after protocol initialization.

17. Cryptographic Commitment
Before cryptographic setup completes, Open Dex creates:
SwapCommitment
Conceptually:
H(
    protocol_version ||
    swap_id ||
    btc_amount ||
    xmr_amount ||
    participants ||
    destinations ||
    network_identifiers
)
The actual domain separation and serialization must be specified by the protocol implementation.
Do not use ad-hoc concatenation.

18. Canonical Encoding
Every signed or hashed structure requires:
canonical encoding
Never hash:
JSON.stringify(object)
unless the exact canonical JSON format is itself part of the protocol.
Prefer:
versioned binary encoding
or the upstream protocol's existing serialization.

19. Domain Separation
Every cryptographic hash should have an explicit domain.
Conceptually:
H(
    "OPENDEX-BTC-XMR-1/..."
    ||
    encoded_message
)
Do not reuse a generic hash across unrelated protocol operations.

20. Cross-Curve DLEQ
The DLEQ component establishes that related public points correspond to the same secret.
The adapter interface:
trait CrossCurveDleq {
    fn prove(
        &self,
        witness: Scalar
    ) -> Result<DleqProof, CryptoError>;

    fn verify(
        &self,
        statement: DleqStatement,
        proof: DleqProof
    ) -> Result<(), CryptoError>;
}

21. DLEQ Statement
Conceptually:
struct DleqStatement {
    btc_point: SecpPoint,
    xmr_point: EdPoint,
}
The actual proof includes the necessary auxiliary points and commitments required by the protocol.

22. DLEQ Security Requirements
The implementation must reject:
invalid point
identity point
wrong subgroup
malformed encoding
non-canonical scalar
wrong curve
wrong generator
invalid proof
before accepting the proof.

23. Cross-Curve Conversion
Do not convert:
secp256k1 point
to:
Ed25519 point
through an arbitrary serialization or coordinate mapping.
The two groups are related through the protocol's scalar witness, not because their points are interchangeable.

24. Adaptor Signature Layer
The Bitcoin leg uses an adaptor-signature construction.
Conceptually:
pre-signature
     +
witness
     =
valid signature
and:
valid signature
     -
pre-signature
     =
witness
The exact algebra depends on the signature scheme and implementation.
Open Dex must use the exact reviewed implementation and test vectors.

25. Bitcoin Signature Boundary
Define:
trait BitcoinAdaptorSignature {
    fn create_presignature(
        &self,
        request: AdaptorSigningRequest
    ) -> Result<AdaptorSignature, CryptoError>;

    fn verify_presignature(
        &self,
        request: AdaptorVerificationRequest
    ) -> Result<(), CryptoError>;

    fn adapt(
        &self,
        presignature: AdaptorSignature,
        witness: Witness
    ) -> Result<BitcoinSignature, CryptoError>;

    fn extract_witness(
        &self,
        presignature: AdaptorSignature,
        signature: BitcoinSignature
    ) -> Result<Witness, CryptoError>;
}

26. No Generic Signature Substitution
The adapter must specify:
signature scheme
curve
hash
nonce construction
encoding
s-value rules
sighash
transaction digest
A valid signature under another construction must not be accepted.

27. Signature Malleability
The adapter must enforce the canonical signature rules expected by the Bitcoin transaction format.
Particular attention must be paid to:
s-value normalization
nonce handling
signature encoding
witness extraction
The current protocol ecosystem has explicitly fixed cryptographic issues around cross-curve proofs, so these checks must be treated as security-critical rather than cosmetic. GitHub

28. Bitcoin Transaction Model
The BTC transaction layer should be represented as:
BitcoinTransactionPlan
rather than immediately as a serialized transaction.
struct BitcoinTransactionPlan {
    inputs: Vec<InputPlan>,
    outputs: Vec<OutputPlan>,
    lock_time: LockTime,
    sequence_rules: SequenceRules,
    signing_requirements: Vec<SigningRequirement>,
}

29. Funding Transaction
The funding transaction commits BTC to the protocol-defined output.
Conceptually:
Alice input(s)
      |
      v
+-----------------------+
| Swap funding output   |
| protocol conditions   |
+-----------------------+
      |
      +--> optional change
The exact script/output construction comes from the upstream protocol.

30. Funding Output Verification
The verifier must establish:
transaction is on correct network
+
correct funding output exists
+
amount >= required amount
+
output matches protocol commitment
+
transaction satisfies protocol requirements
Only then:
BTC_FUNDING_VALID

31. Funding Amount
The transaction amount must be checked against the committed amount.
For example:
required = 1.000 BTC
received = 0.999 BTC
must not silently become:
valid
unless the protocol explicitly defines a permitted fee/underfunding tolerance.

32. Overfunding
If:
received > committed
the protocol must define whether the difference is:
change
additional swap amount
invalid
Open Dex must not invent behavior after seeing the transaction.
The current Eigenwallet implementation has specifically had changes around Bitcoin output selection and underpayment/malicious-swap handling, demonstrating why this belongs in the protocol layer. GitHub

33. Bitcoin Redemption
The redemption transaction spends the protocol-controlled funding output according to the adaptor-signature construction.
Conceptually:
Funding Output
      |
      v
Pre-signature
      |
      +---- witness ----+
                         |
                         v
                  Valid Signature
                         |
                         v
                   BTC Redeem

34. Witness Extraction
When the valid Bitcoin redemption signature becomes observable:
Bitcoin chain
      |
      v
redeem transaction
      |
      v
signature
      |
      v
extract witness
Then:
VerifyWitness(witness)
must succeed.

35. Witness Is Not a Password
The extracted witness is cryptographic protocol material.
It must never be treated as:
user password
API token
authentication credential
It is usable only in the protocol context for which it was generated.

36. Witness Record
struct SwapWitness {
    swap_id: SwapId,
    witness_type: WitnessType,
    encoded_witness: SecretBytes,
    source_txid: TxId,
}
Store the witness encrypted.

37. Witness Source Binding
A witness must be bound to:
swap_id
expected commitment
expected transaction
expected signature
A witness extracted from an unrelated Bitcoin transaction must not be accepted.

38. Monero Key-Share Construction
The Monero side uses a shared spend-key construction.
Conceptually:
k_A
+
k_B
=
k_shared
under the protocol's scalar arithmetic.
The public Monero spend key corresponds to the combined secret.
The published protocol describes splitting the Monero private spend key into two secret shares and transferring knowledge in a way that enables the appropriate participant to reconstruct control at the correct protocol stage. Cryptology ePrint Archive

39. Key-Share Rule
Neither participant should receive the full Monero spend key prematurely.
During setup:
Alice:
share_A

Bob:
share_B
The protocol defines when one participant can derive:
share_A + share_B
and under which conditions.

40. Monero Public Key
The Monero funding output must correspond to the protocol-generated shared key material.
The adapter verifies:
expected shared public key
=
observed protocol output destination
using the exact Monero transaction/output mechanism supported by the implementation.

41. Monero Does Not Use a Bitcoin-Style HTLC
Do not design:
XMR HTLC
for v0.1.
The published construction explicitly avoids requiring Monero-side hashlocks/timelocks and instead uses off-chain cryptographic mechanisms. Cryptology ePrint Archive

42. Monero Funding
Conceptually:
Bob wallet
   |
   v
XMR funding transaction
   |
   v
shared protocol-controlled output
   |
   v
Monero network
The watcher verifies that the expected funds reached the expected protocol state.

43. Monero Redemption
After the appropriate protocol condition is reached:
required key material
       +
protocol transaction data
       |
       v
Monero redemption transaction
The adapter constructs/signs this transaction through the Monero wallet boundary.

44. Monero Refund
A separate refund path must exist for protocol failure.
The current Eigenwallet implementation has explicit Monero redeem/refund construction and publication states, rather than treating the entire operation as a single transaction event. GitHub
Open Dex should preserve that distinction:
XMR_REDEEM_CONSTRUCTED
XMR_REDEEM_PUBLISHED
XMR_REDEEM_CONFIRMED
and:
XMR_REFUND_CONSTRUCTED
XMR_REFUND_PUBLISHED
XMR_REFUND_CONFIRMED

45. Bitcoin Refund
The Bitcoin side uses timelocked recovery/refund mechanisms.
The atomic-swap construction relies on Bitcoin's timelock functionality because Monero itself does not provide equivalent script-level refund branches. Cryptology ePrint Archive

46. Timelock Ordering
The fundamental safety rule is:
BTC refund becomes available
        BEFORE
counterparty's irreversible XMR recovery deadline
with sufficient margin.
This margin must account for:
Bitcoin confirmation latency
Monero confirmation latency
network congestion
node outages
participant reaction time
reorg risk
The protocol literature explicitly identifies timelock ordering as a core atomicity requirement. GitHub

47. Timelocks Are Protocol Parameters
Do not hard-code:
REFUND_AFTER = 24 * 60 * 60;
inside transaction code.
Instead:
struct TimelockParameters {
    bitcoin_cancel: BitcoinLockTime,
    bitcoin_refund: BitcoinLockTime,
    punish_window: BitcoinLockTime,
    safety_margin: Duration,
}
The exact values must come from the pinned protocol version.

48. Current Upstream Example
The current Eigenwallet changelog records a reduction of the cancel timelock to 24 Bitcoin blocks, with a 72-block punish window, but Open Dex should not blindly copy those values. They are implementation-version-specific protocol parameters. GitHub
The correct procedure is:
pin upstream version
        ↓
extract parameters
        ↓
document rationale
        ↓
test
        ↓
freeze Open Dex protocol version

49. Punishment
The current Eigenwallet protocol contains a Bitcoin punish mechanism and exposes a deterministic punish transaction identifier for swaps. GitHub
Open Dex should model punishment as a protocol-defined state:
PUNISH_ELIGIBLE
rather than:
ADMIN_DECISION

50. No Centralized Punishment
Open Dex should not possess:
"punish user"
authority.
Instead:
protocol condition
      |
      v
punishment transaction
is enforced by the underlying transaction rules.

51. Recovery Decision Interface
trait RecoveryPolicy {

    fn evaluate(
        &self,
        swap: &SwapState,
        bitcoin: &BitcoinChainState,
        monero: &MoneroChainState,
    ) -> RecoveryDecision;
}
Possible results:
enum RecoveryDecision {
    Wait,
    Redeem,
    Refund,
    Punish,
    Abort,
    InvalidState,
}

52. Protocol State
Cryptographic protocol state:
enum AtomicSwapState {
    Created,
    Setup,
    SetupVerified,

    BtcFundingPending,
    BtcFundingConfirmed,

    XmrFundingPending,
    XmrFundingConfirmed,

    BothFunded,

    BtcRedeemPending,
    BtcRedeemed,

    WitnessAvailable,

    XmrRedeemConstructed,
    XmrRedeemPublished,
    XmrRedeemed,

    XmrRefundConstructed,
    XmrRefundPublished,
    XmrRefunded,

    BtcRefunded,
    Punished,

    Completed,
    Failed,
}
The final names should map precisely onto the pinned upstream implementation.

53. Cryptographic State vs UI State
Never expose the raw cryptographic state directly to users.
Map:
BtcRedeemPending
to:
"Completing BTC settlement"
and:
XmrRefundConstructed
to:
"Preparing XMR recovery"

54. Transaction Construction Pipeline
Every transaction follows:
Protocol State
      |
      v
Transaction Intent
      |
      v
Protocol Transaction Builder
      |
      v
Unsigned Transaction
      |
      v
Transaction Verification
      |
      v
Wallet Signing
      |
      v
Signed Transaction
      |
      v
Local Verification
      |
      v
Broadcast
      |
      v
Blockchain Observation

55. Never Broadcast an Unverified Transaction
Before broadcast:
verify:
    network
    inputs
    outputs
    amount
    locktime
    sequence
    sighash
    protocol commitment
    signature validity
Then broadcast.

56. Local Transaction Verification
trait TransactionVerifier<Tx> {
    fn verify(
        &self,
        tx: &Tx,
        expected: &TransactionExpectation
    ) -> Result<(), VerificationError>;
}
This provides a second safety barrier before network publication.

57. Transaction Expectation
struct TransactionExpectation {
    swap_id: SwapId,
    transaction_type: TransactionType,

    required_inputs: Vec<OutPoint>,
    required_outputs: Vec<OutputExpectation>,

    required_amount: Amount,

    locktime: Option<LockTime>,

    protocol_commitment: Commitment,
}

58. Bitcoin Fee Handling
The transaction builder estimates:
required fee
but Open Dex's economic layer separately records:
network fee
The protocol fee remains:
protocol_fee
Do not merge them.

59. Fee Buffer
Open Dex may estimate:
network_fee_estimate
and add an application-level buffer.
But the buffer must never alter the cryptographic settlement amount without creating a new immutable transaction commitment.

60. Fee Failure
If the Bitcoin transaction cannot be broadcast because:
fee too low
the swap enters:
BROADCAST_FAILURE
not:
COMPLETED
Whether fee replacement is safe must be defined by the protocol.

61. Fee Replacement
v0.1 default:
fee replacement disabled
unless explicitly supported by the selected protocol implementation.
The reason is that replacing a transaction can alter assumptions about:
transaction identity
timelocks
dependent transactions
witness extraction
refund paths

62. Bitcoin Output Selection
Bitcoin output selection is protocol-sensitive.
The current Eigenwallet changelog records a fix to Bitcoin output selection in 2026. GitHub
Therefore Open Dex must never implement:
find first output with amount >= X
as generic swap logic.
Use:
ProtocolOutputSelector

63. Output Selection Interface
trait ProtocolOutputSelector {

    fn find_funding_output(
        &self,
        tx: &BitcoinTransaction,
        expectation: &FundingExpectation
    ) -> Result<OutPoint, VerificationError>;
}

64. Monero Output Selection
Likewise, Monero output discovery must be performed using the protocol-specific wallet/node mechanisms.
The application must not infer ownership merely from:
amount
because Monero's privacy model deliberately prevents naive public attribution.

65. Cryptographic Verification Pipeline
For every peer-provided proof:
deserialize
   |
   v
canonicality check
   |
   v
curve validation
   |
   v
subgroup validation
   |
   v
proof verification
   |
   v
statement binding
   |
   v
swap binding
Only then:
ACCEPT

66. Malformed Input
Malformed cryptographic data must result in:
ProtocolError::InvalidMessage
not:
panic!
or:
unwrap()
The swap process must fail safely.

67. Secret Zeroization
Use a secret type:
struct SecretScalar {
    bytes: Zeroizing<[u8; 32]>,
}
or the equivalent supported by the cryptographic library.
Never implement:
Debug
for a secret that prints the underlying bytes.

68. Logging Rules
Allowed:
swap_id
message_type
protocol_version
txid
block_height
state
error_code
Forbidden:
private key
secret scalar
DLEQ witness
raw seed
encrypted secret plaintext
full adaptor secret

69. Error Taxonomy
Define:
enum ProtocolError {
    InvalidMessage,
    InvalidProof,
    InvalidPoint,
    InvalidScalar,

    CommitmentMismatch,
    TransactionMismatch,
    AmountMismatch,

    WrongNetwork,
    WrongProtocolVersion,

    InvalidAdaptorSignature,
    WitnessExtractionFailed,

    TimelockViolation,
    RecoveryNotAllowed,

    TransactionConstructionFailed,
    TransactionSigningFailed,
    BroadcastFailed,

    ChainStateUnknown,
    ProtocolStateViolation,
}

70. Error Severity
Classify:
Recoverable
for:
node unavailable
temporary broadcast failure
peer disconnected
and:
Fatal
for:
invalid cryptographic proof
invalid witness
commitment mismatch
unexpected transaction
protocol invariant violation

71. Cryptographic Test Vectors
The implementation must have deterministic test vectors for:
DLEQ
adaptor signatures
witness extraction
transaction construction
transaction verification
refund construction
punish construction
Where upstream test vectors exist, Open Dex must reproduce them exactly before adding Open Dex-specific vectors.
Published engineering references specifically point to DLC adaptor-signature test vectors and the h4sh3d/COMIT cross-curve construction. GitHub+1

72. Negative Test Vectors
For every valid vector create invalid variants:
wrong witness
wrong scalar
wrong curve point
wrong participant
wrong commitment
wrong transaction
wrong amount
wrong sighash
wrong locktime
wrong network
modified signature
modified proof
Each must fail.

73. Property Tests
Test:
adapt(pre_signature, witness)
and:
extract(pre_signature, adapted_signature)
against the protocol invariant.
Conceptually:
extract(
    presignature,
    adapt(presignature, witness)
)
=
witness
for valid protocol inputs.

74. No Cross-Protocol Witnesses
A witness from:
OPENDEX-BTC-XMR-1
must not validate under:
OPENDEX-BTC-XMR-2
unless explicitly designed to do so.
Protocol version must be included in the cryptographic domain.

75. Transaction Serialization
Never sign:
human-readable transaction
Sign exactly:
protocol-defined transaction digest
The serialization implementation must be tested against the Bitcoin/Monero reference implementation.

76. Bitcoin Signing Boundary
Protocol Builder
       |
       | signing digest
       v
Bitcoin Signer
       |
       | signature
       v
Protocol Builder
       |
       v
final transaction
The signer should not know why the transaction exists.

77. Monero Signing Boundary
Likewise:
Protocol Builder
       |
       v
Monero Wallet/Signer
       |
       v
signed transaction
The Open Dex application should never manipulate Monero private spend keys directly.

78. Wallet Interface
trait SwapWallet {

    fn sign_bitcoin(
        &self,
        request: BitcoinSigningRequest
    ) -> Result<SignedBitcoinTransaction, WalletError>;

    fn sign_monero(
        &self,
        request: MoneroSigningRequest
    ) -> Result<SignedMoneroTransaction, WalletError>;
}

79. Wallet Authorization
The wallet should validate:
swap_id
transaction_type
expected outputs
expected amount
network
before signing.
This protects against a compromised swap engine attempting to make an arbitrary transaction.

80. Two-Layer Verification
Before a wallet signs:
Swap Engine verification
and:
Wallet verification
should both pass.
Conceptually:
Protocol
   |
   v
Expected Transaction
   |
   +------> Swap Engine
   |
   +------> Wallet
             |
             v
           SIGN

81. Compromised Coordinator Model
Assume:
Open Dex server is malicious
Can it steal funds?
The desired answer is:
not directly
because the wallet and protocol enforce transaction constraints.
This is a key security objective.

82. Compromised Relay Model
Assume:
relay malicious
It can:
delay messages
drop messages
replay messages
It must not be able to:
forge valid cryptographic protocol messages
steal funds
alter transaction terms

83. Malicious Counterparty Model
Assume Bob:
lies
disconnects
sends malformed proofs
funds incorrectly
attempts premature redemption
Alice must still have a protocol-defined recovery path.
Likewise in the opposite direction.

84. Atomicity Invariant
The core invariant:
If Alice can irreversibly obtain Bob's XMR,
then Bob must have obtained the corresponding BTC
or be able to recover his BTC under the protocol rules.
And conversely:
If Bob obtains BTC,
Alice must be able to obtain XMR
or the protocol must provide her with a valid recovery path.
The precise formal statement belongs in the security proof.

85. Liveness Invariant
Assuming:
both participants follow the protocol
+
blockchains continue progressing
+
required nodes remain available
the swap eventually reaches:
COMPLETED
or:
REFUNDED

86. Safety Invariant
No sequence of:
valid protocol messages
+
valid blockchain events
should permit:
both parties to lose their assets
under the protocol's stated assumptions.

87. Information Leakage
The protocol should minimize public linkage between:
BTC transaction
and:
XMR transaction
The adaptor-signature/scriptless construction is specifically motivated in part by reducing explicit cross-chain swap identification compared with conventional hashlock constructions. GitHub
Open Dex must therefore avoid adding unnecessary metadata such as:
swap_id
order_id
user ID
maker ID
to public blockchain transactions.

88. Swap ID Privacy
Internally:
swap_id = UUID
Externally:
swap_id
must not be embedded in:
Bitcoin OP_RETURN
or equivalent public transaction metadata unless explicitly required.

89. Database-to-Chain Correlation
The server can maintain:
swap_id -> txid
internally.
But the chain should not necessarily contain:
swap_id
that directly reveals the database record.

90. Hermes / Future Offline Completion
The current Eigenwallet protocol has introduced Hermes, which uses Monero transactions for communication so a swap can continue without a live peer-to-peer connection after initial setup. GitHub
Open Dex should therefore define:
trait ProtocolTransport {
    fn send(...);
    fn receive(...);
}
rather than assuming:
live TCP/Tor connection
is required for the entire swap.

91. Transport Independence
Possible transports:
Direct P2P
Tor
libp2p
Relay
Hermes-like blockchain messaging
The cryptographic state machine must remain unchanged.

92. On-Chain-Only Continuation
After the protocol reaches the appropriate lock state:
on-chain settlement
should be capable of continuing even if:
Open Dex relay
goes offline.
This is a major resilience property.

93. Open Dex Server Outage
If Open Dex disappears after both participants have entered the cryptographic settlement phase:
funds must remain governed by the swap protocol
rather than:
funds depend on Open Dex availability

94. Cryptographic Adapter Persistence
The adapter state must be restartable.
Persist:
protocol version
swap ID
encrypted secrets
commitments
peer public data
transaction IDs
state
timelocks
Do not persist:
derived secret material
unless necessary for recovery.

95. Deterministic Reconstruction
Where possible:
state + encrypted secrets + chain evidence
should reconstruct the exact next protocol action.
This is particularly important for:
refund
punish
redeem
operations.

96. Recovery After Crash
Example:
XMR_REDEEM_CONSTRUCTED
server crashes.
On restart:
load transaction intent
      |
      v
search chain for tx
      |
      +--> found -> mark published
      |
      +--> absent -> safely resume construction/publication
The current Eigenwallet changelog specifically mentions avoiding republishing a Monero redeem/refund transaction when it is already on-chain after a restart. GitHub
Open Dex should make this a formal invariant.

97. Transaction Intent Table
transaction_intents
-------------------
id
swap_id
chain
transaction_type
intent_hash
unsigned_tx_hash
txid
status
created_at
broadcast_at
finalized_at
Unique:
swap_id + transaction_type
unless the protocol explicitly permits multiple attempts.

98. Cryptographic Material Table
swap_secrets
------------
swap_id
secret_type
encrypted_value
key_version
created_at
destroyed_at
Access must be tightly restricted.

99. Secret Types
Example:
EPHEMERAL_SWAP_SECRET
DLEQ_WITNESS
ADAPTOR_WITNESS
KEY_SHARE
DERIVED_REDEMPTION_SECRET
The exact types depend on the upstream implementation.

100. Key Encryption
Use an envelope-encryption architecture:
swap secret
    |
    v
data encryption key
    |
    v
KMS / wallet encryption key
The database should never hold an unencrypted long-lived secret.

101. Security Audit Boundary
A third-party audit must specifically review:
cross-curve DLEQ
adaptor signatures
witness extraction
scalar handling
transaction construction
transaction verification
timelock ordering
refund
punish
key-share recovery
wallet authorization
state transitions
The fact that the upstream project is battle-tested is useful evidence, but it is not a substitute for auditing Open Dex's integration.
Eigenwallet itself describes the swap implementation as battle-tested, while its recent changelog demonstrates that security-sensitive protocol fixes continue to occur. GitHub+1

102. Audit Requirement
Open Dex mainnet launch requires:
[ ] independent cryptographic review
[ ] transaction-construction review
[ ] state-machine review
[ ] wallet-boundary review
[ ] recovery review
[ ] fuzzing
[ ] integration tests
[ ] adversarial testing

103. Fuzz Targets
At minimum:
DLEQ proof parser
DLEQ verifier
adaptor signature parser
adaptor verifier
Bitcoin transaction parser
Bitcoin output selector
Monero transaction parser
protocol message parser
state transition engine
recovery engine

104. Differential Testing
Where possible:
Open Dex adapter
        vs
upstream implementation
must produce identical:
transaction structures
proof verification results
state decisions
recovery eligibility
for identical test vectors.

105. Golden Tests
Store:
golden_swap_001
golden_swap_002
...
containing:
protocol messages
public commitments
transaction IDs
serialized transaction fixtures
expected states
expected witnesses
No secrets should be stored in repository fixtures unless they are synthetic test secrets.

106. Mainnet Safety Gate
The protocol cannot be enabled simply by:
enabled: true
Require:
protocol_version
+
network
+
security_review_hash
+
approved_commit
Example:
OPENDEX-BTC-XMR-1:
  enabled: true
  bitcoin: mainnet
  monero: mainnet
  approved_revision: abc123...
  security_review: REVIEW-2026-001

107. Protocol Upgrade
A new protocol version is:
OPENDEX-BTC-XMR-2
not:
OPENDEX-BTC-XMR-1
with silently changed transaction rules.

108. Existing Swaps
When a new version is released:
new swaps -> newest approved version
existing swaps -> original protocol version
Never migrate an active swap between cryptographic protocols.

109. Protocol Sunset
A protocol can be:
ACTIVE
DEPRECATED
DISABLED
DISABLED means:
no new swaps
Existing swaps continue according to their protocol rules where safe.

110. Adapter Package Boundary
Recommended crate:
opendex-btc-xmr-protocol
Dependencies:
crypto primitives
bitcoin transaction library
monero protocol library
upstream swap implementation
serialization
zeroization
It should not depend on:
HTTP
database
UI
order book

111. Pure Core
The ideal architecture is:
             PURE
        CRYPTOGRAPHIC CORE
                |
                v
        TRANSACTION PLANS
                |
                v
          WALLET ADAPTER
                |
                v
          BLOCKCHAIN
The cryptographic core should be testable entirely offline.

112. Example Pure Function
fn verify_btc_funding(
    commitment: &SwapCommitment,
    tx: &BitcoinTransaction,
) -> Result<FundingEvidence, ProtocolError>
No RPC.
No database.
No network.
No filesystem.

113. Example Pure Recovery Function
fn evaluate_recovery(
    protocol_state: &ProtocolState,
    chain_state: &ChainState,
    now: Timestamp,
) -> Result<RecoveryDecision, ProtocolError>
This should be deterministic.

114. Example Transaction Builder
fn build_btc_refund(
    state: &ProtocolState,
    wallet: &WalletContext,
) -> Result<BitcoinTransactionPlan, ProtocolError>
The function should produce an unsigned plan.
Signing is separate.

115. Never Mix Construction and Broadcast
Bad:
swap.refund();
Better:
evaluate recovery
        ↓
build transaction
        ↓
verify transaction
        ↓
request signature
        ↓
verify signature
        ↓
broadcast
        ↓
observe

116. Cryptographic Transaction Lifecycle
                 PROTOCOL
                    |
                    v
              BUILD INTENT
                    |
                    v
             BUILD TX PLAN
                    |
                    v
              VERIFY PLAN
                    |
                    v
                 SIGN
                    |
                    v
             VERIFY SIGNATURE
                    |
                    v
               BROADCAST
                    |
                    v
             OBSERVE CHAIN
                    |
                    v
               FINALIZE

117. Security Invariant
Every arrow above is a security boundary.
There should never be a direct:
user input
   |
   v
signed transaction
path.

118. Open Dex Fee Integration
The cryptographic adapter must receive the already finalized settlement amount.
For example:
quoted trade:
1 BTC = 100 XMR

protocol fee:
0.30%

network cost:
separate
The adapter receives:
BTC settlement amount
XMR settlement amount
It does not calculate:
maker fee
taker fee
spread

119. Why Fees Stay Outside
This allows:
Order Book
to change fee policy without touching:
Atomic Swap Protocol
and allows the protocol to be independently audited.

120. Example
Marketplace:
BTC:
1.00000000

XMR:
100.00000000

Protocol fee:
0.003 BTC equivalent
Settlement engine receives the exact final amounts required by the trade contract.
The cryptographic layer does not care whether the quote came from:
maker spread
protocol fee
promotion
liquidity incentive

121. Security Review Checklist
Before enabling a protocol version:
[ ] Protocol paper reviewed
[ ] Upstream source reviewed
[ ] Exact commit pinned
[ ] Cross-curve proof verified
[ ] Adaptor signature verified
[ ] Scalar range verified
[ ] Point validation verified
[ ] Signature canonicality verified
[ ] Transaction templates verified
[ ] Output selection verified
[ ] Timelocks verified
[ ] Refund path verified
[ ] Punish path verified
[ ] Key-share recovery verified
[ ] Reorg behavior verified
[ ] Crash recovery verified
[ ] Duplicate handling verified
[ ] Wallet authorization verified
[ ] Fuzzing completed
[ ] External audit completed

122. Definition of Done
The Open Dex cryptographic adapter is v0.1 complete when:
[ ] Upstream protocol version pinned
[ ] Adapter trait implemented
[ ] BTC funding construction integrated
[ ] BTC funding verification integrated
[ ] XMR funding construction integrated
[ ] XMR funding verification integrated
[ ] BTC adaptor signature path integrated
[ ] Witness extraction integrated
[ ] XMR redemption integrated
[ ] BTC refund integrated
[ ] XMR refund integrated
[ ] Punish path integrated
[ ] Recovery path integrated
[ ] Transaction intents implemented
[ ] Crash recovery implemented
[ ] Reorg handling implemented
[ ] Golden vectors implemented
[ ] Negative vectors implemented
[ ] Fuzz targets implemented
[ ] Integration tests passing

123. The Most Important Architectural Decision
Open Dex should not fork the cryptographic implementation and immediately modify it.
Instead:
                 Upstream
             BTC/XMR protocol
                    |
                    v
             Compatibility
                Adapter
                    |
                    v
                Open Dex
This gives us:
upstream security fixes
+
Open Dex marketplace
+
stable integration boundary
without turning Open Dex into a second independent cryptography project.

124. What Open Dex Owns
Open Dex owns:
order book
maker quotes
matching
fees
trade IDs
swap IDs
UI
reputation
liquidity management
transport abstraction
database
monitoring
accounting

125. What the Protocol Owns
The atomic-swap implementation owns:
cryptographic secrets
DLEQ
adaptor signatures
key shares
transaction construction
transaction verification
timelocks
refund
punishment
witness extraction

126. What the Blockchain Owns
Bitcoin/Monero consensus owns:
whether a transaction is valid
whether it is included
whether it is confirmed
whether it remains canonical
Open Dex observes these facts.
It does not create them.

127. Final Architecture
                         OPEN DEX
                            |
                    +-------+-------+
                    |               |
                ORDER BOOK       TRADE
                    |             ENGINE
                    +-------+-------+
                            |
                       SWAP ENGINE
                            |
                    +-------+-------+
                    |               |
              PROTOCOL STATE    EVENT LOG
                    |
                    v
          +----------------------+
          | BTC/XMR PROTOCOL     |
          | ADAPTER              |
          +----------+-----------+
                     |
          +----------+-----------+
          |                      |
          v                      v
     Bitcoin Core          Monero Core
          |                      |
          v                      v
       Wallet                 Wallet
          |                      |
          v                      v
      Bitcoin                 Monero
      network                network
          |                      |
          +----------+-----------+
                     |
                  OBSERVERS
                     |
                     v
                SWAP ENGINE

128. Final Engineering Rule
The Open Dex protocol should be designed so that:
The marketplace can be compromised without allowing the marketplace to become the custodian.
That is the architectural target.
The order book can go offline.
The relay can go offline.
The API can restart.
The database can be reconstructed.
The peer can disconnect.
The blockchain can reorganize.
And yet the underlying BTC/XMR settlement rules remain enforced by cryptography and the two blockchains.
That is what turns Open Dex from a conventional exchange backend into a genuinely non-custodial atomic-swap marketplace.

129. Next Implementation Document
The next engineering document should therefore be:
Open Dex BTC/XMR — Rust Crate Architecture & Interface Specification v0.1
It should turn this document into actual code boundaries:
opendex-protocol-types
opendex-btc-xmr
opendex-state-machine
opendex-bitcoin
opendex-monero
opendex-wallet
opendex-transport
opendex-watchers
opendex-recovery
opendex-api
and define the actual Rust structs, traits, enums, database schema, event types, error types, and test layout.
That is the point where we can start building the repository rather than merely describing the protocol.
End of Open Dex BTC/XMR Cryptographic Adapter & Transaction Construction Specification v0.1

---

## Implementation Status — 2026-10-06

Milestones 1-3 completed. The following items from this specification have been implemented:

- **BtcXmrAdapter skeleton** — Created in `opendex-btc-xmr` with message types defined but stub logic
- **SettlementProtocol trait** — The cryptographic boundary is implemented in `opendex-protocol-types`
- **ProtocolMessage types** — Extended with `message_id`, `sender`, `sequence` for transport routing
- **ProtocolEnvelope** — Wraps protocol messages with routing metadata in `opendex-transport`
- **BitcoinChain / MoneroChain traits** — Implemented with mock versions in `opendex-bitcoin` and `opendex-monero`
- **Generic Chain trait** — Implemented in `opendex-chain`

Not yet implemented:
- Real DLEQ proof generation and verification
- Adaptor signature creation and completion
- Bitcoin 2-of-2 multisig transaction construction
- Monero transaction construction with stealth addresses
- Witness extraction from on-chain signatures
- Real Eigenwallet library integration
