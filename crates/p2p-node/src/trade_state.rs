use crate::trades::TradeBook;
use anyhow::Result;
use protocol::message::TradeState;

pub fn trade_state_rank(state: &TradeState) -> u8 {
    match state {
        TradeState::Created => 0,
        TradeState::Accepted => 1,
        TradeState::EscrowFunding => 2,
        TradeState::EscrowFunded => 3,
        TradeState::PaymentPending => 4,
        TradeState::PaymentSent => 5,
        TradeState::PaymentConfirmed => 6,
        TradeState::PayoutPending => 7,
        TradeState::Settled => 8,

        // Exceptional/terminal states are intentionally kept outside
        // the normal linear progression.
        TradeState::Cancelled => 0,
        TradeState::Expired => 0,
        TradeState::Disputed => 0,
    }
}

/// Advance a trade to `target_state`.
///
/// Rules:
/// - The same state is a harmless no-op.
/// - A directly valid protocol transition is always preferred.
/// - A later state on the normal happy path may be received even if
///   intermediate state messages were missed.
/// - Exceptional states (Cancelled, Expired, Disputed) are never reached
///   by blindly walking the happy-path ranking.
/// - Every actual transition is validated by `Trade::transition()`.
pub fn advance_trade_to_state(
    trade_book: &mut TradeBook,
    trade_id: uuid::Uuid,
    target_state: TradeState,
    timestamp: u64,
) -> Result<()> {
    let current_state = match trade_book.get_trade(trade_id) {
        Some(trade) => trade.state.clone(),
        None => {
            anyhow::bail!("Trade {} not found", trade_id);
        }
    };

    // Idempotent duplicate state event.
    if current_state == target_state {
        return Ok(());
    }

    // Always allow a transition explicitly permitted by the protocol.
    //
    // This is important for:
    //   EscrowFunding -> Disputed
    //   PaymentPending -> Expired
    //   PayoutPending -> Disputed
    //   Disputed -> Settled
    //   etc.
    if current_state.can_transition_to(&target_state) {
        trade_book.transition_trade(trade_id, target_state, timestamp)?;
        return Ok(());
    }

    // Exceptional states cannot be inferred from the numeric ranking.
    //
    // For example, Disputed has rank 0, but a trade in Disputed may
    // legitimately transition to Settled. Likewise, we must never turn
    // an arbitrary state into Cancelled/Expired just because its rank is
    // numerically lower.
    if is_exceptional_state(&target_state) {
        anyhow::bail!(
            "Invalid trade state transition: {:?} -> {:?}",
            current_state,
            target_state
        );
    }

    // A happy-path state can be received after earlier messages were
    // missed. Walk the canonical state sequence and apply only valid
    // transitions.
    //
    // Example:
    //
    //     Created -> PaymentSent
    //
    // becomes:
    //
    //     Created
    //       -> Accepted
    //       -> EscrowFunding
    //       -> EscrowFunded
    //       -> PaymentPending
    //       -> PaymentSent
    //
    // We never bypass Trade::transition(), so the protocol state machine
    // remains authoritative.
    let happy_path = [
        TradeState::Created,
        TradeState::Accepted,
        TradeState::EscrowFunding,
        TradeState::EscrowFunded,
        TradeState::PaymentPending,
        TradeState::PaymentSent,
        TradeState::PaymentConfirmed,
        TradeState::PayoutPending,
        TradeState::Settled,
    ];

    let target_rank = trade_state_rank(&target_state);

    for next_state in happy_path {
        let current_state = match trade_book.get_trade(trade_id) {
            Some(trade) => trade.state.clone(),
            None => {
                anyhow::bail!("Trade {} disappeared while advancing state", trade_id);
            }
        };

        if current_state == target_state {
            return Ok(());
        }

        let next_rank = trade_state_rank(&next_state);

        // We only need states up to the requested target.
        if next_rank > target_rank {
            break;
        }

        // Already at this state; continue toward the target.
        if current_state == next_state {
            continue;
        }

        // Apply the next state only if the protocol explicitly permits it.
        if current_state.can_transition_to(&next_state) {
            trade_book.transition_trade(trade_id, next_state, timestamp)?;
        }
    }

    let final_state = trade_book
        .get_trade(trade_id)
        .map(|trade| trade.state.clone());

    if final_state == Some(target_state.clone()) {
        return Ok(());
    }

    anyhow::bail!(
        "Could not advance trade {} from {:?} to {:?}",
        trade_id,
        final_state,
        target_state
    )
}

fn is_exceptional_state(state: &TradeState) -> bool {
    matches!(
        state,
        TradeState::Cancelled | TradeState::Expired | TradeState::Disputed
    )
}
