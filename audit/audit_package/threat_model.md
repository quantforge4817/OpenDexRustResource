# Threat Model

## Assets

1. **User Funds**: BTC and XMR locked in swap escrow transactions
2. **Private Keys**: Secret scalars used for DLEQ proofs and redemption
3. **Swap State**: In-memory and persisted state determining fund flow
4. **Network Messages**: Protocol messages between swap participants

## Threat Actors

| Actor | Capability | Motivation |
|-------|-----------|------------|
| Malicious Counterparty | Controls one side of the swap | Steal funds from the other party |
| Network Adversary | Observes/modifies P2P messages | Replay, reorder, or inject messages |
| Blockchain Adversary | Controls miners/validators | Censor transactions or reorg blocks |
| Compromised Node | Full access to node internals | Extract private keys or manipulate state |

## Attack Vectors

### 1. Cryptographic Attacks (Critical)
- **DLEQ Proof Forgery**: Attacker generates valid-looking proof without knowing the secret
- **Key Derivation Oracle**: Attacker derives counterparty's Monero key from Bitcoin key
- **Nonce Reuse**: DLEQ proof nonce reused, leaking the secret scalar

**Mitigations**: Fiat-Shamir challenge binding, OS RNG for nonces, cross-curve challenge

### 2. State Machine Attacks (High)
- **Invalid Transitions**: Forcing swap into inconsistent state
- **Double Redemption**: Both parties redeem funds
- **Timeout Race**: Racing at timeout boundary to claim/refund

**Mitigations**: Strict state transition validation, on-chain transaction locking

### 3. Network Attacks (Medium)
- **Replay Attacks**: Re-sending old messages
- **Message Reordering**: Delivering messages out of sequence
- **Man-in-the-Middle**: Intercepting and modifying messages

**Mitigations**: MessageStore replay protection, sequence numbers, libp2p encryption

### 4. Blockchain Attacks (Medium)
- **Transaction Censorship**: Preventing funding/redemption tx confirmation
- **Chain Reorg**: Reorganizing blocks to invalidate confirmations
- **Dust Attacks**: Polluting addresses with dust transactions

**Mitigations**: Confirmation thresholds (6 BTC, 10 XMR), reconciliation logic

### 5. Storage Attacks (Low)
- **Event Log Tampering**: Modifying persisted events
- **Database Corruption**: Losing swap state

**Mitigations**: Append-only event log, atomic JSON writes

## Trust Assumptions

1. The Rust standard library and crypto crates (k256, curve25519-dalek, secp256k1) are correctly implemented
2. The Monero daemon RPC returns accurate blockchain data
3. The Electrum server returns accurate Bitcoin blockchain data
4. The operating system's RNG (OsRng) provides cryptographically secure randomness

## Out of Scope

- Social engineering attacks
- Physical security of hardware
- Supply chain attacks on dependencies (covered by cargo-deny)
- Denial of service against the P2P network
