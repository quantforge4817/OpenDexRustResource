use crate::exchange::{Exchange, RecordTrade};
use anyhow::Result;
use libp2p::PeerId;
use std::collections::HashMap;
pub struct PendingTradeState {
    change: TradeStateChanged,
    sender: String,
}
const MAX_PENDING_TRADE_STATES: usize = 10_000;

pub struct HandlerContext<'a> {
    pub swarm: &'a mut libp2p::Swarm<crate::OpenDexBehaviour>,
    pub topic: &'a libp2p::gossipsub::IdentTopic,
    pub local_peer_id: &'a PeerId,
    pub order_book: &'a mut OrderBook,
    pub trade_book: &'a mut TradeBook,
    pub balance_book: &'a mut BalanceBook,
    pub pending_trade_states: &'a mut HashMap<Uuid, PendingTradeState>,
    pub p2p_offer_book: &'a mut P2POfferBook,
}

use uuid::Uuid;

use crate::sync::apply_snapshot;

use protocol::message::{
    MessageType, Message, Order, OrderCancelled, Trade, TradeState, TradeStateChanged,
};
use protocol::p2p::{P2POffer, P2POfferCancelled, P2PTrade, P2PTradeStateChanged};

use protocol::state::StateSnapshot;

use crate::balances::BalanceBook;
use crate::escrow::{fund_local_escrow, release_local_escrow, return_local_escrow};
use crate::network::publish_message;
use crate::orderbook::OrderBook;
use crate::p2p_offer_book::P2POfferBook;
use crate::settlement::settle_local_trade;
use crate::trade_state::{advance_trade_to_state, trade_state_rank};
use crate::trades::TradeBook;

fn is_happy_path_state(state: &TradeState) -> bool {
    matches!(
        state,
        TradeState::Created
            | TradeState::Accepted
            | TradeState::EscrowFunding
            | TradeState::EscrowFunded
            | TradeState::PaymentPending
            | TradeState::PaymentSent
            | TradeState::PaymentConfirmed
            | TradeState::PayoutPending
            | TradeState::Settled
    )
}

pub fn is_trade_participant(trade: &Trade, sender: &str) -> bool {
    sender == trade.buyer || sender == trade.seller
}
pub fn is_authenticated_sender(claimed_sender: &str, authenticated_peer_id: &PeerId) -> bool {
    claimed_sender == authenticated_peer_id.to_string()
}

pub fn is_order_owner(order: &Order, sender: &str) -> bool {
    order.trader == sender
}
pub use crate::sync::validate_state_snapshot;

pub fn handle_state_request(
    order_book: &OrderBook,
    trade_book: &TradeBook,
    swarm: &mut libp2p::Swarm<crate::OpenDexBehaviour>,
    topic: &libp2p::gossipsub::IdentTopic,
    local_peer_id: &PeerId,
) -> Result<()> {
    let state = crate::sync::create_snapshot(order_book, trade_book);

    let message = Message {
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

pub fn handle_order_cancelled(
    cancelled: OrderCancelled,
    order_book: &mut OrderBook,
    balance_book: &mut BalanceBook,
) -> Result<()> {
    // Look up the order before removing it so we can unlock the balance.
    let order = order_book.get_order(&cancelled.order_id).cloned();

    if order_book.remove_order(cancelled.order_id) {
        tracing::info!(order_id = %cancelled.order_id, "Removed cancelled order");

        // Unlock the balance reservation for the cancelled order.
        if let Some(order) = &order {
            if let Err(e) = balance_book.unlock_for_order(order) {
                tracing::warn!(order_id = %cancelled.order_id, error = %e, "Failed to unlock balance for cancelled order");
            }

            balance_book.save()?;
        }
    } else {
        tracing::debug!(order_id = %cancelled.order_id, "Cancelled order not found");
    }

    order_book.save_orders()?;
    order_book.print_orders();

    Ok(())
}

pub fn handle_order_created(
    order: Order,
    actor: &str,
    order_book: &mut OrderBook,
    trade_book: &mut TradeBook,
    balance_book: &mut BalanceBook,
    swarm: &mut libp2p::Swarm<crate::OpenDexBehaviour>,
    topic: &libp2p::gossipsub::IdentTopic,
    local_peer_id: &PeerId,
) -> Result<()> {
    if order_book.is_order_removed(&order.id) {
        tracing::debug!(
            order_id = %order.id,
            "Ignoring remote order because it was already consumed"
        );
    } else if order_book.is_order_active(&order.id) {
        tracing::debug!(order_id = %order.id, "Order already known");
    } else {
        tracing::info!(order_id = %order.id, "=== Remote Order Received ===");
        tracing::debug!("{:?}", order);

        let mut exchange = Exchange::new(order_book, trade_book, balance_book);
        let trades = exchange.submit_order(crate::exchange::SubmitOrder {
            actor: actor.to_string(),
            order: order.clone(),
        })?;

        if trades.is_empty() {
            order_book.print_orders();
        } else {
            for trade in trades {
                tracing::info!(trade_id = %trade.id, "=== Trade Created From Remote Order ===");
                tracing::debug!("{:?}", trade);

                let trade_message = Message {
                    message_type: MessageType::TradeCreated,
                    sender: local_peer_id.to_string(),
                    payload: serde_json::to_string(&trade)?,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                };

                publish_message(swarm, topic, &trade_message)?;
            }

            order_book.print_orders();
        }
    }

    Ok(())
}

pub fn handle_trade_created(
    trade: Trade,
    actor: &str,
    trade_book: &mut TradeBook,
    order_book: &mut OrderBook,
    pending_trade_states: &mut HashMap<Uuid, PendingTradeState>,
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
) -> Result<()> {
    tracing::info!(trade_id = %trade.id, "=== Trade Created Received ===");
    tracing::debug!("{:?}", trade);

    let recorded = {
        let mut exchange = Exchange::new(order_book, trade_book, balance_book);

        exchange.record_trade(RecordTrade {
            actor: actor.to_string(),
            trade: trade.clone(),
        })?
    };

    if !recorded {
        tracing::debug!(trade_id = %trade.id, "Remote trade already known");
        return Ok(());
    }

    trade_book.save_trades()?;

    // Check whether the trade is still pending (both orders not yet
    // available).  apply_trade() inside Exchange::record_trade already
    // removes the pending entry on success, so a count of 0 pending
    // trades means the trade was applied immediately.
    let is_pending = order_book.pending_trades_count() > 0;
    if is_pending {
        tracing::info!(trade_id = %trade.id, "Remote trade is pending because both orders are not available yet");
    } else {
        tracing::info!(trade_id = %trade.id, "Applied remote trade to order book");
    }

    order_book.save_orders()?;

    if let Some(pending) = pending_trade_states.remove(&trade.id) {
        if !is_trade_participant(&trade, &pending.sender) {
            tracing::warn!(
                trade_id = %trade.id,
                sender = %pending.sender,
                "Ignoring unauthorized pending state change"
            );
        } else {
            advance_trade_and_settle(
                trade_book,
                balance_book,
                local_peer_id,
                trade.id,
                pending.change.state.clone(),
                pending.change.timestamp,
            )?;
        }
    }

    trade_book.save_trades()?;

    Ok(())
}

fn advance_trade_and_settle(
    trade_book: &mut TradeBook,
    balance_book: &mut BalanceBook,
    local_peer_id: &PeerId,
    trade_id: Uuid,
    target_state: TradeState,
    timestamp: u64,
) -> Result<()> {
    tracing::info!(
        trade_id = %trade_id,
        target_state = ?target_state,
        "Advancing trade state"
    );

    advance_trade_to_state(trade_book, trade_id, target_state, timestamp)?;

    let Some(updated_trade) = trade_book.get_trade(trade_id).cloned() else {
        anyhow::bail!("Trade {} disappeared after state transition", trade_id);
    };

    match updated_trade.state {
        TradeState::EscrowFunded => {
            // Move seller's locked base asset to escrowed
            tracing::info!(
                trade_id = %trade_id,
                seller = %updated_trade.seller,
                amount = updated_trade.amount,
                "Trade reached EscrowFunded, funding escrow"
            );
            if let Err(e) = fund_local_escrow(balance_book, local_peer_id, &updated_trade) {
                tracing::warn!(
                    trade_id = %trade_id,
                    seller = %updated_trade.seller,
                    amount = updated_trade.amount,
                    error = %e,
                    "Escrow funding failed"
                );
            }
        }
        TradeState::Settled => {
            // Release escrow before normal settlement
            tracing::info!(
                trade_id = %trade_id,
                seller = %updated_trade.seller,
                amount = updated_trade.amount,
                "Trade reached Settled, releasing escrow"
            );
            if let Err(e) = release_local_escrow(balance_book, local_peer_id, &updated_trade) {
                tracing::warn!(
                    trade_id = %trade_id,
                    seller = %updated_trade.seller,
                    amount = updated_trade.amount,
                    error = %e,
                    "Escrow release failed"
                );
            }
            settle_local_trade(balance_book, local_peer_id, &updated_trade)?;
        }
        TradeState::Cancelled => {
            // Return escrowed funds to seller's locked balance
            tracing::info!(
                trade_id = %trade_id,
                seller = %updated_trade.seller,
                amount = updated_trade.amount,
                "Trade reached Cancelled, returning escrow"
            );
            if let Err(e) = return_local_escrow(balance_book, local_peer_id, &updated_trade) {
                tracing::warn!(
                    trade_id = %trade_id,
                    seller = %updated_trade.seller,
                    amount = updated_trade.amount,
                    error = %e,
                    "Escrow return failed"
                );
            }
        }
        _ => {}
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
                tracing::warn!(
                    trade_id = %change.trade_id,
                    sender = %sender,
                    "Ignoring unauthorized state change"
                );
                return Ok(());
            }

            let current_state = trade.state.clone();
            if current_state != change.state {
                advance_trade_and_settle(
                    trade_book,
                    balance_book,
                    local_peer_id,
                    change.trade_id,
                    change.state.clone(),
                    change.timestamp,
                )?;
            }
        }

        None => {
            if pending_trade_states.len() >= MAX_PENDING_TRADE_STATES {
                tracing::warn!(
                    trade_id = %change.trade_id,
                    "Rejecting pending state change: pending-state cache is full"
                );
                return Ok(());
            }

            let pending = PendingTradeState {
                change: change.clone(),
                sender: sender.to_string(),
            };
            pending_trade_states
                .entry(change.trade_id)
                .and_modify(|existing| {
                    let existing_state = &existing.change.state;
                    let incoming_state = &change.state;

                    // A duplicate pending state is harmless. Keep the original
                    // authenticated sender and timestamp.
                    if existing_state == incoming_state {
                        return;
                    }

                    // If the incoming state is a direct protocol transition from
                    // the currently pending state, it is the better candidate.
                    if existing_state.can_transition_to(incoming_state) {
                        *existing = PendingTradeState {
                            change: change.clone(),
                            sender: sender.to_string(),
                        };
                        return;
                    }

                    // For the normal happy path, allow a later state to replace
                    // an earlier pending state even when intermediate messages
                    // were missed.
                    //
                    // This comparison is deliberately restricted to the linear
                    // happy-path states. Exceptional states such as Disputed,
                    // Cancelled and Expired must never be ordered by rank.
                    if is_happy_path_state(existing_state)
                        && is_happy_path_state(incoming_state)
                        && trade_state_rank(incoming_state) > trade_state_rank(existing_state)
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

// ---------------------------------------------------------------------------
// P2P trading message handlers
// ---------------------------------------------------------------------------

pub fn handle_p2p_offer_created(
    offer: P2POffer,
    actor: &str,
    p2p_offer_book: &mut P2POfferBook,
) -> Result<()> {
    if offer.maker != actor {
        tracing::warn!(
            offer_id = %offer.id,
            maker = %offer.maker,
            sender = %actor,
            "Rejecting P2P offer: maker mismatch"
        );
        return Ok(());
    }

    if p2p_offer_book.offers.contains_key(&offer.id) {
        tracing::debug!(offer_id = %offer.id, "P2P offer already known");
        return Ok(());
    }

    tracing::info!(
        offer_id = %offer.id,
        "Received P2P offer: {} {} {} @ {} {} via {}",
        offer.side,
        offer.amount as f64 / protocol::message::AMOUNT_SCALE as f64,
        offer.crypto_asset,
        offer.price as f64 / protocol::p2p::FIAT_SCALE as f64,
        offer.currency,
        offer.payment_method,
    );

    p2p_offer_book.add_offer(offer);
    p2p_offer_book.save_offers()?;
    p2p_offer_book.print_offers();

    Ok(())
}

pub fn handle_p2p_offer_cancelled(
    cancelled: P2POfferCancelled,
    p2p_offer_book: &mut P2POfferBook,
) -> Result<()> {
    if p2p_offer_book.remove_offer(cancelled.offer_id).is_some() {
        tracing::info!(offer_id = %cancelled.offer_id, "Removed cancelled P2P offer");
        p2p_offer_book.save_offers()?;
        p2p_offer_book.print_offers();
    } else {
        tracing::debug!(offer_id = %cancelled.offer_id, "P2P offer not found");
    }

    Ok(())
}

pub fn handle_p2p_trade_created(
    trade: P2PTrade,
    actor: &str,
    p2p_offer_book: &mut P2POfferBook,
) -> Result<()> {
    if !(trade.maker == actor || trade.taker == actor) {
        tracing::warn!(
            trade_id = %trade.id,
            sender = %actor,
            "Rejecting P2P trade: not a participant"
        );
        return Ok(());
    }

    if p2p_offer_book.trades.contains_key(&trade.id) {
        tracing::debug!(trade_id = %trade.id, "P2P trade already known");
        return Ok(());
    }

    tracing::info!(
        trade_id = %trade.id,
        "Received P2P trade: {} {} {} @ {} {}",
        trade.side,
        trade.amount as f64 / protocol::message::AMOUNT_SCALE as f64,
        trade.crypto_asset,
        trade.price as f64 / protocol::p2p::FIAT_SCALE as f64,
        trade.currency,
    );

    p2p_offer_book.add_trade(trade);
    p2p_offer_book.save_trades()?;

    Ok(())
}

pub fn handle_p2p_trade_state_changed(
    change: P2PTradeStateChanged,
    sender: &str,
    p2p_offer_book: &mut P2POfferBook,
) -> Result<()> {
    let trade = match p2p_offer_book.get_trade(change.trade_id) {
        Some(trade) => trade.clone(),
        None => {
            tracing::debug!(
                trade_id = %change.trade_id,
                "P2P trade state change for unknown trade"
            );
            return Ok(());
        }
    };

    if !(trade.maker == sender || trade.taker == sender) {
        tracing::warn!(
            trade_id = %change.trade_id,
            sender = %sender,
            "Ignoring unauthorized P2P trade state change"
        );
        return Ok(());
    }

    let mut trade = trade;
    if trade.transition(change.state.clone(), change.timestamp).is_ok() {
        p2p_offer_book.trades.insert(change.trade_id, trade);
        p2p_offer_book.save_trades()?;
        tracing::info!(
            trade_id = %change.trade_id,
            state = ?change.state,
            "P2P trade state advanced"
        );
    }

    Ok(())
}

pub fn handle_gossipsub_message(
    data: &[u8],
    authenticated_peer_id: &PeerId,
    ctx: &mut HandlerContext<'_>,
) -> Result<()> {
    let msg = match serde_json::from_slice::<Message>(data) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!(error = %e, "Invalid OpenDEX message");
            return Ok(());
        }
    };

    let authenticated_sender = authenticated_peer_id.to_string();

    if !is_authenticated_sender(&msg.sender, authenticated_peer_id) {
        tracing::warn!(
            claimed = %msg.sender,
            authenticated = %authenticated_sender,
            "Rejecting message: sender identity mismatch"
        );
        return Ok(());
    }

    tracing::debug!("Received OpenDEX message");
    tracing::trace!("{:?}", msg);

    match msg.message_type {
        MessageType::OrderCreated => {
            let order = match serde_json::from_str::<Order>(&msg.payload) {
                Ok(order) => order,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to decode order");
                    return Ok(());
                }
            };

            if !is_order_owner(&order, &authenticated_sender) {
                tracing::warn!(
                    order_id = %order.id,
                    trader = %order.trader,
                    sender = %authenticated_sender,
                    "Rejecting unauthorized order"
                );
                return Ok(());
            }

            handle_order_created(
                order,
                &authenticated_sender,
                ctx.order_book,
                ctx.trade_book,
                ctx.balance_book,
                ctx.swarm,
                ctx.topic,
                ctx.local_peer_id,
            )?;
        }

        MessageType::OrderCancelled => {
            let cancelled = match serde_json::from_str::<OrderCancelled>(&msg.payload) {
                Ok(cancelled) => cancelled,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to decode cancellation");
                    return Ok(());
                }
            };
            if cancelled.trader != authenticated_sender {
                tracing::warn!(
                    order_id = %cancelled.order_id,
                    claimed_trader = %cancelled.trader,
                    authenticated = %authenticated_sender,
                    "Rejecting cancellation: trader mismatch"
                );
                return Ok(());
            }

            let order = match ctx.order_book.get_order(&cancelled.order_id) {
                Some(order) => order,
                None => {
                    tracing::debug!(
                        order_id = %cancelled.order_id,
                        "Cancellation received for unknown order"
                    );
                    return Ok(());
                }
            };

            if !is_order_owner(order, &authenticated_sender) {
                tracing::warn!(
                    order_id = %order.id,
                    owner = %order.trader,
                    sender = %authenticated_sender,
                    "Rejecting unauthorized cancellation"
                );
                return Ok(());
            }

            handle_order_cancelled(cancelled, ctx.order_book, ctx.balance_book)?;
        }

        MessageType::StateRequest => {
            handle_state_request(
                ctx.order_book,
                ctx.trade_book,
                ctx.swarm,
                ctx.topic,
                ctx.local_peer_id,
            )?;
        }

        MessageType::StateResponse => {
            let state = match serde_json::from_str::<StateSnapshot>(&msg.payload) {
                Ok(state) => state,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to decode StateResponse");
                    return Ok(());
                }
            };

            validate_state_snapshot(&state, ctx.order_book, ctx.trade_book)?;
            handle_state_response(state, ctx.order_book, ctx.trade_book)?;
        }

        MessageType::TradeCreated => {
            let trade = match serde_json::from_str::<Trade>(&msg.payload) {
                Ok(trade) => trade,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to decode TradeCreated");
                    return Ok(());
                }
            };
            if !is_trade_participant(&trade, &authenticated_sender) {
                tracing::warn!(
                    trade_id = %trade.id,
                    maker = %trade.maker,
                    taker = %trade.taker,
                    sender = %authenticated_sender,
                    "Rejecting unauthorized trade"
                );
                return Ok(());
            }

            handle_trade_created(
                trade,
                &authenticated_sender,
                ctx.trade_book,
                ctx.order_book,
                ctx.pending_trade_states,
                ctx.balance_book,
                ctx.local_peer_id,
            )?;
        }

        MessageType::TradeStateChanged => {
            let change = match serde_json::from_str::<TradeStateChanged>(&msg.payload) {
                Ok(change) => change,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to decode TradeStateChanged");
                    return Ok(());
                }
            };

            handle_trade_state_changed(
                change,
                &authenticated_sender,
                ctx.trade_book,
                ctx.pending_trade_states,
                ctx.balance_book,
                ctx.local_peer_id,
            )?;
        }

        MessageType::PeerAnnouncement => {
            tracing::info!(sender = %authenticated_sender, "Received PeerAnnouncement");
        }

        MessageType::P2POfferCreated => {
            let offer = match serde_json::from_str::<P2POffer>(&msg.payload) {
                Ok(offer) => offer,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to decode P2P offer");
                    return Ok(());
                }
            };

            handle_p2p_offer_created(
                offer,
                &authenticated_sender,
                ctx.p2p_offer_book,
            )?;
        }

        MessageType::P2POfferCancelled => {
            let cancelled = match serde_json::from_str::<P2POfferCancelled>(&msg.payload) {
                Ok(cancelled) => cancelled,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to decode P2P offer cancellation");
                    return Ok(());
                }
            };

            if cancelled.maker != authenticated_sender {
                tracing::warn!(
                    offer_id = %cancelled.offer_id,
                    claimed_maker = %cancelled.maker,
                    authenticated = %authenticated_sender,
                    "Rejecting P2P offer cancellation: maker mismatch"
                );
                return Ok(());
            }

            handle_p2p_offer_cancelled(cancelled, ctx.p2p_offer_book)?;
        }

        MessageType::P2PTradeCreated => {
            let trade = match serde_json::from_str::<P2PTrade>(&msg.payload) {
                Ok(trade) => trade,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to decode P2P trade");
                    return Ok(());
                }
            };

            handle_p2p_trade_created(
                trade,
                &authenticated_sender,
                ctx.p2p_offer_book,
            )?;
        }

        MessageType::P2PTradeStateChanged => {
            let change = match serde_json::from_str::<P2PTradeStateChanged>(&msg.payload) {
                Ok(change) => change,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to decode P2P trade state change");
                    return Ok(());
                }
            };

            handle_p2p_trade_state_changed(
                change,
                &authenticated_sender,
                ctx.p2p_offer_book,
            )?;
        }
    }

    Ok(())
}

pub fn handle_swarm_event(
    event: libp2p::swarm::SwarmEvent<crate::OpenDexBehaviourEvent>,
    ctx: &mut HandlerContext<'_>,
) -> Result<()> {
    match event {
        libp2p::swarm::SwarmEvent::ConnectionEstablished {
            peer_id,
            num_established,
            ..
        } => {
            tracing::info!(
                peer = %peer_id,
                connections = num_established,
                "Connected to peer"
            );
        }

        libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
            tracing::info!(peer = %peer_id, cause = ?cause, "Connection closed");
        }

        libp2p::swarm::SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
            tracing::warn!(peer = ?peer_id, error = %error, "Outgoing connection error");
        }

        libp2p::swarm::SwarmEvent::Behaviour(event) => match event {
            crate::OpenDexBehaviourEvent::Identify(libp2p::identify::Event::Received {
                peer_id,
                info,
                ..
            }) => {
                tracing::info!(peer = %peer_id, "Identify received from peer");

                for addr in info.listen_addrs {
                    tracing::debug!(address = %addr, "Adding address to Kademlia");

                    ctx.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                }

                tracing::info!(peer = %peer_id, "Kademlia peer added");
            }

            crate::OpenDexBehaviourEvent::Gossipsub(event) => match event {
                libp2p::gossipsub::Event::Subscribed {
                    peer_id,
                    topic: subscribed_topic,
                } => {
                    tracing::info!(peer = %peer_id, topic = %subscribed_topic, "Peer subscribed");

                    let request = Message {
                        message_type: MessageType::StateRequest,
                        sender: ctx.local_peer_id.to_string(),
                        payload: "{}".to_string(),
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    };

                    publish_message(ctx.swarm, ctx.topic, &request)?;
                }

                libp2p::gossipsub::Event::Message {
                    propagation_source,
                    message,
                    ..
                } => {
                    handle_gossipsub_message(&message.data, &propagation_source, ctx)?;
                }

                _ => {}
            },

            crate::OpenDexBehaviourEvent::Kademlia(event) => {
                tracing::debug!("Kademlia event: {:?}", event);
            }

            crate::OpenDexBehaviourEvent::Ping(_) => {}

            _ => {}
        },

        _ => {}
    }

    Ok(())
}
