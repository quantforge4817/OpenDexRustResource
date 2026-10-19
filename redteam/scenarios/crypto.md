# Cryptographic Attack Scenarios

These scenarios test cryptographic defenses including HMAC token validation, key management, signature verification, and password hashing.

## Target Code Paths

| File | Function | Role |
|------|----------|------|
| `user/src/auth.rs` | `AuthService` | Registration, login, HMAC-SHA256 token signing |
| `user/src/auth.rs` | `RateLimiter` | Sliding-window rate limiting with max-key eviction |
| `fiat/src/` | Various | Cryptographic operations in fiat trades |

## CA-001: HMAC Token Replay

**Severity:** High  
**Description:** Capture and replay an auth token across nodes.

### Attack Vector
1. Capture a valid HMAC token from network traffic
2. Replay the token 10 times across different nodes
3. Monitor authentication responses

### Expected Behavior
- Token is bound to session and/or IP address
- First use succeeds
- Subsequent replays are rejected
- Rate limiter activates after multiple failures

### Validation Criteria
- [ ] Token rejected after first use
- [ ] Replay attempts are logged
- [ ] Rate limiter activates
- [ ] No unauthorized access granted

### Implementation
`redteam/agent/src/crypto_attacks.rs::hmac_token_replay()`

---

## CA-002: Token Timing Analysis

**Severity:** Medium  
**Description:** Measure response time for token validation to detect timing side-channels.

### Attack Vector
1. Send 100 token validation requests with varying tokens
2. Measure response time for each request
3. Calculate timing variance

### Expected Behavior
- Constant-time comparison is used
- Timing variance is < 1ms
- No correlation between token content and response time

### Validation Criteria
- [ ] Timing variance < 1000μs
- [ ] No timing side-channel detected
- [ ] Response times are consistent

### Implementation
`redteam/agent/src/crypto_attacks.rs::token_timing_analysis()`

---

## CA-003: Key Extraction Attempt

**Severity:** High  
**Description:** Attempt to brute-force HMAC secret.

### Attack Vector
1. Send 1000 authentication attempts with random tokens
2. Monitor for rate limiting activation
3. Measure time to block

### Expected Behavior
- Rate limiter blocks after ~10 attempts
- Strong random keys prevent brute-force
- Account is locked or delayed after threshold

### Validation Criteria
- [ ] Rate limiter activates within 10 attempts
- [ ] No key extracted
- [ ] Account protection mechanisms work

### Implementation
`redteam/agent/src/crypto_attacks.rs::key_extraction_attempt()`

---

## CA-004: Signature Malleability

**Severity:** High  
**Description:** Modify signature bytes to test verification robustness.

### Attack Vector
1. Create 64 variations of a signature, flipping one byte each
2. Submit each variation for verification
3. Monitor verification results

### Expected Behavior
- All modified signatures are rejected
- Signature verification is strict
- No acceptance of malleated signatures

### Validation Criteria
- [ ] All 64 variations rejected
- [ ] No signature accepted with modified bytes
- [ ] Verification errors are logged

### Implementation
`redteam/agent/src/crypto_attacks.rs::signature_malleability()`

---

## CA-005: Key Uniqueness Test

**Severity:** Medium  
**Description:** Verify no key reuse across instances.

### Attack Vector
1. Generate 100 keys across different node instances
2. Collect all generated keys
3. Check for duplicates

### Expected Behavior
- All keys are unique
- No key reuse across instances
- Proper random number generator is used

### Validation Criteria
- [ ] All 100 keys are distinct
- [ ] No duplicate keys found
- [ ] Key generation uses cryptographically secure RNG

### Implementation
`redteam/agent/src/crypto_attacks.rs::key_uniqueness_test()`

---

## CA-006: Argon2id Bypass

**Severity:** High  
**Description:** Attempt password hash collision.

### Attack Vector
1. Generate 1000 Argon2id hashes for different passwords
2. Check for collisions
3. Attempt to find weak parameters

### Expected Behavior
- No collisions found
- Memory-hard hashing prevents rapid computation
- Proper Argon2id parameters are used

### Validation Criteria
- [ ] No hash collisions
- [ ] Hashing takes expected time (~100ms per hash)
- [ ] Parameters meet security guidelines

### Implementation
`redteam/agent/src/crypto_attacks.rs::argon2id_bypass()`

---

## CA-007: DLEQ Proof Forgery

**Severity:** Critical  
**Description:** Construct invalid DLEQ proofs.

### Attack Vector
1. Create DLEQ proofs with:
   - Zero challenge
   - Random challenge
   - Modified response
2. Submit for verification
3. Monitor verification results

### Expected Behavior
- All forged proofs are rejected
- Proof verification is strict
- No acceptance of invalid proofs

### Validation Criteria
- [ ] All forged proofs rejected
- [ ] Verification errors are logged
- [ ] No false positive acceptances

### Implementation
`redteam/agent/src/crypto_attacks.rs::dleq_proof_forgery()`
