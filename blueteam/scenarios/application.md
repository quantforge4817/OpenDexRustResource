# Application Defense Validation Scenarios

These scenarios validate that application-level defenses correctly detect and mitigate attacks targeting the transport layer, state machine, and resource management. Each scenario corresponds to a red team attack in [redteam/scenarios/application.md](../../redteam/scenarios/application.md).

## Monitored Code Paths

| File | Function | Defense Role |
|------|----------|--------------|
| `opendex-transport/src/message_store.rs` | `MessageStore` | Replay protection |
| `p2p-node/src/trade_state.rs` | Trade state management | State machine transitions |
| `p2p-node/src/message_handler.rs` | Message dispatch | Application logic |

---

## BD-016: Envelope Integrity

**Corresponds to:** TA-001: Envelope Tampering  
**Severity:** High  
**Description:** Verify that tampered ProtocolEnvelope fields are detected and rejected.

### Defense Validation
1. Monitor envelope validation while red team executes TA-001
2. Track acceptance/rejection of tampered envelopes
3. Verify integrity checks catch all modifications

### Expected Defense Behavior
- Envelope validation detects all field modifications
- Tampered envelopes are rejected before processing
- Integrity checks fail for modified message_id, sequence, or timestamp

### Validation Criteria
- [ ] All tampered envelopes rejected (0 acceptances)
- [ ] Validation errors are logged for each rejection
- [ ] No state changes from tampered messages

### Detection Metrics
- `opendex_envelope_validation_failure_total` — should increase with each tampered envelope
- `opendex_message_rejected_total` — should include rejected tampered envelopes

### Blue Team Agent Implementation
`blueteam/agent/src/transport_defenses.rs::validate_envelope_integrity()`

---

## BD-017: Sequence Validation

**Corresponds to:** TA-002: Sequence Manipulation  
**Severity:** Medium  
**Description:** Verify that invalid sequence numbers are rejected.

### Defense Validation
1. Monitor sequence validation while red team executes TA-002
2. Track handling of duplicate, negative, and very large sequence numbers
3. Verify no state corruption from invalid sequences

### Expected Defense Behavior
- Sequence validation rejects all invalid sequences
- Duplicate sequences are detected and rejected
- No state corruption from invalid sequence numbers

### Validation Criteria
- [ ] Invalid sequences rejected (0 acceptances)
- [ ] No duplicate processing from duplicate sequences
- [ ] Sequence validation is robust against edge cases

### Detection Metrics
- `opendex_sequence_violation_total` — should increase with each invalid sequence
- `opendex_message_rejected_total` — should include rejected invalid sequences

### Blue Team Agent Implementation
`blueteam/agent/src/transport_defenses.rs::validate_sequence_validation()`

---

## BD-018: Replay Protection

**Corresponds to:** TA-003: MessageStore Bypass  
**Severity:** High  
**Description:** Verify that MessageStore replay protection cannot be bypassed.

### Defense Validation
1. Monitor MessageStore while red team executes TA-003
2. Verify duplicate message_id is rejected regardless of envelope changes
3. Verify per-swap isolation is enforced

### Expected Defense Behavior
- `MessageStore` tracks message IDs per swap
- Duplicate `message_id` is rejected regardless of envelope modifications
- No bypass of replay protection via different swap IDs

### Validation Criteria
- [ ] Replay protection cannot be bypassed (0 successful replays)
- [ ] Message ID tracking works correctly
- [ ] Per-swap isolation is enforced

### Detection Metrics
- `opendex_message_replay_detected_total` — should increase with each bypass attempt
- `opendex_message_accepted_total` — should not increase from replayed messages

### Blue Team Agent Implementation
`blueteam/agent/src/transport_defenses.rs::validate_replay_protection()`

---

## BD-019: Transport Robustness

**Corresponds to:** TA-004: Transport Layer Fuzzing  
**Severity:** Medium  
**Description:** Verify that fuzzed transport input does not crash the node.

### Defense Validation
1. Monitor node stability while red team executes TA-004
2. Verify no crashes or panics from fuzzed input
3. Verify connections are closed gracefully

### Expected Defense Behavior
- Robust parsing handles all invalid input
- No crashes or panics from any fuzzed data
- Connections are closed gracefully without resource leaks

### Validation Criteria
- [ ] No crashes from fuzzed input
- [ ] Invalid connections handled gracefully
- [ ] Node remains stable throughout fuzzing

### Detection Metrics
- Node uptime should remain 100% during fuzzing
- `opendex_connection_error_total` — may increase but node stays up
- Memory usage should remain stable

### Blue Team Agent Implementation
`blueteam/agent/src/transport_defenses.rs::validate_transport_robustness()`

---

## BD-020: State Limits

**Corresponds to:** DA-002: State Bloat  
**Severity:** Medium  
**Description:** Verify that pending trade state limits are enforced.

### Defense Validation
1. Monitor state count while red team executes DA-002
2. Verify state limit is enforced at the configured maximum
3. Verify extra states are rejected

### Expected Defense Behavior
- State limit is enforced at MAX_PENDING_TRADE_STATES (100)
- Extra states beyond the limit are rejected
- No memory exhaustion from state bloat

### Validation Criteria
- [ ] State limit enforced at 100
- [ ] Extra states rejected (not accepted)
- [ ] Memory usage remains stable

### Detection Metrics
- `opendex_pending_trade_states` — should not exceed 100
- `opendex_state_rejected_total` — should increase when limit is reached
- Node memory should remain stable

### Blue Team Agent Implementation
`blueteam/agent/src/dos_defenses.rs::validate_state_limits()`

---

## BD-021: Memory Limits

**Corresponds to:** DA-004: Resource Starvation  
**Severity:** High  
**Description:** Verify that memory usage is bounded during resource starvation attacks.

### Defense Validation
1. Monitor memory usage while red team executes DA-004
2. Verify memory stays within configured bounds
3. Verify no OOM crashes occur

### Expected Defense Behavior
- Memory limits are enforced
- No OOM on honest nodes
- Large payloads are processed or rejected gracefully

### Validation Criteria
- [ ] Memory usage stays within bounds (< 80% of limit)
- [ ] No OOM crashes
- [ ] Node remains responsive

### Detection Metrics
- `opendex_memory_usage_bytes` — should not exceed 80% of configured limit
- Node should not crash (uptime = 100%)
- `opendex_payload_rejected_total` — may increase for oversized payloads

### Blue Team Agent Implementation
`blueteam/agent/src/dos_defenses.rs::validate_memory_limits()`

---

## BD-022: Connection Timeouts

**Corresponds to:** DA-005: Slowloris Attack  
**Severity:** Medium  
**Description:** Verify that idle connections are closed by timeout.

### Defense Validation
1. Monitor connection state while red team executes DA-005
2. Verify idle connections are closed within the timeout period
3. Verify connection slots are freed

### Expected Defense Behavior
- Idle connections are closed by timeout mechanism
- Connection slots are freed after timeout
- Node remains available for new connections

### Validation Criteria
- [ ] Idle connections closed within timeout period
- [ ] Connection slots freed
- [ ] No connection exhaustion from slow connections

### Detection Metrics
- `opendex_connection_count` — should not remain elevated after timeout
- `opendex_connection_timeout_total` — should increase with timed-out connections
- Available connection slots should remain > 0

### Blue Team Agent Implementation
`blueteam/agent/src/dos_defenses.rs::validate_connection_timeouts()`