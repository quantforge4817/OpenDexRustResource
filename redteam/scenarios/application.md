# Application-Level Attack Scenarios

These scenarios test application-level defenses including state machine exploitation, balance manipulation, and MessageStore replay bypass.

## Target Code Paths

| File | Function | Role |
|------|----------|------|
| `opendex-transport/src/message_store.rs` | `MessageStore` | Replay protection |
| `p2p-node/src/trade_state.rs` | Trade state management | State machine transitions |
| `p2p-node/src/message_handler.rs` | Message dispatch | Application logic |

## TA-001: Envelope Tampering

**Severity:** High  
**Description:** Modify `ProtocolEnvelope` fields to test validation.

### Attack Vector
1. Capture a valid `ProtocolEnvelope`
2. Modify fields:
   - Change `message_id` to a different UUID
   - Modify `sequence` number
   - Alter `timestamp`
3. Submit tampered envelopes

### Expected Behavior
- Envelope validation detects modifications
- Tampered envelopes are rejected
- Integrity checks fail for modified fields

### Validation Criteria
- [ ] All tampered envelopes rejected
- [ ] Validation errors are logged
- [ ] No state changes from tampered messages

### Implementation
`redteam/agent/src/protocol_attacks.rs::replay_attack()` (envelope variant)

---

## TA-002: Sequence Manipulation

**Severity:** Medium  
**Description:** Manipulate message sequence numbers.

### Attack Vector
1. Send messages with manipulated sequences:
   - Duplicate sequence numbers
   - Negative sequence numbers
   - Very large sequence numbers (near u64::MAX)
2. Monitor sequence validation

### Expected Behavior
- Sequence validation rejects invalid sequences
- Duplicate sequences are detected
- No state corruption from invalid sequences

### Validation Criteria
- [ ] Invalid sequences rejected
- [ ] No duplicate processing
- [ ] Sequence validation is robust

### Implementation
`redteam/agent/src/protocol_attacks.rs::sequence_manipulation()`

---

## TA-003: MessageStore Bypass

**Severity:** High  
**Description:** Attempt to bypass replay protection.

### Attack Vector
1. Send a valid message with `message_id: "test-001"`
2. Send the same message with modified envelope but same `message_id`
3. Attempt to use different swap IDs to bypass per-swap tracking

### Expected Behavior
- `MessageStore` tracks message IDs per swap
- Duplicate `message_id` is rejected regardless of envelope changes
- No bypass of replay protection

### Validation Criteria
- [ ] Replay protection cannot be bypassed
- [ ] Message ID tracking works correctly
- [ ] Per-swap isolation is enforced

### Implementation
`redteam/agent/src/protocol_attacks.rs::replay_attack()` (MessageStore variant)

---

## TA-004: Transport Layer Fuzzing

**Severity:** Medium  
**Description:** Fuzz raw TCP/UDP transport layer.

### Attack Vector
1. Send random byte sequences to libp2p ports
2. Send truncated messages
3. Send messages with invalid framing

### Expected Behavior
- Robust parsing handles invalid input
- No crashes or panics
- Connections are closed gracefully

### Validation Criteria
- [ ] No crashes from fuzzed input
- [ ] Invalid connections handled gracefully
- [ ] Node remains stable

### Implementation
Manual testing with custom fuzzing tools

---

## DA-002: State Bloat

**Severity:** Medium  
**Description:** Submit MAX_PENDING_TRADE_STATES + 1 states.

### Attack Vector
1. Submit 110 trade states (assuming limit is 100)
2. Monitor state acceptance and rejection

### Expected Behavior
- State limit is enforced
- Extra states are rejected
- No memory exhaustion

### Validation Criteria
- [ ] State limit enforced at 100
- [ ] Extra states rejected
- [ ] Memory usage remains stable

### Implementation
`redteam/agent/src/dos_attacks.rs::state_bloat()`

---

## DA-004: Resource Starvation

**Severity:** High  
**Description:** Send large payloads continuously.

### Attack Vector
1. Send 100 payloads of 1 MB each
2. Monitor memory usage
3. Check for OOM conditions

### Expected Behavior
- Memory limits are enforced
- No OOM on honest nodes
- Payloads are processed or rejected gracefully

### Validation Criteria
- [ ] Memory usage stays within bounds
- [ ] No OOM crashes
- [ ] Node remains responsive

### Implementation
`redteam/agent/src/dos_attacks.rs::resource_starvation()`

---

## DA-005: Slowloris Attack

**Severity:** Medium  
**Description:** Open connections and send data slowly.

### Attack Vector
1. Open 20 connections
2. Send data at 1 byte per second
3. Monitor connection timeout

### Expected Behavior
- Idle connections are closed by timeout
- Connection slots are freed
- Node remains available for new connections

### Validation Criteria
- [ ] Idle connections closed within timeout period
- [ ] Connection slots freed
- [ ] No connection exhaustion

### Implementation
`redteam/agent/src/dos_attacks.rs::slowloris_attack()`
