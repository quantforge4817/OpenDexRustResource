# OpenDex Red Team Action Plan

**Version:** 1.0.0  
**Date:** 2026-10-19  
**Status:** Ready for deployment — requires cloud VPS and domain  
**Prerequisites:** 5 VPS instances (4 honest nodes + 1 attacker), domain name for testnet  

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Infrastructure Requirements](#2-infrastructure-requirements)
3. [Network Topology](#3-network-topology)
4. [Attack Scenario Catalog](#4-attack-scenario-catalog)
5. [Red Team Agent Architecture](#5-red-team-agent-architecture)
6. [Execution Procedures](#6-execution-procedures)
7. [Monitoring & Observability](#7-monitoring--observability)
8. [Incident Response Workflow](#8-incident-response-workflow)
9. [Regression Integration](#9-regression-integration)
10. [CI/CD Integration](#10-ci-cd-integration)
11. [Appendix A — Code Path Mapping](#appendix-a--code-path-mapping)
12. [Appendix B — VPS Provisioning Checklist](#appendix-b--vps-provisioning-checklist)

---

## 1. Executive Summary

### Purpose

This document defines the complete red team testing infrastructure for OpenDex, bridging the gap between in-process unit tests (692 tests, 13 fuzz targets, 26 adversarial scenarios) and real-world network-level penetration testing. It provides a single actionable plan that a team can execute when cloud VPS and domain infrastructure are ready.

### Current Security Coverage

| Category | Coverage | Status |
|----------|----------|--------|
| Unit tests | 692 passed, 0 failed, 5 ignored | ✓ Complete |
| Fuzz targets | 13 targets via cargo-fuzz/libFuzzer | ✓ Complete |
| Adversarial scenarios | 26 fiat scenarios, 15 p2p-node test files | ✓ Complete |
| Supply chain | cargo-deny configured, 0 RUSTSEC vulns | ✓ Complete |
| Safe Rust | 0 unsafe blocks | ✓ Complete |
| **Network-level E2E tests** | **All multi-node tests marked `#[ignore]`** | **✗ Requires VPS** |

### Key Gap

All current tests run in-process with `MockTransport`. The `load_tests.rs` file contains multi-node tests that are explicitly marked `#[ignore]` because they require real network infrastructure. This plan defines that infrastructure and the attack scenarios that run against it.

### Goals

1. **Deployable testnet**: 5-node star topology on separate VPS instances, connected via libp2p with Noise encryption
2. **Adversarial agent**: Dedicated Rust crate that connects as a legitimate peer, then injects attacks through GossipSub
3. **Scenario catalog**: Complete mapping of attack scenarios to code paths, expected behaviors, and validation criteria
4. **Regression pipeline**: Discovered vulnerabilities automatically feed back into permanent regression tests
5. **Repeatable cycles**: Standardized red team execution with reporting and tracking

---

## 2. Infrastructure Requirements

### 2.1 VPS Specifications

| Role | Count | CPU | RAM | Disk | OS | Network |
|------|-------|-----|-----|------|----|---------|
| Honest Node | 4 | 2 vCPU | 4 GB | 40 GB SSD | Ubuntu 24.04 LTS | 1 Gbps |
| Attacker Node | 1 | 4 vCPU | 8 GB | 80 GB SSD | Ubuntu 24.04 LTS | 1 Gbps |
| Monitoring | 1 (shared) | 2 vCPU | 4 GB | 100 GB SSD | Ubuntu 24.04 LTS | 1 Gbps |

**Total:** 6 VPS instances, approximately $150-200/month depending on provider.

### 2.2 Software Requirements

Each VPS requires:
- **Rust toolchain**: Stable 1.75+ with `rustup`
- **Docker**: For containerized node deployment
- **Prometheus node exporter**: System metrics
- **Firewall**: UFW configured per [Appendix B](#appendix-b--vps-provisioning-checklist)

### 2.3 Domain Configuration

| Purpose | Subdomain | Notes |
|---------|-----------|-------|
| Testnet bootstrap | `testnet.opendex.io` | DNS A record pointing to bootstrap node |
| Monitoring dashboard | `monitor.opendex.io` | Grafana access |
| Log aggregation | `logs.opendex.io` | Loki/Vector access |

**Important:** All testnet domains must use separate DNS zones from any production domains to prevent contamination.

### 2.4 Network Security

- **Isolated VPC**: All testnet VPS instances must reside in a dedicated VPC, isolated from production
- **No public exposure**: Honest nodes are not directly accessible from the internet; only the attacker node has inbound SSH
- **Internal networking**: libp2p peers connect via internal VPC IPs
- **Key isolation**: Test cryptographic keys are generated per-deployment and never reused across environments

---

## 3. Network Topology

### 3.1 Star Topology

```
                    ┌─────────────────┐
                    │  Attacker Node  │
                    │  (red-team-agent)│
                    └────────┬────────┘
                             │ libp2p/GossipSub
              ┌──────────────┼──────────────┐
              │              │              │
    ┌─────────▼────┐ ┌──────▼──────┐ ┌─────▼────────┐
    │ Honest Node 1│ │ Honest Node 2│ │ Honest Node 3│
    │ (p2p-node)   │ │ (p2p-node)   │ │ (p2p-node)   │
    └──────────────┘ └──────────────┘ └──────────────┘
              │
    ┌─────────▼────┐
    │ Honest Node 4│
    │ (p2p-node)   │
    └──────────────┘

All nodes also gossip with each other via GossipSub mesh.
```

### 3.2 Peer Discovery

1. **Bootstrap**: Honest nodes start with hardcoded peer IDs of adjacent nodes
2. **Kademlia DHT**: Nodes discover each other via DHT after initial bootstrap
3. **Attacker injection**: Attacker node connects to all honest nodes via their published multiaddrs

### 3.3 Transport Configuration

All nodes use identical libp2p configuration from `p2p-node/src/network.rs::build_swarm()`:
- **Transport**: TCP with Noise encryption (XX handshake)
- **Multiplexing**: Yamux
- **Routing**: Kademlia DHT
- **PubSub**: GossipSub (1.1.0)
- **Identify**: Protocol version identification
- **Ping**: Heartbeat for connection health

---

## 4. Attack Scenario Catalog

### 4.1 Protocol Attacks

**Target:** `p2p-node`, `protocol` crates  
**Existing tests:** `crates/p2p-node/tests/network_protocol_attacks.rs`  
**Existing coverage:** Malformed messages, replay attacks, identity spoofing (in-process)

| Scenario ID | Description | Attack Vector | Expected Defense | Validation |
|-------------|-------------|---------------|------------------|------------|
| PA-001 | Malformed message injection | Publish invalid JSON via GossipSub | Message rejected, peer not banned | Log shows rejection, node continues |
| PA-002 | Replay attack (live network) | Re-publish captured message with same `message_id` | `MessageStore` rejects duplicate | No duplicate processing |
| PA-003 | Sequence manipulation | Send messages out of order | Sequence validation rejects | Messages processed in order or rejected |
| PA-004 | Topic hijacking | Publish to wrong GossipSub topic | Topic validation rejects | Message ignored |
| PA-005 | Identity spoofing | Impersonate honest peer via forged peer ID | Peer ID verification fails | Attacker isolated |
| PA-006 | Protocol downgrade | Force older protocol version | Version negotiation rejects | Connection refused |
| PA-007 | GossipSub flooding | Publish 1000 messages/second | Rate limiting activates | Messages dropped after threshold |
| PA-008 | Message size attack | Publish 10 MB message | Max message size enforced | Connection closed |

**Code paths:**
- `p2p-node/src/message_handler.rs::handle_gossipsub_message()` — message dispatch
- `p2p-node/src/network.rs::build_swarm()` — swarm configuration
- `opendex-transport/src/message_store.rs` — replay protection
- `opendex-transport/src/envelope.rs::ProtocolEnvelope` — message structure

### 4.2 Cryptographic Attacks

**Target:** `user`, `fiat` crates  
**Existing tests:** `crates/p2p-node/tests/crypto_penetration_tests.rs`  
**Existing coverage:** HMAC tokens, key management (in-process)

| Scenario ID | Description | Attack Vector | Expected Defense | Validation |
|-------------|-------------|---------------|------------------|------------|
| CA-001 | HMAC token replay | Capture and replay auth token across nodes | Token bound to session/IP | Replay rejected |
| CA-002 | Token timing analysis | Measure response time for token validation | Constant-time comparison | Timing variance < 1ms |
| CA-003 | Key extraction attempt | Brute-force HMAC secret | Strong random keys, rate limiting | Attack blocked |
| CA-004 | Signature malleability | Modify signature bytes | Signature verification fails | Invalid signature rejected |
| CA-005 | Key uniqueness test | Verify no key reuse across instances | Unique key generation | All keys distinct |
| CA-006 | Argon2id bypass | Attempt password hash collision | Memory-hard hashing | Collision infeasible |
| CA-007 | DLEQ proof forgery | Construct invalid DLEQ proof | Proof verification fails | Invalid proof rejected |

**Code paths:**
- `user/src/auth.rs::AuthService` — HMAC-SHA256 token signing/validation
- `user/src/auth.rs::RateLimiter` — sliding-window rate limiting
- `fiat/src/` — cryptographic operations in fiat trades

### 4.3 Denial of Service Attacks

**Target:** `p2p-node`, `opendex-transport` crates  
**Existing tests:** `crates/p2p-node/tests/load_tests.rs` (all `#[ignore]`)  
**Existing coverage:** Multi-node mesh, message flood (skipped)

| Scenario ID | Description | Attack Vector | Expected Defense | Validation |
|-------------|-------------|---------------|------------------|------------|
| DA-001 | Connection exhaustion | Open 1000 concurrent connections | Max connections enforced | New connections refused |
| DA-002 | State bloat | Submit MAX_PENDING_TRADE_STATES + 1 states | State limit enforced | Extra states rejected |
| DA-003 | GossipSub mesh manipulation | Join/leave mesh rapidly | Peer scoring penalizes | Attacker score decreases |
| DA-004 | Resource starvation | Send large payloads continuously | Memory limits enforced | OOM prevented |
| DA-005 | Slowloris attack | Open connections, send slowly | Connection timeout | Idle connections closed |
| DA-006 | DHT pollution | Insert false records into Kademlia | Record validation | Invalid records rejected |

**Code paths:**
- `p2p-node/src/network.rs` — connection management
- `p2p-node/src/trade_state.rs` — pending state limits
- `opendex-transport/src/message_store.rs` — message storage limits
- `crates/p2p-node/tests/load_tests.rs` — existing ignored tests

### 4.4 Fiat Payment Attacks

**Target:** `fiat` crate  
**Existing tests:** `crates/fiat/tests/adversarial_tests.rs` (26 scenarios)  
**Existing coverage:** Payment spoofing, race conditions (in-process)

| Scenario ID | Description | Attack Vector | Expected Defense | Validation |
|-------------|-------------|---------------|------------------|------------|
| FA-001 | False payment detection | Claim payment received without actual transfer | Bank adapter verification | Payment not confirmed |
| FA-002 | Payment reversal | Reverse bank transfer after confirmation | Reversal detection window | Trade paused |
| FA-003 | Race condition exploit | Concurrent payment claims | Atomic state transitions | Only one claim succeeds |
| FA-004 | Chargeback simulation | Initiate chargeback after settlement | Chargeback monitoring | Trade flagged |
| FA-005 | 2-of-2 multisig edge case | Attempt unilateral withdrawal | Multisig enforcement | Withdrawal blocked |
| FA-006 | Settlement bypass | Skip settlement steps | State machine enforcement | Invalid transition rejected |
| FA-007 | Cross-currency confusion | Mix currencies in payment | Currency validation | Mismatch rejected |

**Code paths:**
- `fiat/src/trade.rs` — fiat trade state machine
- `fiat/src/settlement_engine.rs` — settlement logic
- `fiat/src/bank_adapter.rs` — payment verification
- `crates/fiat/tests/adversarial_tests.rs` — existing adversarial tests

### 4.5 Transport Layer Attacks

**Target:** `opendex-transport` crate  
**Existing tests:** `crates/opendex-transport/tests/multi_node.rs` (4 tests)  
**Existing coverage:** Multi-node adversarial tests

| Scenario ID | Description | Attack Vector | Expected Defense | Validation |
|-------------|-------------|---------------|------------------|------------|
| TA-001 | Envelope tampering | Modify `ProtocolEnvelope` fields | Envelope validation | Tampered envelope rejected |
| TA-002 | Sequence manipulation | Manipulate message sequence numbers | Sequence validation | Out-of-order rejected |
| TA-003 | MessageStore bypass | Bypass replay protection | Message ID tracking | Duplicate detected |
| TA-004 | Transport layer fuzzing | Fuzz raw TCP/UDP transport | Robust parsing | No crashes |

**Code paths:**
- `opendex-transport/src/envelope.rs` — `ProtocolEnvelope` structure
- `opendex-transport/src/message_store.rs` — `MessageStore` replay protection
- `opendex-transport/src/libp2p_transport.rs` — libp2p transport implementation
- `opendex-transport/src/transport.rs` — `Transport` trait

---

## 5. Red Team Agent Architecture

### 5.1 Agent Design

The `red-team-agent` crate connects to the testnet as a legitimate libp2p peer, then executes attack scenarios by:

1. **Publishing adversarial messages** via GossipSub (same channel as honest peers)
2. **Direct peer-to-peer message injection** for targeted attacks
3. **Monitoring honest node responses** to measure defense effectiveness

### 5.2 Crate Structure

```
redteam/agent/
├── Cargo.toml
└── src/
    ├── main.rs              # Agent entry point, libp2p swarm construction
    ├── protocol_attacks.rs  # Protocol-level attack scenarios
    ├── crypto_attacks.rs    # Cryptographic attack scenarios
    ├── dos_attacks.rs       # Denial of service scenarios
    └── fiat_attacks.rs      # Fiat payment attack scenarios
```

### 5.3 Dependencies

The agent depends on:
- `libp2p` — P2P networking (same version as `p2p-node`)
- `protocol` — Message types and serialization
- `opendex-transport` — Transport layer envelope and message store
- `opendex-types` — Core type definitions
- `tokio` — Async runtime

### 5.4 Execution Model

1. **Initialization**: Agent constructs a libp2p swarm, reusing `p2p-node/network.rs::build_swarm()` configuration
2. **Connection**: Agent connects to all honest nodes via their published multiaddrs
3. **Scenario dispatch**: Agent executes attack scenarios sequentially or in parallel
4. **Result collection**: Agent monitors honest node logs and metrics for defense responses
5. **Reporting**: Agent generates structured reports for each scenario

---

## 6. Execution Procedures

### 6.1 Deployment Checklist

1. **Provision VPS instances** per [Appendix B](#appendix-b--vps-provisioning-checklist)
2. **Install Rust toolchain** on all instances
3. **Configure firewall rules** per network topology
4. **Deploy honest nodes** with testnet configuration
5. **Deploy attacker node** with red-team-agent
6. **Verify connectivity** — all nodes discover each other via GossipSub
7. **Deploy monitoring** — Prometheus, Grafana, log aggregation

### 6.2 Scenario Execution

1. **Select scenario category**: Protocol, Crypto, DoS, Fiat, or Transport
2. **Configure test parameters**: Number of iterations, timing, payload sizes
3. **Execute scenario**: Run via `red-team-agent` with scenario ID
4. **Collect results**: Gather logs, metrics, and node state
5. **Analyze results**: Compare actual behavior against expected defense
6. **Document findings**: Use report template from `redteam/reports/template.md`

### 6.3 Result Classification

| Severity | Criteria | Example |
|----------|----------|---------|
| **Critical** | Funds at risk, cryptographic break | Signature forgery succeeds |
| **High** | Service disruption, data leak | DoS succeeds, private key exposed |
| **Medium** | Partial bypass, degraded security | Rate limiter bypassed |
| **Low** | Minor issue, easy mitigation | Log injection possible |
| **Informational** | No security impact, improvement suggestion | Missing log entry |

### 6.4 Regression Workflow

1. **Discovery**: Red team identifies vulnerability during scenario execution
2. **Triage**: Security team assesses severity and impact
3. **Fix**: Development team implements remediation
4. **Regression test**: Add test case to appropriate test file:
   - Protocol attacks → `p2p-node/tests/network_protocol_attacks.rs`
   - Crypto attacks → `p2p-node/tests/crypto_penetration_tests.rs`
   - DoS attacks → `p2p-node/tests/load_tests.rs`
   - Fiat attacks → `fiat/tests/adversarial_tests.rs`
   - Transport attacks → `opendex-transport/tests/multi_node.rs`
5. **Fuzz target**: If applicable, add new fuzz target to `fuzz/fuzz_targets/`
6. **Documentation**: Update `audit/bug_log.md` with finding and remediation
7. **Verification**: Re-run scenario to confirm fix

---

## 7. Monitoring & Observability

### 7.1 Metrics Collection

**Prometheus metrics exposed by each node:**
- `libp2p_connections_total` — Active peer connections
- `libp2p_messages_received_total` — Messages received via GossipSub
- `libp2p_messages_rejected_total` — Messages rejected by validation
- `opendex_trades_active` — Active trade count
- `opendex_auth_failures_total` — Authentication failures
- `opendex_rate_limit_hits_total` — Rate limiter activations
- `opendex_message_replay_detected_total` — Replay attack detections

### 7.2 Grafana Dashboards

**Dashboard 1: Network Health**
- Peer connection count over time
- Message throughput (received/rejected)
- Latency between nodes

**Dashboard 2: Attack Detection**
- Authentication failures by source IP
- Rate limiter activations
- Replay attack detections
- Message rejection reasons

**Dashboard 3: System Resources**
- CPU, memory, disk usage per node
- Network bandwidth utilization
- Process restart count

### 7.3 Log Aggregation

**Vector/Loki configuration:**
- Collect logs from all nodes via Filebeat
- Index by node ID, scenario ID, and severity
- Alert on critical events (e.g., OOM, panic, authentication failure spike)

---

## 8. Incident Response Workflow

### 8.1 During Red Team Testing

1. **Detection**: Monitoring alerts or agent reports abnormal behavior
2. **Containment**: Isolate affected node from testnet
3. **Analysis**: Review logs, metrics, and node state
4. **Classification**: Determine severity and impact
5. **Remediation**: Apply fix or workaround
6. **Recovery**: Restart node and verify functionality
7. **Documentation**: Record incident in `redteam/reports/`

### 8.2 Post-Test Review

1. **Debrief**: Team reviews all findings and false positives
2. **Prioritization**: Rank findings by severity and exploitability
3. **Remediation plan**: Assign fixes to development sprints
4. **Regression tests**: Ensure all findings have corresponding tests
5. **Plan update**: Update this document with lessons learned

---

## 9. Regression Integration

### 9.1 Test File Mapping

| Scenario Category | Regression Test File | Fuzz Target |
|-------------------|---------------------|-------------|
| Protocol attacks | `p2p-node/tests/network_protocol_attacks.rs` | `fuzz/fuzz_targets/message.rs` |
| Crypto attacks | `p2p-node/tests/crypto_penetration_tests.rs` | `fuzz/fuzz_targets/auth_token.rs` |
| DoS attacks | `p2p-node/tests/load_tests.rs` | N/A (integration test) |
| Fiat attacks | `fiat/tests/adversarial_tests.rs` | `fuzz/fuzz_targets/fiat_trade.rs` |
| Transport attacks | `opendex-transport/tests/multi_node.rs` | `fuzz/fuzz_targets/message.rs` |

### 9.2 CI/CD Integration

**GitHub Actions workflow:**
1. **Unit tests**: Run all 692 tests on every PR
2. **Fuzz tests**: Run 13 fuzz targets for 60 seconds each on nightly builds
3. **Adversarial tests**: Run 26 fiat scenarios on every PR
4. **Supply chain**: Run `cargo-deny` on every PR
5. **Red team scenarios**: Run on dedicated schedule (weekly) when VPS available

### 9.3 Test Coverage Tracking

- **Codecov**: Track line coverage for all crates
- **Fuzz coverage**: Track fuzz target coverage via `cargo-fuzz`
- **Scenario coverage**: Track which scenarios have been executed and passed

---

## 10. CI/CD Integration

### 10.1 Automated Red Team Cycles

**Weekly schedule:**
- **Monday**: Protocol attack scenarios
- **Wednesday**: Crypto + Fiat attack scenarios
- **Friday**: DoS + Transport attack scenarios

**Trigger conditions:**
- New code merged to main branch
- Security-related dependencies updated
- Manual trigger via GitHub Actions

### 10.2 Result Reporting

- **Slack/Discord integration**: Post summary of test results
- **GitHub Issues**: Automatically create issues for failed scenarios
- **Dashboard**: Grafana dashboard shows test pass/fail history

---

## Appendix A — Code Path Mapping

### A.1 Protocol Attack Code Paths

| File | Function | Role |
|------|----------|------|
| `p2p-node/src/network.rs` | `build_swarm()` | libp2p swarm configuration |
| `p2p-node/src/message_handler.rs` | `handle_gossipsub_message()` | Message dispatch by type |
| `p2p-node/src/message_handler.rs` | `publish_message()` | JSON serialization and GossipSub publish |
| `opendex-transport/src/envelope.rs` | `ProtocolEnvelope` | Message structure with `message_id`, `sequence` |
| `opendex-transport/src/message_store.rs` | `MessageStore` | Replay protection via message ID tracking |

### A.2 Cryptographic Attack Code Paths

| File | Function | Role |
|------|----------|------|
| `user/src/auth.rs` | `AuthService` | Registration, login, HMAC-SHA256 token signing |
| `user/src/auth.rs` | `RateLimiter` | Sliding-window rate limiting with max-key eviction |
| `fiat/src/` | Various | Cryptographic operations in fiat trades |

### A.3 DoS Attack Code Paths

| File | Function | Role |
|------|----------|------|
| `p2p-node/src/network.rs` | Connection management | Peer connection limits |
| `p2p-node/src/trade_state.rs` | Pending state management | MAX_PENDING_TRADE_STATES limit |
| `opendex-transport/src/message_store.rs` | Message storage | Storage limits |
| `crates/p2p-node/tests/load_tests.rs` | Multi-node tests | Currently ignored, requires VPS |

### A.4 Fiat Attack Code Paths

| File | Function | Role |
|------|----------|------|
| `fiat/src/trade.rs` | Fiat trade state machine | Trade state transitions |
| `fiat/src/settlement_engine.rs` | Settlement logic | Payment verification and settlement |
| `fiat/src/bank_adapter.rs` | Bank API integration | Payment detection |
| `crates/fiat/tests/adversarial_tests.rs` | 26 adversarial scenarios | In-process adversarial tests |

### A.5 Transport Attack Code Paths

| File | Function | Role |
|------|----------|------|
| `opendex-transport/src/envelope.rs` | `ProtocolEnvelope` | Envelope structure and validation |
| `opendex-transport/src/message_store.rs` | `MessageStore` | Replay protection |
| `opendex-transport/src/libp2p_transport.rs` | `Libp2pTransport` | libp2p transport implementation |
| `opendex-transport/src/transport.rs` | `Transport` trait | Send/receive async methods |

---

## Appendix B — VPS Provisioning Checklist

### B.1 Instance Setup

- [ ] Create VPC with isolated subnet
- [ ] Provision 4 honest node instances (2 vCPU, 4 GB RAM, 40 GB SSD)
- [ ] Provision 1 attacker node instance (4 vCPU, 8 GB RAM, 80 GB SSD)
- [ ] Provision 1 monitoring instance (2 vCPU, 4 GB RAM, 100 GB SSD)
- [ ] Configure security groups:
  - Honest nodes: Allow inbound from attacker node only (libp2p ports)
  - Attacker node: Allow inbound SSH only
  - Monitoring: Allow inbound SSH and Grafana port

### B.2 Software Installation

- [ ] Install Ubuntu 24.04 LTS base image
- [ ] Install Rust toolchain via `rustup`
- [ ] Install Docker and Docker Compose
- [ ] Install Prometheus node exporter
- [ ] Configure UFW firewall rules

### B.3 Network Configuration

- [ ] Assign static internal IPs to all instances
- [ ] Configure DNS records for testnet domains
- [ ] Verify libp2p connectivity between nodes
- [ ] Test GossipSub message propagation

### B.4 Monitoring Setup

- [ ] Deploy Prometheus on monitoring instance
- [ ] Deploy Grafana on monitoring instance
- [ ] Configure node exporters on all instances
- [ ] Create dashboards per Section 7.2
- [ ] Configure alerting rules

### B.5 Key Management

- [ ] Generate test cryptographic keys per instance
- [ ] Store keys in secure location (e.g., HashiCorp Vault)
- [ ] Verify no key reuse across instances
- [ ] Document key rotation procedure

---

## Appendix C — Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Testnet contamination | Low | High | Isolated VPC, separate DNS zones |
| Resource exhaustion during DoS tests | High | Medium | Auto-restart scripts, resource limits |
| False positives in timing tests | Medium | Low | Calibrate thresholds, multiple iterations |
| Key leakage to production | Low | Critical | Separate key stores, automated key rotation |
| VPS cost overruns | Medium | Low | Budget alerts, auto-shutdown after tests |
| Attacker node compromise | Low | High | Minimal attack surface, regular updates |

---

## Appendix D — Quick Reference

### Scenario Count by Category

| Category | Scenarios | Existing Tests | New Scenarios |
|----------|-----------|----------------|---------------|
| Protocol | 8 | `network_protocol_attacks.rs` | Live network variants |
| Crypto | 7 | `crypto_penetration_tests.rs` | Live token replay, timing |
| DoS | 6 | `load_tests.rs` (ignored) | Connection exhaustion, state bloat |
| Fiat | 7 | `adversarial_tests.rs` (26) | Live payment spoofing |
| Transport | 4 | `multi_node.rs` | Envelope tampering, MessageStore bypass |
| **Total** | **32** | **Multiple files** | **All require VPS** |

### File Locations

| Purpose | Path |
|---------|------|
| Red team plan | `audit/red_team_plan.md` |
| Infrastructure setup | `redteam/infra/` |
| Attacker agent | `redteam/agent/` |
| Scenario definitions | `redteam/scenarios/` |
| Report templates | `redteam/reports/` |
| Existing fuzz targets | `fuzz/fuzz_targets/` |
| Existing p2p-node tests | `crates/p2p-node/tests/` |
| Existing fiat tests | `crates/fiat/tests/` |
| Existing transport tests | `crates/opendex-transport/tests/` |
