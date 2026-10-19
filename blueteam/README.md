# Blue Team — Defensive Verification & Monitoring

The blue team infrastructure provides **defensive verification and monitoring** that runs alongside the [red team](../redteam/) attacks. While the red team injects offensive attacks into the testnet, the blue team observes, detects, and validates that defenses are working correctly.

## Goal

Enable an **attack → detect → respond → remediate** cycle:
1. The red team executes attacks against the testnet
2. The blue team monitors and validates defenses in real time
3. Prometheus alerts fire when attacks are detected
4. Grafana dashboards show defense health and detection rates
5. Findings feed back into regression tests

## Directory Structure

```
blueteam/
├── README.md                    # This file
├── incident_response.md         # Playbook for blue team operations
├── agent/                       # Blue team observer agent (Rust crate)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs              # Agent entry point
│       ├── protocol_defenses.rs # Protocol defense validation
│       ├── crypto_defenses.rs   # Crypto defense validation
│       ├── dos_defenses.rs      # DoS defense validation
│       ├── fiat_defenses.rs     # Fiat defense validation
│       └── transport_defenses.rs # Transport defense validation
├── scenarios/                   # Defense validation scenarios
│   ├── network.md               # BD-001 through BD-008 (Protocol)
│   ├── crypto.md                # BD-009 through BD-015 (Crypto)
│   ├── application.md           # BD-016 through BD-022 (Application)
│   ├── fiat.md                  # BD-023 through BD-029 (Fiat)
│   └── transport.md             # BD-030 through BD-032 (Transport)
├── infra/                       # Infrastructure configuration
│   ├── docker-compose.blue.yml  # Blue team Docker Compose overlay
│   ├── Dockerfile.observer      # Blue team observer container
│   └── monitoring/
│       ├── alert_rules.yml      # Prometheus alert rules
│       └── grafana/
│           ├── blue-team-dashboard.json # Grafana dashboard
│           ├── dashboards/
│           │   └── dashboards.yml
│           └── datasources/
│               └── datasource.yml
└── reports/
    └── template.md              # Blue team report template
```

## Relationship to Red Team

| Red Team | Blue Team |
|----------|-----------|
| `redteam/agent/` — attack injection | `blueteam/agent/` — defense observation |
| `redteam/scenarios/` — attack definitions | `blueteam/scenarios/` — defense validations |
| `redteam/reports/template.md` — attack results | `blueteam/reports/template.md` — detection results |
| Attacker node (5th node) | Observer node (6th node) |

Each red team attack scenario (PA-001 through FA-007) has a corresponding blue team defense validation (BD-001 through BD-032).

## Quick Start

### Prerequisites

- Docker and Docker Compose
- Rust toolchain (for building the agent locally)

### Run with Red Team

```bash
# Start the full testnet with red team + blue team
cd redteam/infra
docker compose -f docker-compose.yml -f ../blueteam/infra/docker-compose.blue.yml up -d
```

### Run Blue Team Agent Locally

```bash
# Build and run the observer agent
cargo run -p blueteam-agent -- --peer-addr /ip4/127.0.0.1/tcp/30001/p2p/<peer-id>
```

### Monitoring

- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000 (admin/admin)
- **Blue Team Dashboard**: Pre-loaded in Grafana under "Blue Team Defense Monitor"

## Defense Validation Results

Each defense validation produces one of four results:

| Result | Meaning |
|--------|---------|
| ✅ **PASS** | Defense worked as expected |
| ❌ **FAIL** | Defense failed — attack succeeded |
| ⚠️ **PARTIAL** | Defense partially worked — some gaps detected |
| ❓ **ERROR** | Unable to validate — monitoring error |

## Alert Rules

The blue team extends Prometheus with these alert rules:

| Alert | Condition |
|-------|-----------|
| `AttackDetected` | Message rejection rate > 10x baseline |
| `ReplayAttackDetected` | `opendex_message_replay_detected_total` increases |
| `AuthFailureSpike` | Auth failures > 10/min from single source |
| `RateLimiterActivated` | Rate limiter triggers |
| `DoSSuspected` | Connection count > 80% of max |
| `MemoryPressure` | Node memory > 80% of limit |
| `StateBloatDetected` | Pending trade states > 90% of max |
| `InvalidStateTransition` | State machine rejects transition |

## Incident Response

See [incident_response.md](incident_response.md) for the structured playbook covering:
- Pre-attack checklist
- During-attack procedures
- Post-attack analysis
- Gap analysis

## Report Generation

After running defense validations, generate a report using [reports/template.md](reports/template.md). The report focuses on:
- Detection coverage (what was detected vs. what was missed)
- Response times (time from attack to detection)
- Defense gaps (attacks that succeeded)
- Recommendations for defense improvements
