# Crypto Defense Validation Scenarios

These scenarios validate that cryptographic defenses correctly detect and mitigate crypto-level attacks. Each scenario corresponds to a red team attack in [redteam/scenarios/crypto.md](../../redteam/scenarios/crypto.md).

## Monitored Code Paths

| File | Function | Defense Role |
|------|----------|--------------|
| `user/src/auth.rs` | `AuthService` | HMAC-SHA256 token signing and validation |
| `user/src/auth.rs` | `RateLimiter` | Sliding-window rate limiting |
| `fiat/src/` | Various | Cryptographic operations in fiat trades |

---

## BD-009: Token Single-Use

**Corresponds to:** CA-001: HMAC Token Replay  
**Severity:** High  
**Description:** Verify that auth tokens are rejected after first use.

### Defense Validation
1. Monitor authentication logs while red team executes CA-001
2. Track token usage across nodes
3. Verify only the first use of a token succeeds

### Expected Defense Behavior
- Token is bound to session and/or IP address
- First use of the token succeeds
- Subsequent replays are rejected immediately
- Rate limiter activates after multiple failures

### Validation Criteria
- [ ] Token rejected after first use (only 1 success)
- [ ] Replay attempts are logged with warning/error level
- [ ] Rate limiter activates after threshold
- [ ] No unauthorized access granted via replayed token

### Detection Metrics
- `opendex_auth_success_total` — should show only 1 success per token
- `opendex_auth_failure_total` — should increase with each replay
- `opendex_rate_limiter_activations_total` — should increase

### Blue Team Agent Implementation
`blueteam/agent/src/crypto_defenses.rs::validate_token_single_use()`

---

## BD-010: Constant-Time Comparison

**Corresponds to:** CA-002: Token Timing Analysis  
**Severity:** Medium  
**Description:** Verify that token comparison uses constant-time to prevent timing side-channels.

### Defense Validation
1. Monitor response times while red team executes CA-002
2. Measure timing variance across 100 token validation requests
3. Calculate standard deviation of response times

### Expected Defense Behavior
- Constant-time comparison is used for all token validations
- Timing variance is < 1ms across all requests
- No correlation between token content and response time

### Validation Criteria
- [ ] Timing variance < 1000μs (1ms)
- [ ] No timing side-channel detected
- [ ] Response times are consistent across all token types

### Detection Metrics
- Response time standard deviation should be < 1ms
- No pattern in response times correlated with token correctness

### Blue Team Agent Implementation
`blueteam/agent/src/crypto_defenses.rs::validate_constant_time_comparison()`

---

## BD-011: Rate Limiter Activation

**Corresponds to:** CA-003: Key Extraction Attempt  
**Severity:** High  
**Description:** Verify that rate limiter blocks brute-force attempts.

### Defense Validation
1. Monitor rate limiter state while red team executes CA-003
2. Count attempts before rate limiter activates
3. Verify the attacker is blocked

### Expected Defense Behavior
- Rate limiter blocks after ~10 failed attempts
- Strong random keys prevent brute-force extraction
- Account is locked or delayed after threshold

### Validation Criteria
- [ ] Rate limiter activates within 10 attempts
- [ ] No key extracted by attacker
- [ ] Account protection mechanisms work correctly
- [ ] Blocked attempts are logged

### Detection Metrics
- `opendex_rate_limiter_activations_total` — should increase
- `opendex_auth_failure_total` — should show spike then drop (blocked)
- Time to block should be < 30 seconds

### Blue Team Agent Implementation
`blueteam/agent/src/crypto_defenses.rs::validate_rate_limiter_activation()`

---

## BD-012: Signature Strictness

**Corresponds to:** CA-004: Signature Malleability  
**Severity:** High  
**Description:** Verify that all modified signatures are rejected.

### Defense Validation
1. Monitor signature verification while red team executes CA-004
2. Track acceptance/rejection of each of the 64 signature variations
3. Verify zero acceptances of modified signatures

### Expected Defense Behavior
- All modified signatures are rejected
- Signature verification is strict with no tolerance for byte changes
- No acceptance of malleated signatures

### Validation Criteria
- [ ] All 64 variations rejected (0 acceptances)
- [ ] No signature accepted with modified bytes
- [ ] Verification errors are logged for each rejection

### Detection Metrics
- `opendex_signature_verification_failure_total` — should increase to 64
- `opendex_signature_verification_success_total` — should not increase from malleated signatures

### Blue Team Agent Implementation
`blueteam/agent/src/crypto_defenses.rs::validate_signature_strictness()`

---

## BD-013: Key Generation Quality

**Corresponds to:** CA-005: Key Uniqueness Test  
**Severity:** Medium  
**Description:** Verify that all generated keys are unique and use CSPRNG.

### Defense Validation
1. Monitor key generation while red team executes CA-005
2. Collect all 100 generated keys
3. Check for duplicates and analyze randomness

### Expected Defense Behavior
- All generated keys are unique
- No key reuse across instances
- Cryptographically secure random number generator is used

### Validation Criteria
- [ ] All 100 keys are distinct (0 duplicates)
- [ ] No duplicate keys found
- [ ] Key generation uses cryptographically secure RNG

### Detection Metrics
- Unique key count should equal total key count (100)
- Key entropy should be high (no patterns)

### Blue Team Agent Implementation
`blueteam/agent/src/crypto_defenses.rs::validate_key_generation_quality()`

---

## BD-014: Password Hash Strength

**Corresponds to:** CA-006: Argon2id Bypass  
**Severity:** High  
**Description:** Verify that Argon2id hashing is collision-resistant with proper parameters.

### Defense Validation
1. Monitor hash generation while red team executes CA-006
2. Check for collisions among 1000 generated hashes
3. Verify hashing parameters meet security guidelines

### Expected Defense Behavior
- No hash collisions found among 1000 hashes
- Memory-hard hashing prevents rapid computation
- Proper Argon2id parameters are used (memory, iterations, parallelism)

### Validation Criteria
- [ ] No hash collisions (0 collisions among 1000 hashes)
- [ ] Hashing takes expected time (~100ms per hash)
- [ ] Parameters meet security guidelines

### Detection Metrics
- Collision count should be 0
- Average hash time should be ~100ms

### Blue Team Agent Implementation
`blueteam/agent/src/crypto_defenses.rs::validate_password_hash_strength()`

---

## BD-015: Proof Verification

**Corresponds to:** CA-007: DLEQ Proof Forgery  
**Severity:** Critical  
**Description:** Verify that all forged DLEQ proofs are rejected.

### Defense Validation
1. Monitor DLEQ proof verification while red team executes CA-007
2. Track acceptance/rejection of forged proofs
3. Verify zero false positive acceptances

### Expected Defense Behavior
- All forged proofs are rejected
- Proof verification is strict
- No acceptance of invalid proofs with zero, random, or modified challenges

### Validation Criteria
- [ ] All forged proofs rejected (0 acceptances)
- [ ] Verification errors are logged
- [ ] No false positive acceptances

### Detection Metrics
- `opendex_dleq_verification_failure_total` — should increase with each forged proof
- `opendex_dleq_verification_success_total` — should not increase from forged proofs

### Blue Team Agent Implementation
`blueteam/agent/src/crypto_defenses.rs::validate_proof_verification()`