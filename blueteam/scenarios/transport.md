# Transport Defense Validation Scenarios

These scenarios validate that transport-layer defenses correctly detect and mitigate attacks targeting message integrity, replay protection, and transport robustness. These complement the scenarios in [redteam/scenarios/application.md](../../redteam/scenarios/application.md) and [redteam/scenarios/network.md](../../redteam/scenarios/network.md).

## Monitored Code Paths

| File | Function | Defense Role |
|------|----------|--------------|
| `opendex-transport/src/envelope.rs` | `ProtocolEnvelope` | Message structure and validation |
| `opendex-transport/src/message_store.rs` | `MessageStore` | Replay protection |
| `opendex-transport/src/transport.rs` | Transport layer | Message serialization and parsing |

---

## BD-030: Envelope Signature Verification

**Corresponds to:** TA-001: Envelope Tampering (extended)  
**Severity:** High  
**Description:** Verify that envelope cryptographic signatures are validated.

### Defense Validation
1. Monitor envelope signature verification while red team executes TA-001
2. Verify tampered signatures are detected
3. Verify unsigned envelopes are rejected

### Expected Defense Behavior
- Envelope signatures are verified before processing
- Tampered signatures fail verification
- Unsigned envelopes are rejected

### Validation Criteria
- [ ] All tampered signatures rejected
- [ ] Unsigned envelopes rejected
- [ ] Signature verification errors are logged

### Detection Metrics
- `opendex_envelope_signature_failure_total` — should increase with each tampered envelope
- `opendex_message_rejected_total` — should include rejected unsigned envelopes

### Blue Team Agent Implementation
`blueteam/agent/src/transport_defenses.rs::validate_envelope_signature()`

---

## BD-031: Message Timestamp Validation

**Corresponds to:** TA-002: Sequence Manipulation (extended)  
**Severity:** Medium  
**Description:** Verify that message timestamps are validated to prevent replay with old messages.

### Defense Validation
1. Monitor timestamp validation while red team executes TA-002
2. Verify messages with expired timestamps are rejected
3. Verify messages with future timestamps are handled correctly

### Expected Defense Behavior
- Messages with expired timestamps are rejected
- Messages with future timestamps are either rejected or delayed
- Timestamp validation prevents replay of old messages

### Validation Criteria
- [ ] Expired messages rejected
- [ ] Future timestamps handled correctly
- [ ] Timestamp validation prevents replay

### Detection Metrics
- `opendex_timestamp_violation_total` — should increase with invalid timestamps
- `opendex_message_rejected_total` — should include rejected expired messages

### Blue Team Agent Implementation
`blueteam/agent/src/transport_defenses.rs::validate_timestamp_validation()`

---

## BD-032: Transport Message Size Limits

**Corresponds to:** PA-008: Message Size Attack  
**Severity:** Medium  
**Description:** Verify that oversized messages are rejected at the transport layer.

### Defense Validation
1. Monitor message size validation while red team executes PA-008
2. Verify oversized messages are rejected before processing
3. Verify no memory exhaustion from large messages

### Expected Defense Behavior
- GossipSub max message size limit rejects oversized messages
- Connection is handled gracefully (may be closed)
- No memory exhaustion on honest nodes

### Validation Criteria
- [ ] Oversized messages rejected
- [ ] No OOM on honest nodes
- [ ] Connection handled gracefully

### Detection Metrics
- `opendex_message_too_large_total` — should increase with each oversized message
- Node memory should remain stable
- `opendex_connection_error_total` — may increase if connections are closed

### Blue Team Agent Implementation
`blueteam/agent/src/transport_defenses.rs::validate_message_size_limits()`