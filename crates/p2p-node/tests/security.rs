use libp2p::{PeerId, gossipsub, identity};
use p2p_node::balances::BalanceBook;
use p2p_node::message_handler::{
    HandlerContext, handle_gossipsub_message, is_authenticated_sender, is_order_owner,
    is_trade_participant,
};

use p2p_node::network::build_swarm;
use p2p_node::orderbook::OrderBook;
#[cfg(feature = "fiat")]
use p2p_node::p2p_offer_book::P2POfferBook;
use p2p_node::trades::TradeBook;
use protocol::message::{
    AMOUNT_SCALE, Asset, Market, Message, MessageType, Order, OrderCancelled, PRICE_SCALE, Side,
    Trade, TradeState, TradeStateChanged,
};
use protocol::state::StateSnapshot;

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
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
    let buy_order = Uuid::new_v4();
    let sell_order = Uuid::new_v4();
    let price = 100 * PRICE_SCALE;
    let amount = AMOUNT_SCALE;

    let fill_sequence = 10;
    let trade_input = format!(
        "{}:{}:{}:{}:{}",
        buy_order, sell_order, price, amount, fill_sequence,
    );
    let hash = Sha256::digest(trade_input.as_bytes());
    let id = Uuid::from_slice(&hash[..16]).expect("failed to create trade uuid");

    Trade {
        id,
        buy_order,
        sell_order,
        maker: seller.to_string(),
        taker: buyer.to_string(),
        buyer: buyer.to_string(),
        seller: seller.to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        price,
        amount,
        fill_sequence,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    }
}

fn test_message(message_type: MessageType, sender: &str, payload: String) -> Message {
    Message {
        message_type,
        sender: sender.to_string(),
        payload,
        timestamp: chrono::Utc::now().timestamp() as u64,
        schema_version: protocol::message::SCHEMA_VERSION,
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
        order_book.active_orders().is_empty(),
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
        order_book.active_orders().is_empty(),
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

    let cancellation = OrderCancelled {
        order_id,
        trader: owner.to_string(),
    };

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

    order_book.upsert_order(order.clone());
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
        order_book.is_order_active(&order_id),
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
            version: 1,
            orders: vec![order],
            trades: Vec::new(),
            removed_orders: HashSet::new(),
            applied_trades: HashSet::new(),
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
    order_book.removed_orders.insert(order_id);

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
        !order_book.is_order_active(&order_id),
        "state sync must not resurrect a locally removed order"
    );

    assert!(
        order_book.is_order_removed(&order_id),
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
            version: 1,
            orders: Vec::new(),
            trades: vec![stale_trade],
            removed_orders: HashSet::new(),
            applied_trades: HashSet::new(),
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
#[tokio::test]
async fn handler_rejects_invalid_snapshot_without_mutating_state() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    let (_, seller) = peer();

    let existing_order = test_order(&seller);
    let existing_order_id = existing_order.id;

    let message = test_message(
        MessageType::StateResponse,
        &authenticated_peer.to_string(),
        serde_json::to_string(&StateSnapshot {
            version: 1,
            orders: vec![existing_order.clone()],
            trades: vec![Trade {
                id: Uuid::new_v4(),
                buy_order: Uuid::new_v4(),
                sell_order: existing_order_id,
                maker: seller.to_string(),
                taker: seller.to_string(),
                buyer: seller.to_string(),
                seller: seller.to_string(),
                market: existing_order.market.clone(),
                price: existing_order.price,
                amount: 1,
                fill_sequence: 10,
                state: TradeState::Created,
                created_at: 1,
                updated_at: 1,
            }],
            removed_orders: HashSet::new(),
            applied_trades: HashSet::new(),
        })
        .unwrap(),
    );

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-security-test");

    let mut order_book = OrderBook::new(temp_path("snapshot-invalid-orders"));
    let mut trade_book = TradeBook::new(temp_path("snapshot-invalid-trades"));
    let mut balance_book = BalanceBook::new(temp_path("snapshot-invalid-balances"));
    let mut pending_trade_states = HashMap::new();

    order_book.upsert_order(existing_order.clone());

    let original_order_count = order_book.active_orders().len();
    let original_trade_count = trade_book.all_trades().len();
    let original_removed_count = order_book.removed_order_ids().len();
    let original_applied_count = order_book.applied_trade_ids().len();

    let mut ctx = HandlerContext {
        swarm: &mut swarm,
        topic: &topic,
        local_peer_id: &authenticated_peer,
        order_book: &mut order_book,
        trade_book: &mut trade_book,
        balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states,
    };

    let result = handle_gossipsub_message(
        &serde_json::to_vec(&message).unwrap(),
        &authenticated_peer,
        &mut ctx,
    );

    assert!(result.is_err(), "invalid snapshot must be rejected");

    assert_eq!(
        order_book.active_orders().len(),
        original_order_count,
        "invalid snapshot must not change orders"
    );

    assert_eq!(
        trade_book.all_trades().len(),
        original_trade_count,
        "invalid snapshot must not add trades"
    );

    assert_eq!(
        order_book.removed_order_ids().len(),
        original_removed_count,
        "invalid snapshot must not add removal tombstones"
    );

    assert_eq!(
        order_book.applied_trade_ids().len(),
        original_applied_count,
        "invalid snapshot must not mark trades as applied"
    );
}
#[tokio::test]
async fn pending_trade_state_cache_is_bounded() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    let topic = gossipsub::IdentTopic::new("opendex-security-test");
    let mut swarm = build_swarm(key).unwrap();

    let mut order_book = OrderBook::new(temp_path("pending-limit-orders"));
    let mut trade_book = TradeBook::new(temp_path("pending-limit-trades"));
    let mut balance_book = BalanceBook::new(temp_path("pending-limit-balances"));
    let mut pending_trade_states = HashMap::new();

    for _ in 0..10_000 {
        let change = TradeStateChanged {
            trade_id: Uuid::new_v4(),
            state: TradeState::Accepted,
            timestamp: 1,
        };

        let message = test_message(
            MessageType::TradeStateChanged,
            &authenticated_peer.to_string(),
            serde_json::to_string(&change).unwrap(),
        );

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
    }

    assert_eq!(
        pending_trade_states.len(),
        10_000,
        "pending trade-state cache must reach, but not exceed, its configured limit"
    );

    let attack_trade_id = Uuid::new_v4();

    let attack_change = TradeStateChanged {
        trade_id: attack_trade_id,
        state: TradeState::Accepted,
        timestamp: 2,
    };

    let attack_message = test_message(
        MessageType::TradeStateChanged,
        &authenticated_peer.to_string(),
        serde_json::to_string(&attack_change).unwrap(),
    );

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
        &serde_json::to_vec(&attack_message).unwrap(),
        &authenticated_peer,
        &mut ctx,
    )
    .unwrap();

    assert_eq!(
        pending_trade_states.len(),
        10_000,
        "attacker must not be able to grow the pending trade-state cache beyond its limit"
    );

    assert!(
        !pending_trade_states.contains_key(&attack_trade_id),
        "new pending trade state must be rejected when cache is full"
    );
}
#[tokio::test]
async fn handler_rejects_forged_cancellation_trader_without_mutating_state() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    let (_, owner) = peer();
    let order = test_order(&owner);
    let order_id = order.id;

    // The attacker is authenticated as themselves, but the cancellation
    // payload claims that they are the order owner.
    let cancellation = OrderCancelled {
        order_id,
        trader: authenticated_peer.to_string(),
    };

    let message = test_message(
        MessageType::OrderCancelled,
        &authenticated_peer.to_string(),
        serde_json::to_string(&cancellation).unwrap(),
    );

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-security-test");

    let mut order_book = OrderBook::new(temp_path("forged-cancel-orders"));
    let mut trade_book = TradeBook::new(temp_path("forged-cancel-trades"));
    let mut balance_book = BalanceBook::new(temp_path("forged-cancel-balances"));
    let mut pending_trade_states = HashMap::new();
    order_book.upsert_order(order.clone());
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
        order_book.is_order_active(&order_id),
        "forged cancellation must not remove another trader's order"
    );
}

#[test]
fn message_schema_version_constant() {
    assert_eq!(protocol::message::SCHEMA_VERSION, 1);
    assert_eq!(protocol::message::MAX_SCHEMA_VERSION, 1);
}

#[tokio::test]
async fn handler_rejects_stale_message() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    // Create a message with a timestamp 60 seconds in the past
    let stale_timestamp = chrono::Utc::now().timestamp() as u64 - 60;
    let order = test_order(&authenticated_peer.to_string());
    let message = Message {
        message_type: MessageType::OrderCreated,
        sender: authenticated_peer.to_string(),
        payload: serde_json::to_string(&order).unwrap(),
        timestamp: stale_timestamp,
        schema_version: protocol::message::SCHEMA_VERSION,
    };

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-stale-test");

    let mut order_book = OrderBook::new(temp_path("stale-orders"));
    let mut trade_book = TradeBook::new(temp_path("stale-trades"));
    let mut balance_book = BalanceBook::new(temp_path("stale-balances"));
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

    // Stale message should be silently rejected — order book unchanged
    assert!(
        order_book.active_orders().is_empty(),
        "stale message must not add order"
    );
}

#[tokio::test]
async fn handler_rejects_future_message() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    // Create a message with a timestamp 60 seconds in the future
    let future_timestamp = chrono::Utc::now().timestamp() as u64 + 60;
    let order = test_order(&authenticated_peer.to_string());
    let message = Message {
        message_type: MessageType::OrderCreated,
        sender: authenticated_peer.to_string(),
        payload: serde_json::to_string(&order).unwrap(),
        timestamp: future_timestamp,
        schema_version: protocol::message::SCHEMA_VERSION,
    };

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-future-test");

    let mut order_book = OrderBook::new(temp_path("future-orders"));
    let mut trade_book = TradeBook::new(temp_path("future-trades"));
    let mut balance_book = BalanceBook::new(temp_path("future-balances"));
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
        order_book.active_orders().is_empty(),
        "future message must not add order"
    );
}

#[tokio::test]
async fn handler_rejects_unsupported_schema_version() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    let order = test_order(&authenticated_peer.to_string());
    let message = Message {
        message_type: MessageType::OrderCreated,
        sender: authenticated_peer.to_string(),
        payload: serde_json::to_string(&order).unwrap(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        schema_version: 99, // Unsupported version
    };

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-schema-test");

    let mut order_book = OrderBook::new(temp_path("schema-orders"));
    let mut trade_book = TradeBook::new(temp_path("schema-trades"));
    let mut balance_book = BalanceBook::new(temp_path("schema-balances"));
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
        order_book.active_orders().is_empty(),
        "unsupported schema version must not add order"
    );
}

#[tokio::test]
async fn handler_accepts_fresh_message() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());

    let order = test_order(&authenticated_peer.to_string());
    let message = Message {
        message_type: MessageType::OrderCreated,
        sender: authenticated_peer.to_string(),
        payload: serde_json::to_string(&order).unwrap(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        schema_version: protocol::message::SCHEMA_VERSION,
    };

    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-fresh-test");

    let mut order_book = OrderBook::new(temp_path("fresh-orders"));
    let mut trade_book = TradeBook::new(temp_path("fresh-trades"));
    let mut balance_book = BalanceBook::new(temp_path("fresh-balances"));
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

    // Fresh message with valid schema should be accepted
    let orders = order_book.active_orders();
    assert!(
        orders.iter().any(|o| o.id == order.id),
        "fresh message with valid schema should add order"
    );
}
