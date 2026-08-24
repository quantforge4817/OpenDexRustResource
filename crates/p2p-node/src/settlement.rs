use anyhow::Result;
use libp2p::PeerId;

use crate::balances::BalanceBook;
use protocol::message::Trade;

pub fn settle_local_trade(
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
    trade: &Trade,
) -> Result<()> {
    let local_trader = local_peer_id.to_string();

    if local_trader != trade.buyer && local_trader != trade.seller {
        return Ok(());
    }

    balance_book.settle_trade(&local_trader, trade)?;
    balance_book.save()?;

    println!(
        "Settled trade {} locally: buyer={}, seller={}",
        trade.id, trade.buyer, trade.seller
    );

    Ok(())
}
