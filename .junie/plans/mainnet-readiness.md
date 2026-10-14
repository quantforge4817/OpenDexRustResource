---
sessionId: session-261014-102955-1n0v
---

# Plan

Implement all 13 open audit findings: H10-H14 (High), M21-M25 (Medium), L30, L31, L33 (Low). See answer for full details.

# Details

7 stages: 1) miner fees + HashMap panics, 2) RwLock panics, 3) rate limiting, 4) message validation, 5) payment detection + address validation, 6) credentials + deps, 7) log redaction + E2E tests.

# Delivery Steps

### ✓ Step 1: Fix miner fees and HashMap panics
Add miner fee to payout tx and fix HashMap unwrap calls in multisig.rs

### ✓ Step 2: Fix RwLock panics
Replace 26 RwLock unwrap calls in crypto_wallet.rs with panic-safe recovery

### ✓ Step 3: Add rate limiting
Implement sliding window rate limiting for auth (login: 5/min) and settlement (trade creation: 10/hr, payment confirm: 5/min)

### ✓ Step 4: Add P2P message validation and timestamp checks
Reject stale messages (>30s) in message_handler.rs and add schema version to Message struct

### ✓ Step 5: Implement payment detection and address validation
Wire up check_payment() to call PaymentAdapter and validate crypto addresses in create_trade()

### ✓ Step 6: Zeroize credentials, tighten deny.toml, cleanup deps
Zeroize wallet passwords on drop, add critical crate version denials, remove unused deps

### ✓ Step 7: Add log redaction and E2E fiat integration tests
Create tracing layer that redacts sensitive fields and add full fiat lifecycle integration test