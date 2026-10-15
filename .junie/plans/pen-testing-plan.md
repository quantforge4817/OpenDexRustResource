---
sessionId: session-261015-170950-j7eq
---

# Attack Surface Analysis

### OpenDEX Attack Surface

OpenDEX is a P2P trading/exchange application with the following key components that form the attack surface:

**Network Layer** (`crates/p2p-node/src/network.rs`)
- libp2p-based P2P network with Kademlia (peer discovery), Gossipsub (message pub/sub), Ping, and Identify protocols
- TCP transport with Noise encryption protocol (`noise::Config::new`)
- Yamux multiplexing
- Listens on configurable TCP port
- Accepts messages from any peer on the network

**Protocol Messages** (`crates/protocol/src/message.rs`)
- 12 message types: `PeerAnnouncement`, `OrderCreated`, `OrderCancelled`, `TradeCreated`, `TradeStateChanged`, `StateRequest`, `StateResponse`, `P2POfferCreated`, `P2POfferCancelled`, `P2PTradeCreated`, `P2PTradeStateChanged`
- Messages contain: `sender` (String), `payload` (JSON String), `timestamp` (u64), `schema_version` (u32)
- No cryptographic signature on messages themselves — relies on P2P network identity

**Authentication** (`crates/user/src/auth.rs`)
- HMAC-SHA256 signed tokens with 24-hour TTL
- Argon2id password hashing with random salt (`crates/user/src/user.rs`)
- Sliding window rate limiter for login attempts (5 per minute per username)
- Token format: `{user_id}:{random_uuid}:{expires_ts}:{hmac_signature}`
- Signing key: 32 bytes from `getrandom::getrandom()`

**Financial Core** (`crates/p2p-node/src/`)
- Orderbook (`orderbook.rs`): trade matching, validation, idempotency
- Balances (`balances.rs`): three-state model (available, locked, escrowed)
- Settlement (`settlement.rs`): plan-then-commit pattern
- Escrow (`escrow.rs`, `p2p_escrow.rs`): fund holding during trades
- Trade state machine (`trade_state.rs`): 13 states from Created to Settled/Cancelled/Disputed

**Fiat Integration** (`crates/fiat/src/`)
- Bitcoin 2-of-2 multisig escrow (`multisig.rs`): secp256k1 key pairs, P2WSH redemption scripts, partial signature co-signing
- Crypto wallet adapters (`crypto_wallet.rs`): BTC and XMR wallet integration
- Payment adapters
- Bond system
- Dispute resolution

**Data Storage** (`crates/p2p-node/src/persistence.rs`)
- Filesystem-based persistence for orders, trades, and state snapshots

**Cryptography Inventory**
- `hmac` 0.12 + `sha2` 0.10 — HMAC-SHA256 for auth token signatures
- `argon2` 0.5 — Argon2id password hashing with `password-hash` 0.5
- `bitcoin` 0.32 + `secp256k1` 0.29 — Bitcoin ECDSA signatures, multisig scripts
- `libp2p` Noise protocol — transport-layer encryption for P2P communications
- `getrandom` 0.2 — cryptographically secure random number generation for signing keys
- `rand_core` 0.6 — OS-backed RNG for Argon2 salt generation
- `zeroize` 1 — secret key memory zeroization on drop (`EscrowKeyPair`, `MultisigManager`)
- `sha2` (via `libp2p_transport`) — SHA-256 hashing for PeerId-to-ParticipantId conversion


# Penetration Testing Plan

### ✓ Step 1: Confirm and implement penetration testing plan with cryptography focus
### ✓ Step 2: Create cryptography penetration tests (Phase 2.5) — 50 tests all passing
### ✓ Step 3: Update audit documentation with crypto test results
### ✓ Step 4: Run full test suite to verify no regressions — all tests pass (load_tests pre-existing failures unrelated)
### ✓ Step 5: Run Miri undefined behavior detection — 59 tests pass, 0 UB (filesystem/FFI/socket/crypto blocked by Miri limitations)
### ✓ Step 6: Implement Phase 1 Network Protocol Attacks tests — 24 tests all passing
### ✓ Step 7: Implement Phase 2 Authentication Attacks tests — 21 tests all passing
### ✓ Step 8: Implement Phase 3 Financial Logic Attacks tests — 16 tests all passing
### ✓ Step 9: Implement Phase 4-7 tests (Data Integrity, Fiat, DoS, Supply Chain) — 19 tests all passing
### ✓ Step 10: Run full test suite and update audit documentation — 130 pen tests + 59 Miri + 11 Kani = 200+ total, all passing
### ✓ Step 11: Fix pen testing findings — rate limiter DoS protection & zero amount/price validation
### ✓ Step 12: Update all audit documentation and push to Git — commit 89ea462 pushed to origin/main
### Pen Testing Plan for OpenDEX

This plan covers systematic security testing across all attack surfaces. Tests are organized by attack vector, from external network attacks to internal logic vulnerabilities.

---

### Phase 1: Network Protocol Attacks

**Goal:** Test the P2P network layer for message-based attacks

#### 1.1 Malformed Message Injection
- Send messages with invalid JSON payloads via Gossipsub
- Send messages with extreme values (max u64, zero amounts, negative-like values)
- Send messages with schema_version = 0 or schema_version = MAX_INT (version bypass)
- Send messages with empty sender field
- Send messages with payload size > 1MB (buffer overflow / DoS)

**Target:** `crates/p2p-node/src/network.rs`, `crates/p2p-node/src/message_handler.rs`

#### 1.2 Message Replay Attacks
- Capture a valid `TradeCreated` message and replay it to test idempotency
- Replay `OrderCancelled` messages for already-cancelled orders
- Replay `TradeStateChanged` messages with stale state transitions
- Replay `StateResponse` messages with older snapshots to cause state regression

**Target:** `crates/p2p-node/src/orderbook.rs` (idempotency checks), `crates/p2p-node/src/sync.rs` (monotonic snapshot enforcement)

#### 1.3 Race Condition Attacks
- Send conflicting `TradeCreated` messages simultaneously from multiple fake peers
- Send `OrderCancelled` while `TradeCreated` is being processed for the same order
- Rapidly alternate `TradeStateChanged` messages to test state machine consistency
- Send `StateRequest` floods to test sync handling under load

**Target:** `crates/p2p-node/src/trade_state.rs`, `crates/p2p-node/src/message_handler.rs`

#### 1.4 Peer Identity Spoofing
- Send messages claiming to be from known peer IDs
- Send `PeerAnnouncement` messages with conflicting identities
- Test if the `Identify` protocol validation can be bypassed
- Send messages with sender field different from the libp2p PeerId

**Target:** `crates/p2p-node/src/network.rs` (identify behavior), `crates/p2p-node/src/message_handler.rs`

---

### Phase 2: Authentication Attacks

**Goal:** Test the user authentication system for credential and token vulnerabilities

#### 2.1 Brute Force / Credential Stuffing
- Test rate limiter effectiveness: send 100+ login attempts per second
- Test rate limiter with distributed IPs (if applicable)
- Test password reset flow for rate limiting
- Test with common password lists against the Argon2 hashing

**Target:** `crates/user/src/auth.rs` (RateLimiter)

#### 2.2 Token Manipulation
- Modify HMAC token payload (change user_id, extend expiration)
- Test token validation with expired tokens
- Test token validation with malformed signatures
- Test token reuse after expected invalidation
- Test token extraction from logs (verify log redaction works)

**Target:** `crates/user/src/auth.rs` (AuthToken::verify)

#### 2.3 Session Management
- Test concurrent session handling (multiple tokens for same user)
- Test token expiration enforcement (24-hour TTL)
- Test behavior when signing key is rotated

**Target:** `crates/user/src/auth.rs`

---

### Phase 2.5: Cryptography-Specific Attacks

**Goal:** Test all cryptographic primitives, key management, and crypto-dependent flows for vulnerabilities.

This phase has special focus as requested — it covers every cryptographic component in the codebase.

#### 2.5.1 HMAC-SHA256 Token Security (`crates/user/src/auth.rs`)

**Token Structure Analysis:**
- Token format: `{user_id.0}:{random_uuid}:{expires_ts}:{hmac_hex_signature}`
- Signature computed over `user_id:random_uuid:expires_ts` using HMAC-SHA256
- Signing key: 32 bytes from `getrandom::getrandom()`

**Attack Vectors:**
- **Key reuse across instances**: Verify that `AuthService::new()` generates a unique signing key each time. Test that tokens from one instance are rejected by another.
- **Key length validation**: Test `HmacSha256::new_from_slice()` with edge case key sizes (empty, 1 byte, 32 bytes, 256 bytes). The code uses `.expect("HMAC can accept any key size")` — verify this is correct for HMAC-SHA256.
- **Signature malleability**: Test if modified signatures (flipping bits in hex) are rejected. Test uppercase vs lowercase hex decoding in `hex_decode()`.
- **Token parsing edge cases**: The `verify()` method uses `rsplitn(2, ':')` — test tokens with colons in the payload to ensure correct split behavior. Payload contains UUIDs which don't have colons, but test with injected colons.
- **Empty signature bypass**: Test tokens with empty signature hex (should decode to empty bytes, should fail verification).
- **Hex decoding edge cases**: Test `hex_decode()` with odd-length strings, non-hex characters, empty strings, and mixed case.
- **Timing side-channel**: Verify that `mac.verify_slice()` uses constant-time comparison (check the `hmac` crate implementation).
- **Clock manipulation**: Test token verification when `now` is set to past/future times. Test behavior at exact expiration boundary.

**Target:** `crates/user/src/auth.rs` (AuthToken::new, AuthToken::verify, hex_encode, hex_decode)

#### 2.5.2 Argon2id Password Hashing (`crates/user/src/user.rs`)

**Hashing Configuration:**
- Algorithm: Argon2id (hybrid of Argon2i and Argon2d)
- Version: V0x13
- Params: `Params::default()` — **verify default parameters meet security requirements**
- Salt: Generated via `password_hash::SaltString::generate(&mut rand_core::OsRng)`

**Attack Vectors:**
- **Default parameters security**: Check `argon2::Params::default()` values. Verify memory cost, time cost, and parallelism meet OWASP recommendations (minimum 64MB memory, 3 iterations, 1 parallelism for Argon2id).
- **Salt uniqueness**: Verify that two hashes of the same password produce different outputs (different salts).
- **Salt entropy**: Verify `rand_core::OsRng` provides sufficient entropy for salt generation.
- **Hash format integrity**: Test `PasswordHash::verify()` with corrupted hash strings (truncated, modified algorithm identifier, invalid base64).
- **Algorithm downgrade**: Test if a hash with `argon2i` or `argon2d` algorithm identifier is accepted by the verifier (should it be rejected?).
- **Version downgrade**: Test if a hash with an older version identifier is accepted.
- **Empty password handling**: Test hashing and verification with empty passwords (should be rejected by length check).
- **Unicode password handling**: Test passwords with Unicode characters, emoji, and zero-width characters.
- **Timing side-channel**: Verify that `argon2::verify_password()` uses constant-time comparison.
- **Hash collision**: Test if two different passwords can produce the same hash (should be computationally infeasible, but verify the implementation).

**Target:** `crates/user/src/user.rs` (PasswordHash::from_password, PasswordHash::verify)

#### 2.5.3 Bitcoin Multisig Escrow (`crates/fiat/src/multisig.rs`)

**Cryptographic Components:**
- Key generation: `Secp256k1::<All>::new()` with `secp.generate_keypair(&mut thread_rng())`
- Redemption script: `OP_2 <pub1> <pub2> OP_2 OP_CHECKMULTISIG`
- P2WSH: Pay-to-Witness-Script-Hash using `sha256::Hash::hash(witness_script.as_bytes())`
- Signatures: ECDSA with `secp.sign_ecdsa(&message, &key.secret)`
- Sighash: `EcdsaSighashType::All` via `SighashCache::p2wsh_signature_hash()`
- Secret key cleanup: `zeroize::Zeroize` on `Drop`

**Attack Vectors:**
- **Key generation entropy**: Verify `bitcoin::secp256k1::rand::thread_rng()` provides sufficient entropy. Test that repeated key generation produces unique keys.
- **Key derivation from hex**: Test `EscrowKeyPair::from_secret_hex()` with invalid hex, short hex, long hex, and all-zero keys. Verify error handling.
- **Public key consistency**: Verify that `from_secret_hex()` produces the same public key as the original key pair.
- **Redemption script determinism**: Verify that the same public keys always produce the same redemption script hash and address.
- **Script hash collision**: Test if different witness scripts can produce the same SHA-256 hash (should be infeasible, but verify the hash function is used correctly).
- **Signature malleability**: Test if modified ECDSA signatures (DER encoding variations) are accepted. Test low-S vs high-S signature normalization.
- **Sighash type manipulation**: Verify that `EcdsaSighashType::All` is enforced. Test if a signature with a different sighash type can be combined.
- **Partial signature validation**: Test `combine_signatures()` with signatures from wrong keys, wrong transactions, or wrong trade IDs.
- **Witness stack construction**: Verify the witness stack order (empty push, maker sig, taker sig, witness script). Test with reversed order.
- **Network mismatch**: Test if addresses from one network (testnet) are accepted for transactions on another network (mainnet). The `require_network()` check should catch this.
- **Zero-amount escrow**: Test escrow with 0 satoshis. Test with max u64 satoshis.
- **Miner fee edge cases**: Test fee exactly equal to amount (taker gets 0). Test fee exceeding amount (should be rejected).
- **Secret key zeroization**: Verify `Drop` implementations for `EscrowKeyPair` and `MultisigManager` actually zeroize memory. Check if `Clone` bypasses zeroization.
- **Key persistence**: Test if secret keys are serialized to JSON (they should not be — only public keys should be serialized).
- **Transaction version**: Verify `Transaction::version` is set to `Version(2)` for BIP-68 compliance.
- **RBF flag**: Verify `Sequence::ENABLE_RBF_NO_LOCKTIME` is intentional and doesn't allow fee sniping.

**Target:** `crates/fiat/src/multisig.rs` (EscrowKeyPair, RedemptionScript, MultisigEscrow, MultisigManager)

#### 2.5.4 Transport Layer Encryption (`crates/p2p-node/src/network.rs`)

**Cryptographic Components:**
- Noise protocol: `noise::Config::new()` — provides authenticated encryption
- Identity keys: `identity::Keypair` — used for peer authentication
- Gossipsub: `MessageAuthenticity::Signed(key.clone())` — message signing

**Attack Vectors:**
- **Noise protocol configuration**: Verify `noise::Config::new()` uses a secure handshake pattern (XX, IK, or KK). Check the default pattern.
- **Key pair generation**: Verify `identity::Keypair` uses Ed25519 or X25519 (check the libp2p default).
- **Message authenticity**: Verify `MessageAuthenticity::Signed` prevents message forgery. Test if unsigned messages are rejected.
- **Peer ID derivation**: Verify peer IDs are derived from public keys, not chosen by peers.
- **Replay protection**: Test if the Noise protocol provides replay protection for connections.
- **Forward secrecy**: Verify if the Noise pattern provides forward secrecy (ephemeral keys per session).

**Target:** `crates/p2p-node/src/network.rs` (build_swarm, noise::Config)

#### 2.5.5 PeerId-to-ParticipantId Hashing (`crates/opendex-transport/src/libp2p_transport.rs`)

**Cryptographic Components:**
- SHA-256 via `sha2::Sha256` for hashing PeerId strings to UUIDs
- Only first 16 bytes of SHA-256 output used (truncation)

**Attack Vectors:**
- **Truncation collision risk**: Only 128 bits of SHA-256 are used (16 bytes). Test if this is sufficient for collision resistance in the expected peer count.
- **Deterministic mapping**: Verify the same PeerId always produces the same ParticipantId.
- **Hash quality**: Verify SHA-256 output has good distribution (no bias in first 16 bytes).

**Target:** `crates/opendex-transport/src/libp2p_transport.rs` (peer_id_to_participant)

#### 2.5.6 Random Number Generation

**Cryptographic Components:**
- `getrandom::getrandom()` — signing key generation in `AuthService::new()`
- `rand_core::OsRng` — Argon2 salt generation
- `bitcoin::secp256k1::rand::thread_rng()` — Bitcoin key pair generation

**Attack Vectors:**
- **RNG failure handling**: Test behavior when `getrandom` fails (returns error). The code uses `.expect()` — verify this is acceptable.
- **Entropy source**: Verify `getrandom` and `OsRng` use OS-provided entropy (`/dev/urandom` on Linux, `CryptGenRandom`/`BCryptGenRandom` on Windows).
- **Thread RNG seeding**: Verify `thread_rng()` is properly seeded per-thread and doesn't produce predictable values.
- **UUID v4 randomness**: Verify `Uuid::new_v4()` uses cryptographically secure random bytes.

**Target:** All modules using RNG

#### 2.5.7 Cryptographic Dependency Audit

**Attack Vectors:**
- **Version vulnerabilities**: Check all crypto dependency versions against known CVEs:
  - `hmac` 0.12 — check for known issues
  - `sha2` 0.10 — check for known issues
  - `argon2` 0.5 — check for known issues
  - `bitcoin` 0.32 — check for known issues
  - `secp256k1` 0.29 — check for known issues
  - `zeroize` 1 — check for known issues
- **Supply chain**: Verify crypto dependencies are from trusted sources (RustCrypto, bitcoin-dev).
- **Feature flags**: Verify no debug or test features are enabled in crypto dependencies.

**Target:** `Cargo.lock`, `deny.toml`

---

### Phase 3: Financial Logic Attacks

**Goal:** Test the orderbook, balances, settlement, and escrow for economic vulnerabilities

#### 3.1 Balance Manipulation
- Attempt to create trades with zero amount (should be rejected)
- Attempt to create trades with zero price (should be rejected)
- Attempt to spend locked balances
- Attempt to spend escrowed balances
- Test balance overflow with max u64 values
- Test balance underflow with cancellation after partial fill

**Target:** `crates/p2p-node/src/balances.rs`, `crates/p2p-node/src/orderbook.rs`

#### 3.2 Trade State Machine Attacks
- Force invalid state transitions (e.g., Settled → Created)
- Skip states (e.g., Created → Settled, bypassing escrow)
- Test dispute resolution from all possible states
- Test expiration from mid-trade states
- Test concurrent state changes from multiple participants

**Target:** `crates/p2p-node/src/trade_state.rs`, `crates/protocol/src/message.rs` (TradeState::can_transition_to)

#### 3.3 Settlement Attacks
- Test double-spending: attempt to settle the same trade twice
- Test settlement with insufficient funds after lock
- Test settlement idempotency with replayed settlement messages
- Test escrow release without proper trade completion
- Test dispute fund distribution logic

**Target:** `crates/p2p-node/src/settlement.rs`, `crates/p2p-node/src/escrow.rs`

#### 3.4 Order Manipulation
- Create orders with extreme prices (very high/very low)
- Test order cancellation balance unlock (AUDIT-H02 was fixed)
- Test order resurrection after cancellation (tombstone check)
- Test partial fill balance handling
- Test market manipulation via order flooding

**Target:** `crates/p2p-node/src/orderbook.rs`

---

### Phase 4: Data Integrity Attacks

**Goal:** Test persistence, synchronization, and data validation

#### 4.1 Persistence Corruption
- Corrupt saved orderbook state files
- Inject malformed JSON into persisted trade data
- Test recovery from partial writes
- Test concurrent file access scenarios

**Target:** `crates/p2p-node/src/persistence.rs`

#### 4.2 State Synchronization Attacks
- Send `StateResponse` with conflicting state data
- Send `StateResponse` with older timestamps (stale state injection)
- Test sync recovery after network partition simulation
- Test state consistency after forced node restart during sync

**Target:** `crates/p2p-node/src/sync.rs`

---

### Phase 5: Fiat Integration Attacks

**Goal:** Test the Bitcoin multisig escrow and payment adapter layer

#### 5.1 Escrow Attacks
- Test multisig address generation with edge cases
- Test escrow funding with insufficient Bitcoin balance
- Test escrow release with invalid signatures
- Test double-spend protection in escrow
- Test escrow timeout and refund logic

**Target:** `crates/fiat/src/` (escrow modules)

#### 5.2 Payment Adapter Attacks
- Test payment confirmation with fake transaction IDs
- Test payment amount validation (less than required)
- Test payment from unexpected addresses
- Test payment retry and idempotency

**Target:** `crates/fiat/src/` (payment adapters)

---

### Phase 6: Denial of Service

**Goal:** Test resilience against resource exhaustion attacks

#### 6.1 Network DoS
- Flood Gossipsub with valid but high-volume messages
- Send max-size messages to exhaust memory
- Rapid peer connect/disconnect cycles
- Send messages that trigger expensive operations (e.g., large state syncs)

**Target:** `crates/p2p-node/src/network.rs`

#### 6.2 Resource Exhaustion
- Create many pending trades to exhaust memory
- Fill the orderbook with orders to test lookup performance
- Test rate limiter memory growth with many unique keys
- Test log file growth under high message volume

**Target:** All p2p-node modules

---

### Phase 7: Supply Chain & Dependency Attacks

**Goal:** Verify the existing supply chain protections are effective

#### 7.1 Dependency Verification
- Verify `cargo-deny` catches known vulnerable dependencies
- Test behavior if a dependency is replaced with a malicious version
- Verify no git dependencies are allowed (enforced by deny.toml)
- Check for typosquatting vulnerabilities in dependency names

**Target:** `deny.toml`, `Cargo.lock`

---

### Testing Methods

| Method | Tool | Scope |
|--------|------|-------|
| **Fuzzing** | `cargo-fuzz` | Message deserialization, trade validation, state transitions, auth token parsing |
| **Unit tests** | `cargo test` | Edge cases, boundary conditions, error paths, crypto parameter validation |
| **Integration tests** | Custom test harness | Full message flow, multi-peer scenarios |
| **Manual testing** | Custom P2P client | Protocol manipulation, replay attacks |
| **Static analysis** | Clippy, Miri | UB detection, code quality |
| **Dependency audit** | `cargo-deny`, `cargo-audit` | Supply chain security, crypto dependency CVEs |
| **Crypto audit** | Manual code review | HMAC implementation, Argon2 parameters, secp256k1 usage, key management |
| **Side-channel analysis** | Timing tests | Constant-time verification for HMAC and Argon2 comparisons |

---

### Priority Order

1. **Phase 2.5** (Cryptography) — foundational security, all other phases depend on correct crypto
2. **Phase 3** (Financial Logic) — highest impact, direct economic loss
3. **Phase 1** (Network Protocol) — primary attack surface, external access
4. **Phase 2** (Authentication) — credential and session security (builds on crypto)
5. **Phase 4** (Data Integrity) — state consistency and recovery
6. **Phase 5** (Fiat Integration) — external blockchain interaction
7. **Phase 6** (DoS) — availability concerns
8. **Phase 7** (Supply Chain) — ongoing maintenance

---

### Existing Protections to Verify

From the audit, these protections should be verified:
- ✅ Deterministic trade IDs prevent identity spoofing
- ✅ Idempotent trade application prevents duplicate mutation
- ✅ Pending trade mechanism handles out-of-order delivery
- ✅ Order tombstones prevent resurrection
- ✅ Monotonic snapshot synchronization prevents stale state
- ✅ Authenticated sender binding prevents impersonation
- ✅ Trade participant authorization restricts state changes
- ✅ Settlement idempotency prevents double-spending
- ✅ Checked arithmetic prevents overflow
- ✅ Plan-then-commit pattern ensures atomicity
- ✅ Three-state balance model with invariant enforcement
- ✅ Log redaction prevents sensitive data leakage

### Cryptography-Specific Protections to Verify

- ✅ HMAC-SHA256 token signatures prevent token forgery
- ✅ Argon2id with random salt prevents rainbow table attacks
- ✅ `getrandom` for signing key generation ensures key uniqueness
- ✅ `zeroize` on `EscrowKeyPair` and `MultisigManager` prevents secret key leakage in memory
- ✅ Noise protocol encryption provides transport-layer confidentiality
- ✅ Gossipsub `MessageAuthenticity::Signed` prevents message forgery
- ✅ secp256k1 ECDSA signatures for Bitcoin multisig escrow
- ✅ P2WSH redemption script requires 2-of-2 signatures (no unilateral spending)
- ✅ `require_network()` check prevents cross-network address misuse
- ✅ SHA-256 for script hash computation (collision-resistant)
