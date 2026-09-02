use anyhow::Result;

use protocol::state::StateSnapshot;

use crate::orderbook::OrderBook;
use crate::trades::TradeBook;

pub fn validate_state_snapshot(
    state: &StateSnapshot,
    order_book: &OrderBook,
    trade_book: &TradeBook,
) -> Result<()> {
    // A remote snapshot must not contain duplicate order IDs.
    let mut order_ids = std::collections::HashSet::new();

    for order in &state.orders {
        if !order_ids.insert(order.id) {
            anyhow::bail!("StateResponse contains duplicate order {}", order.id);
        }

        if order.amount == 0 {
            anyhow::bail!("StateResponse contains order {} with zero amount", order.id);
        }

        if order.trader.is_empty() {
            anyhow::bail!(
                "StateResponse contains order {} with empty trader",
                order.id
            );
        }

        if order.market.base == order.market.quote {
            anyhow::bail!(
                "StateResponse contains order {} with invalid market",
                order.id
            );
        }
    }

    // A removed order must not simultaneously appear as an active order.
    for order_id in &state.removed_orders {
        if order_ids.contains(order_id) {
            anyhow::bail!(
                "StateResponse contains order {} as both active and removed",
                order_id
            );
        }
    }

    // A remote snapshot must not contain duplicate trade IDs.
    let mut trade_ids = std::collections::HashSet::new();

    for trade in &state.trades {
        if !trade_ids.insert(trade.id) {
            anyhow::bail!("StateResponse contains duplicate trade {}", trade.id);
        }

        if trade.amount == 0 {
            anyhow::bail!("StateResponse contains trade {} with zero amount", trade.id);
        }

        if trade.price == 0 {
            anyhow::bail!("StateResponse contains trade {} with zero price", trade.id);
        }

        if trade.buyer.is_empty() || trade.seller.is_empty() {
            anyhow::bail!(
                "StateResponse contains trade {} with empty participant",
                trade.id
            );
        }

        if trade.buy_order == trade.sell_order {
            anyhow::bail!(
                "StateResponse contains trade {} referencing the same order twice",
                trade.id
            );
        }

        if trade.market.base == trade.market.quote {
            anyhow::bail!(
                "StateResponse contains trade {} with invalid market",
                trade.id
            );
        }

        if trade.created_at == 0 || trade.updated_at < trade.created_at {
            anyhow::bail!(
                "StateResponse contains trade {} with invalid timestamps",
                trade.id
            );
        }

        let is_known_trade = trade_book.trades.iter().any(|t| t.id == trade.id);

        let is_known_order = |order_id: uuid::Uuid| -> bool {
            state.orders.iter().any(|o| o.id == order_id)
                || state.removed_orders.contains(&order_id)
                || order_book.orders.iter().any(|o| o.id == order_id)
                || order_book.removed_orders.contains(&order_id)
        };

        if !is_known_trade {
            if !is_known_order(trade.buy_order) {
                anyhow::bail!(
                    "StateResponse trade {} references unknown buy order {}",
                    trade.id,
                    trade.buy_order
                );
            }

            if !is_known_order(trade.sell_order) {
                anyhow::bail!(
                    "StateResponse trade {} references unknown sell order {}",
                    trade.id,
                    trade.sell_order
                );
            }
        }

        let find_order = |order_id: uuid::Uuid| -> Option<protocol::message::Order> {
            state
                .orders
                .iter()
                .find(|o| o.id == order_id)
                .cloned()
                .or_else(|| order_book.orders.iter().find(|o| o.id == order_id).cloned())
        };

        if let Some(buy_order) = find_order(trade.buy_order) {
            if buy_order.side != protocol::message::Side::Buy {
                anyhow::bail!(
                    "StateResponse trade {} references a non-Buy order as buy_order",
                    trade.id
                );
            }

            if trade.buyer != buy_order.trader {
                anyhow::bail!(
                    "StateResponse trade {} buyer does not match buy order",
                    trade.id
                );
            }

            if trade.market != buy_order.market {
                anyhow::bail!(
                    "StateResponse trade {} market does not match buy order market",
                    trade.id
                );
            }

            if trade.price > buy_order.price {
                anyhow::bail!("StateResponse trade {} exceeds buy order price", trade.id);
            }
        }

        if let Some(sell_order) = find_order(trade.sell_order) {
            if sell_order.side != protocol::message::Side::Sell {
                anyhow::bail!(
                    "StateResponse trade {} references a non-Sell order as sell_order",
                    trade.id
                );
            }

            if trade.seller != sell_order.trader {
                anyhow::bail!(
                    "StateResponse trade {} seller does not match sell order",
                    trade.id
                );
            }

            if trade.market != sell_order.market {
                anyhow::bail!(
                    "StateResponse trade {} market does not match sell order market",
                    trade.id
                );
            }

            if trade.price < sell_order.price {
                anyhow::bail!("StateResponse trade {} is below sell order price", trade.id);
            }
        }
    }

    // Do not allow the remote snapshot to claim that a trade was applied
    // without that trade actually being present in the snapshot or locally.
    for trade_id in &state.applied_trades {
        if trade_id.is_empty() {
            anyhow::bail!("StateResponse contains an empty applied trade ID");
        }

        if !state
            .trades
            .iter()
            .any(|trade| trade.id.to_string() == *trade_id)
            && !trade_book
                .trades
                .iter()
                .any(|trade| trade.id.to_string() == *trade_id)
        {
            anyhow::bail!(
                "StateResponse references unknown applied trade {}",
                trade_id
            );
        }
    }

    Ok(())
}

pub fn create_snapshot(order_book: &OrderBook, trade_book: &TradeBook) -> StateSnapshot {
    let mut orders: Vec<_> = order_book
        .orders
        .iter()
        .filter(|order| !order_book.removed_orders.contains(&order.id))
        .cloned()
        .collect();

    orders.sort_by_key(|order| order.id);
    orders.dedup_by_key(|order| order.id);

    StateSnapshot {
        orders,
        trades: trade_book.trades.clone(),
        removed_orders: order_book.removed_orders.clone(),
        applied_trades: order_book.applied_trades.clone(),
    }
}

pub fn apply_snapshot(
    state: StateSnapshot,
    order_book: &mut OrderBook,
    trade_book: &mut TradeBook,
) -> Result<()> {
    validate_state_snapshot(&state, order_book, trade_book)?;

    println!("=== State Sync Received ===");

    // Apply the snapshot to temporary copies first.
    //
    // This makes snapshot processing transactional from the caller's
    // perspective: if anything fails, the live books are untouched.
    let mut next_order_book = order_book.clone();
    let mut next_trade_book = trade_book.clone();

    for order_id in state.removed_orders {
        if !next_order_book.removed_orders.contains(&order_id) {
            next_order_book.removed_orders.push(order_id);
        }

        next_order_book.orders.retain(|order| order.id != order_id);
    }

    for order in state.orders {
        if next_order_book.removed_orders.contains(&order.id) {
            continue;
        }

        next_order_book.update_order_from_snapshot(order);
    }

    // Snapshot orders are already materialized at their current quantities.
    // Snapshot trades are historical records only and must not be replayed
    for trade in state.trades {
        next_trade_book.add_trade(trade);
    }

    for trade_id in state.applied_trades {
        next_order_book.mark_trade_applied(&trade_id);

        next_order_book
            .pending_trades
            .retain(|trade| trade.id.to_string() != trade_id);
    }

    // Persist the fully merged state before committing it to the live books.
    next_order_book.save_orders()?;
    next_trade_book.save_trades()?;

    // Commit only after all processing and persistence succeeded.
    *order_book = next_order_book;
    *trade_book = next_trade_book;

    order_book.print_orders();
    trade_book.print_trades();

    println!("=== State Sync Complete ===");

    Ok(())
}
