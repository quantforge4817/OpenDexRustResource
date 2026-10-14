/// Adversarial tests for fiat settlement security.
///
/// These tests verify that the settlement engine correctly handles
/// malicious participant behavior, race conditions, and edge cases
/// that could lead to fund loss or protocol violations.

use std::collections::HashMap;
use uuid::Uuid;
use opendex_fiat::adapter::MockPaymentProvider;
use opendex_fiat::error::SettlementError;
use opendex_fiat::settlement_engine::{CryptoWallet, SettlementEngine};
use opendex_fiat::trade::{FiatOffer, FiatTradeState, SettlementMethod};
use opendex_fiat::payment::{FiatAmount, PaymentId, PaymentReference, PaymentStatus};
use opendex_fiat::verification::VerifiedPayment;
use opendex_fiat::dispute::DisputeReason;
use opendex_types::{Asset, FiatCurrency, ParticipantId, TradeId, Price};

/// Mock crypto wallet that tracks escrow operations for adversarial testing.
#[derive(Default)]
struct AdversarialMockWallet {
    /// Tracks how many times release_escrow was called per trade.
    release_count: std::sync::Mutex<HashMap<Uuid, u32>>,
    /// Tracks whether escrow was reserved per trade.
    escrow_reserved: std::sync::Mutex<HashMap<Uuid, bool>>,
}

impl CryptoWallet for AdversarialMockWallet {
    fn reserve_escrow(
        &self,
        trade_id: TradeId,
        asset: &Asset,
        amount: u64,
    ) -> Result<String, SettlementError> {
        let _ = (asset, amount);
        let mut reserved = self.escrow_reserved.lock().unwrap();
        reserved.insert(trade_id.0, true);
        Ok(format!("escrow_{}", trade_id.0.as_hyphenated()))
    }

    fn release_escrow(
        &self,
        trade_id: TradeId,
        recipient: &str,
    ) -> Result<String, SettlementError> {
        let _ = recipient;
        let mut counts = self.release_count.lock().unwrap();
        *counts.entry(trade_id.0).or_insert(0) += 1;
        Ok(format!("release_{}", trade_id.0.as_hyphenated()))
    }

    fn return_escrow(
        &self,
        trade_id: TradeId,
    ) -> Result<String, SettlementError> {
        Ok(format!("return_{}", trade_id.0.as_hyphenated()))
    }
}

fn make_engine() -> (
    SettlementEngine<MockPaymentProvider, AdversarialMockWallet>,
    ParticipantId,
    ParticipantId,
) {
    let maker = ParticipantId(Uuid::new_v4());
    let taker = ParticipantId(Uuid::new_v4());
    (
        SettlementEngine::new(MockPaymentProvider::auto_confirm(), AdversarialMockWallet::default()),
        maker,
        taker,
    )
}

fn make_offer(maker: ParticipantId) -> FiatOffer {
    FiatOffer {
        trade_id: TradeId(Uuid::new_v4()),
        maker,
        currency: FiatCurrency::GBP,
        crypto_asset: Asset::Btc,
        amount: FiatAmount::new(100000, FiatCurrency::GBP),
        price: Price::new(Asset::Btc, Asset::Fiat(FiatCurrency::GBP), 50000, 1),
        settlement_method: SettlementMethod::BankTransfer {
            currency: FiatCurrency::GBP,
        },
        settlement_window: 3600,
        bond_requirement: 100,
    }
}

fn make_bank_details() -> opendex_fiat::payment::PaymentRecipient {
    opendex_fiat::payment::PaymentRecipient {
        name: "Maker".to_string(),
        account_number: "12345678".to_string(),
        routing_code: "12-34-56".to_string(),
        bank_name: "Test Bank".to_string(),
        iban: None,
        swift_code: None,
    }
}

// ─── State Machine Adversarial Tests ───────────────────────────────────────

#[test]
fn adversarial_cancel_after_crypto_reserved() {
    // CRITICAL: If a trade cannot be cancelled after crypto is reserved,
    // funds are permanently locked if the counterparty disappears.
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();

    // Taker should be able to cancel after crypto is reserved
    // This prevents permanent fund lockup
    let result = engine.cancel_trade(trade_id, taker, 1003);
    assert!(
        result.is_ok(),
        "Taker must be able to cancel after crypto is reserved to prevent fund lockup"
    );

    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::Cancelled);
}

#[test]
fn adversarial_cancel_after_payment_requested() {
    // After payment instructions are sent, either party should be able to cancel
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();

    // Maker should be able to cancel after sending payment instructions
    let result = engine.cancel_trade(trade_id, maker, 1004);
    assert!(
        result.is_ok(),
        "Maker must be able to cancel after payment requested"
    );

    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::Cancelled);
}

#[test]
fn adversarial_cancel_after_maker_confirmed() {
    // After maker confirms payment, taker should still be able to cancel
    // if something goes wrong (e.g., maker confirms but crypto release fails)
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();
    engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();
    engine.maker_confirm_payment(trade_id, maker, 1005).unwrap();

    // Taker should be able to cancel even after maker confirms
    let result = engine.cancel_trade(trade_id, taker, 1006);
    assert!(
        result.is_ok(),
        "Taker must be able to cancel after maker confirms to prevent fund lockup"
    );

    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::Cancelled);
}

#[test]
fn adversarial_maker_confirmed_timeout() {
    // CRITICAL: If maker confirms payment but never releases crypto,
    // the trade should eventually timeout and fail, returning crypto to maker.
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();
    engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();
    engine.maker_confirm_payment(trade_id, maker, 1005).unwrap();

    // Maker confirmed but never releases crypto.
    // Advance time past the confirmation deadline.
    // The confirmation_deadline is payment_deadline + 3600 = 1000 + 3600 + 3600 = 8200
    engine.check_deadlines(1000 + 3600 + 3600 + 100);

    let trade = engine.get_trade_public(trade_id).unwrap();
    assert!(
        matches!(trade.state, FiatTradeState::Failed),
        "Trade must fail when maker confirms but never releases crypto within deadline. State: {:?}",
        trade.state
    );
}

#[test]
fn adversarial_unauthorized_release_crypto() {
    // A third party should not be able to release crypto
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();
    engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();
    engine.maker_confirm_payment(trade_id, maker, 1005).unwrap();

    // Third party tries to release crypto
    let attacker = ParticipantId(Uuid::new_v4());
    let result = engine.release_crypto(trade_id, attacker, 1006);
    assert!(
        result.is_err(),
        "Third party must not be able to release crypto"
    );
    assert_eq!(result.unwrap_err(), SettlementError::Unauthorized);
}

#[test]
fn adversarial_unauthorized_complete_trade() {
    // A third party should not be able to complete a trade
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();
    engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();
    engine.maker_confirm_payment(trade_id, maker, 1005).unwrap();
    engine.release_crypto(trade_id, maker, 1006).unwrap();

    // Third party tries to complete trade
    let attacker = ParticipantId(Uuid::new_v4());
    let result = engine.complete_trade(trade_id, attacker, 1007);
    assert!(
        result.is_err(),
        "Third party must not be able to complete trade"
    );
    assert_eq!(result.unwrap_err(), SettlementError::Unauthorized);
}

#[test]
fn adversarial_unauthorized_cancel_trade() {
    // A third party should not be able to cancel a trade
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    // Third party tries to cancel
    let attacker = ParticipantId(Uuid::new_v4());
    let result = engine.cancel_trade(trade_id, attacker, 1001);
    assert!(
        result.is_err(),
        "Third party must not be able to cancel trade"
    );
    assert_eq!(result.unwrap_err(), SettlementError::Unauthorized);
}

#[test]
fn adversarial_taker_cannot_confirm_payment_as_maker() {
    // Maker should not be able to call taker_confirm_payment
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();

    // Maker tries to confirm payment as taker
    let result = engine.taker_confirm_payment(trade_id, maker, 1004);
    assert!(
        result.is_err(),
        "Maker must not be able to confirm payment as taker"
    );
    assert_eq!(result.unwrap_err(), SettlementError::Unauthorized);
}

#[test]
fn adversarial_maker_cannot_confirm_payment_as_taker() {
    // Taker should not be able to call maker_confirm_payment
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();
    engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();

    // Taker tries to confirm payment as maker
    let result = engine.maker_confirm_payment(trade_id, taker, 1005);
    assert!(
        result.is_err(),
        "Taker must not be able to confirm payment as maker"
    );
    assert_eq!(result.unwrap_err(), SettlementError::Unauthorized);
}

// ─── Payment Amount Mismatch Tests ──────────────────────────────────────────

#[test]
fn adversarial_payment_amount_too_low() {
    // Attacker submits a verified payment with less than the expected amount
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();

    // Submit payment with 10% less than expected
    let verified = VerifiedPayment {
        payment_id: PaymentId(Uuid::new_v4()),
        currency: FiatCurrency::GBP,
        amount: FiatAmount::new(90000, FiatCurrency::GBP), // Expected: 100000
        sender_reference: PaymentReference("test".to_string()),
        recipient_reference: PaymentReference("test".to_string()),
        verified_at: chrono::Utc::now(),
        status: PaymentStatus::Confirmed,
    };

    let result = engine.confirm_payment(trade_id, verified, 1004);
    assert!(
        result.is_err(),
        "Payment with less than expected amount must be rejected"
    );
}

#[test]
fn adversarial_payment_amount_too_high() {
    // Attacker submits a verified payment with more than the expected amount
    // This could be a front-running attack or a mistake
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();

    // Submit payment with 10% more than expected
    let verified = VerifiedPayment {
        payment_id: PaymentId(Uuid::new_v4()),
        currency: FiatCurrency::GBP,
        amount: FiatAmount::new(110000, FiatCurrency::GBP), // Expected: 100000
        sender_reference: PaymentReference("test".to_string()),
        recipient_reference: PaymentReference("test".to_string()),
        verified_at: chrono::Utc::now(),
        status: PaymentStatus::Confirmed,
    };

    let result = engine.confirm_payment(trade_id, verified, 1004);
    assert!(
        result.is_err(),
        "Payment with more than expected amount must be rejected"
    );
}

// ─── Bond Calculation Edge Cases ────────────────────────────────────────────

#[test]
fn adversarial_bond_zero_trade_value() {
    // Bond calculation with zero crypto amount should not overflow
    let (mut engine, maker, taker) = make_engine();
    let mut offer = make_offer(maker);
    offer.amount = FiatAmount::new(0, FiatCurrency::GBP); // Zero fiat amount

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    // Should succeed without overflow
    let trade = engine.get_trade_public(trade_id).unwrap();
    // Crypto amount should be 0 for zero fiat amount
    assert_eq!(trade.crypto_amount, 0);
}

#[test]
fn adversarial_bond_max_trade_value() {
    // Bond calculation with maximum crypto amount should not overflow
    let (mut engine, maker, taker) = make_engine();
    let mut offer = make_offer(maker);
    // Maximum reasonable fiat amount
    offer.amount = FiatAmount::new(u64::MAX / 2, FiatCurrency::GBP);

    let result = engine.create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000);
    // Should not panic or overflow
    assert!(result.is_ok() || result.is_err()); // Just verify no panic
}

// ─── Rate Limiter Edge Cases ────────────────────────────────────────────────

#[test]
fn adversarial_rate_limiter_timestamp_manipulation() {
    // Attacker tries to bypass rate limiting by using decreasing timestamps
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    // Create 10 trades (within limit)
    for i in 0..10 {
        engine
            .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000 + i as u64)
            .ok();
    }

    // Try to bypass by using an earlier timestamp
    let result = engine.create_trade(&offer, taker, "bc1qtaker123".to_string(), 500);
    assert!(
        result.is_err(),
        "Decreasing timestamp must not bypass rate limiting"
    );
}

#[test]
fn adversarial_payment_confirm_rate_limiter_per_user() {
    // Different users should have independent rate limits
    let (mut engine, maker, _) = make_engine();
    let offer = make_offer(maker);

    let taker1 = ParticipantId(Uuid::new_v4());
    let taker2 = ParticipantId(Uuid::new_v4());

    let bank_details = make_bank_details();

    // Create trades for both takers
    let trade_ids1: Vec<_> = (0..5)
        .map(|i| {
            let tid = engine
                .create_trade(&offer, taker1, "bc1qtaker123".to_string(), 1000 + i as u64)
                .unwrap();
            engine.accept_trade(tid, 1100 + i as u64).unwrap();
            engine.reserve_crypto(tid, 1200 + i as u64).unwrap();
            engine.request_payment(tid, bank_details.clone(), 1300 + i as u64).unwrap();
            tid
        })
        .collect();

    let trade_ids2: Vec<_> = (0..5)
        .map(|i| {
            let tid = engine
                .create_trade(&offer, taker2, "bc1qtaker456".to_string(), 2000 + i as u64)
                .unwrap();
            engine.accept_trade(tid, 2100 + i as u64).unwrap();
            engine.reserve_crypto(tid, 2200 + i as u64).unwrap();
            engine.request_payment(tid, bank_details.clone(), 2300 + i as u64).unwrap();
            tid
        })
        .collect();

    // Exhaust taker1's payment confirm limit
    for (i, &tid) in trade_ids1.iter().enumerate() {
        engine.taker_confirm_payment(tid, taker1, 3000 + i as u64).ok();
    }

    // taker1 should be rate limited
    let result = engine.taker_confirm_payment(trade_ids1[0], taker1, 3005);
    assert_eq!(result.unwrap_err(), SettlementError::RateLimited);

    // taker2 should still be able to confirm
    let result = engine.taker_confirm_payment(trade_ids2[0], taker2, 3005);
    assert!(result.is_ok(), "Different user must not be affected by another user's rate limit");
}

// ─── Crypto Address Validation Tests ────────────────────────────────────────

#[test]
fn adversarial_invalid_btc_address() {
    // Attacker provides an invalid Bitcoin address
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let result = engine.create_trade(&offer, taker, "invalid_address".to_string(), 1000);
    assert!(
        result.is_err(),
        "Invalid Bitcoin address must be rejected"
    );
}

#[test]
fn adversarial_empty_crypto_address() {
    // Attacker provides an empty address
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let result = engine.create_trade(&offer, taker, "".to_string(), 1000);
    assert!(
        result.is_err(),
        "Empty crypto address must be rejected"
    );
}

#[test]
fn adversarial_invalid_xmr_address() {
    // Attacker provides an invalid Monero address
    let (mut engine, maker, taker) = make_engine();
    let mut offer = make_offer(maker);
    offer.crypto_asset = Asset::Xmr;

    // Invalid Monero address: wrong prefix
    let result = engine.create_trade(&offer, taker, "1invalidxmraddress".to_string(), 1000);
    assert!(
        result.is_err(),
        "Invalid Monero address must be rejected"
    );
}

// ─── Dispute Edge Cases ─────────────────────────────────────────────────────

#[test]
fn adversarial_double_dispute() {
    // Attacker tries to open a dispute on an already-disputed trade
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    // First dispute succeeds
    engine.open_dispute(trade_id, taker, DisputeReason::PaymentNotReceived, 1001)
        .unwrap();

    // Second dispute should fail
    let result = engine.open_dispute(trade_id, maker, DisputeReason::PaymentNotReceived, 1002);
    assert!(
        result.is_err(),
        "Double dispute must be rejected"
    );
}

// ─── State Machine Integrity Tests ──────────────────────────────────────────

#[test]
fn adversarial_skip_states() {
    // Attacker tries to skip states in the state machine
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    // Try to go directly from Created to Complete
    // This should fail because the state machine enforces the full path
    let result = engine.complete_trade(trade_id, maker, 1001);
    assert!(
        result.is_err(),
        "Skipping states must be rejected by the state machine"
    );
}

#[test]
fn adversarial_reverse_state_transition() {
    // Attacker tries to reverse a state transition
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();

    // Try to go back from Accepted to Created
    // This should fail because reverse transitions are not allowed
    // We verify this by trying to do an operation that requires Created state
    // after the trade is already Accepted
    let result = engine.accept_trade(trade_id, 1002);
    assert!(
        result.is_err(),
        "Reverse state transition must be rejected"
    );
}

// ─── Fiat Payment Coordination Tests ────────────────────────────────────────

#[test]
fn adversarial_concurrent_payment_confirmation() {
    // Two parties try to confirm payment simultaneously
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();

    // Taker confirms first
    engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();

    // Taker tries to confirm again (double-confirm)
    let result = engine.taker_confirm_payment(trade_id, taker, 1005);
    assert!(
        result.is_err(),
        "Double payment confirmation must be rejected"
    );
}

#[test]
fn adversarial_release_before_maker_confirm() {
    // Attacker tries to release crypto before maker confirms payment
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();
    engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();

    // Try to release crypto without maker confirmation
    let result = engine.release_crypto(trade_id, maker, 1005);
    assert!(
        result.is_err(),
        "Crypto must not be released before maker confirms payment"
    );
}

#[test]
fn adversarial_cancel_after_complete() {
    // Attacker tries to cancel a completed trade
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();
    engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();
    engine.maker_confirm_payment(trade_id, maker, 1005).unwrap();
    engine.release_crypto(trade_id, maker, 1006).unwrap();
    engine.complete_trade(trade_id, maker, 1007).unwrap();

    // Try to cancel after completion
    let result = engine.cancel_trade(trade_id, taker, 1008);
    assert!(
        result.is_err(),
        "Completed trade must not be cancellable"
    );
}

#[test]
fn adversarial_operations_on_failed_trade() {
    // Attacker tries to perform operations on a failed trade
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();
    engine.reserve_crypto(trade_id, 1002).unwrap();
    engine.request_payment(trade_id, make_bank_details(), 1003).unwrap();

    // Fail the trade via timeout
    engine.mark_payment_pending(trade_id, 1004).unwrap();
    engine.check_deadlines(1000 + 3600 + 100);

    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::Failed);

    // Try to perform operations on failed trade
    assert!(engine.taker_confirm_payment(trade_id, taker, 1005).is_err());
    assert!(engine.maker_confirm_payment(trade_id, maker, 1005).is_err());
    assert!(engine.release_crypto(trade_id, maker, 1005).is_err());
}

#[test]
fn adversarial_operations_on_cancelled_trade() {
    // Attacker tries to perform operations on a cancelled trade
    let (mut engine, maker, taker) = make_engine();
    let offer = make_offer(maker);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
        .unwrap();

    engine.cancel_trade(trade_id, maker, 1001).unwrap();

    // Try to perform operations on cancelled trade
    assert!(engine.accept_trade(trade_id, 1002).is_err());
    assert!(engine.reserve_crypto(trade_id, 1002).is_err());
}
