use anyhow::Result;
use libp2p::PeerId;
use std::collections::HashMap;
use uuid::Uuid;

use crate::sync::apply_snapshot;

use protocol::message::{
    MessageType, OpenDexMessage, Order, OrderCancelled, Trade, TradeState, TradeStateChanged,
};

use protocol::state::StateSnapshot;

use crate::balances::BalanceBook;
use crate::network::publish_message;
use crate::orderbook::OrderBook;
use crate::settlement::settle_local_trade;
use crate::trade_state::{advance_trade_to_state, trade_state_rank};
use crate::trades::TradeBook;

pub fn handle_state_request(
    order_book: &OrderBook,
    trade_book: &TradeBook,
    swarm: &mut libp2p::Swarm<crate::OpenDexBehaviour>,
    topic: &libp2p::gossipsub::IdentTopic,
    local_peer_id: &PeerId,
) -> Result<()> {
    let state = crate::sync::create_snapshot(order_book, trade_book);

    let message = OpenDexMessage {
        message_type: MessageType::StateResponse,
        sender: local_peer_id.to_string(),
        payload: serde_json::to_string(&state)?,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };

    publish_message(swarm, topic, &message)?;

    Ok(())
}

pub fn handle_state_response(
    state: StateSnapshot,
    order_book: &mut OrderBook,
    trade_book: &mut TradeBook,
) -> Result<()> {
    apply_snapshot(state, order_book, trade_book)
}

pub fn handle_order_cancelled(cancelled: OrderCancelled, order_book: &mut OrderBook) -> Result<()> {
    if order_book.remove_order(cancelled.order_id) {
        println!("Removed cancelled order: {}", cancelled.order_id);
    } else {
        println!("Cancelled order not found: {}", cancelled.order_id);
    }

    order_book.save_orders()?;
    order_book.print_orders();

    Ok(())
}

pub fn handle_order_created(
    order: Order,
    order_book: &mut OrderBook,
    trade_book: &mut TradeBook,
    swarm: &mut libp2p::Swarm<crate::OpenDexBehaviour>,
    topic: &libp2p::gossipsub::IdentTopic,
    local_peer_id: &PeerId,
) -> Result<()> {
    if order_book.removed_orders.contains(&order.id) {
        println!(
            "Ignoring remote order {} because it was already consumed",
            order.id
        );
    } else if order_book.orders.iter().any(|o| o.id == order.id) {
        println!("Order already known: {}", order.id);
    } else {
        println!("=== Remote Order Received ===");
        println!("{:?}", order);

        let trades = order_book.match_order(&order);

        order_book.add_order(order.clone());

        if trades.is_empty() {
            order_book.save_orders()?;
            order_book.print_orders();
        } else {
            for trade in trades {
                println!("=== Trade Created From Remote Order ===");
                println!("{:?}", trade);

                if !trade_book.add_trade(trade.clone()) {
                    println!("Trade already known: {}", trade.id);
                    continue;
                }

                if !order_book.apply_trade(&trade) {
                    eprintln!("Trade {} could not be applied after matching", trade.id);
                    continue;
                }

                order_book
                    .pending_trades
                    .retain(|pending| pending.id != trade.id);

                order_book.save_orders()?;
                trade_book.save_trades()?;

                let trade_message = OpenDexMessage {
                    message_type: protocol::message::MessageType::TradeCreated,
                    sender: local_peer_id.to_string(),
                    payload: serde_json::to_string(&trade)?,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                };

                publish_message(swarm, topic, &trade_message)?;
            }

            order_book.save_orders()?;
            trade_book.save_trades()?;
            order_book.print_orders();
        }
    }

    Ok(())
}

pub fn handle_trade_created(
    trade: Trade,
    trade_book: &mut TradeBook,
    order_book: &mut OrderBook,
    pending_trade_states: &mut HashMap<Uuid, TradeStateChanged>,
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
) -> Result<()> {
    println!("=== Trade Created Received ===");
    println!("{:?}", trade);

    if trade_book.add_trade(trade.clone()) {
        trade_book.save_trades()?;

        order_book.mark_trade_applied(&trade.id.to_string());

        if !order_book.removed_orders.contains(&trade.buy_order) {
            order_book.removed_orders.push(trade.buy_order);
        }

        if !order_book.removed_orders.contains(&trade.sell_order) {
            order_book.removed_orders.push(trade.sell_order);
        }

        order_book
            .orders
            .retain(|order| order.id != trade.buy_order && order.id != trade.sell_order);

        order_book
            .pending_trades
            .retain(|pending| pending.id != trade.id);

        order_book.save_orders()?;

        if let Some(pending_change) = pending_trade_states.remove(&trade.id) {
            advance_trade_to_state(
                trade_book,
                trade.id,
                pending_change.state.clone(),
                pending_change.timestamp,
            )?;

            if pending_change.state == TradeState::Settled {
                if let Some(updated_trade) = trade_book.get_trade(trade.id).cloned() {
                    settle_local_trade(balance_book, local_peer_id, &updated_trade)?;
                }
            }
        }

        trade_book.save_trades()?;
    } else {
        println!("Remote trade already known: {}", trade.id);
    }

    Ok(())
}

pub fn handle_trade_state_changed(
    change: TradeStateChanged,
    trade_book: &mut TradeBook,
    pending_trade_states: &mut HashMap<Uuid, TradeStateChanged>,
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
) -> Result<()> {
    match trade_book.get_trade(change.trade_id) {
        Some(trade) => {
            let current_state = trade.state.clone();

            if current_state != change.state {
                advance_trade_to_state(
                    trade_book,
                    change.trade_id,
                    change.state.clone(),
                    change.timestamp,
                )?;

                if change.state == TradeState::Settled {
                    if let Some(updated_trade) = trade_book.get_trade(change.trade_id).cloned() {
                        settle_local_trade(balance_book, local_peer_id, &updated_trade)?;
                    }
                }
            }
        }

        None => {
            pending_trade_states
                .entry(change.trade_id)
                .and_modify(|existing| {
                    let existing_rank = trade_state_rank(&existing.state);
                    let incoming_rank = trade_state_rank(&change.state);

                    if incoming_rank > existing_rank
                        || existing.state.can_transition_to(&change.state)
                    {
                        *existing = change.clone();
                    }
                })
                .or_insert(change);
        }
    }

    Ok(())
}
