use anyhow::{Result, anyhow};
use libp2p::PeerId;
use protocol::message::{PRICE_SCALE, Trade};

use crate::balances::BalanceBook;

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
    let quote_amount = trade
        .price
        .checked_mul(trade.amount)
        .ok_or_else(|| anyhow!("trade quote amount overflow"))?
        / PRICE_SCALE;

    // Validate the entire settlement before mutating anything.
    if local_trader == trade.buyer {
        let quote_balance = balance_book.balance(&local_trader, &trade.market.quote);
        if quote_balance.locked < quote_amount {
            return Err(anyhow!("insufficient locked quote balance"));
        }

        let base_balance = balance_book.balance(&local_trader, &trade.market.base);
        base_balance
            .available
            .checked_add(trade.amount)
            .ok_or_else(|| anyhow!("base balance overflow"))?;
    }

    if local_trader == trade.seller {
        let base_balance = balance_book.balance(&local_trader, &trade.market.base);
        if base_balance.locked < trade.amount {
            return Err(anyhow!("insufficient locked base balance"));
        }

        let quote_balance = balance_book.balance(&local_trader, &trade.market.quote);
        quote_balance
            .available
            .checked_add(quote_amount)
            .ok_or_else(|| anyhow!("quote balance overflow"))?;
    }

    // All validation has succeeded, so mutations below cannot fail.
    if local_trader == trade.buyer {
        balance_book.consume_locked(&local_trader, trade.market.quote.clone(), quote_amount)?;

        balance_book.credit(&local_trader, trade.market.base.clone(), trade.amount)?;
    }

    if local_trader == trade.seller {
        balance_book.consume_locked(&local_trader, trade.market.base.clone(), trade.amount)?;

        balance_book.credit(&local_trader, trade.market.quote.clone(), quote_amount)?;
    }

    balance_book.settled_trades.push(trade_id);
    balance_book.save()?;

    println!(
        "Settled trade {} locally: buyer={}, seller={}",
        trade.id, trade.buyer, trade.seller
    );

    Ok(())
}
