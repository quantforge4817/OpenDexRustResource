use anyhow::{Result, anyhow};
use libp2p::PeerId;
use protocol::message::{Asset, Trade, TradeState};
use uuid::Uuid;

use crate::balances::{BalanceBook, quote_amount};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementPlan {
    pub trade_id: Uuid,
    pub trader: String,
    pub debit_asset: Asset,
    pub debit_amount: u64,
    pub credit_asset: Asset,
    pub credit_amount: u64,
}

pub fn plan_local_settlement(
    balance_book: &BalanceBook,
    local_peer_id: &PeerId,
    trade: &Trade,
) -> Result<Option<SettlementPlan>> {
    let local_trader = local_peer_id.to_string();

    // This node is not involved in this trade.
    if local_trader != trade.buyer && local_trader != trade.seller {
        return Ok(None);
    }

    // Idempotency belongs to planning as well: an already-settled trade
    // produces no work.
    let trade_id = trade.id.to_string();
    if balance_book.settled_trades.iter().any(|id| id == &trade_id) {
        return Ok(None);
    }

    if trade.state != TradeState::Settled {
        return Err(anyhow!(
            "cannot settle trade {} in state {:?}",
            trade.id,
            trade.state
        ));
    }

    // Perform all arithmetic before constructing a mutation plan.
    let quote = quote_amount(trade.price, trade.amount)?;

    if local_trader == trade.buyer {
        let quote_balance = balance_book.balance(&local_trader, &trade.market.quote);

        if quote_balance.locked < quote {
            return Err(anyhow!("insufficient locked quote balance"));
        }

        let base_balance = balance_book.balance(&local_trader, &trade.market.base);

        base_balance
            .available
            .checked_add(trade.amount)
            .ok_or_else(|| anyhow!("base balance overflow"))?;

        Ok(Some(SettlementPlan {
            trade_id: trade.id,
            trader: local_trader,
            debit_asset: trade.market.quote.clone(),
            debit_amount: quote,
            credit_asset: trade.market.base.clone(),
            credit_amount: trade.amount,
        }))
    } else {
        let base_balance = balance_book.balance(&local_trader, &trade.market.base);

        if base_balance.locked < trade.amount {
            return Err(anyhow!("insufficient locked base balance"));
        }

        let quote_balance = balance_book.balance(&local_trader, &trade.market.quote);

        quote_balance
            .available
            .checked_add(quote)
            .ok_or_else(|| anyhow!("quote balance overflow"))?;

        Ok(Some(SettlementPlan {
            trade_id: trade.id,
            trader: local_trader,
            debit_asset: trade.market.base.clone(),
            debit_amount: trade.amount,
            credit_asset: trade.market.quote.clone(),
            credit_amount: quote,
        }))
    }
}

fn apply_settlement_plan(balance_book: &mut BalanceBook, plan: &SettlementPlan) -> Result<()> {
    balance_book.consume_locked(&plan.trader, plan.debit_asset.clone(), plan.debit_amount)?;

    balance_book.credit(&plan.trader, plan.credit_asset.clone(), plan.credit_amount)?;

    balance_book.settled_trades.push(plan.trade_id.to_string());

    Ok(())
}

pub fn commit_local_settlement(
    balance_book: &mut BalanceBook,
    plan: &SettlementPlan,
) -> Result<()> {
    // Apply the complete plan to a candidate copy. The caller's live state
    // remains untouched until the candidate has been persisted successfully.
    let mut updated_book = balance_book.clone();

    apply_settlement_plan(&mut updated_book, plan)?;

    updated_book.save()?;

    *balance_book = updated_book;

    Ok(())
}

pub fn settle_local_trade(
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
    trade: &Trade,
) -> Result<bool> {
    let Some(plan) = plan_local_settlement(balance_book, local_peer_id, trade)? else {
        return Ok(false);
    };

    commit_local_settlement(balance_book, &plan)?;

    println!(
        "Settled trade {} locally: buyer={}, seller={}",
        trade.id, trade.buyer, trade.seller
    );

    Ok(true)
}
