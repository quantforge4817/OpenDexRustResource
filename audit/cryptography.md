# OpenDex Cryptography Document

**Version:** 0.1.0-security-review
**Date:** 2026-10-14
**Status:** Authoritative cryptography reference
**Scope:** All cryptographic primitives, libraries, and protocol constructions

---

## Table of Contents

1. [Overview](#1-overview)
2. [Cryptographic Libraries](#2-cryptographic-libraries)
3. [Elliptic Curve Primitives](#3-elliptic-curve-primitives)
4. [Hash Functions](#4-hash-functions)
5. [DLEQ Proof System](#5-dleq-proof-system)
6. [Bitcoin Multisig](#6-bitcoin-multisig)
7. [Authentication Cryptography](#7-authentication-cryptography)
8. [Random Number Generation](#8-random-number-generation)
9. [Key Management](#9-key-management)
10. [Cryptographic Assumptions](#10-cryptographic-assumptions)
11. [Implementation Gaps](#11-implementation-gaps)

---

## 1. Overview

OpenDex uses only standard, well-reviewed cryptographic primitives. No custom cryptography is implemented. All cryptographic operations rely on established libraries with published specifications.

### 1.1 Design Principle

> The purpose is not to design new cryptography. OpenDex integrates reviewed implementations of established constructions and places a stable adapter around them.

### 1.2 Non-Goals

v0.1 does **not** define:

- New cryptographic primitives
- New signature schemes
- New DLEQ construction
- New Monero consensus rules
- New Bitcoin consensus rules

### 1.3 Cryptographic Inventory

| Primitive | Type | Library | Standard |
|-----------|------|---------|----------|
| secp256k1 signatures | ECDSA/Schnorr | `secp256k1` crate v0.29 | Bitcoin standard |
| Ed25519 signatures | EdDSA | `ed25519` crate | Monero standard |
| Cross-curve DLEQ proofs | Zero-knowledge proof | `dleq` crate v0.4 | Eigenwallet/COMIT construction |
| SHA-256/SHA-512 | Hash functions | `sha2` crate v0.10 | NIST FIPS 180-4 |
| Argon2id | Password hashing | `argon2` crate | RFC 9106 |
| HMAC-SHA256 | Token signing | `hmac` + `sha2` crates | RFC 2104 |
| 2-of-2 multisig | P2WSH redemption script | `bitcoin` crate v0.32 | Bitcoin standard |
| AES-GCM | Encryption (libp2p) | `aes-gcm` crate | NIST SP 800-38D |

---

## 2. Cryptographic Libraries

### 2.1 secp256k1

**Crate:** `secp256k1` v0.29
**Purpose:** Bitcoin secp256k1 ECDSA/Schnorr signatures
**Specification:** SEC 2 Elliptic Curve Cryptography

| Operation | Function | Use Case |
|-----------|----------|----------|
| Key generation | `Secp256k1::<All>::generate_keypair()` | Multisig escrow key pairs |
| Signing | `sign_ecdsa()`, `sign_schnorr()` | Bitcoin transaction signing |
| Verification | `verify_ecdsa()`, `verify_schnorr()` | Transaction signature verification |
| Public key derivation | `PublicKey::from_secret_key()` | Key exchange |

**Security properties:**
- Keys are 256-bit secret scalars
- Public keys are compressed (33 bytes) or uncompressed (65 bytes)
- Signatures are deterministic (RFC 6979 for ECDSA)

### 2.2 Bitcoin

**Crate:** `bitcoin` v0.32
**Purpose:** Bitcoin transaction and address handling
**Specification:** Bitcoin Protocol Specification

| Operation | Function | Use Case |
|-----------|----------|----------|
| Transaction construction | `Transaction::new()` | Build funding, redemption, refund transactions |
| Address parsing | `Address::from_str()` | Validate Bitcoin addresses |
| Script construction | `Script::new_p2wsh()` | Build multisig redemption scripts |
| Witness stacking | `Witness::new()` | Build multisig witness data |
| Transaction decoding | `Transaction::consensus_decode()` | Parse raw transactions from blockchain |

### 2.3 DLEQ

**Crate:** `dleq` v0.4
**Purpose:** Cross-curve Discrete Log Equality proof generation and verification
**Specification:** Eigenwallet/COMIT BTC-XMR atomic swap construction

**Current status:** Stub implementation. The `dleq` crate is a dependency, but the full integration with Eigenwallet's BTC-XMR library is pending. Current `generate_proof()` and `verify_proof()` methods use simplified placeholders.

### 2.4 SHA-2

**Crate:** `sha2` v0.10
**Purpose:** SHA-256 and SHA-512 hash functions
**Specification:** NIST FIPS 180-4

| Function | Output | Use Case |
|----------|--------|----------|
| SHA-256 | 256 bits | Transaction IDs, message digests |
| SHA-512 | 512 bits | Key derivation, Monero scalar clamping |

### 2.5 Argon2

**Crate:** `argon2` (version from Cargo.lock)
**Purpose:** Password hashing for user authentication
**Specification:** RFC 9106

| Parameter | Value |
|-----------|-------|
| Variant | Argon2id |
| Memory | 64 MB (default) |
| Iterations | 3 (default) |
| Parallelism | 1 (default) |
| Salt | 16 bytes random |
| Output | 32 bytes hash |

### 2.6 HMAC

**Crate:** `hmac` + `sha2`
**Purpose:** Authentication token signing
**Specification:** RFC 2104

| Parameter | Value |
|-----------|-------|
| Algorithm | HMAC-SHA256 |
| Key | Per-user secret |
| Input | Token payload |
| Output | 32 bytes signature |

### 2.7 Ed25519

**Crate:** `ed25519` (via Monero dependencies)
**Purpose:** Monero Ed25519 signatures
**Specification:** RFC 8032 (with Monero-specific modifications)

**Note:** Monero uses a modified version of Ed25519 (called "ed25519-zebra" or "Monero Ed25519") with different hashing and scalar clamping. The standard `ed25519` crate may not be directly compatible with Monero's signature scheme.

### 2.8 Monero RPC

**Crate:** `monero-rpc`
**Purpose:** Monero daemon JSON-RPC interface
**Specification:** Monero RPC API

| Method | Purpose |
|--------|---------|
| `get_transaction` | Lookup transaction by hash |
| `get_height` | Get current block height |
| `on_submit_block_submit` | Submit transaction for relay |

**Current status:** Stub implementation. The RPC methods are defined but not fully implemented.

---

## 3. Elliptic Curve Primitives

### 3.1 secp256k1 (Bitcoin)

**Curve parameters:**

| Parameter | Value |
|-----------|-------|
| Field prime p | 2^256 - 2^32 - 977 |
| Order n | 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141 |
| Base point G | Standard secp256k1 generator |
| Curve equation | y^2 = x^3 + 7 |

**Key generation:**

```
1. Generate 256-bit random scalar k using thread_rng()
2. Compute public key P = k * G
3. Return (k, P) pair
```

**Security assumption:** The elliptic curve discrete logarithm problem (ECDLP) on secp256k1 is hard. Given P = k * G, it is computationally infeasible to determine k.

### 3.2 Ed25519 (Monero)

**Curve parameters:**

| Parameter | Value |
|-----------|-------|
| Curve | Curve25519 (Twisted Edwards form) |
| Field prime p | 2^255 - 19 |
| Order l | 2^252 + 27742317777372353535851937790883648493 |
| Base point | Standard Curve25519 generator |

**Key generation:**

```
1. Generate 256-bit random seed using thread_rng()
2. Hash seed with SHA-512
3. Clamp the first half to produce scalar s
4. Compute public key P = s * B
5. Return (s, P) pair
```

**Security assumption:** The elliptic curve discrete logarithm problem (ECDLP) on Curve25519 is hard. Given P = s * B, it is computationally infeasible to determine s.

### 3.3 Cross-Curve Key Derivation

For BTC-XMR atomic swaps, the same secret scalar must be used on both curves:

```
1. Generate 256-bit random scalar k
2. Reduce k modulo min(n_secp256k1, l_ed25519)
3. Compute Bitcoin public key: P_btc = k * G_secp256k1
4. Compute Monero public key: P_xmr = k * G_ed25519
5. Return (k, P_btc, P_xmr) triple
```

**Security assumption:** The reduced scalar is valid on both curves. The minimum of the two curve orders ensures the scalar is within the valid range for both.

---

## 4. Hash Functions

### 4.1 SHA-256

**Use cases:**

| Context | Input | Output |
|---------|-------|--------|
| Transaction ID | Raw transaction bytes | 256-bit hash |
| Message digest | Protocol message payload | 256-bit hash |
| HMAC key derivation | Password + salt | 256-bit key |

**Security properties:**
- Preimage resistance: Given H, finding M such that SHA-256(M) = H is infeasible
- Second preimage resistance: Given M1, finding M2 such that SHA-256(M1) = SHA-256(M2) is infeasible
- Collision resistance: Finding M1 ≠ M2 such that SHA-256(M1) = SHA-256(M2) is infeasible

### 4.2 SHA-512

**Use cases:**

| Context | Input | Output |
|---------|-------|--------|
| Key derivation | Random seed | 512-bit hash (split for scalar and public key mask) |
| Monero scalar clamping | SHA-512 output | Clamped scalar for Ed25519 |

---

## 5. DLEQ Proof System

### 5.1 Purpose

The DLEQ (Discrete Log Equality) proof allows one party to prove to another that the same secret scalar is used on both secp256k1 and Ed25519 curves, without revealing the secret.

### 5.2 Mathematical Construction

Given:
- Secret scalar: k
- Bitcoin public key: P_btc = k * G_secp256k1
- Monero public key: P_xmr = k * G_ed25519

The prover wants to prove: ∃k such that P_btc = k * G_secp256k1 ∧ P_xmr = k * G_ed25519

Without revealing k.

### 5.3 Proof Protocol (Sigma Protocol)

The DLEQ proof follows a standard three-move sigma protocol:

```
1. Commitment:
   - Prover generates random r
   - Computes R_btc = r * G_secp256k1
   - Computes R_xmr = r * G_ed25519
   - Sends (R_btc, R_xmr) to verifier

2. Challenge:
   - Verifier computes c = H(P_btc, P_xmr, R_btc, R_xmr)
   - Sends c to prover

3. Response:
   - Prover computes s = r + c * k (mod order)
   - Sends s to verifier

4. Verification:
   - Verifier checks: s * G_secp256k1 == R_btc + c * P_btc
   - Verifier checks: s * G_ed25519 == R_xmr + c * P_xmr
   - Both equations must hold
```

### 5.4 Security Properties

| Property | Description |
|----------|-------------|
| **Completeness** | Honest prover can always convince honest verifier |
| **Soundness** | Cheating prover cannot convince verifier without knowing k |
| **Zero-knowledge** | Verifier learns nothing about k beyond the fact that the same k is used on both curves |

### 5.5 Current Implementation

**Crate:** `opendex-btc-xmr`

```rust
struct DleqProofData {
    btc_public_key: Vec<u8>,    // secp256k1 public key (Bitcoin)
    xmr_public_key: Vec<u8>,    // Ed25519 public key (Monero)
    proof: Vec<u8>,             // DLEQ proof bytes
    version: u32,               // Protocol version
}
```

**Current status:** **Stub implementation.**

| Method | Current Behavior | Required Behavior |
|--------|-----------------|-------------------|
| `generate_proof()` | Returns placeholder bytes | Execute full sigma protocol |
| `verify_proof()` | Returns `true` without verification | Verify both curve equations |
| `derive_monero_public_key()` | SHA-512 with Ed25519 scalar clamping | Full cross-curve key derivation |

**Remediation:** Full Eigenwallet library integration required before production use.

### 5.6 Adaptor Signatures

The BTC-XMR protocol uses adaptor signatures to enable atomic redemption:

**Concept:** An adaptor signature is a pre-signature that can be "adapted" into a valid signature given a witness (secret).

**Protocol flow:**

```
1. Alice creates adaptor signature on Bob's Bitcoin redemption
   - Signature is valid but incomplete without the witness
2. Bob can only adapt the signature if he reveals the witness
3. Revealing the witness allows Alice to claim Bob's Monero
4. This creates an atomic exchange: one party cannot claim without the other being able to claim
```

**Current status:** Not yet implemented. Part of the pending Eigenwallet integration.

---

## 6. Bitcoin Multisig

### 6.1 2-of-2 P2WSH Multisig

**Purpose:** Fiat trade escrow — both parties must co-sign to release funds.

**Redemption script:**

```
OP_2 <pub1> <pub2> OP_2 OP_CHECKMULTISIG
```

**Witness structure for payout:**

```
[0] <signature2>
[1] <signature1>
[2] <redemption_script>
```

### 6.2 Co-signing Protocol

```
1. Key exchange:
   - Each party generates EscrowKeyPair (secp256k1)
   - Public keys are exchanged via P2P transport

2. Address derivation:
   - Both parties compute the 2-of-2 P2WSH address
   - Maker sends BTC to the address

3. Payout co-signing:
   - Each party creates a partial signature on the payout transaction
   - Partial signatures are exchanged via P2P transport
   - Signatures are combined into a final transaction
   - Final transaction is broadcast

4. Return co-signing (on dispute/cancellation):
   - Each party creates a partial signature on the return transaction
   - Same exchange and broadcast process
```

### 6.3 Key Security

**Crate:** `fiat::multisig`

| Component | Security Measure |
|-----------|-----------------|
| `EscrowKeyPair` | `Drop` implementation calls `zeroize()` on secret key |
| `PartialSignature` | In-memory only; not persisted |
| `RedemptionScript` | Derived from public keys; no secret data |

### 6.4 Transaction Construction

| Transaction | Inputs | Outputs | Signatures Required |
|-------------|--------|---------|-------------------|
| Deposit | Maker's UTXO | 2-of-2 P2WSH address | Maker only |
| Payout | 2-of-2 P2WSH address | Taker's address | Both parties |
| Return | 2-of-2 P2WSH address | Maker's address | Both parties |

---

## 7. Authentication Cryptography

### 7.1 Password Hashing

**Algorithm:** Argon2id
**Crate:** `argon2`

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Variant | Argon2id | Hybrid of Argon2i and Argon2d; resistant to side-channel and GPU attacks |
| Memory | 64 MB | High memory cost prevents brute-force |
| Iterations | 3 | Multiple passes over memory |
| Parallelism | 1 | Single-threaded; sufficient for server-side |
| Salt | 16 bytes random | Per-user unique salt |
| Output | 32 bytes | Sufficient for comparison |

**Process:**

```
1. Generate 16-byte random salt
2. Hash password with Argon2id(salt, password, memory=64MB, iterations=3)
3. Store salt + hash in user record
4. On login, recompute hash with stored salt and compare
```

### 7.2 Token Signing

**Algorithm:** HMAC-SHA256
**Crates:** `hmac` + `sha2`

**Process:**

```
1. Generate per-user HMAC secret (256-bit random)
2. On login, create token payload (user_id, timestamp, expiry)
3. Compute HMAC-SHA256(secret, payload)
4. Return token = payload + signature
5. On subsequent requests, verify HMAC signature
```

**Security properties:**
- Tokens are bound to the user's HMAC secret
- Tokens expire after a configurable duration
- Tokens cannot be forged without the HMAC secret

### 7.3 Rate Limiting

**Implementation:** Per-username login attempt limiting

| Parameter | Value |
|-----------|-------|
| Max attempts | 5 |
| Time window | 60 seconds |
| Action on exceed | Reject login attempt |

---

## 8. Random Number Generation

### 8.1 Source

**Crate:** `getrandom` (via `rand`/`rand_core`)
**Function:** `thread_rng()`

**Implementation:** OS-provided CSPRNG

| OS | Source |
|----|--------|
| Linux | `/dev/urandom` |
| macOS | `getentropy()` |
| Windows | `BCryptGenRandom()` |

### 8.2 Usage

| Context | Function | Security Level |
|---------|----------|---------------|
| Key generation | `thread_rng()` | Cryptographic |
| Salt generation | `thread_rng()` | Cryptographic |
| Nonce generation | `thread_rng()` | Cryptographic |
| Test data | `thread_rng()` | Sufficient for testing |

### 8.3 Security Assumption

The OS CSPRNG provides cryptographically secure random bytes. An attacker cannot predict future outputs given past outputs.

---

## 9. Key Management

### 9.1 Key Lifecycle

| Phase | Action | Security Measure |
|-------|--------|-----------------|
| Generation | `thread_rng()` + curve operation | CSPRNG source |
| Storage | In-memory only | No disk persistence for secrets |
| Usage | Signing, verification | Constant-time operations where applicable |
| Destruction | `zeroize()` on `Drop` | Memory wiped when key goes out of scope |

### 9.2 Key Types

| Key Type | Generation | Storage | Destruction |
|----------|-----------|---------|-------------|
| secp256k1 escrow keys | Per-trade `EscrowKeyPair::new()` | In-memory | `Drop` → `zeroize()` |
| DLEQ secrets | Derived from participant key | In-memory during swap | Dropped after swap |
| libp2p identity keys | `Keypair::generate_ed25519()` | Persistent file | Not destroyed on normal exit |
| HMAC secrets | Per-user random generation | Stored in user record | Not destroyed (long-lived) |
| RPC credentials | Passed from GUI | In `GuiNodeConfig` | `Drop` → `zeroize()` |

### 9.3 Zeroization

**Crate:** `zeroize`

All credential zeroization uses the `zeroize` crate's safe API — zero `unsafe` blocks are needed.

| Location | Method |
|----------|---------|
| `p2p-node::GuiNodeConfig::drop()` | `String::zeroize()` — safe API from `zeroize` crate |
| `fiat::multisig::EscrowKeyPair::drop()` | `zeroize()` on secret key bytes via `zeroize` trait |
| `desktop-gui` password fields | `Zeroizing<String>` — automatic zeroization on drop |
| `opendex-storage` encryption keys | `TestMasterKeyProvider` in tests — no unsafe env var access |

**Implementation:**

```rust
impl Drop for GuiNodeConfig {
    fn drop(&mut self) {
        if let Some(mut pass) = self.monero_rpc_pass.take() {
            pass.zeroize();
        }
        if let Some(mut pass) = self.btc_rpc_pass.take() {
            pass.zeroize();
        }
    }
}
```

---

## 10. Cryptographic Assumptions

### 10.1 Mathematical Assumptions

| Assumption | If Violated | Impact |
|------------|-------------|--------|
| secp256k1 ECDLP is hard | Secret keys can be derived from public keys | All Bitcoin signatures forgeable |
| Ed25519 ECDLP is hard | Secret keys can be derived from public keys | All Monero signatures forgeable |
| SHA-256 is collision-resistant | Two inputs produce same hash | Transaction ID collision; signature malleability |
| SHA-512 is collision-resistant | Two inputs produce same hash | Key derivation collision |
| DLEQ proof is sound | False proofs can be generated | Cross-curve key binding broken |

### 10.2 Implementation Assumptions

| Assumption | If Violated | Impact |
|------------|-------------|--------|
| `getrandom` provides CSPRNG output | Predictable random values | All keys guessable |
| `zeroize()` actually zeros memory | Keys remain in memory after drop | Key leakage via memory dump |
| Library implementations are correct | Subtle bugs in cryptographic operations | Unknown security impact |
| No side-channel leakage | Timing/power analysis reveals keys | Key compromise |

### 10.3 Protocol Assumptions

| Assumption | If Violated | Impact |
|------------|-------------|--------|
| Participants use correct key derivation | Wrong keys used for DLEQ proof | Proof verification fails |
| Blockchain transactions are valid | Invalid transactions rejected | Swap fails |
| Network messages are not modified | Tampered messages processed | Protocol state corruption |

---

## 11. Implementation Gaps

### 11.1 Critical Gaps

| Gap | Component | Status | Remediation |
|-----|-----------|--------|-------------|
| DLEQ proof generation | `opendex-btc-xmr` | **Stub** | Integrate Eigenwallet library |
| DLEQ proof verification | `opendex-btc-xmr` | **Stub** (always returns true) | Integrate Eigenwallet library |
| Adaptor signatures | `opendex-btc-xmr` | **Not implemented** | Integrate Eigenwallet library |
| Monero daemon RPC | `opendex-monero` | **Stub** | Implement full RPC methods |

### 11.2 Known Limitations

| Limitation | Description | Impact |
|------------|-------------|--------|
| No constant-time verification | Signature verification may leak timing information | Side-channel attack possible |
| No hardware security module | Keys stored in software memory | Memory dump can extract keys |
| No key rotation | HMAC secrets are long-lived | Compromised secret affects all past tokens |
| No encryption at rest | Swap state files are plaintext | File access reveals swap details | **Resolved** — AES-256-GCM encryption implemented in `opendex-storage` |

### 11.3 Future Enhancements

| Enhancement | Purpose | Priority |
|-------------|---------|----------|
| Eigenwallet integration | Full DLEQ proof implementation | **Critical** |
| Constant-time operations | Prevent side-channel attacks | High |
| Encryption at rest | Protect stored state files | Medium | **Resolved** — AES-256-GCM with HKDF-SHA256 key derivation
| Key rotation | Limit impact of compromised secrets | Medium |
| Hardware security module | Protect keys from memory extraction | Low |

---

## Appendix A: Library Versions

| Library | Version | License |
|---------|---------|---------|
| `secp256k1` | 0.29 | Apache-2.0 OR MIT |
| `bitcoin` | 0.32 | MIT |
| `dleq` | 0.4 | MIT |
| `sha2` | 0.10 | MIT |
| `argon2` | (from Cargo.lock) | MIT OR Apache-2.0 |
| `hmac` | (from Cargo.lock) | MIT |
| `ed25519` | (from Cargo.lock) | MIT |
| `zeroize` | (from Cargo.lock) | Apache-2.0 OR MIT |
| `getrandom` | (from Cargo.lock) | MIT OR Apache-2.0 |
| `rand` | (from Cargo.lock) | MIT |

---

## Appendix B: Cryptographic Protocol References

| Reference | Description |
|-----------|-------------|
| Eigenwallet/COMIT BTC-XMR atomic swap | Cross-chain atomic swap protocol using DLEQ proofs |
| Farcaster project (https://github.com/farcaster-project) | BTC/XMR atomic swap implementation reference |
| RFC 9106 | Argon2 memory-harden password hashing |
| RFC 8032 | Ed25519 signatures |
| RFC 2104 | HMAC keyed-hash message authentication |
| NIST FIPS 180-4 | SHA-2 hash functions |
| SEC 2 | secp256k1 curve parameters |
