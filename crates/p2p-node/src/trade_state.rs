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

        // Terminal/exception states are not part of the
        // linear happy-path progression.
        TradeState::Cancelled => 0,
        TradeState::Expired => 0,
        TradeState::Disputed => 0,
    }
}

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

    if current_state == target_state {
        return Ok(());
    }

    // First try the exact transition. This handles branching
    // states such as EscrowFunding -> Disputed and
    // Disputed -> Settled.
    if current_state.can_transition_to(&target_state) {
        trade_book.transition_trade(trade_id, target_state, timestamp)?;
        return Ok(());
    }

    // For the normal happy path, a node may receive a later state
    // without having observed every preceding state.
    let states = [
        TradeState::Accepted,
        TradeState::EscrowFunding,
        TradeState::EscrowFunded,
        TradeState::PaymentPending,
        TradeState::PaymentSent,
        TradeState::PaymentConfirmed,
        TradeState::PayoutPending,
        TradeState::Settled,
    ];

    for state in states {
        if trade_state_rank(&state) > trade_state_rank(&target_state) {
            break;
        }

        let current_state = match trade_book.get_trade(trade_id) {
            Some(trade) => trade.state.clone(),
            None => {
                anyhow::bail!("Trade {} disappeared while advancing state", trade_id);
            }
        };

        if current_state == state {
            continue;
        }

        if current_state.can_transition_to(&state) {
            trade_book.transition_trade(trade_id, state, timestamp)?;
        }
    }

    let final_state = trade_book
        .get_trade(trade_id)
        .map(|trade| trade.state.clone());

    if final_state != Some(target_state.clone()) {
        anyhow::bail!(
            "Could not advance trade {} from {:?} to {:?}",
            trade_id,
            final_state,
            target_state
        );
    }

    Ok(())
}
