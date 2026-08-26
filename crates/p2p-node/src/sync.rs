use anyhow::Result;

use protocol::state::StateSnapshot;

use crate::orderbook::OrderBook;
use crate::trades::TradeBook;

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
    println!("=== State Sync Received ===");

    for order_id in state.removed_orders {
        if !order_book.removed_orders.contains(&order_id) {
            order_book.removed_orders.push(order_id);
        }

        order_book.orders.retain(|order| order.id != order_id);
    }

    for order in state.orders {
        if order_book.removed_orders.contains(&order.id) {
            continue;
        }

        order_book.update_order_from_snapshot(order);
    }

    for trade in state.trades {
        trade_book.add_trade(trade);
    }

    for trade_id in state.applied_trades {
        order_book.mark_trade_applied(&trade_id);

        order_book
            .pending_trades
            .retain(|trade| trade.id.to_string() != trade_id);
    }

    order_book.save_orders()?;
    trade_book.save_trades()?;

    order_book.print_orders();
    trade_book.print_trades();

    println!("=== State Sync Complete ===");

    Ok(())
}
