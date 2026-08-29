use anyhow::Result;
use libp2p::PeerId;

use std::collections::HashMap;
pub struct PendingTradeState {
    change: TradeStateChanged,
    sender: String,
}

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

fn is_trade_participant(trade: &Trade, sender: &str) -> bool {
    sender == trade.buyer || sender == trade.seller
}

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
    pending_trade_states: &mut HashMap<Uuid, PendingTradeState>,
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
) -> Result<()> {
    println!("=== Trade Created Received ===");
    println!("{:?}", trade);

    if trade_book.add_trade(trade.clone()) {
        trade_book.save_trades()?;

        if order_book.apply_trade(&trade) {
            println!("Applied remote trade to order book: {}", trade.id);

            order_book
                .pending_trades
                .retain(|pending| pending.id != trade.id);

            order_book.save_orders()?;
        } else {
            println!(
                "Remote trade {} is pending because both orders are not available yet",
                trade.id
            );

            order_book.save_orders()?;
        }

        if let Some(pending) = pending_trade_states.remove(&trade.id) {
            if !is_trade_participant(&trade, &pending.sender) {
                eprintln!(
                    "Ignoring unauthorized pending state change for trade {} from {}",
                    trade.id, pending.sender
                );
            } else {
                advance_trade_to_state(
                    trade_book,
                    trade.id,
                    pending.change.state.clone(),
                    pending.change.timestamp,
                )?;

                if pending.change.state == TradeState::Settled {
                    if let Some(updated_trade) = trade_book.get_trade(trade.id).cloned() {
                        settle_local_trade(balance_book, local_peer_id, &updated_trade)?;
                    }
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
    sender: &str,
    trade_book: &mut TradeBook,
    pending_trade_states: &mut HashMap<Uuid, PendingTradeState>,
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
) -> Result<()> {
    match trade_book.get_trade(change.trade_id) {
        Some(trade) => {
            if !is_trade_participant(trade, sender) {
                eprintln!(
                    "Ignoring unauthorized state change for trade {} from {}",
                    change.trade_id, sender
                );
                return Ok(());
            }

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
            let pending = PendingTradeState {
                change: change.clone(),
                sender: sender.to_string(),
            };

            pending_trade_states
                .entry(change.trade_id)
                .and_modify(|existing| {
                    let existing_rank = trade_state_rank(&existing.change.state);
                    let incoming_rank = trade_state_rank(&change.state);

                    if incoming_rank > existing_rank
                        || existing.change.state.can_transition_to(&change.state)
                    {
                        *existing = PendingTradeState {
                            change: change.clone(),
                            sender: sender.to_string(),
                        };
                    }
                })
                .or_insert(pending);
        }
    }

    Ok(())
}
pub fn handle_gossipsub_message(
    data: &[u8],
    swarm: &mut libp2p::Swarm<crate::OpenDexBehaviour>,
    topic: &libp2p::gossipsub::IdentTopic,
    local_peer_id: &PeerId,
    order_book: &mut OrderBook,
    trade_book: &mut TradeBook,
    balance_book: &mut BalanceBook,
    pending_trade_states: &mut HashMap<Uuid, PendingTradeState>,
) -> Result<()> {
    let msg = match serde_json::from_slice::<OpenDexMessage>(data) {
        Ok(msg) => msg,
        Err(e) => {
            println!("Invalid OpenDEX message: {:?}", e);
            return Ok(());
        }
    };

    println!("Received OpenDEX message:");
    println!("{:?}", msg);

    match msg.message_type {
        MessageType::OrderCreated => {
            let order = match serde_json::from_str::<Order>(&msg.payload) {
                Ok(order) => order,
                Err(e) => {
                    println!("Failed to decode order: {:?}", e);
                    return Ok(());
                }
            };

            handle_order_created(order, order_book, trade_book, swarm, topic, local_peer_id)?;
        }

        MessageType::OrderCancelled => {
            let cancelled = match serde_json::from_str::<OrderCancelled>(&msg.payload) {
                Ok(cancelled) => cancelled,
                Err(e) => {
                    println!("Failed to decode cancellation: {:?}", e);
                    return Ok(());
                }
            };

            handle_order_cancelled(cancelled, order_book)?;
        }

        MessageType::StateRequest => {
            handle_state_request(order_book, trade_book, swarm, topic, local_peer_id)?;
        }

        MessageType::StateResponse => {
            let state = match serde_json::from_str::<StateSnapshot>(&msg.payload) {
                Ok(state) => state,
                Err(e) => {
                    eprintln!("Failed to decode StateResponse: {:?}", e);
                    return Ok(());
                }
            };

            handle_state_response(state, order_book, trade_book)?;
        }

        MessageType::TradeCreated => {
            let trade = match serde_json::from_str::<Trade>(&msg.payload) {
                Ok(trade) => trade,
                Err(e) => {
                    eprintln!("Failed to decode TradeCreated: {:?}", e);
                    return Ok(());
                }
            };

            handle_trade_created(
                trade,
                trade_book,
                order_book,
                pending_trade_states,
                balance_book,
                local_peer_id,
            )?;
        }

        MessageType::TradeStateChanged => {
            let change = match serde_json::from_str::<TradeStateChanged>(&msg.payload) {
                Ok(change) => change,
                Err(e) => {
                    eprintln!("Failed to decode TradeStateChanged: {:?}", e);
                    return Ok(());
                }
            };

            handle_trade_state_changed(
                change,
                &msg.sender,
                trade_book,
                pending_trade_states,
                balance_book,
                local_peer_id,
            )?;
        }

        MessageType::PeerAnnouncement => {
            println!("Received PeerAnnouncement from {}", msg.sender);
        }
    }

    Ok(())
}

pub fn handle_swarm_event(
    event: libp2p::swarm::SwarmEvent<crate::OpenDexBehaviourEvent>,
    swarm: &mut libp2p::Swarm<crate::OpenDexBehaviour>,
    topic: &libp2p::gossipsub::IdentTopic,
    local_peer_id: &PeerId,
    order_book: &mut OrderBook,
    trade_book: &mut TradeBook,
    balance_book: &mut BalanceBook,
    pending_trade_states: &mut HashMap<Uuid, PendingTradeState>,
) -> Result<()> {
    match event {
        libp2p::swarm::SwarmEvent::ConnectionEstablished {
            peer_id,
            num_established,
            ..
        } => {
            println!(
                "Connected to peer: {:?}, total connections: {}",
                peer_id, num_established
            );
        }

        libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
            println!("Connection closed with {:?}, cause: {:?}", peer_id, cause);
        }

        libp2p::swarm::SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
            println!("Outgoing connection error to {:?}: {:?}", peer_id, error);
        }

        libp2p::swarm::SwarmEvent::Behaviour(event) => match event {
            crate::OpenDexBehaviourEvent::Identify(libp2p::identify::Event::Received {
                peer_id,
                info,
                ..
            }) => {
                println!("Identify received from peer: {:?}", peer_id);

                for addr in info.listen_addrs {
                    println!("Adding address to Kademlia: {}", addr);

                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }

                println!("Kademlia peer added: {:?}", peer_id);
            }

            crate::OpenDexBehaviourEvent::Gossipsub(event) => match event {
                libp2p::gossipsub::Event::Subscribed {
                    peer_id,
                    topic: subscribed_topic,
                } => {
                    println!("Peer {:?} subscribed to {:?}", peer_id, subscribed_topic);

                    let request = OpenDexMessage {
                        message_type: MessageType::StateRequest,
                        sender: local_peer_id.to_string(),
                        payload: "{}".to_string(),
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    };

                    publish_message(swarm, topic, &request)?;
                }

                libp2p::gossipsub::Event::Message { message, .. } => {
                    handle_gossipsub_message(
                        &message.data,
                        swarm,
                        topic,
                        local_peer_id,
                        order_book,
                        trade_book,
                        balance_book,
                        pending_trade_states,
                    )?;
                }

                _ => {}
            },

            crate::OpenDexBehaviourEvent::Kademlia(event) => {
                println!("Kademlia event: {:?}", event);
            }

            crate::OpenDexBehaviourEvent::Ping(_) => {}

            _ => {}
        },

        _ => {}
    }

    Ok(())
}
