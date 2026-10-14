# OpenDex Protocol Specification

**Version:** 0.1.0-security-review
**Date:** 2026-10-14
**Status:** Authoritative protocol reference
**Scope:** BTC-XMR atomic swap protocol and fiat-crypto P2P trading protocol

---

## Table of Contents

1. [Protocol Overview](#1-protocol-overview)
2. [BTC-XMR Atomic Swap Protocol](#2-btc-xmr-atomic-swap-protocol)
3. [Fiat-Crypto P2P Trading Protocol](#3-fiat-crypto-p2p-trading-protocol)
4. [P2P Network Protocol](#4-p2p-network-protocol)
5. [Timeout Mechanisms](#5-timeout-mechanisms)
6. [Protocol Assumptions](#6-protocol-assumptions)
7. [Protocol Versioning](#7-protocol-versioning)

---

## 1. Protocol Overview

OpenDex implements two distinct trading protocols that share the same P2P transport and user management infrastructure:

1. **BTC-XMR Atomic Swap Protocol** — Cryptographically guaranteed, trustless cross-chain swaps between Bitcoin and Monero using DLEQ (Discrete Log Equality) proofs and adaptor signatures.
2. **Fiat-Crypto P2P Trading Protocol** — Fiat-to-crypto trades (GBP/EUR/USD ↔ BTC/XMR) using payment adapters, Bitcoin 2-of-2 multisig escrow, security bonds, and dispute resolution.

The protocols are separated by design: the marketplace layer sees only the `SettlementProtocol` trait and never implementation details. The atomic swap protocol owns cryptographic settlement; the fiat protocol owns payment coordination and escrow.

### Key Design Principles

- **Non-custodial:** The DEX never takes custody of user funds. Atomic swaps use protocol-level locking; fiat trades use 2-of-2 multisig requiring both parties to co-sign.
- **Integer-only arithmetic:** All monetary values use integer types (`Satoshis`, `Piconero`, `FiatAmount` in minor units). No floating-point arithmetic.
- **Event sourcing:** Every state change produces an immutable event. Full state can be reconstructed from the event log.
- **Replay protection:** All protocol messages carry unique IDs and monotonically increasing sequence numbers.
- **State machine enforcement:** All transitions are validated by explicit `can_transition_to()` methods. Invalid transitions return errors.

---

## 2. BTC-XMR Atomic Swap Protocol

**Crate:** `opendex-protocol-types` (abstractions), `opendex-btc-xmr` (implementation), `opendex-swap` (engine)

### 2.1 Protocol Participants

| Role | Description |
|------|-------------|
| Maker | Initiates the swap; provides the trading pair and amounts |
| Taker | Accepts the swap; provides counterparty funds |

### 2.2 Swap Context

Before protocol initialization, an immutable `SwapContext` is established:

```
SwapContext
├── swap_id: SwapId              // Unique swap identifier
├── trade_id: TradeId            // Parent trade reference
├── btc_amount: Satoshis         // Bitcoin amount (integer satoshis)
├── xmr_amount: Piconero         // Monero amount (integer piconeros)
├── bitcoin_network: BitcoinNetwork  // Mainnet/Testnet
├── monero_network: MoneroNetwork    // Mainnet/Testnet
├── maker: ParticipantId         // Maker's participant ID
├── taker: ParticipantId         // Taker's participant ID
├── btc_destination: String      // Maker's BTC destination address
├── xmr_destination: String      // Taker's XMR destination address
└── protocol_version: ProtocolVersion  // Negotiated protocol version
```

### 2.3 State Machine

**10 States:**

```
Created → Negotiating → ProtocolSetup → Funding → Funded → Redeeming → Completed
   │           │             │            │        │          │
   └───────────┴─────────────┴────────────┴────────┴──────────┘
                        ↓
                     Failed
```

**Additional states:**

- `Recovering` — Entered from any active state when recovery is triggered
- `Refunded` — Reached from `Recovering` when funds are returned
- `WaitingForPeer` — Entered when counterparty disconnects; can resume on reconnection

**Complete transition matrix (from `SwapState::can_transition_to()`):**

| From | To |
|------|-----|
| `Created` | `Negotiating`, `Recovering`, `Failed` |
| `Negotiating` | `ProtocolSetup`, `Recovering`, `Failed`, `WaitingForPeer` |
| `ProtocolSetup` | `Funding`, `Recovering`, `Failed`, `WaitingForPeer` |
| `Funding` | `Funded`, `Recovering`, `Failed`, `WaitingForPeer` |
| `Funded` | `Redeeming`, `Recovering`, `Failed`, `WaitingForPeer` |
| `Redeeming` | `Completed`, `Recovering`, `Failed`, `WaitingForPeer` |
| `Recovering` | `Refunded`, `Failed` |
| `WaitingForPeer` | `Negotiating`, `ProtocolSetup`, `Funding`, `Funded`, `Redeeming`, `Failed` |

### 2.4 Protocol Actions

After processing a message or blockchain observation, the protocol returns a `Vec<ProtocolAction>`:

| Action | Description |
|--------|-------------|
| `SendMessage(ProtocolMessage)` | Send a message to the counterparty |
| `BuildTransaction(TransactionIntent)` | Build and broadcast a transaction |
| `Wait(WaitCondition)` | Wait for a specific condition (funding, redemption, timeout) |
| `Complete` | The swap is complete |
| `Recover(RecoveryAction)` | Begin recovery procedure |
| `Fail(ProtocolFailure)` | The swap has failed |

### 2.5 Transaction Types

| Type | Chain | Description |
|------|-------|-------------|
| `BtcFunding` | Bitcoin | Lock BTC in the atomic swap contract |
| `XmrFunding` | Monero | Lock XMR in the atomic swap contract |
| `BtcRedemption` | Bitcoin | Redeem BTC after counterparty claims XMR |
| `XmrRedemption` | Monero | Redeem XMR after revealing secret |
| `BtcRefund` | Bitcoin | Refund BTC after timeout |
| `XmrRefund` | Monero | Refund XMR after timeout |

### 2.6 Blockchain Observations

The protocol receives `ChainObservation` events from blockchain watchers:

| Observation | Description |
|-------------|-------------|
| `BtcFundingSeen` | BTC funding transaction detected on-chain |
| `BtcFundingConfirmed` | BTC funding transaction has required confirmations |
| `BtcRedemptionSeen` | BTC redemption transaction detected |
| `XmrFundingSeen` | XMR funding transaction detected on-chain |
| `XmrFundingConfirmed` | XMR funding transaction has required confirmations |
| `XmrRedemptionSeen` | XMR redemption transaction detected |
| `BtcRefundSeen` | BTC refund transaction detected |
| `XmrRefundSeen` | XMR refund transaction detected |
| `Reorg` | Blockchain reorganization detected |

### 2.7 Swap Sequence (Happy Path)

```
Maker (sells BTC, buys XMR)          Taker (sells XMR, buys BTC)
            │                              │
            ├─ SwapContext created ────────┤
            │                              │
            ├─ Negotiate protocol version ─┤
            │                              │
            ├─ Exchange DLEQ proofs ──────┤  (ProtocolSetup)
            │                              │
            ├─ Lock BTC on-chain ─────────┤  (Funding)
            │  (BtcFunding transaction)    │
            │                              │
            │  ← BtcFundingConfirmed ─────┤  (Chain observation)
            │                              │
            │  ── XmrFunding transaction → │  (Funded)
            ├─ Lock XMR on-chain ─────────┤
            │                              │
            │  ← XmrFundingConfirmed ─────┤  (Chain observation)
            │                              │
            ├─ Reveal secret, claim XMR ──┤  (Redeeming)
            │  (XmrRedemption)             │
            │                              │
            ├─ Use secret, claim BTC ─────┤  (Redeeming)
            │  (BtcRedemption)             │
            │                              │
            ├─ Swap COMPLETED ────────────┤
```

### 2.8 SettlementProtocol Trait

The central interface that the marketplace uses without knowing implementation details:

```rust
pub trait SettlementProtocol {
    type State: Serialize + Deserialize + Send + Sync;
    type Message: Serialize + Deserialize + Send + Sync;
    type Error: From<ProtocolError> + Error + Send + Sync;

    fn initialize(&self, context: SwapContext) -> Result<Self::State, Self::Error>;
    fn process_message(&self, state: &mut Self::State, message: Self::Message)
        -> Result<Vec<ProtocolAction>, Self::Error>;
    fn observe_chain(&self, state: &mut Self::State, observation: ChainObservation)
        -> Result<Vec<ProtocolAction>, Self::Error>;
    fn recover(&self, state: &Self::State) -> Result<Vec<ProtocolAction>, Self::Error>;
}
```

### 2.9 Recovery Engine

**Crate:** `opendex-recovery`

Evaluates swap state combined with blockchain observations and returns one of:

| Decision | Action |
|----------|--------|
| `Wait` | Continue monitoring; no action needed |
| `Redeem` | Counterparty has locked funds; initiate redemption |
| `Refund` | Swap cannot complete; refund locked funds |
| `Punish` | Counterparty violated protocol; claim penalty |
| `Abort` | Irrecoverable error; abandon swap |

### 2.10 Message Structure

All protocol messages carry replay protection metadata:

```
ProtocolMessage
├── swap_id: SwapId              // Which swap this message belongs to
├── payload: Vec<u8>             // Protocol-specific serialized payload
├── message_id: MessageId        // Unique ID for replay protection
├── sender: ParticipantId        // Who sent this message
└── sequence: u64                // Monotonically increasing sequence number
```

The `MessageStore` in `opendex-transport` tracks seen message IDs and rejects replays.

---

## 3. Fiat-Crypto P2P Trading Protocol

**Crate:** `fiat`

### 3.1 Protocol Participants

| Role | Description |
|------|-------------|
| Maker | Creates the fiat offer; sells crypto, receives fiat |
| Taker | Accepts the offer; buys crypto, sends fiat |

### 3.2 Settlement Methods

| Method | Description |
|--------|-------------|
| `BankTransfer` | SEPA/UK Faster Payments/IBAN bank transfer |
| `CashDeposit` | Cash deposited at a designated location |
| `CashInPerson` | In-person cash exchange |

### 3.3 State Machine

**16 States with two parallel happy paths:**

```
Created → Accepted → CryptoReserved → PaymentRequested
                                        ├─ Manual path:
                                        │   PaymentSubmitted → MakerConfirmed → CryptoRelease → Complete
                                        │
                                        └─ Automatic path:
                                            PaymentPending → Detected → Confirming → Confirmed
                                                → CryptoRelease → Complete
```

**Exceptional paths:**

- Any active state → `Cancelled` (cancellation by either party)
- `PaymentPending` → `Failed` (payment timeout)
- `PaymentSubmitted` → `Failed` (payment timeout)
- `Confirming` → `Failed` (confirmation timeout)
- `MakerConfirmed` → `Failed` (confirmation timeout)
- `CryptoReserved` → `Failed` (crypto reservation timeout)
- `PaymentRequested` → `Failed` (payment request timeout)
- `CryptoRelease` → `Failed` (crypto release timeout)
- Most active states → `Disputed` (dispute opened)
- `Complete` → `ReversalDetected` → `Disputed` (post-completion payment reversal)
- `Disputed` → `Complete` / `Failed` / `Cancelled` (dispute resolution)

**Complete transition matrix (from `FiatTradeState::can_transition_to()`):**

| From | To |
|------|-----|
| `Created` | `Accepted`, `Cancelled`, `Disputed` |
| `Accepted` | `CryptoReserved`, `Cancelled`, `Disputed` |
| `CryptoReserved` | `PaymentRequested`, `Failed`, `Cancelled`, `Disputed` |
| `PaymentRequested` | `PaymentSubmitted`, `PaymentPending`, `Failed`, `Cancelled`, `Disputed` |
| `PaymentSubmitted` | `MakerConfirmed`, `Failed`, `Cancelled`, `Disputed` |
| `PaymentPending` | `Detected`, `Failed`, `Cancelled`, `Disputed` |
| `Detected` | `Confirming`, `Cancelled`, `Disputed` |
| `Confirming` | `Confirmed`, `Failed`, `Cancelled`, `Disputed` |
| `MakerConfirmed` | `CryptoRelease`, `Failed`, `Cancelled`, `Disputed` |
| `Confirmed` | `CryptoRelease`, `Cancelled`, `Disputed` |
| `CryptoRelease` | `Complete`, `Failed`, `Disputed` |
| `Complete` | `ReversalDetected` |
| `ReversalDetected` | `Disputed` |
| `Disputed` | `Complete`, `Failed`, `Cancelled` |

### 3.4 Fiat Trade Structure

```
FiatTrade
├── id: TradeId
├── maker: ParticipantId
├── taker: ParticipantId
├── crypto_asset: Asset              // BTC or XMR
├── fiat_currency: FiatCurrency      // GBP, EUR, USD
├── crypto_amount: u64               // Satoshis or piconeros
├── fiat_amount: FiatAmount          // Minor units (pence, cents)
├── settlement_method: SettlementMethod
├── fee_schedule: FeeSchedule        // Basis points for maker and taker
├── state: FiatTradeState
├── maker_bank_details: Option<PaymentRecipient>
├── taker_crypto_address: String     // Validated before storage
├── payment_id: Option<PaymentId>
├── payment_reference: Option<PaymentReference>
├── deadlines: SettlementDeadlines
├── created_at: u64                  // Unix timestamp
└── updated_at: u64                  // Unix timestamp
```

### 3.5 Settlement Deadlines

```
SettlementDeadlines
├── payment_deadline: u64       // Taker must submit payment by this time
├── confirmation_deadline: u64  // Payment must be confirmed by this time
└── dispute_deadline: u64       // Disputes can be opened until this time
```

### 3.6 Fee Schedule

Fees are expressed in basis points (100 bps = 1%):

| Method | Maker Fee | Taker Fee |
|--------|-----------|-----------|
| Bank Transfer | 25 bps (0.25%) | 50 bps (0.50%) |
| Cash | 50 bps (0.50%) | 100 bps (1.00%) |

### 3.7 Payment Adapter Architecture

```rust
trait PaymentAdapter {
    fn detect_payment(&self, payment_id: &PaymentId)
        -> Result<Option<VerifiedPayment>, PaymentError>;
    fn get_payment_status(&self, payment_id: &PaymentId)
        -> Result<PaymentStatus, PaymentError>;
}
```

**Implementations:**

| Adapter | Network |
|---------|---------|
| `SepaProvider` | SEPA bank transfers |
| `UkFasterPayments` | UK Faster Payments |

### 3.8 Bitcoin 2-of-2 Multisig Escrow

**Crate:** `fiat::multisig`

The fiat protocol uses Bitcoin 2-of-2 P2WSH multisig for crypto escrow:

```
MultisigManager
├── EscrowKeyPair          // secp256k1 key pair with zeroize-on-drop
├── RedemptionScript       // 2-of-2 P2WSH witness script
├── MultisigEscrow         // Per-trade escrow state tracking
└── PartialSignature       // Co-signing protocol for payout/return
```

**Escrow Flow:**

1. Each party generates an `EscrowKeyPair` and shares the public key
2. Maker sends BTC to the 2-of-2 P2WSH address
3. After fiat payment confirmation, both parties co-sign the payout transaction
4. On dispute/cancellation, both parties co-sign the return transaction

**Redemption Script:** `OP_2 <pub1> <pub2> OP_2 OP_CHECKMULTISIG`

### 3.9 Security Bonds

Both parties must lock security bonds before trade execution. Bonds are calculated as a percentage of the trade value in basis points. Bond states:

| State | Description |
|-------|-------------|
| `Pending` | Bond not yet locked |
| `Locked` | Bond locked in escrow |
| `Required` | Bond required for trade |
| `Released` | Bond returned after trade completion |

Bond transitions include `Required → Released` for early cancellation scenarios.

### 3.10 Dispute Resolution

A dispute can be opened from most active states. The dispute process:

1. Either party opens a dispute, transitioning the trade to `Disputed`
2. Evidence is collected from both parties
3. The dispute is resolved to `Complete`, `Failed`, or `Cancelled`
4. Funds are distributed according to the resolution

### 3.11 Fiat Trade Message Flow

```
Maker                              Taker
  │                                  │
  ├─ FiatOffer created ─────────────►│
  │  (currency, amount, price,       │
  │   settlement method, bond)       │
  │                                  │
  │◄── Accept trade ────────────────┤  (Accepted)
  │                                  │
  ├─ Reserve crypto in escrow ──────┤  (CryptoReserved)
  │  (2-of-2 multisig address)      │
  │                                  │
  ├─ Bank details + payment ref ───►│  (PaymentRequested)
  │                                  │
  │  [Taker makes bank transfer]     │
  │                                  │
  │◄── Payment submitted ───────────┤  (PaymentSubmitted)
  │  or                             │
  │◄── Payment detected ────────────┤  (Detected, auto mode)
  │                                  │
  ├─ Confirm payment received ──────┤  (MakerConfirmed/Confirmed)
  │                                  │
  ├─ Release crypto to taker ───────┤  (CryptoRelease)
  │  (co-sign payout transaction)   │
  │                                  │
  ├─ Trade COMPLETE ────────────────┤
```

---

## 4. P2P Network Protocol

**Crate:** `p2p-node` (libp2p integration), `opendex-transport` (message delivery)

### 4.1 Network Behaviors

The P2P node combines four libp2p behaviors:

| Behavior | Purpose |
|----------|---------|
| Kademlia | Peer discovery and DHT |
| Ping | Liveness detection |
| Identify | Protocol version negotiation |
| Gossipsub | Message pub/sub |

### 4.2 Protocol Envelope

All network messages are wrapped in a `ProtocolEnvelope`:

```
ProtocolEnvelope
├── protocol_version: ProtocolVersion
├── swap_id: Option<SwapId>
├── message_id: MessageId
├── sender: ParticipantId
├── sequence: u64
└── payload: Vec<u8>
```

### 4.3 Message Validation

- Schema version validation: messages must match the expected protocol version
- Timestamp freshness: messages older than 30 seconds are rejected
- Replay protection: duplicate message IDs are rejected via `MessageStore`
- Sequence number ordering: messages are processed in sequence order

### 4.4 Transport Trait

```rust
pub trait Transport {
    fn send(&self, envelope: ProtocolEnvelope) -> Result<(), TransportError>;
    fn receive(&self) -> Result<ProtocolEnvelope, TransportError>;
}
```

**Implementations:**

| Implementation | Use Case |
|----------------|----------|
| `Libp2pTransport` | Production P2P network |
| `MockTransport` | Testing |

---

## 5. Timeout Mechanisms

### 5.1 Atomic Swap Timeouts

| Condition | Timeout | Action |
|-----------|---------|--------|
| `BtcFunding` not seen | Configurable per swap | Transition to `Recovering` |
| `XmrFunding` not seen | Configurable per swap | Transition to `Recovering` |
| Redemption not completed | Configurable per swap | Transition to `Recovering` |
| `WaitCondition::Timeout(u64)` | Unix timestamp | Trigger recovery |

The `WaitCondition::Timeout` variant carries a Unix timestamp. When the current time exceeds this timestamp, the swap engine triggers the recovery engine.

### 5.2 Fiat Trade Timeouts

| State | Deadline | Action |
|-------|----------|--------|
| `PaymentPending` | `payment_deadline` | Transition to `Failed` |
| `PaymentSubmitted` | `payment_deadline` | Transition to `Failed` |
| `Confirming` | `confirmation_deadline` | Transition to `Failed` |
| `MakerConfirmed` | `confirmation_deadline` | Transition to `Failed` |
| `CryptoReserved` | Configurable | Transition to `Failed` |
| `PaymentRequested` | Configurable | Transition to `Failed` |
| `CryptoRelease` | Configurable | Transition to `Failed` |

The `check_deadlines()` method in `SettlementEngine` evaluates all active trades against their deadlines and transitions expired trades to `Failed`.

### 5.3 Dispute Deadline

Disputes can be opened until `dispute_deadline`. After this deadline, the trade cannot transition to `Disputed`.

---

## 6. Protocol Assumptions

### 6.1 Blockchain Assumptions

| Assumption | Rationale |
|------------|-----------|
| Bitcoin finality after 6 confirmations | Standard Bitcoin security assumption |
| Monero finality after 10 confirmations | Standard Monero security assumption |
| Blockchain watchers provide accurate observations | Watchers poll real nodes; reorgs are detected |
| Transaction fees are predictable | Fee estimation is used; unexpected fees may cause delays |
| Nodes are available and responsive | External dependency; node outages affect swap progress |

### 6.2 Cryptographic Assumptions

| Assumption | Rationale |
|------------|-----------|
| secp256k1 discrete log problem is hard | Standard Bitcoin assumption |
| Ed25519 discrete log problem is hard | Standard Monero assumption |
| DLEQ proofs are sound | Eigenwallet/COMIT construction; **stub implementation pending** |
| SHA-256 is collision-resistant | Standard hash function assumption |
| `thread_rng()` provides cryptographically secure randomness | Rust `getrandom` crate uses OS CSPRNG |

### 6.3 Network Assumptions

| Assumption | Rationale |
|------------|-----------|
| Messages are eventually delivered | P2P network provides best-effort delivery |
| Messages may be delayed or reordered | Sequence numbers handle ordering |
| Counterparties may disconnect at any time | `WaitingForPeer` state handles disconnection |
| Messages may be replayed | `MessageStore` provides replay protection |
| Timestamps are approximately synchronized | 30-second freshness window allows clock skew |

### 6.4 Fiat Payment Assumptions

| Assumption | Rationale |
|------------|-----------|
| Bank transfers are irreversible once settled | Standard assumption; chargebacks are rare but possible |
| Payment adapters accurately detect payments | External dependency; adapter reliability varies |
| Participants provide accurate bank details | Participant-provided data; validated format only |
| Fiat payments are atomic with crypto release | **NOT guaranteed** — fiat and crypto settle independently |

### 6.5 Participant Assumptions

| Assumption | Rationale |
|------------|-----------|
| Participants act rationally | Protocol is designed to be incentive-compatible |
| Participants have access to blockchain nodes | External requirement; not enforced by protocol |
| Participants' wallets are functional | External dependency; wallet failures affect recovery |
| Participants do not collude to exploit the protocol | Protocol is designed to be safe against single-party adversaries |

---

## 7. Protocol Versioning

### 7.1 Version Negotiation

**Crate:** `opendex-protocol`

The protocol registry maintains a `HashMap` of registered protocol versions per trading pair. When two participants want to swap, `negotiate()` finds the highest common version both support.

```rust
pub struct ProtocolRegistry {
    versions: HashMap<TradingPair, Vec<ProtocolVersion>>,
}

impl ProtocolRegistry {
    pub fn negotiate(
        &self,
        pair: &TradingPair,
        maker_versions: &[ProtocolVersion],
        taker_versions: &[ProtocolVersion],
    ) -> Option<ProtocolVersion>;
}
```

### 7.2 Current Version

The current protocol version is `0.1.0`. All 23 crates are at version `0.1.0`.

### 7.3 Version Compatibility

Protocol messages carry the `protocol_version` field. Messages with unsupported versions are rejected during validation.

---

## Appendix A: Protocol Message Types

### Atomic Swap Messages

| Message | Direction | Description |
|---------|-----------|-------------|
| DLEQ Proof | Bidirectional | Cross-curve discrete log equality proof |
| Funding Transaction | Bidirectional | Broadcast transaction ID and lock details |
| Redemption Signal | Bidirectional | Secret revelation for counterparty redemption |
| Refund Request | Bidirectional | Request to refund locked funds |

### Fiat Trade Messages

| Message | Direction | Description |
|---------|-----------|-------------|
| FiatOffer | Maker → Taker | Offer to trade crypto for fiat |
| AcceptTrade | Taker → Maker | Acceptance of the offer |
| PaymentInstructions | Maker → Taker | Bank details and payment reference |
| PaymentSubmitted | Taker → Maker | Claim that payment has been sent |
| PaymentConfirmed | Maker → Taker | Confirmation of payment receipt |
| DisputeOpen | Either | Open a dispute |
| DisputeEvidence | Either | Submit evidence for dispute |
| CancelTrade | Either | Request to cancel the trade |

### P2P Network Messages

| Message | Direction | Description |
|---------|-----------|-------------|
| PeerDiscovery | Bidirectional | Kademlia DHT queries |
| Ping/Pong | Bidirectional | Liveness check |
| Identify | Bidirectional | Protocol version negotiation |
| Gossipsub | Pub/Sub | Trade and order messages |

---

## Appendix B: State Machine Diagrams

### Atomic Swap State Machine

```
                    ┌─────────┐
                    │ Created │
                    └────┬────┘
                         │
                         ▼
                   ┌─────────────┐
                   │ Negotiating │
                   └──────┬──────┘
                          │
                          ▼
                   ┌───────────────┐
                   │ ProtocolSetup │
                   └──────┬────────┘
                          │
                          ▼
                     ┌────────┐
                     │ Funding│
                     └───┬────┘
                         │
                         ▼
                     ┌───────┐
                     │ Funded│
                     └───┬───┘
                         │
                         ▼
                   ┌──────────┐
                   │Redeeming │
                   └─────┬────┘
                         │
                   ┌─────┴──────┐
                   ▼            ▼
            ┌──────────┐  ┌──────────┐
            │Completed │  │Recovering│
            └──────────┘  └─────┬────┘
                                │
                          ┌─────┴──────┐
                          ▼            ▼
                   ┌──────────┐  ┌────────┐
                   │ Refunded │  │ Failed │
                   └──────────┘  └────────┘

  Any active state ──► WaitingForPeer ──► Resume or Failed
```

### Fiat Trade State Machine

```
  Created → Accepted → CryptoReserved → PaymentRequested
                                         ├─ Manual:
                                         │   PaymentSubmitted → MakerConfirmed
                                         │       → CryptoRelease → Complete
                                         │
                                         └─ Automatic:
                                             PaymentPending → Detected
                                                 → Confirming → Confirmed
                                                     → CryptoRelease → Complete

  Any active state ──► Cancelled
  Most active states ──► Disputed ──► Complete / Failed / Cancelled
  Complete ──► ReversalDetected ──► Disputed
  Timeout states ──► Failed
```
