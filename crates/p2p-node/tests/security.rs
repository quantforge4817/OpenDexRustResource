use libp2p::{PeerId, gossipsub, identity};
use p2p_node::balances::BalanceBook;
use p2p_node::message_handler::{
    HandlerContext, handle_gossipsub_message, is_authenticated_sender, is_order_owner,
    is_trade_participant,
};
use p2p_node::network::build_swarm;
use p2p_node::orderbook::OrderBook;
use p2p_node::trades::TradeBook;
use protocol::message::{
    AMOUNT_SCALE, Asset, Market, MessageType, OpenDexMessage, Order, OrderCancelled, PRICE_SCALE,
    Side, Trade, TradeState, TradeStateChanged,
};
use protocol::state::StateSnapshot;

use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

fn peer() -> (PeerId, String) {
    let key = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(key.public());
    let trader = peer_id.to_string();

    (peer_id, trader)
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "opendex-test-security-{name}-{}.json",
        Uuid::new_v4()
    ))
}

fn test_order(trader: &str) -> Order {
    Order {
        id: Uuid::new_v4(),
        trader: trader.to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Buy,
        price: 100 * PRICE_SCALE,
        amount: AMOUNT_SCALE,
    }
}

fn test_trade(buyer: &str, seller: &str) -> Trade {
    Trade {
        id: Uuid::new_v4(),
        buy_order: Uuid::new_v4(),
        sell_order: Uuid::new_v4(),
        maker: seller.to_string(),
        taker: buyer.to_string(),
        buyer: buyer.to_string(),
        seller: seller.to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        price: 100 * PRICE_SCALE,
        amount: AMOUNT_SCALE,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    }
}

fn test_message(message_type: MessageType, sender: &str, payload: String) -> OpenDexMessage {
    OpenDexMessage {
        message_type,
        sender: sender.to_string(),
        payload,
        timestamp: 1,
    }
}

#[test]
fn rejects_message_with_forged_sender() {
    let (peer_a, _) = peer();
    let (_, peer_b) = peer();

    assert!(
        !is_authenticated_sender(&peer_b, &peer_a),
        "a message claiming B must not authenticate as A"
    );
}

#[test]
fn accepts_message_from_authenticated_sender() {
    let (peer_a, trader_a) = peer();

    assert!(
        is_authenticated_sender(&trader_a, &peer_a),
        "a message claiming A must authenticate as A"
    );
}

#[test]
fn rejects_order_from_impersonated_trader() {
    let (_, trader_a) = peer();
    let (_, trader_b) = peer();

    let order = test_order(&trader_b);

    assert!(
        !is_order_owner(&order, &trader_a),
        "A must not be able to submit an order belonging to B"
    );
}

#[test]
fn rejects_order_cancellation_by_non_owner() {
    let (_, trader_a) = peer();
    let (_, trader_b) = peer();

    let order = test_order(&trader_b);

    assert!(
        !is_order_owner(&order, &trader_a),
        "A must not be able to cancel B's order"
    );
}

#[test]
fn rejects_trade_state_change_from_unauthorized_peer() {
    let (_, buyer) = peer();
    let (_, seller) = peer();
    let (_, attacker) = peer();

    let trade = test_trade(&buyer, &seller);

    assert!(
        !is_trade_participant(&trade, &attacker),
        "a non-participant must not be authorized to change trade state"
    );
}

#[test]
fn accepts_order_from_authenticated_trader() {
    let (_, trader) = peer();

    let order = test_order(&trader);

    assert!(
        is_order_owner(&order, &trader),
        "a trader must be able to authenticate its own order"
    );
}

#[test]
fn accepts_trade_state_change_from_buyer() {
    let (_, buyer) = peer();
    let (_, seller) = peer();

    let trade = test_trade(&buyer, &seller);

    assert!(is_trade_participant(&trade, &buyer));
}

#[test]
fn accepts_trade_state_change_from_seller() {
    let (_, buyer) = peer();
    let (_, seller) = peer();

    let trade = test_trade(&buyer, &seller);

    assert!(is_trade_participant(&trade, &seller));
}

#[tokio::test]
async fn handler_rejects_forged_sender_without_mutating_state() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    let (_, attacker) = peer();
    let order = test_order(&attacker);

    let message = test_message(
        MessageType::OrderCreated,
        &attacker,
        serde_json::to_string(&order).unwrap(),
    );

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-security-test");

    let mut order_book = OrderBook::new(temp_path("forged-sender-orders"));
    let mut trade_book = TradeBook::new(temp_path("forged-sender-trades"));
    let mut balance_book = BalanceBook::new(temp_path("forged-sender-balances"));
    let mut pending_trade_states = HashMap::new();

    let mut ctx = HandlerContext {
        swarm: &mut swarm,
        topic: &topic,
        local_peer_id: &authenticated_peer,
        order_book: &mut order_book,
        trade_book: &mut trade_book,
        balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states,
    };

    handle_gossipsub_message(
        &serde_json::to_vec(&message).unwrap(),
        &authenticated_peer,
        &mut ctx,
    )
    .unwrap();

    assert!(
        order_book.orders.is_empty(),
        "forged message must not create an order"
    );
}

#[tokio::test]
async fn handler_rejects_order_from_impersonated_trader_without_mutating_state() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let authenticated_trader = authenticated_peer.to_string();

    let (_, victim) = peer();
    let order = test_order(&victim);

    assert_ne!(authenticated_trader, victim);

    let message = test_message(
        MessageType::OrderCreated,
        &victim,
        serde_json::to_string(&order).unwrap(),
    );

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-security-test");

    let mut order_book = OrderBook::new(temp_path("impersonated-orders"));
    let mut trade_book = TradeBook::new(temp_path("impersonated-trades"));
    let mut balance_book = BalanceBook::new(temp_path("impersonated-balances"));
    let mut pending_trade_states = HashMap::new();

    let mut ctx = HandlerContext {
        swarm: &mut swarm,
        topic: &topic,
        local_peer_id: &authenticated_peer,
        order_book: &mut order_book,
        trade_book: &mut trade_book,
        balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states,
    };

    handle_gossipsub_message(
        &serde_json::to_vec(&message).unwrap(),
        &authenticated_peer,
        &mut ctx,
    )
    .unwrap();

    assert!(
        order_book.orders.is_empty(),
        "impersonated order must not enter the order book"
    );
}

#[tokio::test]
async fn handler_rejects_unauthorized_trade_state_change() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    let (_, buyer) = peer();
    let (_, seller) = peer();
    let (_, attacker) = peer();

    let trade = test_trade(&buyer, &seller);
    let trade_id = trade.id;

    let mut trade_book = TradeBook::new(temp_path("unauthorized-trade"));
    assert!(trade_book.add_trade(trade));

    trade_book.save_trades().unwrap();

    let change = TradeStateChanged {
        trade_id,
        state: TradeState::Accepted,
        timestamp: 2,
    };

    let message = test_message(
        MessageType::TradeStateChanged,
        &attacker,
        serde_json::to_string(&change).unwrap(),
    );

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-security-test");

    let mut order_book = OrderBook::new(temp_path("unauthorized-state-orders"));
    let mut balance_book = BalanceBook::new(temp_path("unauthorized-state-balances"));
    let mut pending_trade_states = HashMap::new();

    let mut ctx = HandlerContext {
        swarm: &mut swarm,
        topic: &topic,
        local_peer_id: &authenticated_peer,
        order_book: &mut order_book,
        trade_book: &mut trade_book,
        balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states,
    };

    handle_gossipsub_message(
        &serde_json::to_vec(&message).unwrap(),
        &authenticated_peer,
        &mut ctx,
    )
    .unwrap();

    assert_eq!(
        trade_book.get_trade(trade_id).unwrap().state,
        TradeState::Created,
        "unauthorized peer must not change trade state"
    );
}

#[tokio::test]
async fn handler_rejects_cancellation_by_non_owner() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    let (_, owner) = peer();
    let order = test_order(&owner);
    let order_id = order.id;

    let cancellation = OrderCancelled { order_id };

    let message = test_message(
        MessageType::OrderCancelled,
        &authenticated_peer.to_string(),
        serde_json::to_string(&cancellation).unwrap(),
    );

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-security-test");

    let mut order_book = OrderBook::new(temp_path("cancel-orders"));
    let mut trade_book = TradeBook::new(temp_path("cancel-trades"));
    let mut balance_book = BalanceBook::new(temp_path("cancel-balances"));
    let mut pending_trade_states = HashMap::new();

    order_book.add_order(order.clone());
    order_book.save_orders().unwrap();

    let mut ctx = HandlerContext {
        swarm: &mut swarm,
        topic: &topic,
        local_peer_id: &authenticated_peer,
        order_book: &mut order_book,
        trade_book: &mut trade_book,
        balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states,
    };

    handle_gossipsub_message(
        &serde_json::to_vec(&message).unwrap(),
        &authenticated_peer,
        &mut ctx,
    )
    .unwrap();

    assert!(
        order_book.orders.iter().any(|order| order.id == order_id),
        "non-owner cancellation must not remove the order"
    );
}

#[tokio::test]
async fn handler_state_response_does_not_resurrect_removed_order() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    let owner = authenticated_peer.to_string();
    let order = test_order(&owner);
    let order_id = order.id;

    let message = test_message(
        MessageType::StateResponse,
        &authenticated_peer.to_string(),
        serde_json::to_string(&StateSnapshot {
            orders: vec![order],
            trades: Vec::new(),
            removed_orders: Vec::new(),
            applied_trades: Vec::new(),
        })
        .unwrap(),
    );

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-security-test");

    let mut order_book = OrderBook::new(temp_path("snapshot-resurrection-orders"));
    let mut trade_book = TradeBook::new(temp_path("snapshot-resurrection-trades"));
    let mut balance_book = BalanceBook::new(temp_path("snapshot-resurrection-balances"));
    let mut pending_trade_states = HashMap::new();

    // Local node has already permanently removed this order.
    order_book.removed_orders.push(order_id);

    let mut ctx = HandlerContext {
        swarm: &mut swarm,
        topic: &topic,
        local_peer_id: &authenticated_peer,
        order_book: &mut order_book,
        trade_book: &mut trade_book,
        balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states,
    };

    handle_gossipsub_message(
        &serde_json::to_vec(&message).unwrap(),
        &authenticated_peer,
        &mut ctx,
    )
    .unwrap();

    assert!(
        !order_book.orders.iter().any(|o| o.id == order_id),
        "state sync must not resurrect a locally removed order"
    );

    assert!(
        order_book.removed_orders.contains(&order_id),
        "the removal tombstone must survive state sync"
    );
}

#[tokio::test]
async fn handler_state_response_does_not_replace_existing_trade() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    let (_, buyer) = peer();
    let (_, seller) = peer();

    let mut existing_trade = test_trade(&buyer, &seller);
    existing_trade.state = TradeState::Settled;
    existing_trade.updated_at = 100;

    let trade_id = existing_trade.id;

    let mut stale_trade = existing_trade.clone();
    stale_trade.state = TradeState::Created;
    stale_trade.updated_at = 1;

    let message = test_message(
        MessageType::StateResponse,
        &authenticated_peer.to_string(),
        serde_json::to_string(&StateSnapshot {
            orders: Vec::new(),
            trades: vec![stale_trade],
            removed_orders: Vec::new(),
            applied_trades: Vec::new(),
        })
        .unwrap(),
    );

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-security-test");

    let mut order_book = OrderBook::new(temp_path("snapshot-trade-rollback-orders"));
    let mut trade_book = TradeBook::new(temp_path("snapshot-trade-rollback-trades"));
    let mut balance_book = BalanceBook::new(temp_path("snapshot-trade-rollback-balances"));
    let mut pending_trade_states = HashMap::new();

    assert!(trade_book.add_trade(existing_trade));

    let mut ctx = HandlerContext {
        swarm: &mut swarm,
        topic: &topic,
        local_peer_id: &authenticated_peer,
        order_book: &mut order_book,
        trade_book: &mut trade_book,
        balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states,
    };

    handle_gossipsub_message(
        &serde_json::to_vec(&message).unwrap(),
        &authenticated_peer,
        &mut ctx,
    )
    .unwrap();

    let trade = trade_book
        .get_trade(trade_id)
        .expect("existing trade must remain present");

    assert_eq!(
        trade.state,
        TradeState::Settled,
        "stale state sync must not roll a trade backward"
    );

    assert_eq!(
        trade.updated_at, 100,
        "stale state sync must not replace the newer trade"
    );
}
