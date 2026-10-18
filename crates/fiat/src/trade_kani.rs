//! Kani formal verification proof harnesses for `FiatTradeState`.
//!
//! Proves properties about the 16-state FiatTradeState machine (256 possible pairs)
//! using symbolic execution, ensuring correctness for all possible inputs.

use crate::payment::FiatAmount;
use crate::trade::{FeeSchedule, FiatTrade, FiatTradeState, SettlementDeadlines, SettlementMethod};
use opendex_types::{Asset, FiatCurrency, ParticipantId, TradeId};
use uuid::Uuid;

/// Prove that only valid transitions return `true` among all 256 state pairs (16×16).
#[kani::proof]
fn verify_fiat_trade_transition_graph() {
    let current: FiatTradeState = kani::any();
    let next: FiatTradeState = kani::any();

    let valid = current.can_transition_to(&next);

    // Prove that if valid, the pair is one of the known valid transitions
    if valid {
        assert!(matches!(
            (&current, &next),
            // Normal flow — both paths share the early states
            (FiatTradeState::Created, FiatTradeState::Accepted)
                    | (FiatTradeState::Accepted, FiatTradeState::CryptoReserved)
                    | (FiatTradeState::CryptoReserved, FiatTradeState::PaymentRequested)
                // Manual (button-based) flow
                    | (FiatTradeState::PaymentRequested, FiatTradeState::PaymentSubmitted)
                    | (FiatTradeState::PaymentSubmitted, FiatTradeState::MakerConfirmed)
                    | (FiatTradeState::MakerConfirmed, FiatTradeState::CryptoRelease)
                // Automatic (adapter-based) flow
                    | (FiatTradeState::PaymentRequested, FiatTradeState::PaymentPending)
                    | (FiatTradeState::PaymentPending, FiatTradeState::Detected)
                    | (FiatTradeState::Detected, FiatTradeState::Confirming)
                    | (FiatTradeState::Confirming, FiatTradeState::Confirmed)
                    | (FiatTradeState::Confirmed, FiatTradeState::CryptoRelease)
                    | (FiatTradeState::CryptoRelease, FiatTradeState::Complete)
                // Timeout/failure paths
                    | (FiatTradeState::PaymentPending, FiatTradeState::Failed)
                    | (FiatTradeState::PaymentSubmitted, FiatTradeState::Failed)
                    | (FiatTradeState::Confirming, FiatTradeState::Failed)
                    | (FiatTradeState::CryptoReserved, FiatTradeState::Failed)
                    | (FiatTradeState::PaymentRequested, FiatTradeState::Failed)
                    | (FiatTradeState::MakerConfirmed, FiatTradeState::Failed)
                    | (FiatTradeState::CryptoRelease, FiatTradeState::Failed)
                // Cancellation paths
                    | (FiatTradeState::Created, FiatTradeState::Cancelled)
                    | (FiatTradeState::Accepted, FiatTradeState::Cancelled)
                    | (FiatTradeState::CryptoReserved, FiatTradeState::Cancelled)
                    | (FiatTradeState::PaymentRequested, FiatTradeState::Cancelled)
                    | (FiatTradeState::PaymentSubmitted, FiatTradeState::Cancelled)
                    | (FiatTradeState::PaymentPending, FiatTradeState::Cancelled)
                    | (FiatTradeState::Detected, FiatTradeState::Cancelled)
                    | (FiatTradeState::Confirming, FiatTradeState::Cancelled)
                    | (FiatTradeState::MakerConfirmed, FiatTradeState::Cancelled)
                    | (FiatTradeState::Confirmed, FiatTradeState::Cancelled)
                // Dispute paths
                    | (FiatTradeState::Created, FiatTradeState::Disputed)
                    | (FiatTradeState::Accepted, FiatTradeState::Disputed)
                    | (FiatTradeState::CryptoReserved, FiatTradeState::Disputed)
                    | (FiatTradeState::PaymentRequested, FiatTradeState::Disputed)
                    | (FiatTradeState::PaymentSubmitted, FiatTradeState::Disputed)
                    | (FiatTradeState::PaymentPending, FiatTradeState::Disputed)
                    | (FiatTradeState::Detected, FiatTradeState::Disputed)
                    | (FiatTradeState::Confirming, FiatTradeState::Disputed)
                    | (FiatTradeState::MakerConfirmed, FiatTradeState::Disputed)
                    | (FiatTradeState::Confirmed, FiatTradeState::Disputed)
                    | (FiatTradeState::CryptoRelease, FiatTradeState::Disputed)
                    | (FiatTradeState::Complete, FiatTradeState::ReversalDetected)
                    | (FiatTradeState::ReversalDetected, FiatTradeState::Disputed)
                // Dispute resolution
                    | (FiatTradeState::Disputed, FiatTradeState::Complete)
                    | (FiatTradeState::Disputed, FiatTradeState::Failed)
                    | (FiatTradeState::Disputed, FiatTradeState::Cancelled)
        ));
    }
}

/// Prove that no FiatTradeState transitions to itself.
#[kani::proof]
fn verify_fiat_trade_no_self_transition() {
    let state: FiatTradeState = kani::any();
    assert!(!state.can_transition_to(&state));
}

/// Prove that `Complete`, `Failed`, and `Cancelled` are terminal states
/// (no outgoing transitions except `Complete → ReversalDetected`).
#[kani::proof]
fn verify_fiat_trade_terminal_states() {
    let next: FiatTradeState = kani::any();

    // Complete can only go to ReversalDetected
    let complete = FiatTradeState::Complete;
    if complete.can_transition_to(&next) {
        assert!(matches!(next, FiatTradeState::ReversalDetected));
    }

    // Failed has no outgoing transitions
    let failed = FiatTradeState::Failed;
    assert!(!failed.can_transition_to(&next));

    // Cancelled has no outgoing transitions
    let cancelled = FiatTradeState::Cancelled;
    assert!(!cancelled.can_transition_to(&next));
}

/// Prove that `Disputed` is reachable from all active states.
#[kani::proof]
fn verify_fiat_trade_dispute_from_active() {
    // All active states can transition to Disputed
    let active_states = [
        FiatTradeState::Created,
        FiatTradeState::Accepted,
        FiatTradeState::CryptoReserved,
        FiatTradeState::PaymentRequested,
        FiatTradeState::PaymentSubmitted,
        FiatTradeState::PaymentPending,
        FiatTradeState::Detected,
        FiatTradeState::Confirming,
        FiatTradeState::MakerConfirmed,
        FiatTradeState::Confirmed,
        FiatTradeState::CryptoRelease,
    ];

    for active in active_states.iter() {
        assert!(active.can_transition_to(&FiatTradeState::Disputed));
    }
}

/// Helper to create a FiatTrade with a fixed ID for Kani proofs.
fn make_trade(state: FiatTradeState) -> FiatTrade {
    FiatTrade {
        id: TradeId(Uuid::nil()),
        maker: ParticipantId(Uuid::nil()),
        taker: ParticipantId(Uuid::nil()),
        crypto_asset: Asset::Btc,
        fiat_currency: FiatCurrency::GBP,
        crypto_amount: 1_000_000,
        fiat_amount: FiatAmount::new(7500000, FiatCurrency::GBP),
        settlement_method: SettlementMethod::BankTransfer {
            currency: FiatCurrency::GBP,
        },
        fee_schedule: FeeSchedule::bank_transfer_default(),
        state,
        maker_bank_details: None,
        taker_crypto_address: "bc1qtest123".to_string(),
        payment_id: None,
        payment_reference: None,
        deadlines: SettlementDeadlines {
            payment_deadline: 1000000,
            confirmation_deadline: 2000000,
            dispute_deadline: 3000000,
        },
        created_at: 1,
        updated_at: 1,
    }
}

/// Prove that specific invalid transitions are rejected.
/// Tests concrete invalid paths and verifies terminal state properties.
#[kani::proof]
fn verify_fiat_trade_transition_rejects_invalid() {
    // Created → Complete (skips intermediate states)
    assert!(!FiatTradeState::Created.can_transition_to(&FiatTradeState::Complete));

    // Complete → Created (reverse transition)
    assert!(!FiatTradeState::Complete.can_transition_to(&FiatTradeState::Created));

    // Failed → any state (terminal)
    let next: FiatTradeState = kani::any();
    assert!(!FiatTradeState::Failed.can_transition_to(&next));

    // Cancelled → any state (terminal)
    let next: FiatTradeState = kani::any();
    assert!(!FiatTradeState::Cancelled.can_transition_to(&next));

    // Self-transitions are invalid for all states
    let state: FiatTradeState = kani::any();
    assert!(!state.can_transition_to(&state));

    // PaymentSubmitted → CryptoRelease (skips MakerConfirmed)
    assert!(!FiatTradeState::PaymentSubmitted.can_transition_to(&FiatTradeState::CryptoRelease));

    // PaymentPending → Confirmed (skips Detected, Confirming)
    assert!(!FiatTradeState::PaymentPending.can_transition_to(&FiatTradeState::Confirmed));
}
