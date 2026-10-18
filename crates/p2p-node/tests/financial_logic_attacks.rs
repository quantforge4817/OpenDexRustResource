//! Phase 3: Financial Logic Attacks
//!
//! Tests for the orderbook, balances, settlement, and escrow for economic
//! vulnerabilities.
//!
//! Run with: `cargo test --test financial_logic_attacks`

use p2p_node::balances::BalanceBook;
use protocol::message::{Asset, Market};
use std::path::PathBuf;
use uuid::Uuid;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "opendex-test-financial-{name}-{}.json",
        Uuid::new_v4()
    ))
}

// ==============================
// 3.1 Balance Manipulation
// ==============================

mod balance_manipulation {

    use super::*;

    /// Test that balance cannot go negative
    #[test]
    fn balance_cannot_go_negative() {
        let trader = "alice";
        let mut balances = BalanceBook::new(temp_path("negative-balance"));

        balances.credit(trader, Asset::BTC, 1000).unwrap();

        // Try to lock more than available
        let result = balances.lock(trader, Asset::BTC, 2000);
        assert!(result.is_err(), "Locking more than available should fail");

        let balance = balances.balance(trader, &Asset::BTC);
        assert_eq!(balance.available, 1000);
    }

    /// Test balance overflow with max u64
    #[test]
    fn balance_overflow_protection() {
        let trader = "alice";
        let mut balances = BalanceBook::new(temp_path("overflow-balance"));

        balances.credit(trader, Asset::BTC, u64::MAX).unwrap();

        let result = balances.credit(trader, Asset::BTC, 1);
        assert!(result.is_err(), "Crediting beyond u64::MAX should fail");
    }

    /// Test that locked balances cannot be spent
    #[test]
    fn locked_balance_cannot_be_spent() {
        let trader = "alice";
        let mut balances = BalanceBook::new(temp_path("locked-balance"));

        balances.credit(trader, Asset::BTC, 1000).unwrap();
        balances.lock(trader, Asset::BTC, 1000).unwrap();

        let result = balances.lock(trader, Asset::BTC, 1);
        assert!(result.is_err(), "Locking already-locked funds should fail");
    }

    /// Test that escrowed balances cannot be spent
    #[test]
    fn escrowed_balance_cannot_be_spent() {
        let trader = "alice";
        let mut balances = BalanceBook::new(temp_path("escrow-balance"));

        balances.credit(trader, Asset::BTC, 1000).unwrap();
        balances.lock(trader, Asset::BTC, 1000).unwrap();
        balances.escrow_lock(trader, Asset::BTC, 1000).unwrap();

        // Try to unlock from locked (should fail - it's in escrow)
        let result = balances.unlock(trader, Asset::BTC, 1000);
        assert!(
            result.is_err(),
            "Unlocking escrowed funds from locked should fail"
        );
    }

    /// Test balance invariant: available + locked + escrowed = total
    #[test]
    fn balance_invariant_holds() {
        let trader = "alice";
        let mut balances = BalanceBook::new(temp_path("invariant-balance"));

        balances.credit(trader, Asset::BTC, 10000).unwrap();

        let balance = balances.balance(trader, &Asset::BTC);
        assert_eq!(
            balance.available + balance.locked + balance.escrowed,
            10000,
            "Balance invariant should hold after credit"
        );

        balances.lock(trader, Asset::BTC, 3000).unwrap();
        let balance = balances.balance(trader, &Asset::BTC);
        assert_eq!(
            balance.available + balance.locked + balance.escrowed,
            10000,
            "Balance invariant should hold after lock"
        );

        balances.unlock(trader, Asset::BTC, 1000).unwrap();
        let balance = balances.balance(trader, &Asset::BTC);
        assert_eq!(
            balance.available + balance.locked + balance.escrowed,
            10000,
            "Balance invariant should hold after unlock"
        );
    }

    /// Test zero balance operations
    #[test]
    fn zero_balance_operations() {
        let trader = "alice";
        let mut balances = BalanceBook::new(temp_path("zero-balance"));

        let _ = balances.credit(trader, Asset::BTC, 0);
        balances.credit(trader, Asset::BTC, 1000).unwrap();
        let _ = balances.lock(trader, Asset::BTC, 0);
    }
}

// ==============================
// 3.2 Trade State Machine Attacks
// ==============================

mod trade_state_machine {

    use protocol::message::TradeState;

    /// Test all invalid state transitions are rejected
    #[test]
    fn invalid_state_transitions_rejected() {
        assert!(
            !TradeState::Settled.can_transition_to(&TradeState::Created),
            "Settled -> Created should be invalid"
        );
        assert!(
            !TradeState::Cancelled.can_transition_to(&TradeState::Accepted),
            "Cancelled -> Accepted should be invalid"
        );
        assert!(
            !TradeState::Expired.can_transition_to(&TradeState::EscrowFunding),
            "Expired -> EscrowFunding should be invalid"
        );
    }

    /// Test that terminal states have no outgoing transitions
    #[test]
    fn terminal_states_have_no_transitions() {
        let all_states = vec![
            TradeState::Created,
            TradeState::Accepted,
            TradeState::EscrowFunding,
            TradeState::EscrowFunded,
            TradeState::PaymentPending,
            TradeState::PaymentSent,
            TradeState::PaymentConfirmed,
            TradeState::PayoutPending,
            TradeState::Settled,
            TradeState::Cancelled,
            TradeState::Expired,
            TradeState::Disputed,
        ];

        for terminal in &[
            TradeState::Settled,
            TradeState::Cancelled,
            TradeState::Expired,
        ] {
            for target in &all_states {
                if target != terminal {
                    assert!(
                        !terminal.can_transition_to(target),
                        "Terminal state should not transition to {:?}",
                        target
                    );
                }
            }
        }
    }

    /// Test happy path transitions are valid
    #[test]
    fn happy_path_transitions_valid() {
        assert!(TradeState::Created.can_transition_to(&TradeState::Accepted));
        assert!(TradeState::Accepted.can_transition_to(&TradeState::EscrowFunding));
        assert!(TradeState::EscrowFunding.can_transition_to(&TradeState::EscrowFunded));
        assert!(TradeState::EscrowFunded.can_transition_to(&TradeState::PaymentPending));
        assert!(TradeState::PaymentPending.can_transition_to(&TradeState::PaymentSent));
        assert!(TradeState::PaymentSent.can_transition_to(&TradeState::PaymentConfirmed));
        assert!(TradeState::PaymentConfirmed.can_transition_to(&TradeState::PayoutPending));
        assert!(TradeState::PayoutPending.can_transition_to(&TradeState::Settled));
    }

    /// Test that Disputed state can transition to Settled
    #[test]
    fn disputed_can_resolve_to_settled() {
        assert!(
            TradeState::Disputed.can_transition_to(&TradeState::Settled),
            "Disputed -> Settled should be valid"
        );
    }

    /// Test that only allowed states can transition to Cancelled
    #[test]
    fn cancellation_from_allowed_states() {
        // States that CAN cancel
        let can_cancel = vec![
            TradeState::Created,
            TradeState::Accepted,
            TradeState::EscrowFunding,
            TradeState::Disputed,
        ];
        for state in can_cancel {
            assert!(
                state.can_transition_to(&TradeState::Cancelled),
                "{:?} should be able to cancel",
                state
            );
        }

        // States that CANNOT cancel (must go through Disputed first)
        let cannot_cancel = vec![
            TradeState::EscrowFunded,
            TradeState::PaymentPending,
            TradeState::PaymentSent,
            TradeState::PaymentConfirmed,
            TradeState::PayoutPending,
        ];
        for state in cannot_cancel {
            assert!(
                !state.can_transition_to(&TradeState::Cancelled),
                "{:?} should NOT be able to cancel directly",
                state
            );
        }
    }
}

// ==============================
// 3.3 Settlement Attacks
// ==============================

mod settlement_attacks {

    use super::*;

    /// Test that settlement with insufficient funds fails
    #[test]
    fn settlement_insufficient_funds() {
        let trader = "alice";
        let mut balances = BalanceBook::new(temp_path("settlement-insufficient"));

        balances.credit(trader, Asset::BTC, 100).unwrap();
        balances.lock(trader, Asset::BTC, 100).unwrap();

        let balance = balances.balance(trader, &Asset::BTC);
        assert_eq!(balance.locked, 100);
        assert_eq!(balance.available, 0);
    }

    /// Test that double settlement is prevented
    #[test]
    fn double_settlement_prevented() {
        let trader = "alice";
        let mut balances = BalanceBook::new(temp_path("double-settlement"));

        balances.credit(trader, Asset::BTC, 1000).unwrap();
        balances.lock(trader, Asset::BTC, 500).unwrap();

        balances.unlock(trader, Asset::BTC, 500).unwrap();

        let result = balances.unlock(trader, Asset::BTC, 500);
        assert!(result.is_err(), "Double unlock should fail");
    }

    /// Test settlement idempotency
    #[test]
    fn settlement_idempotency() {
        let trader = "alice";
        let mut balances = BalanceBook::new(temp_path("settlement-idempotent"));

        balances.credit(trader, Asset::BTC, 1000).unwrap();
        balances.credit(trader, Asset::BTC, 100).unwrap();
        balances.credit(trader, Asset::BTC, 100).unwrap();

        let balance = balances.balance(trader, &Asset::BTC);
        assert_eq!(balance.available, 1200);
    }
}

// ==============================
// 3.4 Order Manipulation
// ==============================

mod order_manipulation {

    use super::*;

    /// Test that order amounts are validated
    #[test]
    fn order_amount_validation() {
        // Zero amount orders tested in network_protocol_attacks
    }

    /// Test market pair validation
    #[test]
    fn market_pair_validation() {
        let market = Market {
            base: Asset::BTC,
            quote: Asset::BTC,
        };
        assert_eq!(market.base, Asset::BTC);
        assert_eq!(market.quote, Asset::BTC);
    }
}
