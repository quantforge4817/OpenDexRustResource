use anyhow::Result;

use protocol::state::StateSnapshot;

use crate::orderbook::OrderBook;
use crate::trades::TradeBook;

pub fn validate_state_snapshot(
    state: &StateSnapshot,
    order_book: &OrderBook,
    trade_book: &TradeBook,
) -> Result<()> {
    // AUDIT-010: Reject snapshots with invalid version numbers.
    // Version 0 indicates an uninitialized snapshot and must be rejected.
    if state.version == 0 {
        anyhow::bail!("StateResponse has invalid version 0");
    }

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
    // A remote snapshot must not delete an order that is still
    // locally active. Order removal must come from an authenticated
    // local protocol event or a validated trade.
    for order_id in &state.removed_orders {
        if order_book.is_order_active(order_id) {
            anyhow::bail!(
                "StateResponse attempts to remove locally active order {}",
                order_id
            );
        }
    }

    // A remote snapshot must not contain duplicate trade IDs.
    let mut trade_ids = std::collections::HashSet::new();

    for trade in &state.trades {
        order_book.validate_trade_structure(trade)?;
        order_book.validate_trade_identity(trade)?;

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

        let is_known_trade = trade_book.has_trade(&trade.id);

        let is_known_order = |order_id: uuid::Uuid| -> bool {
            state.orders.iter().any(|o| o.id == order_id)
                || state.removed_orders.contains(&order_id)
                || order_book.is_order_active(&order_id)
                || order_book.is_order_removed(&order_id)
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
                .or_else(|| order_book.get_order(&order_id).cloned())
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
        if !state
            .trades
            .iter()
            .any(|trade| trade.id == *trade_id)
            && !trade_book.has_trade(trade_id)
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
    // AUDIT-010: Version is derived from the last accepted version + 1,
    // starting at 1 for the first snapshot.
    let version = order_book.snapshot_version() + 1;

    StateSnapshot {
        version,
        orders: order_book.active_orders(),
        trades: trade_book.all_trades(),
        removed_orders: order_book.removed_order_ids(),
        applied_trades: order_book.applied_trade_ids(),
    }
}

pub fn apply_snapshot(
    state: StateSnapshot,
    order_book: &mut OrderBook,
    trade_book: &mut TradeBook,
) -> Result<()> {
    validate_state_snapshot(&state, order_book, trade_book)?;

    // AUDIT-010: Reject stale or duplicate snapshots.
    // The incoming version must be strictly greater than the last accepted version.
    if state.version <= order_book.snapshot_version() {
        tracing::warn!(
            incoming_version = state.version,
            last_accepted_version = order_book.snapshot_version(),
            "Rejecting stale snapshot"
        );
        return Ok(());
    }

    tracing::info!(
        version = state.version,
        "=== State Sync Received ==="
    );

    // Remote snapshots are reconciliation data, not an authority over
    // already-known local state.
    //
    // In particular, a snapshot must never:
    //   - delete a locally known active order;
    //   - reduce a locally known order amount;
    //   - mark a locally known trade as applied.
    //
    // Those state transitions must come from authenticated protocol events
    // and validated trades.

    let mut next_order_book = order_book.clone();
    let mut next_trade_book = trade_book.clone();

    // Reconcile orders from the materialized snapshot.
    //
    // Existing orders are updated through update_order_from_snapshot(),
    // which permits a legitimate reduction in remaining amount while
    // rejecting an increase. Orders already known to be removed cannot
    // be resurrected.
    for order in state.orders {
        if next_order_book.contains_removed_order(&order.id) {
            continue;
        }

        if next_order_book.is_order_active(&order.id) {
            next_order_book.update_order_from_snapshot(order);
        } else {
            next_order_book.upsert_order(order);
        }
    }

    // Historical trades may be learned from a snapshot, but adding them
    // must never replay them against the order book.
    for trade in state.trades {
        next_trade_book.add_trade(trade);
    }

    // DO NOT import remote removed_orders into the local removal set.
    //
    // A remote peer cannot prove through a bare snapshot that one of our
    // locally active orders has been consumed/cancelled.
    //
    // DO NOT import remote applied_trades either.
    //
    // A trade becomes applied only when our local validated trade
    // processing actually applies it.

    // AUDIT-010: Update the last accepted version after successful application.
    next_order_book.accept_snapshot_version(state.version);

    // Persist the merged informational state before committing it.
    next_order_book.save_orders()?;
    next_trade_book.save_trades()?;

    *order_book = next_order_book;
    *trade_book = next_trade_book;

    order_book.print_orders();
    trade_book.print_trades();

    tracing::info!(
        version = state.version,
        "=== State Sync Complete ==="
    );

    Ok(())
}
