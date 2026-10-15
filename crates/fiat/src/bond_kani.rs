//! Kani formal verification proof harnesses for `BondState`.
//!
//! Proves properties about the 6-state BondState machine (36 possible pairs)
//! using symbolic execution, ensuring correctness for all possible inputs.

use crate::bond::{BondState, SecurityBond};
use opendex_types::{Asset, ParticipantId, TradeId};
use uuid::Uuid;

/// Prove that only the 9 valid transitions return `true` among all 36 state pairs.
#[kani::proof]
fn verify_bond_transition_graph() {
    let current: BondState = kani::any();
    let next: BondState = kani::any();

    // Count valid transitions by checking each pair
    let valid = current.can_transition_to(&next);

    // The 9 valid transitions are:
    // Required -> Locked
    // Required -> Released
    // Locked -> Released
    // Locked -> PartiallySlashed
    // Locked -> Slashed
    // PartiallySlashed -> Released
    // PartiallySlashed -> Slashed
    // Slashed -> Refunded
    // Released -> Refunded

    // Prove that if valid, the pair is one of the 9 known transitions
    if valid {
        assert!(
            matches!(
                (&current, &next),
                (BondState::Required, BondState::Locked)
                    | (BondState::Required, BondState::Released)
                    | (BondState::Locked, BondState::Released)
                    | (BondState::Locked, BondState::PartiallySlashed)
                    | (BondState::Locked, BondState::Slashed)
                    | (BondState::PartiallySlashed, BondState::Released)
                    | (BondState::PartiallySlashed, BondState::Slashed)
                    | (BondState::Slashed, BondState::Refunded)
                    | (BondState::Released, BondState::Refunded)
            )
        );
    }
}

/// Prove that no BondState transitions to itself.
#[kani::proof]
fn verify_bond_no_self_transition() {
    let state: BondState = kani::any();
    assert!(!state.can_transition_to(&state));
}

/// Prove that `Released` and `Slashed` have no outgoing transitions except to `Refunded`.
#[kani::proof]
fn verify_bond_terminal_states() {
    let next: BondState = kani::any();

    // Released can only go to Refunded
    let released = BondState::Released;
    if released.can_transition_to(&next) {
        assert!(matches!(next, BondState::Refunded));
    }

    // Slashed can only go to Refunded
    let slashed = BondState::Slashed;
    if slashed.can_transition_to(&next) {
        assert!(matches!(next, BondState::Refunded));
    }

    // Refunded has no outgoing transitions at all
    let refunded = BondState::Refunded;
    assert!(!refunded.can_transition_to(&next));
}

/// Helper to create a SecurityBond with a fixed ID for Kani proofs.
fn make_bond(state: BondState) -> SecurityBond {
    SecurityBond {
        id: crate::bond::BondId(Uuid::nil()),
        trade_id: TradeId(Uuid::nil()),
        participant: ParticipantId(Uuid::nil()),
        asset: Asset::Btc,
        amount: 100_000,
        state,
        slash_reason: None,
        rule_violated: None,
        updated_at: 1,
    }
}

/// Prove that `lock()` only succeeds when the state is `Required`.
#[kani::proof]
fn verify_bond_lock_safety() {
    // Only Required can transition to Locked
    let state: BondState = kani::any();
    
    // If lock would succeed, the state must be Required
    if state.can_transition_to(&BondState::Locked) {
        assert!(matches!(state, BondState::Required));
    }
    
    // Required can lock
    assert!(BondState::Required.can_transition_to(&BondState::Locked));
    
    // Non-Required states cannot lock
    assert!(!BondState::Locked.can_transition_to(&BondState::Locked));
    assert!(!BondState::Released.can_transition_to(&BondState::Locked));
    assert!(!BondState::PartiallySlashed.can_transition_to(&BondState::Locked));
    assert!(!BondState::Slashed.can_transition_to(&BondState::Locked));
    assert!(!BondState::Refunded.can_transition_to(&BondState::Locked));
}

/// Prove that `slash()` only succeeds from `Locked` or `PartiallySlashed`.
#[kani::proof]
fn verify_bond_slash_safety() {
    let state: BondState = kani::any();

    // If slash would succeed, the state must be Locked or PartiallySlashed
    if state.can_transition_to(&BondState::Slashed) {
        assert!(
            matches!(state, BondState::Locked | BondState::PartiallySlashed)
        );
    }

    // Locked can slash
    assert!(BondState::Locked.can_transition_to(&BondState::Slashed));
    // PartiallySlashed can slash
    assert!(BondState::PartiallySlashed.can_transition_to(&BondState::Slashed));
    
    // Other states cannot slash
    assert!(!BondState::Required.can_transition_to(&BondState::Slashed));
    assert!(!BondState::Released.can_transition_to(&BondState::Slashed));
    assert!(!BondState::Slashed.can_transition_to(&BondState::Slashed));
    assert!(!BondState::Refunded.can_transition_to(&BondState::Slashed));
}

/// Prove that `release()` only succeeds from valid states.
#[kani::proof]
fn verify_bond_release_safety() {
    let state: BondState = kani::any();

    // If release would succeed, the state must be one that can transition to Released
    if state.can_transition_to(&BondState::Released) {
        assert!(
            matches!(state, BondState::Required | BondState::Locked | BondState::PartiallySlashed)
        );
    }

    // Required can release (early cancellation)
    assert!(BondState::Required.can_transition_to(&BondState::Released));
    // Locked can release
    assert!(BondState::Locked.can_transition_to(&BondState::Released));
    // PartiallySlashed can release
    assert!(BondState::PartiallySlashed.can_transition_to(&BondState::Released));
    
    // Other states cannot release
    assert!(!BondState::Released.can_transition_to(&BondState::Released));
    assert!(!BondState::Slashed.can_transition_to(&BondState::Released));
    assert!(!BondState::Refunded.can_transition_to(&BondState::Released));
}
