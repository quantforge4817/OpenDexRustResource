# OpenDEX Security / Correctness Audit

## 1. Scope

Audited components:
- crates/protocol/src/message.rs
- crates/protocol/src/state.rs
- crates/p2p-node/src/order_factory.rs
- crates/p2p-node/src/orderbook.rs
- crates/p2p-node/src/trades.rs
- crates/p2p-node/src/balances.rs
- crates/p2p-node/src/settlement.rs
- crates/p2p-node/src/trade_state.rs
- crates/p2p-node/src/sync.rs

## 2. Severity

- Critical
- High
- Medium
- Low
- Informational

## 3. Findings

### AUDIT-001 — [Title]

**Severity:** High  
**Status:** Open  
**Location:** `crates/...:line`

**Description:**

...

**Impact:**

...

**Recommendation:**

...

**Evidence:**

...

## 4. Positive Findings

- ...
- ...

## 5. Test Coverage

- [ ] Settlement tests
- [ ] Balance overflow tests
- [ ] Duplicate trade tests
- [ ] Invalid transition tests
- [ ] Snapshot consistency tests
- [ ] Order matching tests
- [ ] Malicious/malformed message tests

## 6. Overall Assessment

Status: Pending

Critical: 0
High: 0
Medium: 0
Low: 0
Informational: 0
