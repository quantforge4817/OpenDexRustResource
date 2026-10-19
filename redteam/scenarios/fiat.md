# Fiat Payment Attack Scenarios

These scenarios test fiat payment defenses including payment detection, reversal handling, settlement logic, and multisig enforcement.

## Target Code Paths

| File | Function | Role |
|------|----------|------|
| `fiat/src/trade.rs` | Fiat trade state machine | Trade state transitions |
| `fiat/src/settlement_engine.rs` | Settlement logic | Payment verification and settlement |
| `fiat/src/bank_adapter.rs` | Bank API integration | Payment detection |
| `crates/fiat/tests/adversarial_tests.rs` | 26 adversarial scenarios | In-process adversarial tests |

## FA-001: False Payment Detection

**Severity:** High  
**Description:** Claim payment received without actual bank transfer.

### Attack Vector
1. Create a fiat trade
2. Claim payment received without actual bank transfer
3. Submit false payment proof:
   - USD $100.00 (fake-transaction-001)
   - EUR $200.00 (fake-transaction-002)
   - GBP $150.00 (fake-transaction-003)

### Expected Behavior
- Bank adapter verifies payment with actual bank API
- False claims are rejected
- Trade remains in pending state

### Validation Criteria
- [ ] All false payment claims rejected
- [ ] Bank adapter verification works
- [ ] Trade state not advanced without real payment

### Implementation
`redteam/agent/src/fiat_attacks.rs::false_payment_detection()`

---

## FA-002: Payment Reversal

**Severity:** High  
**Description:** Reverse bank transfer after confirmation.

### Attack Vector
1. Create a fiat trade
2. Confirm payment (legitimate transfer)
3. Initiate reversal of the bank transfer
4. Monitor trade state

### Expected Behavior
- Reversal detection window is active
- Trade is paused when reversal is detected
- Funds are protected

### Validation Criteria
- [ ] Reversal is detected
- [ ] Trade is paused
- [ ] Funds are not released

### Implementation
`redteam/agent/src/fiat_attacks.rs::payment_reversal()`

---

## FA-003: Race Condition Exploit

**Severity:** Critical  
**Description:** Submit concurrent payment claims to detect TOCTOU vulnerabilities.

### Attack Vector
1. Create a fiat trade
2. Submit 10 concurrent payment claims simultaneously
3. Monitor which claims succeed

### Expected Behavior
- Atomic state transitions prevent race conditions
- Only one claim succeeds
- Other claims are rejected

### Validation Criteria
- [ ] Only one claim succeeds
- [ ] No double-payment
- [ ] State machine remains consistent

### Implementation
`redteam/agent/src/fiat_attacks.rs::race_condition_exploit()`

---

## FA-004: Chargeback Simulation

**Severity:** High  
**Description:** Initiate chargeback after settlement.

### Attack Vector
1. Complete a fiat trade with settlement
2. Initiate chargeback through bank
3. Monitor chargeback detection

### Expected Behavior
- Chargeback monitoring is active
- Trade is flagged when chargeback detected
- Recovery procedures are triggered

### Validation Criteria
- [ ] Chargeback is detected
- [ ] Trade is flagged
- [ ] Recovery procedures work

### Implementation
`redteam/agent/src/fiat_attacks.rs::chargeback_simulation()`

---

## FA-005: 2-of-2 Multisig Edge Case

**Severity:** Critical  
**Description:** Attempt unilateral withdrawal from 2-of-2 multisig.

### Attack Vector
1. Create a 2-of-2 P2WSH multisig escrow
2. Attempt withdrawal with:
   - Single signature
   - Invalid signature
   - No signature
3. Monitor withdrawal results

### Expected Behavior
- Multisig enforcement requires 2 signatures
- Unilateral withdrawals are blocked
- Funds remain in escrow

### Validation Criteria
- [ ] All unilateral withdrawals blocked
- [ ] Multisig enforcement works
- [ ] Funds are protected

### Implementation
`redteam/agent/src/fiat_attacks.rs::multisig_edge_case()`

---

## FA-006: Settlement Bypass

**Severity:** High  
**Description:** Attempt to skip settlement steps in the state machine.

### Attack Vector
1. Create a fiat trade
2. Attempt invalid state transitions:
   - Created → Completed (skip payment)
   - Created → Settled (skip payment and confirmation)
   - PaymentPending → Completed (skip confirmation)
   - PaymentReceived → Cancelled (invalid cancellation)

### Expected Behavior
- State machine enforces valid transitions
- Invalid transitions are rejected
- Trade follows correct flow

### Validation Criteria
- [ ] All invalid transitions rejected
- [ ] State machine is robust
- [ ] No bypass of settlement steps

### Implementation
`redteam/agent/src/fiat_attacks.rs::settlement_bypass()`

---

## FA-007: Cross-Currency Confusion

**Severity:** Medium  
**Description:** Mix currencies in payment to test currency validation.

### Attack Vector
1. Create a trade expecting USD payment
2. Send EUR payment instead
3. Create a trade expecting EUR payment
4. Send GBP payment instead
5. Create a trade expecting BTC payment
6. Send XMR payment instead

### Expected Behavior
- Currency validation detects mismatch
- Payment is rejected
- Trade remains in pending state

### Validation Criteria
- [ ] All currency mismatches detected
- [ ] Payments rejected
- [ ] No confusion between currencies

### Implementation
`redteam/agent/src/fiat_attacks.rs::cross_currency_confusion()`
