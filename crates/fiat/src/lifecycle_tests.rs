//! Integration tests for the complete fiat payment lifecycle.
//!
//! Tests the full flow: maker creates offer → taker accepts → crypto reserved
//! → payment instructions → payment submitted → detected → confirmed → crypto
//! released → trade complete.
//!
//! Also tests failure paths: timeout, dispute, and reversal.

use crate::*;
use opendex_types::{Asset, FiatCurrency, ParticipantId, TradeId};
use uuid::Uuid;

// ─── Helper: Create a test trade ────────────────────────────────────────────

fn make_test_trade() -> FiatTrade {
    FiatTrade {
        id: TradeId(Uuid::new_v4()),
        maker: ParticipantId(Uuid::new_v4()),
        taker: ParticipantId(Uuid::new_v4()),
        crypto_asset: Asset::Btc,
        fiat_currency: FiatCurrency::GBP,
        crypto_amount: 1_000_000,
        fiat_amount: FiatAmount::new(7500000, FiatCurrency::GBP),
        settlement_method: SettlementMethod::BankTransfer {
            currency: FiatCurrency::GBP,
        },
        fee_schedule: FeeSchedule::bank_transfer_default(),
        state: FiatTradeState::Created,
        maker_bank_details: None,
        taker_crypto_address: "bc1qtest123".to_string(),
        payment_id: None,
        payment_reference: None,
        deadlines: SettlementDeadlines {
            payment_deadline: 1_000_000,
            confirmation_deadline: 2_000_000,
            dispute_deadline: 3_000_000,
        },
        created_at: 1,
        updated_at: 1,
    }
}

// ─── Happy Path: Complete Trade Lifecycle ───────────────────────────────────

#[test]
fn complete_trade_lifecycle() {
    let mut trade = make_test_trade();

    // 1. Trade created
    assert_eq!(trade.state, FiatTradeState::Created);

    // 2. Both parties accept
    trade.transition(FiatTradeState::Accepted, 2).unwrap();
    assert_eq!(trade.state, FiatTradeState::Accepted);

    // 3. Crypto reserved in escrow
    trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();
    assert_eq!(trade.state, FiatTradeState::CryptoReserved);

    // 4. Payment instructions sent
    trade
        .transition(FiatTradeState::PaymentRequested, 4)
        .unwrap();
    assert_eq!(trade.state, FiatTradeState::PaymentRequested);

    // 5. Taker submits payment
    trade.transition(FiatTradeState::PaymentPending, 5).unwrap();
    assert_eq!(trade.state, FiatTradeState::PaymentPending);

    // 6. Payment detected by adapter
    trade.transition(FiatTradeState::Detected, 6).unwrap();
    assert_eq!(trade.state, FiatTradeState::Detected);

    // 7. Payment under confirmation
    trade.transition(FiatTradeState::Confirming, 7).unwrap();
    assert_eq!(trade.state, FiatTradeState::Confirming);

    // 8. Payment confirmed
    trade.transition(FiatTradeState::Confirmed, 8).unwrap();
    assert_eq!(trade.state, FiatTradeState::Confirmed);

    // 9. Crypto released to taker
    trade.transition(FiatTradeState::CryptoRelease, 9).unwrap();
    assert_eq!(trade.state, FiatTradeState::CryptoRelease);

    // 10. Trade complete
    trade.transition(FiatTradeState::Complete, 10).unwrap();
    assert_eq!(trade.state, FiatTradeState::Complete);
}

// ─── Failure Path: Payment Timeout ──────────────────────────────────────────

#[test]
fn payment_timeout_failure() {
    let mut trade = make_test_trade();

    trade.transition(FiatTradeState::Accepted, 2).unwrap();
    trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();
    trade
        .transition(FiatTradeState::PaymentRequested, 4)
        .unwrap();
    trade.transition(FiatTradeState::PaymentPending, 5).unwrap();

    // Payment deadline expires — trade fails
    trade.transition(FiatTradeState::Failed, 6).unwrap();
    assert_eq!(trade.state, FiatTradeState::Failed);
}

// ─── Dispute Path ───────────────────────────────────────────────────────────

#[test]
fn dispute_from_payment_pending() {
    let mut trade = make_test_trade();

    trade.transition(FiatTradeState::Accepted, 2).unwrap();
    trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();
    trade
        .transition(FiatTradeState::PaymentRequested, 4)
        .unwrap();
    trade.transition(FiatTradeState::PaymentPending, 5).unwrap();

    // Taker opens dispute
    trade.transition(FiatTradeState::Disputed, 6).unwrap();
    assert_eq!(trade.state, FiatTradeState::Disputed);

    // Dispute resolved in favor of taker — trade completed
    trade.transition(FiatTradeState::Complete, 7).unwrap();
    assert_eq!(trade.state, FiatTradeState::Complete);
}

#[test]
fn dispute_resolved_with_refund() {
    let mut trade = make_test_trade();

    trade.transition(FiatTradeState::Accepted, 2).unwrap();
    trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();
    trade
        .transition(FiatTradeState::PaymentRequested, 4)
        .unwrap();
    trade.transition(FiatTradeState::PaymentPending, 5).unwrap();
    trade.transition(FiatTradeState::Detected, 6).unwrap();

    // Maker opens dispute — payment amount incorrect
    trade.transition(FiatTradeState::Disputed, 7).unwrap();

    // Dispute resolved — refund
    trade.transition(FiatTradeState::Failed, 8).unwrap();
    assert_eq!(trade.state, FiatTradeState::Failed);
}

// ─── Reversal Path ──────────────────────────────────────────────────────────

#[test]
fn payment_reversal_after_completion() {
    let mut trade = make_test_trade();

    // Complete the trade first
    complete_trade(&mut trade);
    assert_eq!(trade.state, FiatTradeState::Complete);

    // Payment reversal detected
    trade
        .transition(FiatTradeState::ReversalDetected, 11)
        .unwrap();
    assert_eq!(trade.state, FiatTradeState::ReversalDetected);

    // Dispute opened for reversal
    trade.transition(FiatTradeState::Disputed, 12).unwrap();
    assert_eq!(trade.state, FiatTradeState::Disputed);
}

fn complete_trade(trade: &mut FiatTrade) {
    trade.transition(FiatTradeState::Accepted, 2).unwrap();
    trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();
    trade
        .transition(FiatTradeState::PaymentRequested, 4)
        .unwrap();
    trade.transition(FiatTradeState::PaymentPending, 5).unwrap();
    trade.transition(FiatTradeState::Detected, 6).unwrap();
    trade.transition(FiatTradeState::Confirming, 7).unwrap();
    trade.transition(FiatTradeState::Confirmed, 8).unwrap();
    trade.transition(FiatTradeState::CryptoRelease, 9).unwrap();
    trade.transition(FiatTradeState::Complete, 10).unwrap();
}

// ─── Mock Payment Provider Integration ──────────────────────────────────────

#[test]
fn mock_provider_full_payment_flow() {
    let provider = MockPaymentProvider::auto_confirm();

    // Create payment request
    let request = PaymentRequest {
        trade_id: TradeId(Uuid::new_v4()),
        amount: FiatAmount::new(10000, FiatCurrency::GBP),
        reference: PaymentReference::generate(),
        payer_id: ParticipantId(Uuid::new_v4()),
        payee_id: ParticipantId(Uuid::new_v4()),
    };

    // Create payment
    let payment = provider.create_payment(request).unwrap();
    assert_eq!(payment.status, PaymentStatus::Confirmed);

    // Check status
    let status = provider.check_status(&payment).unwrap();
    assert_eq!(status, PaymentStatus::Confirmed);

    // Verify payment
    let verified = provider.verify(&payment).unwrap();
    assert_eq!(verified.status, PaymentStatus::Confirmed);
    assert_eq!(verified.currency, FiatCurrency::GBP);
    assert_eq!(verified.amount.minor_units, 10000);
}

#[test]
fn mock_provider_pending_then_verify_fails() {
    let provider = MockPaymentProvider::new();

    let request = PaymentRequest {
        trade_id: TradeId(Uuid::new_v4()),
        amount: FiatAmount::new(10000, FiatCurrency::EUR),
        reference: PaymentReference::generate(),
        payer_id: ParticipantId(Uuid::new_v4()),
        payee_id: ParticipantId(Uuid::new_v4()),
    };

    let payment = provider.create_payment(request).unwrap();
    assert_eq!(payment.status, PaymentStatus::Created);

    // Cannot verify a payment that hasn't been confirmed
    assert!(provider.verify(&payment).is_err());
}

// ─── Security Bond Integration ──────────────────────────────────────────────

#[test]
fn bond_lifecycle_with_trade() {
    let trade_id = TradeId(Uuid::new_v4());
    let maker = ParticipantId(Uuid::new_v4());

    // Maker locks bond
    let mut maker_bond = SecurityBond::new(trade_id, maker, Asset::Btc, 100_000, 1);
    assert_eq!(maker_bond.state, BondState::Required);

    maker_bond.lock(2).unwrap();
    assert_eq!(maker_bond.state, BondState::Locked);

    // Trade completes — bond released
    maker_bond.release(3).unwrap();
    assert_eq!(maker_bond.state, BondState::Released);
}

#[test]
fn bond_slashed_on_misbehavior() {
    let trade_id = TradeId(Uuid::new_v4());
    let taker = ParticipantId(Uuid::new_v4());

    let mut taker_bond = SecurityBond::new(trade_id, taker, Asset::Btc, 100_000, 1);
    taker_bond.lock(2).unwrap();

    // Taker fails to pay — bond slashed
    taker_bond
        .slash("Payment not received by deadline", "Rule 3.1", 3)
        .unwrap();

    assert_eq!(taker_bond.state, BondState::Slashed);
    assert!(taker_bond.slash_reason.is_some());
    assert!(taker_bond.rule_violated.is_some());
}

// ─── Dispute Integration ────────────────────────────────────────────────────

#[test]
fn dispute_with_evidence_collection() {
    let trade_id = TradeId(Uuid::new_v4());
    let maker = ParticipantId(Uuid::new_v4());
    let taker = ParticipantId(Uuid::new_v4());

    let mut dispute = Dispute::new(trade_id, maker, DisputeReason::IncorrectAmount, 1);

    // Evidence collection phase
    dispute
        .transition(DisputeState::EvidenceCollection, 2)
        .unwrap();

    // Taker submits bank statement
    let evidence = Evidence {
        id: Uuid::new_v4(),
        kind: EvidenceType::BankStatement,
        content_hash: "sha256:abc123".to_string(),
        storage_reference: "s3://evidence/bank-statement.pdf".to_string(),
        submitted_by: taker,
        timestamp: 3,
    };
    dispute.add_evidence(evidence, 3);

    // Under review
    dispute.transition(DisputeState::UnderReview, 4).unwrap();

    // Resolved
    dispute
        .resolve("Payment amount verified, trade completed", 5)
        .unwrap();

    assert_eq!(dispute.state, DisputeState::Resolved);
    assert_eq!(dispute.evidence.len(), 1);
    assert!(dispute.resolution.is_some());
}

// ─── Invalid State Transitions ──────────────────────────────────────────────

#[test]
fn cannot_skip_states() {
    let mut trade = make_test_trade();

    // Cannot jump from Created to Complete
    assert!(trade.transition(FiatTradeState::Complete, 2).is_err());

    // Cannot jump from Created to Confirmed
    assert!(trade.transition(FiatTradeState::Confirmed, 2).is_err());

    // Cannot go backwards from Accepted to Created
    trade.transition(FiatTradeState::Accepted, 2).unwrap();
    assert!(trade.transition(FiatTradeState::Created, 3).is_err());
}

#[test]
fn cannot_complete_from_failed() {
    let mut trade = make_test_trade();
    trade.transition(FiatTradeState::Accepted, 2).unwrap();
    trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();
    trade.transition(FiatTradeState::Failed, 4).unwrap();

    // Cannot complete a failed trade
    assert!(trade.transition(FiatTradeState::Complete, 5).is_err());
}

// ─── Fiat Amount Arithmetic ─────────────────────────────────────────────────

#[test]
fn fiat_amount_arithmetic() {
    let a = FiatAmount::new(10000, FiatCurrency::GBP);
    let b = FiatAmount::new(5000, FiatCurrency::GBP);

    let sum = a.checked_add(&b).unwrap();
    assert_eq!(sum.minor_units, 15000);

    let diff = sum.checked_sub(&a).unwrap();
    assert_eq!(diff.minor_units, 5000);

    // Cannot add different currencies
    let c = FiatAmount::new(5000, FiatCurrency::EUR);
    assert!(a.checked_add(&c).is_none());
}

// ─── Settlement Method ──────────────────────────────────────────────────────

#[test]
fn settlement_method_currency() {
    let method = SettlementMethod::BankTransfer {
        currency: FiatCurrency::EUR,
    };
    assert_eq!(method.currency(), FiatCurrency::EUR);

    let method = SettlementMethod::CashInPerson {
        currency: FiatCurrency::USD,
    };
    assert_eq!(method.currency(), FiatCurrency::USD);
}

// ─── Manual Flow: Button-based Confirmation ─────────────────────────────────

#[test]
fn manual_trade_lifecycle() {
    // Manual flow: maker sends bank details → taker pays → taker confirms →
    // maker confirms → crypto released → complete
    let mut trade = make_test_trade();

    // 1. Trade created
    assert_eq!(trade.state, FiatTradeState::Created);

    // 2. Both parties accept
    trade.transition(FiatTradeState::Accepted, 2).unwrap();

    // 3. Crypto reserved in escrow
    trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();

    // 4. Maker sends bank details (payment instructions)
    trade
        .transition(FiatTradeState::PaymentRequested, 4)
        .unwrap();
    assert_eq!(trade.state, FiatTradeState::PaymentRequested);

    // 5. Taker makes payment externally, then clicks "I've paid"
    trade
        .transition(FiatTradeState::PaymentSubmitted, 5)
        .unwrap();
    assert_eq!(trade.state, FiatTradeState::PaymentSubmitted);

    // 6. Maker checks bank account, clicks "I confirm payment"
    trade.transition(FiatTradeState::MakerConfirmed, 6).unwrap();
    assert_eq!(trade.state, FiatTradeState::MakerConfirmed);

    // 7. Crypto released to taker
    trade.transition(FiatTradeState::CryptoRelease, 7).unwrap();
    assert_eq!(trade.state, FiatTradeState::CryptoRelease);

    // 8. Trade complete
    trade.transition(FiatTradeState::Complete, 8).unwrap();
    assert_eq!(trade.state, FiatTradeState::Complete);
}

#[test]
fn manual_flow_cannot_skip_maker_confirm() {
    let mut trade = make_test_trade();
    trade.transition(FiatTradeState::Accepted, 2).unwrap();
    trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();
    trade
        .transition(FiatTradeState::PaymentRequested, 4)
        .unwrap();
    trade
        .transition(FiatTradeState::PaymentSubmitted, 5)
        .unwrap();

    // Cannot skip maker confirmation and go straight to crypto release
    assert!(trade.transition(FiatTradeState::CryptoRelease, 6).is_err());
    assert_eq!(trade.state, FiatTradeState::PaymentSubmitted);
}

#[test]
fn manual_flow_timeout() {
    // If maker doesn't confirm in time, trade fails
    let mut trade = make_test_trade();
    trade.transition(FiatTradeState::Accepted, 2).unwrap();
    trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();
    trade
        .transition(FiatTradeState::PaymentRequested, 4)
        .unwrap();
    trade
        .transition(FiatTradeState::PaymentSubmitted, 5)
        .unwrap();

    // Timeout — maker never confirmed
    trade.transition(FiatTradeState::Failed, 6).unwrap();
    assert_eq!(trade.state, FiatTradeState::Failed);
}

#[test]
fn manual_flow_dispute_from_payment_submitted() {
    // Either party can open a dispute after taker confirms payment
    let mut trade = make_test_trade();
    trade.transition(FiatTradeState::Accepted, 2).unwrap();
    trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();
    trade
        .transition(FiatTradeState::PaymentRequested, 4)
        .unwrap();
    trade
        .transition(FiatTradeState::PaymentSubmitted, 5)
        .unwrap();

    // Maker opens dispute — claims no payment received
    trade.transition(FiatTradeState::Disputed, 6).unwrap();
    assert_eq!(trade.state, FiatTradeState::Disputed);
}
