use anyhow::{Result, anyhow};
use libp2p::PeerId;
use protocol::message::{Asset, Trade, TradeState};
use uuid::Uuid;

use crate::balances::BalanceBook;

fn format_amount(amount: u64) -> String {
    let scaled = amount as f64 / protocol::message::AMOUNT_SCALE as f64;
    format!("{:.8}", scaled)
}

/// Escrow action types that move funds between balance states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscrowAction {
    /// locked → escrowed (seller funds escrow on EscrowFunded)
    Fund,
    /// escrowed → available (release on settlement)
    Release,
    /// escrowed → locked (return on dispute cancellation)
    Return,
}

/// Describes a single escrow mutation to be applied to the BalanceBook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowPlan {
    pub trade_id: Uuid,
    pub trader: String,
    pub action: EscrowAction,
    pub asset: Asset,
    pub amount: u64,
    /// The order ID whose reservation is consumed during escrow fund.
    /// Present when per-order reservations are used; `None` falls back
    /// to aggregate locked consumption for backward compatibility.
    pub order_id: Option<Uuid>,
}

/// Validates whether a local escrow operation should be performed and returns
/// the corresponding plan. Returns `Ok(None)` if no action is needed
/// (e.g., not a local trade, already escrowed, or wrong state).
pub fn plan_local_escrow(
    balance_book: &BalanceBook,
    local_peer_id: &PeerId,
    trade: &Trade,
    action: EscrowAction,
) -> Result<Option<EscrowPlan>> {
    let local_trader = local_peer_id.to_string();

    match &action {
        EscrowAction::Fund => {
            // Only the seller funds escrow
            if local_trader != trade.seller {
                tracing::debug!(
                    trade_id = %trade.id,
                    local_peer = %local_trader,
                    seller = %trade.seller,
                    "Skipping escrow fund: not the seller"
                );
                return Ok(None);
            }

            // Idempotency: skip if already escrowed
            if balance_book.escrowed_trades.contains(&trade.id) {
                tracing::info!(
                    trade_id = %trade.id,
                    "Skipping escrow fund: already escrowed (idempotent)"
                );
                return Ok(None);
            }

            // Must be in EscrowFunded state
            if trade.state != TradeState::EscrowFunded {
                return Err(anyhow!(
                    "cannot fund escrow for trade {} in state {:?}",
                    trade.id,
                    trade.state
                ));
            }

            // Check the per-order reservation first; fall back to aggregate locked.
            let has_sufficient_reservation = balance_book
                .order_reservation(&trade.sell_order)
                .map(|res| res >= trade.amount)
                .unwrap_or(false);

            if !has_sufficient_reservation {
                let base_balance = balance_book.balance(&local_trader, &trade.market.base);
                if base_balance.locked < trade.amount {
                    tracing::warn!(
                        trade_id = %trade.id,
                        seller = %trade.seller,
                        locked = format_amount(base_balance.locked),
                        required = format_amount(trade.amount),
                        "Insufficient locked balance for escrow funding"
                    );
                    return Err(anyhow!(
                        "insufficient locked base balance for escrow funding"
                    ));
                }
            }

            tracing::info!(
                trade_id = %trade.id,
                seller = %trade.seller,
                asset = ?trade.market.base,
                amount = format_amount(trade.amount),
                "Planning escrow fund"
            );

            Ok(Some(EscrowPlan {
                trade_id: trade.id,
                trader: local_trader,
                action: EscrowAction::Fund,
                asset: trade.market.base.clone(),
                amount: trade.amount,
                order_id: Some(trade.sell_order),
            }))
        }

        EscrowAction::Release => {
            // Only the seller releases escrow (base asset)
            if local_trader != trade.seller {
                tracing::debug!(
                    trade_id = %trade.id,
                    local_peer = %local_trader,
                    seller = %trade.seller,
                    "Skipping escrow release: not the seller"
                );
                return Ok(None);
            }

            // Must have been escrowed
            if !balance_book.escrowed_trades.contains(&trade.id) {
                tracing::debug!(
                    trade_id = %trade.id,
                    "Skipping escrow release: trade was never escrowed"
                );
                return Ok(None);
            }

            // Check sufficient escrowed base balance
            let base_balance = balance_book.balance(&local_trader, &trade.market.base);
            if base_balance.escrowed < trade.amount {
                tracing::warn!(
                    trade_id = %trade.id,
                    seller = %trade.seller,
                    escrowed = format_amount(base_balance.escrowed),
                    required = format_amount(trade.amount),
                    "Insufficient escrowed balance for release"
                );
                return Err(anyhow!(
                    "insufficient escrowed base balance for release"
                ));
            }

            tracing::info!(
                trade_id = %trade.id,
                seller = %trade.seller,
                asset = ?trade.market.base,
                amount = format_amount(trade.amount),
                "Planning escrow release"
            );

            Ok(Some(EscrowPlan {
                trade_id: trade.id,
                trader: local_trader,
                action: EscrowAction::Release,
                asset: trade.market.base.clone(),
                amount: trade.amount,
                order_id: Some(trade.sell_order),
            }))
        }

        EscrowAction::Return => {
            // Only the seller returns escrow
            if local_trader != trade.seller {
                tracing::debug!(
                    trade_id = %trade.id,
                    local_peer = %local_trader,
                    seller = %trade.seller,
                    "Skipping escrow return: not the seller"
                );
                return Ok(None);
            }

            // Must have been escrowed
            if !balance_book.escrowed_trades.contains(&trade.id) {
                tracing::debug!(
                    trade_id = %trade.id,
                    "Skipping escrow return: trade was never escrowed"
                );
                return Ok(None);
            }

            // Check sufficient escrowed base balance
            let base_balance = balance_book.balance(&local_trader, &trade.market.base);
            if base_balance.escrowed < trade.amount {
                tracing::warn!(
                    trade_id = %trade.id,
                    seller = %trade.seller,
                    escrowed = format_amount(base_balance.escrowed),
                    required = format_amount(trade.amount),
                    "Insufficient escrowed balance for return"
                );
                return Err(anyhow!(
                    "insufficient escrowed base balance for return"
                ));
            }

            tracing::info!(
                trade_id = %trade.id,
                seller = %trade.seller,
                asset = ?trade.market.base,
                amount = format_amount(trade.amount),
                "Planning escrow return"
            );

            Ok(Some(EscrowPlan {
                trade_id: trade.id,
                trader: local_trader,
                action: EscrowAction::Return,
                asset: trade.market.base.clone(),
                amount: trade.amount,
                order_id: Some(trade.sell_order),
            }))
        }
    }
}

/// Applies an escrow plan to the given BalanceBook.
fn apply_escrow_plan(balance_book: &mut BalanceBook, plan: &EscrowPlan) -> Result<()> {
    let action_label = match plan.action {
        EscrowAction::Fund => "fund",
        EscrowAction::Release => "release",
        EscrowAction::Return => "return",
    };

    tracing::debug!(
        trade_id = %plan.trade_id,
        trader = %plan.trader,
        action = action_label,
        asset = ?plan.asset,
        amount = format_amount(plan.amount),
        "Applying escrow plan"
    );

    match plan.action {
        EscrowAction::Fund => {
            // Use per-order escrow when the order reservation is available;
            // fall back to aggregate when the reservation was already removed.
            match plan.order_id {
                Some(order_id) if balance_book.order_reservation(&order_id).is_some() => {
                    balance_book.escrow_lock_for_order(
                        &order_id,
                        &plan.trader,
                        plan.asset.clone(),
                        plan.amount,
                    )?;
                }
                _ => {
                    balance_book.escrow_lock(&plan.trader, plan.asset.clone(), plan.amount)?;
                }
            }
            balance_book.escrowed_trades.insert(plan.trade_id);
        }
        EscrowAction::Release => {
            balance_book.escrow_release(&plan.trader, plan.asset.clone(), plan.amount)?;
            balance_book.escrowed_trades.remove(&plan.trade_id);
        }
        EscrowAction::Return => {
            balance_book.escrow_return(&plan.trader, plan.asset.clone(), plan.amount)?;
            balance_book.escrowed_trades.remove(&plan.trade_id);
        }
    }

    Ok(())
}

/// Commits an escrow plan using the plan-then-commit atomicity pattern:
/// clone → apply → save → swap.
pub fn commit_local_escrow(
    balance_book: &mut BalanceBook,
    plan: &EscrowPlan,
) -> Result<()> {
    tracing::debug!(
        trade_id = %plan.trade_id,
        "Committing escrow plan (clone → apply → save → swap)"
    );

    let mut updated_book = balance_book.clone();

    apply_escrow_plan(&mut updated_book, plan)?;

    tracing::debug!(trade_id = %plan.trade_id, "Escrow plan applied to cloned BalanceBook");

    updated_book.save()?;

    tracing::debug!(trade_id = %plan.trade_id, "Escrow plan persisted");

    *balance_book = updated_book;

    tracing::debug!(trade_id = %plan.trade_id, "Escrow plan committed (swap complete)");

    Ok(())
}

/// Orchestrates escrow funding when a trade reaches `EscrowFunded`.
/// Moves the seller's locked base-asset amount to escrowed.
pub fn fund_local_escrow(
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
    trade: &Trade,
) -> Result<bool> {
    let Some(plan) = plan_local_escrow(balance_book, local_peer_id, trade, EscrowAction::Fund)?
    else {
        return Ok(false);
    };

    commit_local_escrow(balance_book, &plan)?;

    // Log the resulting balance
    let balance = balance_book.balance(&trade.seller, &trade.market.base);
    tracing::info!(
        trade_id = %trade.id,
        seller = %trade.seller,
        asset = ?trade.market.base,
        amount = format_amount(trade.amount),
        locked = format_amount(balance.locked),
        escrowed = format_amount(balance.escrowed),
        "Funded escrow locally"
    );

    Ok(true)
}

/// Orchestrates escrow release when a trade reaches `Settled`.
/// Moves the seller's escrowed base-asset amount to available.
pub fn release_local_escrow(
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
    trade: &Trade,
) -> Result<bool> {
    let Some(plan) =
        plan_local_escrow(balance_book, local_peer_id, trade, EscrowAction::Release)?
    else {
        return Ok(false);
    };

    commit_local_escrow(balance_book, &plan)?;

    // Log the resulting balance
    let balance = balance_book.balance(&trade.seller, &trade.market.base);
    tracing::info!(
        trade_id = %trade.id,
        seller = %trade.seller,
        asset = ?trade.market.base,
        amount = format_amount(trade.amount),
        available = format_amount(balance.available),
        escrowed = format_amount(balance.escrowed),
        "Released escrow locally"
    );

    Ok(true)
}

/// Orchestrates escrow return when a trade is cancelled after being in escrow.
/// Moves the seller's escrowed base-asset amount back to locked.
pub fn return_local_escrow(
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
    trade: &Trade,
) -> Result<bool> {
    let Some(plan) =
        plan_local_escrow(balance_book, local_peer_id, trade, EscrowAction::Return)?
    else {
        return Ok(false);
    };

    commit_local_escrow(balance_book, &plan)?;

    // Log the resulting balance
    let balance = balance_book.balance(&trade.seller, &trade.market.base);
    tracing::info!(
        trade_id = %trade.id,
        seller = %trade.seller,
        asset = ?trade.market.base,
        amount = format_amount(trade.amount),
        locked = format_amount(balance.locked),
        escrowed = format_amount(balance.escrowed),
        "Returned escrow locally"
    );

    Ok(true)
}
