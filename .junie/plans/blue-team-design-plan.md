---
sessionId: session-261019-153047-7uho
---

# Requirements

### Overview & Goals

The red team infrastructure (`redteam/`) is a complete offensive testing framework: 5-node testnet (4 honest + 1 attacker), a Rust-based `red-team-agent` that injects protocol, crypto, DoS, fiat, and transport attacks, scenario definitions, and reporting templates. The blue team plan complements this by creating a **defensive verification and monitoring infrastructure** that runs alongside the red team attacks to detect, validate, and respond to them in real time.

The goal is to enable an **attack → detect → respond → remediate** cycle: the red team attacks, the blue team monitors and validates defenses, and findings feed back into regression tests.

### User Stories

- As a security engineer, I want to deploy a blue team monitor alongside the red team testnet so I can observe attack detection in real time.
- As a developer, I want automated defense validation scenarios so I can verify that each red team attack is properly detected and mitigated.
- As an operator, I want alerting dashboards that fire when attacks succeed, so I know which defenses need attention.
- As a security auditor, I want structured blue team reports that complement red team findings, showing what was detected, how fast, and what the response was.

### Scope

**In Scope:**
- Blue team monitoring agent crate (complements `redteam/agent`)
- Defense validation scenarios (one per red team attack category)
- Alerting rules for Prometheus/Grafana
- Blue team Docker Compose integration (extends existing `redteam/infra/docker-compose.yml`)
- Blue team report template
- Incident response playbook for blue team operations
- Defense verification test files in existing test directories

**Out of Scope:**
- Modifying the core application crates (`p2p-node`, `fiat`, `opendex-transport`, `user`) — the blue team observes, it doesn't change product code
- Production deployment tooling (this is testnet-only)
- GUI/frontend security testing

### Functional Requirements

1. **Blue Team Monitor Agent**: A Rust crate that connects to the testnet as an observer node, collecting metrics, logs, and network events from honest nodes.
2. **Defense Validation Scenarios**: For each red team attack category (Protocol, Crypto, DoS, Fiat, Transport), define what the blue team should observe when the attack runs.
3. **Real-time Alerting**: Prometheus alert rules that fire when attack signatures are detected (e.g., spike in rejected messages, auth failures, replay detections).
4. **Grafana Blue Team Dashboard**: Dedicated dashboard showing defense health, attack detection rates, and response times.
5. **Report Generation**: Structured blue team reports documenting detection coverage, response times, and gaps.

### Non-Functional Requirements

- Blue team infrastructure must not interfere with red team attacks or honest node operation
- Monitoring overhead should be < 5% CPU on observer node
- Alert latency should be < 30 seconds from attack execution to alert firing
- All blue team artifacts must be reproducible via Docker Compose


# Technical Design

### Current Implementation

The red team infrastructure consists of:
- **`redteam/agent/`** — Rust crate with attack modules (`protocol_attacks.rs`, `crypto_attacks.rs`, `dos_attacks.rs`, `fiat_attacks.rs`)
- **`redteam/infra/`** — Docker Compose with 4 honest nodes + 1 attacker + Prometheus + Grafana
- **`redteam/scenarios/`** — Attack scenario definitions (`network.md`, `crypto.md`, `application.md`, `fiat.md`)
- **`redteam/reports/template.md`** — Red team report template
- **`audit/red_team_plan.md`** — Comprehensive action plan with incident response workflow

The honest nodes expose Prometheus metrics at `/metrics` (configured in `redteam/infra/monitoring/prometheus.yml`). Grafana dashboards exist for network health, attack detection, and system resources.

### Key Decisions

1. **Blue team as observer node, not a separate service**: The blue team monitor joins the testnet as a 6th node (like the attacker is a 5th). This gives it access to the same GossipSub messages and peer events, enabling it to detect attacks as they propagate.

2. **Defense scenarios mirror attack scenarios**: Each red team scenario (PA-001 through FA-007) gets a corresponding blue team validation scenario (BD-001 through BD-032). This ensures 1:1 coverage.

3. **Alerting via existing Prometheus/Grafana**: Rather than building a separate alerting system, we extend the existing monitoring infrastructure with blue team-specific alert rules and dashboards.

4. **Blue team agent as a separate Rust crate**: Placed at `blueteam/agent/` to mirror the `redteam/agent/` structure, keeping concerns cleanly separated.

### Proposed Changes

#### 1. Blue Team Agent Crate (`blueteam/agent/`)

A Rust crate that connects to the testnet as an observer peer. It mirrors the red team agent's structure:

```
blueteam/
├── README.md
├── agent/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs              # Agent entry point, libp2p swarm setup
│       ├── protocol_defenses.rs # Protocol defense validation
│       ├── crypto_defenses.rs   # Crypto defense validation
│       ├── dos_defenses.rs      # DoS defense validation
│       ├── fiat_defenses.rs     # Fiat defense validation
│       └── transport_defenses.rs # Transport defense validation
├── scenarios/
│   ├── network.md               # Protocol defense scenarios
│   ├── crypto.md                # Crypto defense scenarios
│   ├── application.md           # Application defense scenarios
│   ├── fiat.md                  # Fiat defense scenarios
│   └── transport.md             # Transport defense scenarios
├── infra/
│   └── docker-compose.blue.yml  # Blue team services (extends red team compose)
└── reports/
    └── template.md              # Blue team report template
```

The agent:
- Connects to the testnet via libp2p/GossipSub (same as red team agent)
- Subscribes to all topics to observe message flow
- Validates that defenses are working by checking:
  - Message rejection rates match expected patterns
  - Replay protection is active (MessageStore tracking)
  - Rate limiting triggers correctly
  - Auth failures are logged and rate-limited
  - State machine transitions are valid
- Produces structured results (Pass/Fail/Partial/Error) per scenario

#### 2. Defense Validation Scenarios

Each red team attack gets a blue team defense validation:

| Red Team Scenario | Blue Team Validation | What to Observe |
|---|---|---|
| PA-001: Malformed Message | BD-001: Message Parsing Robustness | All malformed messages rejected, no crashes, attacker not banned |
| PA-002: Replay Attack | BD-002: Replay Protection | MessageStore rejects duplicates, only first message processed |
| PA-003: Sequence Manipulation | BD-003: Sequence Validation | Out-of-order messages rejected or reordered correctly |
| PA-004: Topic Hijacking | BD-004: Topic Isolation | Messages to wrong topics ignored |
| PA-005: Peer Identity Spoofing | BD-005: Peer Authentication | Spoofed peer IDs rejected |
| PA-006: GossipSub Flooding | BD-006: Message Rate Limiting | Flood messages rate-limited, honest traffic unaffected |
| PA-007: Kademlia Poisoning | BD-007: DHT Integrity | Poisoned records rejected or expired |
| PA-008: Connection Exhaustion | BD-008: Connection Limits | Max connections enforced, excess rejected |
| CA-001: HMAC Token Replay | BD-009: Token Single-Use | Token rejected after first use |
| CA-002: Token Timing | BD-010: Constant-Time Comparison | Timing variance < 1ms |
| CA-003: Key Extraction | BD-011: Rate Limiter Activation | Rate limiter blocks after ~10 attempts |
| CA-004: Signature Malleability | BD-012: Signature Strictness | All modified signatures rejected |
| CA-005: Key Uniqueness | BD-013: Key Generation Quality | All keys unique, CSPRNG used |
| CA-006: Argon2id Bypass | BD-014: Password Hash Strength | No collisions, proper parameters |
| CA-007: DLEQ Proof Forgery | BD-015: Proof Verification | All forged proofs rejected |
| TA-001: Envelope Tampering | BD-016: Envelope Integrity | Tampered envelopes rejected |
| TA-002: Sequence Manipulation | BD-017: Sequence Validation | Invalid sequences rejected |
| TA-003: MessageStore Bypass | BD-018: Replay Protection | No bypass possible |
| TA-004: Transport Fuzzing | BD-019: Transport Robustness | No crashes from fuzzed input |
| DA-002: State Bloat | BD-020: State Limits | State limits enforced |
| DA-004: Resource Starvation | BD-021: Memory Limits | Memory bounded, no OOM |
| DA-005: Slowloris | BD-022: Connection Timeouts | Idle connections closed |
| FA-001: False Payment | BD-023: Payment Verification | False claims rejected |
| FA-002: Payment Reversal | BD-024: Reversal Detection | Reversals detected, trade paused |
| FA-003: Race Condition | BD-025: Atomic Transitions | Only one claim succeeds |
| FA-004: Chargeback | BD-026: Chargeback Monitoring | Chargebacks detected |
| FA-005: Multisig Edge Case | BD-027: Multisig Enforcement | Unilateral withdrawals blocked |
| FA-006: Settlement Bypass | BD-028: State Machine Integrity | Invalid transitions rejected |
| FA-007: Cross-Currency | BD-029: Currency Validation | Currency mismatches detected |

#### 3. Extended Monitoring Infrastructure

**New Prometheus alert rules** (`blueteam/infra/monitoring/alert_rules.yml`):
- `AttackDetected` — fires when message rejection rate spikes > 10x baseline
- `ReplayAttackDetected` — fires when `opendex_message_replay_detected_total` increases
- `AuthFailureSpike` — fires when auth failures > 10/min from single source
- `RateLimiterActivated` — fires when rate limiter triggers
- `DoSSuspected` — fires when connection count > 80% of max
- `MemoryPressure` — fires when node memory > 80% of limit
- `StateBloatDetected` — fires when pending trade states > 90% of max
- `InvalidStateTransition` — fires when state machine rejects transition

**New Grafana Blue Team Dashboard**:
- Panel 1: Attack Detection Timeline — shows when attacks were detected vs executed
- Panel 2: Defense Health Score — composite metric of all defense validations
- Panel 3: Message Flow Analysis — valid vs rejected messages over time
- Panel 4: Authentication Security — auth failures, rate limiter activations
- Panel 5: Resource Protection — memory, CPU, connection limits
- Panel 6: Fiat Trade Security — payment verification, reversal detection

#### 4. Docker Compose Integration

A `blueteam/infra/docker-compose.blue.yml` that extends the red team setup:
- Adds the blue team observer node (6th node in the testnet)
- Adds alertmanager for Prometheus alerts
- Adds blue team Grafana dashboard provisioning
- Shares the same `testnet` network

#### 5. Blue Team Report Template

Mirrors the red team report template but focuses on:
- Detection coverage (what was detected vs what was missed)
- Response times (time from attack to detection)
- Defense gaps (attacks that succeeded)
- Recommendations for defense improvements

#### 6. Incident Response Playbook

`blueteam/incident_response.md` — structured playbook for blue team operations:
- Pre-attack checklist (verify monitoring is active, baselines established)
- During-attack procedures (observe, document, alert)
- Post-attack analysis (compare red team findings with blue team detections)
- Gap analysis (what the blue team missed)

### Architecture Diagram

``` mermaid
graph LR
    subgraph Testnet
        H1[Honest Node 1]
        H2[Honest Node 2]
        H3[Honest Node 3]
        H4[Honest Node 4]
        R[Red Team Agent]
        B[Blue Team Observer]
    end
    subgraph Monitoring
        P[Prometheus]
        A[Alertmanager]
        G[Grafana]
    end
    R -->|Attacks| H1
    R -->|Attacks| H2
    R -->|Attacks| H3
    R -->|Attacks| H4
    B -->|Observes| H1
    B -->|Observes| H2
    B -->|Observes| H3
    B -->|Observes| H4
    H1 -->|Metrics| P
    H2 -->|Metrics| P
    H3 -->|Metrics| P
    H4 -->|Metrics| P
    P -->|Alerts| A
    P -->|Data| G
    B -->|Results| G
    A -->|Notifications| G
```

### Risks

- **Blue team observer may affect network behavior**: Adding a 6th node changes GossipSub mesh. Mitigation: configure the observer as a passive subscriber with minimal mesh participation.
- **Detection gaps**: The blue team may not detect all attacks. Mitigation: defense scenarios are validated against known attack patterns; gaps are documented and fed back into monitoring improvements.
- **Alert fatigue**: Too many alerts may obscure real issues. Mitigation: alert rules use thresholds based on baseline metrics, not absolute counts.


# Testing

### Validation Approach

The blue team infrastructure is validated by running it alongside the red team attacks and verifying that:
1. The blue team observer connects to the testnet successfully
2. Defense validation scenarios produce correct results when red team attacks run
3. Prometheus alerts fire when expected
4. Grafana dashboards display correct data
5. Blue team reports are generated correctly

### Key Scenarios

1. **End-to-end attack-detection cycle**: Run a red team attack (e.g., PA-001), verify the blue team detects it, and the Prometheus alert fires.
2. **Replay attack detection**: Run PA-002, verify BD-002 confirms MessageStore is working, and the `ReplayAttackDetected` alert fires.
3. **DoS protection**: Run DA-004 (resource starvation), verify BD-021 confirms memory limits, and the `MemoryPressure` alert fires.
4. **Fiat trade security**: Run FA-003 (race condition), verify BD-025 confirms atomic transitions.
5. **Full scenario coverage**: Run all red team scenarios and verify all blue team validations pass.

### Edge Cases

- Blue team observer disconnects during attack — should handle reconnection gracefully
- Honest node crashes during attack — blue team should detect and report the crash
- Multiple attacks run simultaneously — blue team should correlate and report each independently
- No attacks running — blue team should report all defenses as healthy (baseline validation)


# Delivery Steps

### ✓ Step 1: Create blue team directory structure and documentation
Set up the `blueteam/` directory with README, scenario definitions, report template, and incident response playbook.
- Create `blueteam/README.md` documenting the blue team infrastructure, its relationship to the red team, and usage instructions.
- Create `blueteam/scenarios/network.md` with protocol defense validation scenarios (BD-001 through BD-008), mirroring the red team's `redteam/scenarios/network.md`.
- Create `blueteam/scenarios/crypto.md` with crypto defense validation scenarios (BD-009 through BD-015), mirroring `redteam/scenarios/crypto.md`.
- Create `blueteam/scenarios/application.md` with application defense validation scenarios (BD-016 through BD-022), mirroring `redteam/scenarios/application.md`.
- Create `blueteam/scenarios/fiat.md` with fiat defense validation scenarios (BD-023 through BD-029), mirroring `redteam/scenarios/fiat.md`.
- Create `blueteam/scenarios/transport.md` with transport defense validation scenarios (BD-030 through BD-032).
- Create `blueteam/reports/template.md` — blue team report template focusing on detection coverage, response times, and defense gaps.
- Create `blueteam/incident_response.md` — structured playbook for blue team operations (pre-attack, during-attack, post-attack procedures).

### ✓ Step 2: Implement blue team agent crate
Create the `blueteam/agent/` Rust crate that connects to the testnet as an observer node.
- Create `blueteam/agent/Cargo.toml` with dependencies (libp2p, tracing, serde, clap, anyhow) mirroring the red team agent.
- Create `blueteam/agent/src/main.rs` — agent entry point with libp2p swarm setup, GossipSub subscription to all topics, CLI for scenario selection, and structured result output.
- Create `blueteam/agent/src/protocol_defenses.rs` — validates protocol defenses (message parsing, replay protection, sequence validation, topic isolation, peer auth, rate limiting, DHT integrity, connection limits).
- Create `blueteam/agent/src/crypto_defenses.rs` — validates crypto defenses (token single-use, constant-time comparison, rate limiter, signature strictness, key uniqueness, password hash strength, proof verification).
- Create `blueteam/agent/src/dos_defenses.rs` — validates DoS defenses (state limits, memory limits, connection timeouts).
- Create `blueteam/agent/src/fiat_defenses.rs` — validates fiat defenses (payment verification, reversal detection, atomic transitions, chargeback monitoring, multisig enforcement, state machine integrity, currency validation).
- Create `blueteam/agent/src/transport_defenses.rs` — validates transport defenses (envelope integrity, replay protection, transport robustness).

### ✓ Step 3: Extend monitoring infrastructure with blue team alerting
Add blue team-specific Prometheus alert rules, Grafana dashboards, and Docker Compose integration.
- Create `blueteam/infra/monitoring/alert_rules.yml` with alert rules: `AttackDetected`, `ReplayAttackDetected`, `AuthFailureSpike`, `RateLimiterActivated`, `DoSSuspected`, `MemoryPressure`, `StateBloatDetected`, `InvalidStateTransition`.
- Create `blueteam/infra/monitoring/grafana/blue-team-dashboard.json` — Grafana dashboard with panels for attack detection timeline, defense health score, message flow analysis, authentication security, resource protection, and fiat trade security.
- Create `blueteam/infra/monitoring/grafana/dashboards/dashboards.yml` — Grafana dashboard provisioning config.
- Create `blueteam/infra/docker-compose.blue.yml` — extends the red team Docker Compose by adding the blue team observer node and Alertmanager service.
- Update `redteam/infra/monitoring/prometheus.yml` to include the blue team alert rules file and scrape the blue team observer node.

### ✓ Step 4: Integrate blue team with root Cargo workspace and validate
Wire the blue team agent into the project's Cargo workspace and verify the complete setup.
- Update the root `Cargo.toml` workspace members to include `blueteam/agent`.
- Verify the blue team agent compiles (`cargo check -p blueteam-agent`).
- Verify the Docker Compose blue team overlay starts correctly alongside the red team infrastructure.
- Create a `blueteam/infra/Dockerfile.observer` for the blue team observer container, mirroring `redteam/infra/Dockerfile.attacker`.
- Validate the end-to-end flow: red team attack → blue team detection → Prometheus alert → Grafana dashboard update.