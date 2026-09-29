use anyhow::{Result, anyhow};
use libp2p::PeerId;
use protocol::message::Trade;

use crate::balances::{BalanceBook, quote_amount};

pub fn settle_local_trade(
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
    trade: &Trade,
) -> Result<()> {
    let local_trader = local_peer_id.to_string();
    // This node is not involved in this trade.
    if local_trader != trade.buyer && local_trader != trade.seller {
        return Ok(());
    }

    let trade_id = trade.id.to_string();
    // Idempotency: never settle the same trade twice.
    if balance_book.settled_trades.iter().any(|id| id == &trade_id) {
        return Ok(());
    }
    // Calculate this before mutating balances so an overflow leaves
    // the balance book unchanged.
    let quote_amount = quote_amount(trade.price, trade.amount)?;

    // Work on a copy so persistence failure cannot partially mutate
    // the caller's in-memory balance book.
    let mut updated_book = balance_book.clone();
    // Validate the entire settlement before mutating anything.

    if local_trader == trade.buyer {
        let quote_balance = updated_book.balance(&local_trader, &trade.market.quote);
        if quote_balance.locked < quote_amount {
            return Err(anyhow!("insufficient locked quote balance"));
        }

        let base_balance = updated_book.balance(&local_trader, &trade.market.base);
        base_balance
            .available
            .checked_add(trade.amount)
            .ok_or_else(|| anyhow!("base balance overflow"))?;

        updated_book.consume_locked(&local_trader, trade.market.quote.clone(), quote_amount)?;

        updated_book.credit(&local_trader, trade.market.base.clone(), trade.amount)?;
    }

    if local_trader == trade.seller {
        let base_balance = updated_book.balance(&local_trader, &trade.market.base);
        if base_balance.locked < trade.amount {
            return Err(anyhow!("insufficient locked base balance"));
        }

        let quote_balance = updated_book.balance(&local_trader, &trade.market.quote);
        quote_balance
            .available
            .checked_add(quote_amount)
            .ok_or_else(|| anyhow!("quote balance overflow"))?;

        updated_book.consume_locked(&local_trader, trade.market.base.clone(), trade.amount)?;

        updated_book.credit(&local_trader, trade.market.quote.clone(), quote_amount)?;
    }

    updated_book.settled_trades.push(trade_id);
    // Persist the complete new state before committing it to the caller.
    updated_book.save()?;
    // Commit only after persistence succeeds.
    *balance_book = updated_book;

    println!(
        "Settled trade {} locally: buyer={}, seller={}",
        trade.id, trade.buyer, trade.seller
    );

    Ok(())
}
