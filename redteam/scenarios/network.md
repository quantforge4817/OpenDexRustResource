# Network-Level Attack Scenarios

These scenarios test the P2P network layer defenses, including GossipSub message handling, Kademlia DHT, peer identity, and connection management.

## Target Code Paths

| File | Function | Role |
|------|----------|------|
| `p2p-node/src/network.rs` | `build_swarm()` | libp2p swarm configuration |
| `p2p-node/src/message_handler.rs` | `handle_gossipsub_message()` | Message dispatch by type |
| `p2p-node/src/message_handler.rs` | `publish_message()` | JSON serialization and GossipSub publish |
| `opendex-transport/src/envelope.rs` | `ProtocolEnvelope` | Message structure with `message_id`, `sequence` |
| `opendex-transport/src/message_store.rs` | `MessageStore` | Replay protection via message ID tracking |

## PA-001: Malformed Message Injection

**Severity:** Medium  
**Description:** Publish invalid JSON via GossipSub to test message parsing robustness.

### Attack Vector
1. Connect to the testnet as a legitimate peer
2. Publish messages with invalid JSON syntax:
   - `"not json at all"`
   - `"{ invalid json }"`
   - `"{\"type\": \"UnknownType\"}"`
   - `"{\"type\": \"Order\", \"data\": null}"`
   - `"{\"type\": \"Order\"}"` (missing required fields)

### Expected Behavior
- `handle_gossipsub_message()` catches JSON parse errors
- Malformed messages are logged and rejected
- Peer is NOT banned for sending malformed messages
- Node continues operating normally

### Validation Criteria
- [ ] All malformed messages are rejected
- [ ] No panics or crashes in message handler
- [ ] Attacker peer is not banned
- [ ] Honest nodes continue processing valid messages

### Implementation
`redteam/agent/src/protocol_attacks.rs::malformed_message_injection()`

---

## PA-002: Replay Attack (Live Network)

**Severity:** High  
**Description:** Capture a valid message and re-publish it with the same `message_id`.

### Attack Vector
1. Capture a valid message from the network (e.g., an Order or TradeState)
2. Re-publish the exact same message 5 times
3. Monitor honest node logs for duplicate processing

### Expected Behavior
- `MessageStore` tracks seen message IDs per swap
- First message is accepted and processed
- Subsequent messages with the same `message_id` are rejected
- No duplicate state transitions occur

### Validation Criteria
- [ ] Only the first message is processed
- [ ] Duplicate messages are logged as rejected
- [ ] No duplicate state changes in trade state machine
- [ ] `MessageStore` correctly tracks message IDs

### Implementation
`redteam/agent/src/protocol_attacks.rs::replay_attack()`

---

## PA-003: Sequence Manipulation

**Severity:** Medium  
**Description:** Send messages out of order to test sequence validation.

### Attack Vector
1. Create 5 TradeState messages with sequences [5, 1, 3, 2, 4]
2. Publish them in that order via GossipSub
3. Monitor honest node processing order

### Expected Behavior
- Messages are either processed in correct order (1, 2, 3, 4, 5)
- Or out-of-order messages are rejected until the expected sequence arrives
- No gaps in the sequence are allowed to persist

### Validation Criteria
- [ ] Messages processed in correct order or rejected
- [ ] No state corruption from out-of-order processing
- [ ] Sequence validation logs show rejections

### Implementation
`redteam/agent/src/protocol_attacks.rs::sequence_manipulation()`

---

## PA-004: Topic Hijacking

**Severity:** Low  
**Description:** Publish messages to wrong GossipSub topics.

### Attack Vector
1. Publish valid messages to incorrect topics:
   - `opendex-admin` (non-existent)
   - `bitcoin-mainnet` (wrong network)
   - `random-topic-12345` (random)
2. Also publish to correct topics for comparison

### Expected Behavior
- Messages to wrong topics are ignored by honest nodes
- Only messages to subscribed topics are processed
- No error or panic from unknown topics

### Validation Criteria
- [ ] Messages to wrong topics are ignored
- [ ] Messages to correct topics are processed
- [ ] No resource leak from unknown topics

### Implementation
`redteam/agent/src/protocol_attacks.rs::topic_hijacking()`

---

## PA-005: Identity Spoofing

**Severity:** High  
**Description:** Attempt to impersonate an honest peer via forged peer ID.

### Attack Vector
1. Attempt to forge messages that appear to come from honest peer IDs
2. Use invalid Ed25519 signatures
3. Monitor peer verification responses

### Expected Behavior
- GossipSub signed messages require valid Ed25519 signatures
- Forged signatures are rejected at the transport layer
- Attacker cannot impersonate honest peers

### Validation Criteria
- [ ] All forged messages are rejected
- [ ] Signature verification fails for attacker messages
- [ ] Honest peer identities are not compromised

### Implementation
`redteam/agent/src/protocol_attacks.rs::identity_spoofing()`

---

## PA-006: Protocol Downgrade

**Severity:** Medium  
**Description:** Attempt to force older protocol version.

### Attack Vector
1. Use the Identify protocol to advertise older versions:
   - `opendex/0.1.0`
   - `opendex/0.0.1`
   - `opendex-testnet/0.9.9`
2. Attempt to establish connections with these versions

### Expected Behavior
- Version negotiation rejects connections with older versions
- Only the current protocol version is accepted
- Connection is refused or closed

### Validation Criteria
- [ ] All protocol downgrade attempts are rejected
- [ ] No connection established with older versions
- [ ] Version mismatch is logged

### Implementation
`redteam/agent/src/protocol_attacks.rs::protocol_downgrade()`

---

## PA-007: GossipSub Flooding

**Severity:** High  
**Description:** Publish 1000 messages/second to test rate limiting.

### Attack Vector
1. Publish 100 messages as fast as possible
2. Measure message throughput and rejection rate
3. Monitor honest node resource usage

### Expected Behavior
- Rate limiting activates after threshold
- Excess messages are dropped
- Honest nodes remain responsive

### Validation Criteria
- [ ] Rate limiting activates within expected time
- [ ] Messages dropped after threshold
- [ ] Honest nodes do not crash or become unresponsive
- [ ] CPU and memory usage remain within bounds

### Implementation
`redteam/agent/src/protocol_attacks.rs::gossipsub_flooding()`

---

## PA-008: Message Size Attack

**Severity:** Medium  
**Description:** Publish a 10 MB message to test max message size enforcement.

### Attack Vector
1. Create a 10 MB JSON message
2. Attempt to publish via GossipSub
3. Monitor connection state

### Expected Behavior
- GossipSub's max message size limit rejects the message
- Connection may be closed
- No memory exhaustion on honest nodes

### Validation Criteria
- [ ] Large message is rejected
- [ ] No OOM on honest nodes
- [ ] Connection handled gracefully

### Implementation
`redteam/agent/src/protocol_attacks.rs::message_size_attack()`

---

## DA-001: Connection Exhaustion

**Severity:** High  
**Description:** Open 1000 concurrent connections to test max connection limits.

### Attack Vector
1. Open 100 concurrent connections to a single honest node
2. Monitor connection acceptance and refusal

### Expected Behavior
- Max connections limit is enforced
- New connections are refused after limit
- Node remains responsive to existing connections

### Validation Criteria
- [ ] Connection limit is enforced
- [ ] Excess connections are refused
- [ ] Existing connections remain functional

### Implementation
`redteam/agent/src/dos_attacks.rs::connection_exhaustion()`

---

## DA-003: GossipSub Mesh Manipulation

**Severity:** Medium  
**Description:** Join and leave the GossipSub mesh rapidly.

### Attack Vector
1. Perform 50 join/leave cycles in rapid succession
2. Monitor peer scoring changes

### Expected Behavior
- Peer scoring penalizes rapid join/leave behavior
- Attacker's score decreases over time
- Mesh stability is maintained

### Validation Criteria
- [ ] Peer score decreases with each cycle
- [ ] Mesh remains stable
- [ ] Honest peers are not affected

### Implementation
`redteam/agent/src/dos_attacks.rs::gossipsub_mesh_manipulation()`

---

## DA-006: DHT Pollution

**Severity:** Low  
**Description:** Insert false records into Kademlia DHT.

### Attack Vector
1. Insert 3 false peer records with invalid addresses
2. Monitor DHT record validation

### Expected Behavior
- Invalid DHT records are rejected
- No pollution of the DHT
- Honest peer discovery is not affected

### Validation Criteria
- [ ] False records are rejected
- [ ] DHT remains clean
- [ ] Peer discovery works correctly

### Implementation
`redteam/agent/src/dos_attacks.rs::dht_pollution()`
