# Fiat Defense Validation Scenarios

These scenarios validate that fiat payment defenses correctly detect and mitigate attacks targeting payment verification, settlement logic, and multisig enforcement. Each scenario corresponds to a red team attack in [redteam/scenarios/fiat.md](../../redteam/scenarios/fiat.md).

## Monitored Code Paths

| File | Function | Defense Role |
|------|----------|--------------|
| `fiat/src/trade.rs` | Fiat trade state machine | Trade state transitions |
| `fiat/src/settlement_engine.rs` | Settlement logic | Payment verification and settlement |
| `fiat/src/bank_adapter.rs` | Bank API integration | Payment detection |
| `fiat/src/multisig.rs` | Multisig escrow | Withdrawal enforcement |

---

## BD-023: Payment Verification

**Corresponds to:** FA-001: False Payment Detection  
**Severity:** High  
**Description:** Verify that false payment claims are rejected.

### Defense Validation
1. Monitor payment verification while red team executes FA-001
2. Track acceptance/rejection of false payment claims
3. Verify bank adapter checks against actual bank API

### Expected Defense Behavior
- Bank adapter verifies payment with actual bank API
- False claims without real bank transfers are rejected
- Trade remains in pending state without real payment

### Validation Criteria
- [ ] All false payment claims rejected (0 acceptances)
- [ ] Bank adapter verification works correctly
- [ ] Trade state not advanced without real payment

### Detection Metrics
- `opendex_fiat_payment_rejected_total` — should increase with each false claim
- `opendex_fiat_trade_state` — should remain in pending state
- Bank adapter query count should increase

### Blue Team Agent Implementation
`blueteam/agent/src/fiat_defenses.rs::validate_payment_verification()`

---

## BD-024: Reversal Detection

**Corresponds to:** FA-002: Payment Reversal  
**Severity:** High  
**Description:** Verify that payment reversals are detected and trades are paused.

### Defense Validation
1. Monitor trade state while red team executes FA-002
2. Verify reversal detection window is active
3. Verify trade is paused when reversal is detected

### Expected Defense Behavior
- Reversal detection window is active after payment confirmation
- Trade is paused immediately when reversal is detected
- Funds are protected and not released

### Validation Criteria
- [ ] Reversal is detected within the detection window
- [ ] Trade is paused (state changed to Paused or similar)
- [ ] Funds are not released to the seller

### Detection Metrics
- `opendex_fiat_reversal_detected_total` — should increase
- `opendex_fiat_trade_state` — should show paused state
- Trade completion should be blocked

### Blue Team Agent Implementation
`blueteam/agent/src/fiat_defenses.rs::validate_reversal_detection()`

---

## BD-025: Atomic Transitions

**Corresponds to:** FA-003: Race Condition Exploit  
**Severity:** Critical  
**Description:** Verify that concurrent payment claims are handled atomically.

### Defense Validation
1. Monitor state transitions while red team executes FA-003
2. Count how many of the 10 concurrent claims succeed
3. Verify only one claim succeeds

### Expected Defense Behavior
- Atomic state transitions prevent race conditions
- Only one of the 10 concurrent claims succeeds
- Other 9 claims are rejected

### Validation Criteria
- [ ] Only one claim succeeds (exactly 1 success)
- [ ] No double-payment occurs
- [ ] State machine remains consistent

### Detection Metrics
- `opendex_fiat_payment_success_total` — should show exactly 1 success
- `opendex_fiat_payment_rejected_total` — should show 9 rejections
- State machine should have no inconsistent states

### Blue Team Agent Implementation
`blueteam/agent/src/fiat_defenses.rs::validate_atomic_transitions()`

---

## BD-026: Chargeback Monitoring

**Corresponds to:** FA-004: Chargeback Simulation  
**Severity:** High  
**Description:** Verify that chargebacks are detected after settlement.

### Defense Validation
1. Monitor chargeback detection while red team executes FA-004
2. Verify trade is flagged when chargeback is detected
3. Verify recovery procedures are triggered

### Expected Defense Behavior
- Chargeback monitoring is active post-settlement
- Trade is flagged when chargeback is detected
- Recovery procedures are triggered automatically

### Validation Criteria
- [ ] Chargeback is detected
- [ ] Trade is flagged (state changed to Chargeback or similar)
- [ ] Recovery procedures work correctly

### Detection Metrics
- `opendex_fiat_chargeback_detected_total` — should increase
- `opendex_fiat_trade_state` — should show flagged state
- Recovery procedure logs should appear

### Blue Team Agent Implementation
`blueteam/agent/src/fiat_defenses.rs::validate_chargeback_monitoring()`

---

## BD-027: Multisig Enforcement

**Corresponds to:** FA-005: 2-of-2 Multisig Edge Case  
**Severity:** Critical  
**Description:** Verify that unilateral withdrawals from 2-of-2 multisig are blocked.

### Defense Validation
1. Monitor withdrawal attempts while red team executes FA-005
2. Verify all unilateral withdrawal attempts are blocked
3. Verify funds remain in escrow

### Expected Defense Behavior
- Multisig enforcement requires exactly 2 signatures
- Unilateral withdrawals with single, invalid, or no signatures are blocked
- Funds remain securely in escrow

### Validation Criteria
- [ ] All unilateral withdrawals blocked (0 successes)
- [ ] Multisig enforcement works correctly
- [ ] Funds are protected in escrow

### Detection Metrics
- `opendex_fiat_withdrawal_rejected_total` — should increase with each attempt
- `opendex_fiat_multisig_enforcement_total` — should show enforcement
- Escrow balance should remain unchanged

### Blue Team Agent Implementation
`blueteam/agent/src/fiat_defenses.rs::validate_multisig_enforcement()`

---

## BD-028: State Machine Integrity

**Corresponds to:** FA-006: Settlement Bypass  
**Severity:** High  
**Description:** Verify that invalid state transitions are rejected.

### Defense Validation
1. Monitor state transitions while red team executes FA-006
2. Verify all invalid transitions are rejected
3. Verify trade follows correct state flow

### Expected Defense Behavior
- State machine enforces valid transitions only
- Invalid transitions (Created → Completed, Created → Settled, etc.) are rejected
- Trade follows the correct state flow

### Validation Criteria
- [ ] All invalid transitions rejected (0 acceptances)
- [ ] State machine is robust against bypass attempts
- [ ] No bypass of settlement steps

### Detection Metrics
- `opendex_fiat_invalid_transition_total` — should increase with each invalid attempt
- `opendex_fiat_trade_state` — should not change from invalid transitions
- State machine logs should show rejections

### Blue Team Agent Implementation
`blueteam/agent/src/fiat_defenses.rs::validate_state_machine_integrity()`

---

## BD-029: Currency Validation

**Corresponds to:** FA-007: Cross-Currency Confusion  
**Severity:** Medium  
**Description:** Verify that currency mismatches are detected.

### Defense Validation
1. Monitor currency validation while red team executes FA-007
2. Verify all currency mismatches are detected
3. Verify payments with wrong currency are rejected

### Expected Defense Behavior
- Currency validation detects all mismatches
- Payments with wrong currency are rejected
- Trade remains in pending state

### Validation Criteria
- [ ] All currency mismatches detected (0 false acceptances)
- [ ] Payments with wrong currency are rejected
- [ ] No confusion between currencies

### Detection Metrics
- `opendex_fiat_currency_mismatch_total` — should increase with each mismatch
- `opendex_fiat_payment_rejected_total` — should include rejected mismatched payments
- Trade state should remain pending

### Blue Team Agent Implementation
`blueteam/agent/src/fiat_defenses.rs::validate_currency_validation()`