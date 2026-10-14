# Testnet Test Results

This directory stores all test results, reports, and verification artifacts for the OpenDex testnet deployment.

## Directory Structure

```
test-results/
├── README.md           ← This file
├── reports/            ← Test reports and summaries
├── logs/               ← Raw logs from test runs
├── snapshots/          ← Screenshots or state snapshots
└── verification/       ← Verification scripts and checklists
```

## How to Use

### Running Tests

After deploying the testnet, run the verification checks:

```bash
# Quick health check
bash docs/testnet/test-results/verification/health-check.sh

# Full end-to-end test
bash docs/testnet/test-results/verification/e2e-test.sh
```

### Recording Results

- **Reports** (`reports/`) — Save markdown or JSON summaries of test runs.
- **Logs** (`logs/`) — Capture raw output from node logs, wallet RPC responses, or HTTP checks.
- **Snapshots** (`snapshots/`) — Store screenshots of the web portal or node state dumps.
- **Verification** (`verification/`) — Keep test scripts and checklists here.

### Example: Recording a Test Run

```bash
# Create a timestamped report
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
bash docs/testnet/test-results/verification/health-check.sh \
  > "docs/testnet/test-results/logs/health-check-${TIMESTAMP}.log" 2>&1
```

## Test Categories

1. **Web Portal Tests** — Verify Nginx serves pages, HTTPS works, WASM loads.
2. **P2P Node Tests** — Verify bootstrap and test nodes are running and connected.
3. **Wallet RPC Tests** — Verify Monero stagenet and Bitcoin testnet RPCs respond.
4. **End-to-End Tests** — Place orders, verify gossipsub sync, check trade execution.

## Compliance

Test results support compliance documentation in `audit/regulatory_compliance/`. Key areas:
- **Audit trails** — Node logs show all order/trade operations.
- **KYC/AML readiness** — Test that identity verification flows work.
- **Data retention** — Verify persistence layer saves and reloads state.
