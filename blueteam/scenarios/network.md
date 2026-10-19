# Protocol Defense Validation Scenarios

These scenarios validate that the P2P network layer defenses correctly detect and mitigate protocol-level attacks. Each scenario corresponds to a red team attack in [redteam/scenarios/network.md](../../redteam/scenarios/network.md).

## Monitored Code Paths

| File | Function | Defense Role |
|------|----------|--------------|
| `p2p-node/src/message_handler.rs` | `handle_gossipsub_message()` | Message parsing and rejection |
| `opendex-transport/src/message_store.rs` | `MessageStore` | Replay protection tracking |
| `opendex-transport/src/envelope.rs` | `ProtocolEnvelope` | Message validation |
| `p2p-node/src/network.rs` | `build_swarm()` | Connection limits and rate limiting |

---

## BD-001: Message Parsing Robustness

**Corresponds to:** PA-001: Malformed Message Injection  
**Severity:** Medium  
**Description:** Verify that all malformed messages are rejected without crashing the node.

### Defense Validation
1. Monitor honest node logs while red team executes PA-001
2. Count rejected malformed messages vs. total malformed messages sent
3. Verify no panics or crashes occur
4. Verify the attacker peer is NOT banned

### Expected Defense Behavior
- `handle_gossipsub_message()` catches all JSON parse errors
- Malformed messages are logged with `error` or `warn` level
- Node continues processing valid messages normally
- Attacker peer remains connected (not banned)

### Validation Criteria
- [ ] All malformed messages are rejected (rejection rate = 100%)
- [ ] No panics or crashes in message handler
- [ ] Attacker peer is not banned
- [ ] Honest nodes continue processing valid messages
- [ ] Prometheus metric `opendex_message_rejected_total` increases

### Detection Metrics
- `opendex_message_rejected_total` — should increase with each malformed message
- `opendex_message_processed_total` — should remain steady for valid messages
- Node CPU/memory usage should remain stable

### Blue Team Agent Implementation
`blueteam/agent/src/protocol_defenses.rs::validate_message_parsing_robustness()`

---

## BD-002: Replay Protection

**Corresponds to:** PA-002: Replay Attack  
**Severity:** High  
**Description:** Verify that MessageStore correctly rejects duplicate messages.

### Defense Validation
1. Monitor MessageStore activity while red team executes PA-002
2. Track which message IDs are accepted vs. rejected
3. Verify only the first message with a given ID is processed
4. Check for duplicate state transitions

### Expected Defense Behavior
- `MessageStore` tracks seen message IDs per swap
- First message is accepted and processed normally
- Subsequent messages with the same `message_id` are rejected immediately
- No duplicate state transitions occur in the trade state machine

### Validation Criteria
- [ ] Only the first message is processed (acceptance count = 1)
- [ ] Duplicate messages are logged as rejected
- [ ] No duplicate state changes in trade state machine
- [ ] `MessageStore` correctly tracks message IDs
- [ ] Prometheus metric `opendex_message_replay_detected_total` increases

### Detection Metrics
- `opendex_message_replay_detected_total` — should increase with each replay attempt
- `opendex_message_accepted_total` — should show only 1 acceptance per unique message ID
- Trade state transition count should match expected transitions

### Blue Team Agent Implementation
`blueteam/agent/src/protocol_defenses.rs::validate_replay_protection()`

---

## BD-003: Sequence Validation

**Corresponds to:** PA-003: Sequence Manipulation  
**Severity:** Medium  
**Description:** Verify that out-of-order messages are handled correctly.

### Defense Validation
1. Monitor message processing order while red team executes PA-003
2. Track sequence numbers of accepted vs. rejected messages
3. Verify messages are either reordered or rejected properly

### Expected Defense Behavior
- Messages are processed in correct sequence order (1, 2, 3, 4, 5)
- OR out-of-order messages are rejected until the expected sequence arrives
- No gaps in the sequence are allowed to persist
- No state corruption from out-of-order processing

### Validation Criteria
- [ ] Messages processed in correct order or rejected
- [ ] No state corruption from out-of-order processing
- [ ] Sequence validation logs show rejections for out-of-order messages
- [ ] No duplicate processing of the same sequence number

### Detection Metrics
- `opendex_sequence_violation_total` — should increase with out-of-order messages
- `opendex_message_rejected_total` — should include rejected out-of-order messages
- Trade state should remain consistent

### Blue Team Agent Implementation
`blueteam/agent/src/protocol_defenses.rs::validate_sequence_validation()`

---

## BD-004: Topic Isolation

**Corresponds to:** PA-004: Topic Hijacking  
**Severity:** Low  
**Description:** Verify that messages published to wrong topics are ignored.

### Defense Validation
1. Monitor message processing while red team executes PA-004
2. Verify messages to wrong topics are not processed
3. Verify messages to correct topics are still processed normally

### Expected Defense Behavior
- Messages to wrong topics are ignored by honest nodes
- Only messages to subscribed topics are processed
- No errors or panics from unknown topics
- No resource leaks from processing unknown topics

### Validation Criteria
- [ ] Messages to wrong topics are ignored (0 processed)
- [ ] Messages to correct topics are processed normally
- [ ] No resource leak from unknown topics
- [ ] No error logs from topic mismatch

### Detection Metrics
- `opendex_message_processed_total` — should only count messages from subscribed topics
- Node memory should remain stable (no leak from unknown topics)

### Blue Team Agent Implementation
`blueteam/agent/src/protocol_defenses.rs::validate_topic_isolation()`

---

## BD-005: Peer Authentication

**Corresponds to:** PA-005: Identity Spoofing  
**Severity:** High  
**Description:** Verify that forged peer identities are rejected.

### Defense Validation
1. Monitor peer authentication while red team executes PA-005
2. Verify forged signatures are rejected at the transport layer
3. Verify honest peer identities remain uncompromised

### Expected Defense Behavior
- GossipSub signed messages require valid Ed25519 signatures
- Forged signatures are rejected before reaching application logic
- Attacker cannot impersonate honest peers
- Peer identity verification is strict

### Validation Criteria
- [ ] All forged messages are rejected
- [ ] Signature verification fails for attacker messages
- [ ] Honest peer identities are not compromised
- [ ] Authentication failures are logged

### Detection Metrics
- `opendex_auth_failure_total` — should increase with each spoofing attempt
- `opendex_peer_rejected_total` — should count rejected forged peers
- No state changes from spoofed messages

### Blue Team Agent Implementation
`blueteam/agent/src/protocol_defenses.rs::validate_peer_authentication()`

---

## BD-006: Message Rate Limiting

**Corresponds to:** PA-007: GossipSub Flooding  
**Severity:** High  
**Description:** Verify that rate limiting activates during message floods.

### Defense Validation
1. Monitor message throughput while red team executes PA-007
2. Measure when rate limiting activates
3. Verify honest traffic is not affected by rate limiting

### Expected Defense Behavior
- Rate limiting activates after the message threshold is exceeded
- Excess messages from the attacker are dropped
- Honest node message processing remains unaffected
- CPU and memory usage remain within bounds

### Validation Criteria
- [ ] Rate limiting activates within expected time (< 5 seconds)
- [ ] Messages dropped after threshold
- [ ] Honest nodes do not crash or become unresponsive
- [ ] CPU and memory usage remain within bounds
- [ ] Honest traffic is not rate-limited

### Detection Metrics
- `opendex_rate_limiter_activations_total` — should increase
- `opendex_message_dropped_total` — should increase during flood
- Node CPU usage should not exceed 80%
- Node memory usage should remain stable

### Blue Team Agent Implementation
`blueteam/agent/src/protocol_defenses.rs::validate_rate_limiting()`

---

## BD-007: DHT Integrity

**Corresponds to:** DA-006: DHT Pollution  
**Severity:** Low  
**Description:** Verify that false DHT records are rejected.

### Defense Validation
1. Monitor DHT record insertions while red team executes DA-006
2. Verify invalid records are rejected
3. Verify honest peer discovery is not affected

### Expected Defense Behavior
- Invalid DHT records are rejected by validation logic
- No pollution of the DHT with false peer records
- Honest peer discovery continues to work correctly
- DHT remains clean and functional

### Validation Criteria
- [ ] False records are rejected
- [ ] DHT remains clean (no false records stored)
- [ ] Peer discovery works correctly
- [ ] DHT query responses are accurate

### Detection Metrics
- `opendex_dht_record_rejected_total` — should increase with false records
- `opendex_dht_query_success_total` — should remain steady
- DHT storage size should not increase from false records

### Blue Team Agent Implementation
`blueteam/agent/src/protocol_defenses.rs::validate_dht_integrity()`

---

## BD-008: Connection Limits

**Corresponds to:** DA-001: Connection Exhaustion  
**Severity:** High  
**Description:** Verify that connection limits are enforced during connection exhaustion attacks.

### Defense Validation
1. Monitor connection counts while red team executes DA-001
2. Verify max connections limit is enforced
3. Verify existing connections remain functional

### Expected Defense Behavior
- Max connections limit is enforced strictly
- New connections are refused after the limit is reached
- Existing connections remain functional and responsive
- Node does not crash from connection overload

### Validation Criteria
- [ ] Connection limit is enforced
- [ ] Excess connections are refused (not accepted)
- [ ] Existing connections remain functional
- [ ] Node remains responsive

### Detection Metrics
- `opendex_connection_count` — should not exceed max limit
- `opendex_connection_rejected_total` — should increase when limit is reached
- Node resource usage should remain stable

### Blue Team Agent Implementation
`blueteam/agent/src/protocol_defenses.rs::validate_connection_limits()`