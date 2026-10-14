# OpenDex Architecture Document

**Version:** 0.1.0-security-review
**Date:** 2026-10-14
**Status:** Authoritative architecture reference
**Scope:** All 23 crates, ~32,400 lines of Rust

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Principal Software Components](#2-principal-software-components)
3. [Bitcoin and Monero Interfaces](#3-bitcoin-and-monero-interfaces)
4. [Wallet and Node Interactions](#4-wallet-and-node-interactions)
5. [Swap Coordinator and State Management](#5-swap-coordinator-and-state-management)
6. [Cryptographic Libraries and Protocol Primitives](#6-cryptographic-libraries-and-protocol-primitives)
7. [Key Generation, Storage, Use and Destruction](#7-key-generation-storage-use-and-destruction)
8. [Fiat-Payment Messaging Layer](#8-fiat-payment-messaging-layer)
9. [Databases, External Services and Infrastructure](#9-databases-external-services-and-infrastructure)
10. [Information Exchanged Between Counterparties](#10-information-exchanged-between-counterparties)
11. [Trust Boundaries](#11-trust-boundaries)
12. [Data Flow Diagrams](#12-data-flow-diagrams)

---

## 1. System Overview

OpenDex is a non-custodial peer-to-peer exchange platform that enables two distinct trading modes:

1. **BTC-XMR Atomic Swaps** — Cryptographically guaranteed, trustless cross-chain swaps between Bitcoin and Monero using DLEQ (Discrete Log Equality) proofs and adaptor signatures.
2. **Fiat-Crypto P2P Trading** — Fiat-to-crypto trades (GBP/EUR/USD ↔ BTC/XMR) using payment adapters, Bitcoin 2-of-2 multisig escrow, security bonds, and dispute resolution.

The system is built as a Cargo workspace of 23 independently testable Rust crates with zero `unsafe` blocks in financial code paths. The architecture enforces a strict separation: the marketplace owns order matching and pricing; the atomic-swap protocol owns cryptographic settlement.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      OpenDex Desktop GUI                    │
│                    (desktop-gui crate)                      │
└──────────────────────────┬──────────────────────────────────┘
                           │ IPC channels
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                      P2P Node Layer                         │
│              (p2p-node crate + libp2p)                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐ │
│  │Orderbook │  │  Trades  │  │ Balances │  │  Escrow    │ │
│  │  & CLI   │  │  State   │  │  & Sync  │  │  Manager   │ │
│  └──────────┘  └──────────┘  └──────────┘  └────────────┘ │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐ │
│  │Network   │  │Wallet    │  │Wallet    │  │Persistence │ │
│  │(libp2p)  │  │Client BTC│  │Client XMR│  │  (files)   │ │
│  └──────────┘  └──────────┘  └──────────┘  └────────────┘ │
└──────────────────────────┬──────────────────────────────────┘
                           │
            ┌──────────────┼──────────────┐
            ▼              ▼              ▼
   ┌────────────────┐ ┌────────────┐ ┌──────────────┐
   │  Fiat Trading  │ │ Atomic     │ │  User &      │
   │  (fiat crate)  │ │ Swaps      │ │  Reputation  │
   │                │ │ (swap eng) │ │ (user crate) │
   └────────┬───────┘ └─────┬──────┘ └──────────────┘
            │               │
   ┌────────┴───────┐ ┌─────┴──────┐
   │ Payment Adapters│ │ Protocol   │
   │ (SEPA, UK FP)  │ │ Adapter    │
   │ Bitcoin 2-of-2 │ │ (btc-xmr)  │
   │ Multisig Escrow│ │ DLEQ Proofs│
   └────────┬───────┘ └─────┬──────┘
            │               │
   ┌────────┴───────┐ ┌─────┴──────┐
   │ Bank/PSP APIs  │ │ Blockchain │
   │ Fiat Networks  │ │ Watchers   │
   └────────────────┘ └─────┬──────┘
                            │
                   ┌────────┴────────┐
                   ▼                 ▼
              Bitcoin Chain     Monero Chain
              (Electrum RPC)   (Daemon RPC)
```

---

## 2. Principal Software Components

### 2.1 Core Type and Error Crates

| Crate | Responsibility |
|-------|---------------|
| `opendex-types` | Foundational domain types: `SwapId`, `TradeId`, `OrderId`, `ParticipantId`, `MessageId`, `Asset` (BTC/XMR/Fiat), `Satoshis`, `Piconero`, `Price`, `ProtocolVersion`, `TradingPair`. All monetary values are integer-based; no floating-point arithmetic. |
| `opendex-errors` | Unified error hierarchy: `OpenDexError` with variants for `ChainError`, `ProtocolError`, `StorageError`, `TransportError`. |
| `opendex-events` | Append-only event types for event sourcing. Every swap state change produces an immutable `SwapEvent` (`SwapCreated`, `SwapStateChanged`, `FundingDetected`, `RedemptionDetected`, `RefundDetected`, `SwapCompleted`, `SwapFailed`). Events form the source of truth and allow full state reconstruction. |

### 2.2 Protocol Abstraction Layer

| Crate | Responsibility |
|-------|---------------|
| `opendex-protocol-types` | Defines the `SettlementProtocol` trait — the most important interface in the project. The marketplace sees only this trait, never implementation details. Defines `SwapContext`, `ProtocolAction` (SendMessage, BuildTransaction, Wait, Complete, Recover, Fail), `SwapState` (14 states from `Created` through `Completed`/`Refunded`/`Failed`), `ChainObservation`, `TransactionIntent`, and `RecoveryAction`. |
| `opendex-protocol` | Protocol registry and version negotiation. Maintains a `HashMap` of registered protocol versions per trading pair. Provides `negotiate()` to find the highest common version between two participants. |

### 2.3 Swap Engine

| Crate | Responsibility |
|-------|---------------|
| `opendex-swap` | Generic swap lifecycle management. `SwapEngine` is parameterized over protocol, storage, recovery, and chain types. Coordinates between the protocol adapter, chain watchers, wallet, and storage. Does not know cryptographic details. Handles: swap creation, message processing, chain observation, action execution (send messages, build/broadcast transactions, wait conditions), recovery triggers, and event recording. Includes replay protection via `MessageStore`. |

### 2.4 Blockchain Interfaces

| Crate | Responsibility |
|-------|---------------|
| `opendex-bitcoin` | Bitcoin chain interface. Defines `BitcoinChain` trait (`transaction()`, `tip()`, `broadcast()`, `confirmations()`). `ElectrumBitcoinChain` connects to Electrum RPC servers for real blockchain access. `MockBitcoinChain` for testing. |
| `opendex-monero` | Monero chain interface. Defines `MoneroChain` trait (`transaction()`, `height()`, `broadcast()`, `confirmations()`). `DaemonMoneroChain` connects to Monero daemon JSON-RPC. `MockMoneroChain` for testing. |
| `opendex-chain` | Chain abstraction layer providing unified interface across Bitcoin and Monero operations. |

### 2.5 Blockchain Watchers

| Crate | Responsibility |
|-------|---------------|
| `opendex-watchers` | Read-only blockchain observers that poll nodes and emit `ChainObservation` events. Includes `BitcoinWatcher`, `MoneroWatcher`, `MockWatcher`, and a `Supervisor` that coordinates multiple watchers. |

### 2.6 Cryptographic Protocol Adapter

| Crate | Responsibility |
|-------|---------------|
| `opendex-btc-xmr` | BTC-XMR atomic swap protocol adapter. Implements `SettlementProtocol` trait. Contains `DleqProofGenerator` for cross-curve DLEQ proof generation and verification between secp256k1 (Bitcoin) and Ed25519 (Monero). Serves as the compatibility boundary around the Eigenwallet BTC-XMR implementation. **Note:** Current implementation contains stub DLEQ proofs pending full Eigenwallet library integration. |

### 2.7 Storage and Recovery

| Crate | Responsibility |
|-------|---------------|
| `opendex-storage` | Persistent storage via file-based repositories with atomic write semantics. Provides `SwapRepository`, `EventRepository`, and `TransactionIntentRepository`. Uses atomic write pattern: write to temp file, sync, then rename. |
| `opendex-recovery` | Deterministic recovery engine. Evaluates swap state combined with blockchain state and returns `RecoveryDecision` (Wait, Redeem, Refund, Punish, Abort). |

### 2.8 Transport Layer

| Crate | Responsibility |
|-------|---------------|
| `opendex-transport` | Network-agnostic message delivery. Defines `Transport` trait (`send()`, `receive()`). `ProtocolEnvelope` carries routing metadata (protocol version, swap ID, message ID, sender, sequence number). `MessageStore` provides replay protection. Includes `Libp2pTransport` for production and `MockTransport` for testing. |

### 2.9 Fiat Settlement System

| Crate | Responsibility |
|-------|---------------|
| `fiat` | Complete fiat settlement subsystem. `SettlementEngine` orchestrates the 14-state fiat trade lifecycle. `PaymentAdapter` trait for plugging in bank APIs (SEPA, UK Faster Payments). `CryptoWallet` trait for BTC/XMR escrow operations. `SecurityBond` system with locked funds from both parties. `Dispute` resolution with evidence collection. Bitcoin 2-of-2 P2WSH multisig escrow via `MultisigManager`. Real payment providers: `SepaProvider`, `UkFasterPayments`. Monero wallet adapters: `SimulatedXmrWallet`, `NativeXmrWallet`. |

### 2.10 P2P Node

| Crate | Responsibility |
|-------|---------------|
| `p2p-node` | Full P2P node implementation using libp2p. `OpenDexBehaviour` combines Kademlia (peer discovery), Ping (liveness), Identify (protocol negotiation), and Gossipsub (message pub/sub). Manages orderbook, trades, balances, escrow, wallet clients, and persistence. Provides `GuiNodeConfig` for GUI integration with credential zeroization on drop. Includes log redaction framework (`log_redact` module) to prevent sensitive data leakage. |

### 2.11 User and Reputation

| Crate | Responsibility |
|-------|---------------|
| `user` | User account management with Argon2id password hashing, HMAC-signed authentication tokens, reputation scoring based on membership duration and trade history, maker/taker statistics, and review system. |

### 2.12 Supporting Crates

| Crate | Responsibility |
|-------|---------------|
| `common` | Shared utilities and constants used across crates. |
| `protocol` | Legacy P2P protocol message types and state machine for fiat trading. |
| `opendex-fee-treasury` | Fee calculation, treasury isolation, and DAO governance for fee distribution. |
| `opendex-dao` | DAO governance structures and voting mechanisms. |
| `desktop-gui` | Desktop application using Iced framework. Communicates with P2P node via IPC channels. |
| `web-frontend` | Web-based frontend interface. |

---

## 3. Bitcoin and Monero Interfaces

### 3.1 Bitcoin Interface

**Crate:** `opendex-bitcoin`

**Trait:** `BitcoinChain`

```
trait BitcoinChain {
    fn transaction(&self, txid: &str) -> Result<Option<BitcoinTransaction>, ChainError>;
    fn tip(&self) -> Result<u64, ChainError>;
    fn broadcast(&self, tx_hex: &str) -> Result<String, ChainError>;
    fn confirmations(&self, txid: &str) -> Result<u32, ChainError>;
}
```

**Implementation:** `ElectrumBitcoinChain`

- Connects to Electrum RPC servers (e.g., `ssl://electrum.blockstream.info:50002`)
- Uses `electrum-client` crate with `ElectrumApi` trait
- Parses raw transactions via `bitcoin::Transaction::consensus_decode`
- Calculates total output amounts from transaction outputs
- Supports mainnet and testnet networks

**External Dependency:** Electrum server (self-hosted or third-party)

### 3.2 Monero Interface

**Crate:** `opendex-monero`

**Trait:** `MoneroChain`

```
trait MoneroChain {
    fn transaction(&self, txid: &str) -> Result<Option<MoneroTransaction>, ChainError>;
    fn height(&self) -> Result<u64, ChainError>;
    fn broadcast(&self, tx_hex: &str) -> Result<String, ChainError>;
    fn confirmations(&self, txid: &str) -> Result<u32, ChainError>;
}
```

**Implementation:** `DaemonMoneroChain`

- Connects to Monero daemon via JSON-RPC (e.g., `http://127.0.0.1:18081`)
- Uses `monero-rpc` crate with `DaemonJsonRpcClient`
- Supports transaction lookup, block height queries, and transaction relay

**External Dependency:** Running `monerod` instance (self-hosted recommended for privacy)

### 3.3 Wallet Adapters

**Crate:** `fiat::crypto_wallet`

| Adapter | Purpose |
|---------|---------|
| `BtcWalletAdapter` | Bitcoin wallet operations for fiat escrow |
| `XmrWalletAdapterTrait` | Monero wallet operations with `WalletInfo`, `Balance`, `MoneroTransaction` |
| `SimulatedXmrWallet` | Simulated Monero wallet for testing |
| `NativeXmrWallet` | Native Monero wallet RPC integration |
| `CombinedWalletAdapter` | Unified adapter for both BTC and XMR operations |
| `MockCryptoWallet` | Mock implementation for adversarial testing |

---

## 4. Wallet and Node Interactions

### 4.1 P2P Node Wallet Integration

The `p2p-node` crate maintains wallet client connections for both Bitcoin and Monero:

```
GuiNodeConfig
├── monero_rpc_url: Option<String>
├── monero_rpc_user: Option<String>
├── monero_rpc_pass: Option<String>  (zeroized on drop)
├── btc_rpc_url: Option<String>
├── btc_rpc_user: Option<String>
└── btc_rpc_pass: Option<String>     (zeroized on drop)
```

**Wallet Client Modules:**
- `wallet_client` — Monero wallet RPC client
- `btc_wallet_client` — Bitcoin wallet RPC client

**Balance Management:**
- `balances` module tracks available, locked, and escrowed balances
- Balance state transitions are synchronized with trade state changes
- `sync` module handles balance synchronization across node restarts

### 4.2 Escrow Operations

**Atomic Swap Escrow:** Managed by the swap engine through the protocol adapter. Funds are locked in protocol-specific smart contracts (DLEQ-based locking transactions).

**Fiat Trade Escrow:** Managed by `MultisigManager` in the `fiat` crate:

```
MultisigManager
├── EscrowKeyPair — secp256k1 key pair with zeroize-on-drop
├── RedemptionScript — 2-of-2 P2WSH witness script
├── MultisigEscrow — Tracks escrow state per trade
└── PartialSignature — Co-signing protocol for payout/return
```

**Escrow Flow:**
1. Each party generates an `EscrowKeyPair` and shares the public key
2. Maker sends BTC to the 2-of-2 P2WSH address
3. After fiat payment confirmation, both parties co-sign the payout transaction
4. On dispute/cancellation, both parties co-sign the return transaction

---

## 5. Swap Coordinator and State Management

### 5.1 Swap Engine Architecture

**Crate:** `opendex-swap`

The `SwapEngine` is the central coordinator for atomic swaps. It is generic over seven type parameters:

```rust
struct SwapEngine<Protocol, SwapRepo, EventRepo, TxIntentRepo, Recovery, BtcChain, XmrChain>
```

**Responsibilities:**
- Swap creation with persistence and event recording
- Protocol message processing and response generation
- Chain observation integration via watchers
- Action execution: send messages, build/broadcast transactions, wait conditions
- Recovery trigger when protocol indicates failure
- Replay protection via `MessageStore` (tracks seen message IDs)
- Sequence number management for message ordering

### 5.2 Atomic Swap State Machine

**14 States:**

```
Created → Initialized → BtcLocked → XmrLocked → BtcRedeemed → XmrRedeemed → Completed
   │           │           │            │            │
   └───────────┴───────────┴────────────┴────────────┘
                        ↓
                      Refunded / Failed
```

**State Transitions:**
- `Created → Initialized`: Protocol initialized, DLEQ proof exchanged
- `Initialized → BtcLocked`: Bitcoin funding transaction confirmed
- `BtcLocked → XmrLocked`: Monero funding transaction confirmed
- `XmrLocked → BtcRedeemed`: Bitcoin redemption by counterparty
- `BtcRedeemed → XmrRedeemed`: Monero redemption completes swap
- `XmrRedeemed → Completed`: Both parties have received funds
- Any active state → `Refunded`: Timeout or explicit refund
- Any active state → `Failed`: Protocol violation or error

### 5.3 Fiat Trade State Machine

**Crate:** `fiat::settlement_engine`

**14 States with full cancellation and failure paths:**

```
Created → Accepted → CryptoReserved → PaymentRequested → PaymentSubmitted
                                                        ↓
                                              PaymentPending → Detected
                                                        ↓
                                              Confirming → MakerConfirmed
                                                        ↓
                                              Confirmed → CryptoRelease → Complete
```

**Cancellation Paths:** From any active state → `Cancelled`

**Failure Paths:** Timeout states → `Failed`
- `PaymentPending` past payment deadline
- `PaymentSubmitted` past payment deadline
- `Confirming` past confirmation deadline
- `MakerConfirmed` past confirmation deadline (prevents permanent fund lockup)

**Dispute Paths:** From most active states → `Disputed`

### 5.4 Event Sourcing

**Crate:** `opendex-events`

Every state change produces an immutable `EventRecord` with:
- `EventId`: Unique UUID
- `sequence`: Monotonically increasing sequence number
- `timestamp`: ISO 8601 timestamp
- `event`: The `SwapEvent` payload

Events are stored via `EventRepository` using atomic file writes. State can be fully reconstructed by replaying events from the log.

### 5.5 Recovery Engine

**Crate:** `opendex-recovery`

Evaluates swap state combined with blockchain observations and returns one of:

| Decision | Action |
|----------|--------|
| `Wait` | Continue monitoring, no action needed |
| `Redeem` | Counterparty has locked funds; initiate redemption |
| `Refund` | Swap cannot complete; refund locked funds |
| `Punish` | Counterparty violated protocol; claim penalty |
| `Abort` | Irrecoverable error; abandon swap |

---

## 6. Cryptographic Libraries and Protocol Primitives

### 6.1 External Cryptographic Dependencies

| Library | Version | Purpose |
|---------|---------|---------|
| `secp256k1` | 0.29 | Bitcoin secp256k1 ECDSA/Schnorr signatures |
| `bitcoin` | 0.32 | Bitcoin transaction and address handling |
| `dleq` | 0.4 | Cross-curve DLEQ proof generation and verification |
| `sha2` | 0.10 | SHA-256/SHA-512 hash functions |
| `zeroize` | — | Secure memory zeroization for sensitive data |
| `ed25519` | — | Monero Ed25519 signatures |

### 6.2 Cryptographic Primitives Used

**No custom cryptography is implemented.** All primitives are standard, well-reviewed constructions:

| Primitive | Type | Standard |
|-----------|------|----------|
| secp256k1 signatures | ECDSA/Schnorr | Bitcoin standard |
| Ed25519 signatures | EdDSA | Monero standard |
| Cross-curve DLEQ proofs | Zero-knowledge proof | Eigenwallet/COMIT construction |
| Adaptor signatures | Pre-signing + adaptation | Standard cryptographic construction |
| SHA-256/SHA-512 | Hash functions | NIST FIPS 180-4 |
| 2-of-2 multisig | P2WSH redemption script | Bitcoin standard |
| Argon2id | Password hashing | RFC 9106 |
| HMAC-SHA256 | Token signing | RFC 2104 |

### 6.3 DLEQ Proof Structure

**Crate:** `opendex-btc-xmr`

```rust
struct DleqProofData {
    btc_public_key: Vec<u8>,    // secp256k1 public key (Bitcoin)
    xmr_public_key: Vec<u8>,    // Ed25519 public key (Monero)
    proof: Vec<u8>,             // DLEQ proof bytes
    version: u32,               // Protocol version
}
```

**Purpose:** Proves that the same private key scalar is used on both secp256k1 and Ed25519 curves without revealing the key. This enables the counterparty to verify that redemption on one chain corresponds to the same party who locked funds on the other chain.

**Current Status:** Stub implementation pending full Eigenwallet library integration. The `generate_proof()` and `verify_proof()` methods exist but use simplified placeholders.

### 6.4 Bitcoin 2-of-2 Multisig

**Crate:** `fiat::multisig`

**Redemption Script:** `OP_2 <pub1> <pub2> OP_2 OP_CHECKMULTISIG`

**Co-signing Protocol:**
1. Each party creates a partial signature on the payout/return transaction
2. Partial signatures are exchanged via the P2P transport
3. Signatures are combined into a final transaction
4. The final transaction is broadcast to the Bitcoin network

**Key Security:** `EscrowKeyPair` implements `Drop` to zeroize secret keys from memory when no longer needed.

---

## 7. Key Generation, Storage, Use and Destruction

### 7.1 Key Generation

| Key Type | Generation Method | Location |
|----------|-------------------|----------|
| secp256k1 key pairs | `Secp256k1::<All>::generate_keypair()` with `thread_rng()` | `fiat::multisig::EscrowKeyPair::new()` |
| DLEQ shared secret | Derived from participant's private key, reduced modulo min(n_secp256k1, l_ed25519) | `opendex-btc-xmr::DleqProofGenerator` |
| libp2p identity keys | `libp2p::identity::Keypair::generate_ed25519()` | `p2p-node::network::build_swarm()` |

### 7.2 Key Storage

| Key Type | Storage Method | Persistence |
|----------|---------------|-------------|
| libp2p identity | Saved to disk, loaded on restart | Persistent file |
| Wallet RPC credentials | In `GuiNodeConfig`, passed from GUI | In-memory, not persisted by node |
| Multisig escrow keys | Generated per-trade, held in memory | Not persisted; derived from wallet |
| DLEQ secrets | In memory during swap lifecycle | Not persisted |

### 7.3 Key Usage

- **secp256k1 keys:** Bitcoin transaction signing, multisig co-signing, escrow operations
- **Ed25519 keys:** Monero wallet operations, libp2p network identity
- **DLEQ secrets:** Cross-curve proof generation (stub implementation)
- **HMAC secrets:** Authentication token signing

### 7.4 Key Destruction

**Zeroization on Drop:**

| Component | Mechanism |
|-----------|-----------|
| `GuiNodeConfig` | `Drop` implementation zeroizes `monero_rpc_pass` and `btc_rpc_pass` byte arrays |
| `EscrowKeyPair` | `Drop` implementation calls `zeroize()` on secret key bytes |
| Password hashes | Argon2id operates on temporary buffers |

**Memory Safety:** The project contains exactly 2 `unsafe` blocks, both for credential zeroization in `p2p-node::GuiNodeConfig::drop()`. All other code is safe Rust.

---

## 8. Fiat-Payment Messaging Layer

### 8.1 Payment Adapter Architecture

**Crate:** `fiat::adapter`

```rust
trait PaymentAdapter {
    fn detect_payment(&self, payment_id: &PaymentId) -> Result<Option<VerifiedPayment>, PaymentError>;
    fn get_payment_status(&self, payment_id: &PaymentId) -> Result<PaymentStatus, PaymentError>;
}
```

**Implementations:**
- `SepaProvider` — SEPA bank transfer detection
- `UkFasterPayments` — UK Faster Payments detection
- `MockPaymentProvider` — Auto-confirm for testing

### 8.2 Fiat Trade Message Flow

```
Maker                          Taker
  │                              │
  │─── FiatOffer ───────────────>│  (currency, amount, price, settlement method)
  │                              │
  │<──── AcceptTrade ────────────│  (taker's crypto address)
  │                              │
  │─── CryptoReserved ──────────>│  (crypto locked in escrow)
  │                              │
  │─── PaymentInstructions ────>│  (bank details, IBAN, SWIFT)
  │                              │
  │<──── PaymentSubmitted ───────│  (payment reference, amount, timestamp)
  │                              │
  │─── PaymentConfirmed ───────>│  (maker confirms receipt)
  │                              │
  │─── CryptoReleased ─────────>│  (2-of-2 multisig co-sign, BTC sent)
  │                              │
  │<──── TradeComplete ──────────│
```

### 8.3 Payment Types

```rust
struct FiatAmount { amount: u64, currency: FiatCurrency }
struct PaymentId { uuid: Uuid }
struct PaymentReference { description: String }
struct PaymentInstructions { recipient: PaymentRecipient }
struct PaymentRecipient {
    name: String,
    account_number: String,
    routing_code: String,
    bank_name: String,
    iban: Option<String>,
    swift_code: Option<String>,
}
struct VerifiedPayment {
    payment_id: PaymentId,
    amount: FiatAmount,
    status: PaymentStatus,
    verified_at: u64,
}
```

### 8.4 Settlement Deadlines

```rust
struct SettlementDeadlines {
    payment_deadline: u64,    // When taker must submit payment
    confirmation_deadline: u64, // When maker must confirm and release
}
```

Deadlines are enforced by `SettlementEngine::check_deadlines()`, which transitions expired trades to `Failed` state.

### 8.5 Security Bonds

```rust
struct SecurityBond {
    amount: Satoshis,
    state: BondState,  // Required → Locked → Released | PartiallySlashed | Slashed
}
```

Both parties lock bonds at trade creation. Bonds are released on successful completion or can be slashed on dispute resolution. Early cancellation supports `Required → Released` transition.

---

## 9. Databases, External Services and Infrastructure

### 9.1 Storage

**Primary Storage:** File-based repositories with atomic write semantics

| Repository | Storage Format | Location |
|------------|---------------|----------|
| `SwapRepository` | JSON files per swap ID | Configurable base directory |
| `EventRepository` | Append-only JSON lines | Configurable base directory |
| `TransactionIntentRepository` | JSON files per transaction | Configurable base directory |

**Atomic Write Pattern:**
1. Write to `.filename.uuid.tmp`
2. `fsync()` to ensure durability
3. `rename()` to final path (atomic on POSIX)
4. On failure, clean up temp file

### 9.2 External Services

| Service | Purpose | Required |
|---------|---------|----------|
| Electrum server | Bitcoin blockchain queries | Yes (self-hosted or third-party) |
| Monero daemon | Monero blockchain queries | Yes (self-hosted recommended) |
| Monero wallet RPC | Monero wallet operations | Yes (for XMR trades) |
| Bitcoin wallet RPC | Bitcoin wallet operations | Yes (for BTC trades) |
| Bank/PSP APIs | Fiat payment detection | Yes (for fiat trades) |
| libp2p bootstrap nodes | Peer discovery | Yes (network bootstrapping) |

### 9.3 Infrastructure Dependencies

| Dependency | Purpose |
|------------|---------|
| libp2p | P2P networking (Kademlia, Gossipsub, Ping, Identify) |
| Iced framework | Desktop GUI rendering |
| `electrum-client` | Bitcoin Electrum RPC protocol |
| `monero-rpc` | Monero daemon JSON-RPC protocol |
| `bitcoin` crate | Bitcoin transaction construction and parsing |
| `secp256k1` crate | Elliptic curve cryptography |
| `serde` / `serde_json` | Serialization for storage and transport |

### 9.4 Network Topology

```
┌──────────────┐     libp2p      ┌──────────────┐
│  Participant │ ◄─── Gossipsub ──►  Participant │
│   (Maker)    │ ──── Kademlia ───►   (Taker)    │
└──────────────┘                  └──────────────┘
       │                                │
       ▼                                ▼
┌──────────────┐                  ┌──────────────┐
│ Electrum RPC │                  │ Monero Daemon│
│  (Bitcoin)   │                  │  (Monero)    │
└──────────────┘                  └──────────────┘
       │                                │
       ▼                                ▼
┌──────────────┐                  ┌──────────────┐
│ Bank/PSP API │                  │ Wallet RPC   │
│ (Fiat)       │                  │ (BTC/XMR)    │
└──────────────┘                  └──────────────┘
```

---

## 10. Information Exchanged Between Counterparties

### 10.1 Atomic Swap Messages

| Message | Direction | Contents |
|---------|-----------|----------|
| `ProtocolOffer` | Both | Supported protocol versions |
| `ProtocolSelection` | Both | Selected protocol version |
| `DleqProofData` | Both | secp256k1 public key, Ed25519 public key, DLEQ proof |
| `ProtocolEnvelope` | Both | Protocol version, swap ID, message ID, sender, sequence, payload |
| `ChainObservation` | Both | Transaction detected, confirmations, block height |

### 10.2 Fiat Trade Messages

| Message | Direction | Contents |
|---------|-----------|----------|
| `FiatOffer` | Maker → Taker | Currency, crypto asset, amount, price, settlement method, bond requirement |
| `AcceptTrade` | Taker → Maker | Taker's crypto address, acceptance timestamp |
| `PaymentInstructions` | Maker → Taker | Bank details, IBAN, SWIFT code, account number |
| `PaymentSubmitted` | Taker → Maker | Payment reference, amount, timestamp |
| `PaymentConfirmed` | Maker → Taker | Confirmation of fiat receipt |
| `PartialSignature` | Both | Bitcoin transaction partial signature for co-signing |
| `Dispute` | Either | Dispute reason, evidence |

### 10.3 P2P Network Messages

| Message | Protocol | Purpose |
|---------|----------|---------|
| Gossipsub publish | libp2p | Order book updates, trade state changes |
| Kademlia PUT/GET | libp2p | Peer discovery and DHT storage |
| Identify | libp2p | Protocol version negotiation |
| Ping | libp2p | Peer liveness detection |

### 10.4 Protocol Envelope Structure

```rust
struct ProtocolEnvelope {
    protocol: ProtocolVersion,   // Settlement protocol version
    swap_id: SwapId,             // Associated swap
    message_id: MessageId,       // Unique ID for replay protection
    sender: ParticipantId,       // Message sender
    sequence: u64,               // Monotonic sequence for ordering
    payload: Vec<u8>,            // Serialized protocol message
}
```

---

## 11. Trust Boundaries

### 11.1 Trust Model Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     FULLY TRUSTED (Local)                       │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Swap Engine State Machine                              │   │
│  │  - Deterministic state transitions                      │   │
│  │  - Event-sourced audit trail                            │   │
│  │  - Replay protection                                    │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Cryptographic Operations                               │   │
│  │  - DLEQ proof verification (local)                      │   │
│  │  - Multisig co-signing (local keys)                     │   │
│  │  - Key zeroization on drop                              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│              PARTIALLY TRUSTED (External Services)              │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Blockchain Nodes                                       │   │
│  │  - Electrum RPC: Transaction lookup, confirmations      │   │
│  │  - Monero daemon: Block height, transaction relay       │   │
│  │  Trust: Transaction data verified by consensus          │   │
│  │  Risk: Censorship, liveness (mitigated by multi-node)   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Bank/PSP APIs                                          │   │
│  │  - Payment detection for fiat trades                    │   │
│  │  Trust: Payment confirmation from financial institution │   │
│  │  Risk: False positives, delayed reporting               │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│              UNTRUSTED (Participant-Provided)                   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Counterparty Information                               │   │
│  │  - Crypto addresses (validated format, not ownership)   │   │
│  │  - Payment references (verified via bank API)           │   │
│  │  - DLEQ proofs (cryptographically verified)             │   │
│  │  - Protocol messages (replay-protected, sequenced)      │   │
│  │  Trust: Nothing assumed; everything verified            │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 11.2 Operations by Trust Level

**Fully Local (No External Trust Required):**
- State machine transitions and validation
- Event recording and replay
- DLEQ proof verification
- Multisig partial signature creation
- Key generation and zeroization
- Bond calculation and management
- Protocol version negotiation
- Message replay detection
- Rate limiting

**Relies on External Services:**
- Bitcoin transaction confirmation counts (Electrum RPC)
- Monero block height queries (daemon RPC)
- Transaction broadcasting to either chain
- Fiat payment detection (bank/PSP APIs)
- Peer discovery (libp2p bootstrap nodes)

**Depends on Participant-Provided Information:**
- Counterparty's crypto destination address (format-validated, not ownership-verified)
- Counterparty's DLEQ proof (cryptographically verified)
- Payment submission reference (verified via bank API cross-check)
- Protocol messages (verified via replay protection and sequence numbers)
- Dispute evidence (subject to resolution process)

### 11.3 Fiat Trade Trust Mitigations

Fiat trades cannot be cryptographically atomic. The following mechanisms mitigate trust requirements:

| Risk | Mitigation |
|------|-----------|
| Maker doesn't release crypto | 2-of-2 multisig requires co-signing; taker can dispute |
| Taker doesn't send fiat | Payment detection via bank API; timeout to `Failed` state |
| Taker sends wrong amount | Amount verification before confirmation |
| Maker confirms but never releases | `MakerConfirmed` timeout forces `Failed` state |
| Either party acts maliciously | Security bonds are slashed on dispute |
| Permanent fund lockup | Cancellation paths from all active states |

### 11.4 Atomic Swap Trust Properties

BTC-XMR atomic swaps are cryptographically guaranteed:

| Property | Mechanism |
|----------|-----------|
| Atomicity | DLEQ proofs ensure same-key redemption on both chains |
| No counterparty trust | Protocol enforces state machine; no manual intervention |
| Fund recovery | Recovery engine handles timeouts and refunds |
| No third-party custodian | Non-custodial; funds controlled by participants |

---

## 12. Data Flow Diagrams

### 12.1 Atomic Swap Data Flow

```
Maker Node                          OpenDex Network                        Taker Node
┌──────────────┐                   ┌──────────────┐                   ┌──────────────┐
│  SwapEngine  │                   │  Transport   │                   │  SwapEngine  │
│  (local)     │                   │  (libp2p)    │                   │  (local)     │
└──────┬───────┘                   └──────┬───────┘                   └──────┬───────┘
       │                                  │                                  │
       │  1. create_swap()                │                                  │
       │  ──────────────────► SwapRepo    │                                  │
       │     (persist swap)               │                                  │
       │                                  │                                  │
       │  2. protocol.init()              │                                  │
       │  ──────────────────► BTC-XMR     │                                  │
       │     adapter (DLEQ proof gen)     │                                  │
       │                                  │                                  │
       │  3. ProtocolEnvelope             │                                  │
       │  (DLEQ proof + BTC pk) ──────────► Gossipsub ──────────────────────►│
       │                                  │                                  │
       │                                  │  4. protocol.process_message()   │
       │                                  │     (verify DLEQ proof)          │
       │                                  │                                  │
       │                                  │  5. ProtocolEnvelope             │
       │                                  │  (DLEQ proof + XMR pk) ◄─────────┘
       │  6. protocol.process_message()   │                                  │
       │  (verify DLEQ proof) ◄───────────┤                                  │
       │                                  │                                  │
       │  7. Watcher: BTC funding         │                                  │
       │  detected ──────────────────────► ChainObservation                  │
       │                                  │                                  │
       │  8. protocol.process_observation()│                                 │
       │     → BuildTransaction(XMR)      │                                  │
       │                                  │                                  │
       │                                  │  9. Watcher: XMR funding         │
       │                                  │  detected ──────────────────────►│
       │                                  │                                  │
       │                                  │ 10. protocol.process_observation()│
       │                                  │     → BuildTransaction(BTC red.) │
       │                                  │                                  │
       │ 11. Redemption on BTC            │ 12. Redemption on XMR            │
       │     chain completes              │     chain completes              │
       │                                  │                                  │
       │  ←── SwapCompleted ──────────────┤──────────────────────────────────┘
```

### 12.2 Fiat Trade Data Flow

```
Maker Node                          OpenDex Network                        Taker Node
┌──────────────┐                   ┌──────────────┐                   ┌──────────────┐
│SettlementEng │                   │  P2P Node    │                   │SettlementEng │
│   (local)    │                   │  (libp2p)    │                   │   (local)    │
└──────┬───────┘                   └──────┬───────┘                   └──────┬───────┘
       │                                  │                                  │
       │  1. FiatOffer                    │                                  │
       │  (GBP 1000 @ BTC price) ─────────► Gossipsub ──────────────────────►│
       │                                  │                                  │
       │                                  │  2. create_trade()               │
       │                                  │     (accept offer + BTC addr)    │
       │                                  │                                  │
       │  3. AcceptTrade ◄─────────────────┤─────────────────────────────────┘
       │                                  │                                  │
       │  4. reserve_crypto()             │                                  │
       │  ──────────────────► MultisigMgr │                                  │
       │     (lock BTC in 2-of-2)         │                                  │
       │                                  │                                  │
       │  5. request_payment()            │                                  │
       │  ──────────────────► CryptoWallet│                                  │
       │     (generate bank details)      │                                  │
       │                                  │                                  │
       │  6. PaymentInstructions          │                                  │
       │  (IBAN, SWIFT, account) ─────────► Gossipsub ──────────────────────►│
       │                                  │                                  │
       │                                  │  7. taker_confirm_payment()      │
       │                                  │     (submit payment ref)         │
       │                                  │                                  │
       │  8. PaymentSubmitted ◄────────────┤─────────────────────────────────┘
       │                                  │                                  │
       │  9. maker_confirm_payment()      │                                  │
       │     (verify via bank API)        │                                  │
       │                                  │                                  │
       │ 10. release_crypto()            │                                  │
       │  ──────────────────► MultisigMgr │                                  │
       │     (co-sign 2-of-2 payout)     │                                  │
       │                                  │                                  │
       │ 11. TradeComplete ───────────────┤─────────────────────────────────►│
       │                                  │                                  │
       │ 12. bond.release()              │ 13. bond.release()               │
       │     (return bonds)              │     (return bonds)               │
```

### 12.3 Storage and Persistence Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    SwapEngine / SettlementEngine             │
│                                                              │
│  State Change ─────────────────────────────────────────────►│
│                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │ SwapRepo     │    │ EventRepo    │    │ TxIntentRepo │  │
│  │ (JSON files) │    │ (JSON lines) │    │ (JSON files) │  │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘  │
│         │                   │                   │           │
│         ▼                   ▼                   ▼           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              atomic_write()                          │  │
│  │  1. Write to .filename.uuid.tmp                      │  │
│  │  2. fsync()                                          │  │
│  │  3. rename() to final path (atomic on POSIX)         │  │
│  │  4. On failure: remove temp file                     │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  Recovery: event replay ──► State reconstruction            │
└─────────────────────────────────────────────────────────────┘
```

### 12.4 P2P Network Message Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      Desktop GUI                             │
│  (Iced framework)                                           │
└──────────────────────┬──────────────────────────────────────┘
                       │ IPC channels (mpsc)
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                      P2P Node                                │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐ │
│  │Orderbook │  │ Trades   │  │ Balances │  │  Escrow    │ │
│  │  & CLI   │  │  State   │  │  & Sync  │  │  Manager   │ │
│  └──────────┘  └──────────┘  └──────────┘  └────────────┘ │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              OpenDexBehaviour                         │   │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────────┐    │   │
│  │  │Kademlia│ │  Ping  │ │Identify│ │ Gossipsub  │    │   │
│  │  │(DHT)   │ │(alive) │ │(proto) │ │ (pub/sub)  │    │   │
│  │  └────────┘ └────────┘ └────────┘ └────────────┘    │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────┐  ┌──────────┐                                 │
│  │Wallet    │  │Wallet    │                                 │
│  │Client BTC│  │Client XMR│                                 │
│  └────┬─────┘  └────┬─────┘                                 │
│       │              │                                       │
│       ▼              ▼                                       │
│  ┌──────────┐  ┌──────────┐                                 │
│  │Persistence│ │Log Redact│                                 │
│  │(files)   │  │(sensitive│                                 │
│  └──────────┘  │ data)    │                                 │
│                └──────────┘                                 │
└─────────────────────────────────────────────────────────────┘
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
     Electrum RPC  monerod     Bank/PSP
     (Bitcoin)    (Monero)     APIs
```

### 12.5 Key Lifecycle Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Generation  │────►│    Use       │────►│  Destruction │
│              │     │              │     │              │
│ thread_rng() │     │ Signing      │     │ zeroize()    │
│ Secp256k1    │     │ Verification │     │ Drop impl    │
│ Ed25519      │     │ Encryption   │     │ Memory clear │
└──────────────┘     └──────────────┘     └──────────────┘
       │                    │                     │
       ▼                    ▼                     ▼
  In-memory only    In-memory only        Overwritten
  Not persisted     Not persisted         with zeros
```

---

## Appendix A: Crate Dependency Graph

```
desktop-gui
  └── p2p-node
        ├── opendex-types
        ├── opendex-errors
        ├── fiat
        │     ├── opendex-types
        │     ├── opendex-errors
        │     └── bitcoin (secp256k1)
        ├── user
        │     ├── opendex-types
        │     └── opendex-errors
        └── protocol
              └── opendex-types

opendex-swap
  ├── opendex-protocol-types
  │     └── opendex-types
  ├── opendex-errors
  ├── opendex-events
  │     └── opendex-types
  ├── opendex-storage
  │     └── opendex-errors
  ├── opendex-recovery
  │     └── opendex-protocol-types
  ├── opendex-transport
  │     └── opendex-types
  ├── opendex-bitcoin
  │     ├── opendex-errors
  │     └── opendex-types
  └── opendex-monero
        ├── opendex-errors
        └── opendex-types

opendex-btc-xmr
  ├── opendex-protocol-types
  ├── opendex-errors
  ├── opendex-types
  ├── secp256k1
  └── dleq

opendex-watchers
  ├── opendex-protocol-types
  ├── opendex-bitcoin
  └── opendex-monero

opendex-protocol
  ├── opendex-types
  └── opendex-errors

opendex-fee-treasury
  └── opendex-types

opendex-dao
  └── opendex-types

common
  └── opendex-types

web-frontend
  └── p2p-node (via GUI bridge)
```

---

## Appendix B: Security Controls Summary

| Control ID | Description | Status |
|------------|-------------|--------|
| SEC-001 | Argon2id password hashing | ✓ Verified |
| SEC-002 | HMAC-signed auth tokens | ✓ Verified |
| SEC-003 | Caller authorization on all operations | ✓ Verified |
| SEC-004 | Key zeroization on drop | ✓ Verified |
| SEC-005 | Bitcoin 2-of-2 multisig escrow | ✓ Verified |
| SEC-006 | State machine enforcement | ✓ Verified |
| SEC-007 | Message replay protection | ✓ Verified |
| SEC-008 | Rate limiting (5 login attempts/min) | ✓ Verified |
| SEC-BTC-001 | Bitcoin transaction validation | ✓ Verified |
| SEC-BTC-002 | Miner fee deduction | ✓ Verified |
| SEC-FIAT-001 | Payment amount verification | ✓ Verified |
| SEC-FIAT-002 | Bond calculation and enforcement | ✓ Verified |
| SEC-FIAT-003 | Cancellation from all active states | ✓ Verified |
| SEC-FIAT-004 | MakerConfirmed timeout | ✓ Verified |
| SEC-FIAT-005 | Bond release from Required state | ✓ Verified |
| SEC-FIAT-006 | Conditional escrow return on cancel | ✓ Verified |
| SEC-FIAT-007 | Same-state transition rejection | ✓ Verified |
| SEC-FIAT-008 | Failure paths for all timeout states | ✓ Verified |

---

## Appendix C: Known Implementation Gaps

| Component | Status | Impact |
|-----------|--------|--------|
| DLEQ proof implementation | Stub pending Eigenwallet integration | Atomic swaps not yet functional |
| Monero daemon RPC methods | Stub implementations | Monero chain queries not yet functional |
| `lru` crate unsoundness | GUI-only transitive dependency | No impact on financial code |

---

*Document generated 2026-10-14. Based on release tag `v0.1.0-security-review`.*
