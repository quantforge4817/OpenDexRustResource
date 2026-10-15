//! Phase 1: Network Protocol Attacks
//!
//! Tests for the P2P network layer message validation, replay protection,
//! and peer identity verification.
//!
//! Run with: `cargo test --test network_protocol_attacks`

use libp2p::{PeerId, gossipsub, identity};
use p2p_node::balances::BalanceBook;
use p2p_node::message_handler::{
    HandlerContext, handle_gossipsub_message, is_authenticated_sender,
};
use p2p_node::network::build_swarm;
use p2p_node::orderbook::OrderBook;
use p2p_node::trades::TradeBook;
use p2p_node::p2p_offer_book::P2POfferBook;
use protocol::message::{
    AMOUNT_SCALE, Asset, Market, MessageType, Message, Order, OrderCancelled, PRICE_SCALE,
    Side, Trade, TradeState, TradeStateChanged,
};
use protocol::state::StateSnapshot;

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

fn peer() -> (PeerId, String) {
    let key = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(key.public());
    (peer_id, peer_id.to_string())
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "opendex-test-network-{name}-{}.json",
        Uuid::new_v4()
    ))
}

fn test_order(trader: &str) -> Order {
    Order {
        id: Uuid::new_v4(),
        trader: trader.to_string(),
        market: Market { base: Asset::BTC, quote: Asset::XMR },
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
    let trade_input = format!("{}:{}:{}:{}:{}", buy_order, sell_order, price, amount, fill_sequence);
    let hash = Sha256::digest(trade_input.as_bytes());
    let id = Uuid::from_slice(&hash[..16]).expect("failed to create trade uuid");
    Trade {
        id, buy_order, sell_order, maker: seller.to_string(), taker: buyer.to_string(),
        buyer: buyer.to_string(), seller: seller.to_string(),
        market: Market { base: Asset::BTC, quote: Asset::XMR },
        price, amount, fill_sequence, state: TradeState::Created,
        created_at: 1, updated_at: 1,
    }
}

fn test_message(message_type: MessageType, sender: &str, payload: String) -> Message {
    Message {
        message_type, sender: sender.to_string(), payload,
        timestamp: chrono::Utc::now().timestamp() as u64,
        schema_version: protocol::message::SCHEMA_VERSION,
    }
}

// ==============================
// 1.1 Malformed Message Injection
// ==============================

#[tokio::test]
async fn rejects_invalid_json_payload() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let message = test_message(MessageType::OrderCreated, &authenticated_peer.to_string(), "{invalid json}".to_string());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-malformed-json");
    let mut order_book = OrderBook::new(temp_path("malformed-json-orders"));
    let mut trade_book = TradeBook::new(temp_path("malformed-json-trades"));
    let mut balance_book = BalanceBook::new(temp_path("malformed-json-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
    assert!(order_book.active_orders().is_empty(), "invalid JSON payload must not create an order");
}

#[tokio::test]
async fn rejects_invalid_outer_json() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-invalid-outer");
    let mut order_book = OrderBook::new(temp_path("invalid-outer-orders"));
    let mut trade_book = TradeBook::new(temp_path("invalid-outer-trades"));
    let mut balance_book = BalanceBook::new(temp_path("invalid-outer-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(b"{not valid json at all}", &authenticated_peer, &mut ctx);
    assert!(result.is_ok(), "handler should not error on invalid outer JSON");
}

#[tokio::test]
async fn rejects_empty_payload() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let message = test_message(MessageType::OrderCreated, &authenticated_peer.to_string(), "".to_string());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-empty-payload");
    let mut order_book = OrderBook::new(temp_path("empty-payload-orders"));
    let mut trade_book = TradeBook::new(temp_path("empty-payload-trades"));
    let mut balance_book = BalanceBook::new(temp_path("empty-payload-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
    assert!(order_book.active_orders().is_empty(), "empty payload must not create an order");
}

#[tokio::test]
async fn rejects_empty_sender() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let order = test_order(&authenticated_peer.to_string());
    let message = test_message(MessageType::OrderCreated, "", serde_json::to_string(&order).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-empty-sender");
    let mut order_book = OrderBook::new(temp_path("empty-sender-orders"));
    let mut trade_book = TradeBook::new(temp_path("empty-sender-trades"));
    let mut balance_book = BalanceBook::new(temp_path("empty-sender-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
}

#[tokio::test]
async fn rejects_schema_version_max_int() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let order = test_order(&authenticated_peer.to_string());
    let message = Message {
        message_type: MessageType::OrderCreated, sender: authenticated_peer.to_string(),
        payload: serde_json::to_string(&order).unwrap(),
        timestamp: chrono::Utc::now().timestamp() as u64, schema_version: u32::MAX,
    };
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-schema-max");
    let mut order_book = OrderBook::new(temp_path("schema-max-orders"));
    let mut trade_book = TradeBook::new(temp_path("schema-max-trades"));
    let mut balance_book = BalanceBook::new(temp_path("schema-max-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
    assert!(order_book.active_orders().is_empty(), "schema_version MAX_INT must be rejected");
}

#[tokio::test]
async fn rejects_zero_amount_order() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let order = Order {
        id: Uuid::new_v4(), trader: authenticated_peer.to_string(),
        market: Market { base: Asset::BTC, quote: Asset::XMR }, side: Side::Buy,
        price: 100 * PRICE_SCALE, amount: 0,
    };
    let message = test_message(MessageType::OrderCreated, &authenticated_peer.to_string(), serde_json::to_string(&order).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-zero-amount");
    let mut order_book = OrderBook::new(temp_path("zero-amount-orders"));
    let mut trade_book = TradeBook::new(temp_path("zero-amount-trades"));
    let mut balance_book = BalanceBook::new(temp_path("zero-amount-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    // The handler returns an error when the exchange rejects the order
    assert!(result.is_err() || order_book.active_orders().is_empty(),
        "zero amount order must be rejected");
}

#[tokio::test]
async fn rejects_zero_price_order() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let order = Order {
        id: Uuid::new_v4(), trader: authenticated_peer.to_string(),
        market: Market { base: Asset::BTC, quote: Asset::XMR }, side: Side::Buy,
        price: 0, amount: AMOUNT_SCALE,
    };
    let message = test_message(MessageType::OrderCreated, &authenticated_peer.to_string(), serde_json::to_string(&order).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-zero-price");
    let mut order_book = OrderBook::new(temp_path("zero-price-orders"));
    let mut trade_book = TradeBook::new(temp_path("zero-price-trades"));
    let mut balance_book = BalanceBook::new(temp_path("zero-price-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    // The handler returns an error when the exchange rejects the order
    assert!(result.is_err() || order_book.active_orders().is_empty(),
        "zero price order must be rejected");
}

#[tokio::test]
async fn handles_max_u64_values_order() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let order = Order {
        id: Uuid::new_v4(), trader: authenticated_peer.to_string(),
        market: Market { base: Asset::BTC, quote: Asset::XMR }, side: Side::Buy,
        price: u64::MAX, amount: u64::MAX,
    };
    let message = test_message(MessageType::OrderCreated, &authenticated_peer.to_string(), serde_json::to_string(&order).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-max-values");
    let mut order_book = OrderBook::new(temp_path("max-values-orders"));
    let mut trade_book = TradeBook::new(temp_path("max-values-trades"));
    let mut balance_book = BalanceBook::new(temp_path("max-values-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let _ = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
}

#[tokio::test]
async fn handles_large_payload() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let large_payload = "x".repeat(1_000_000);
    let message = test_message(MessageType::OrderCreated, &authenticated_peer.to_string(), large_payload);
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-large-payload");
    let mut order_book = OrderBook::new(temp_path("large-payload-orders"));
    let mut trade_book = TradeBook::new(temp_path("large-payload-trades"));
    let mut balance_book = BalanceBook::new(temp_path("large-payload-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
    assert!(order_book.active_orders().is_empty(), "large invalid payload must not create an order");
}

// ==============================
// 1.2 Message Replay Attacks
// ==============================

#[tokio::test]
async fn trade_created_replay_is_idempotent() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let (_, buyer) = peer();
    let (_, seller) = peer();
    let trade = test_trade(&buyer, &seller);
    let trade_id = trade.id;
    let message = test_message(MessageType::TradeCreated, &authenticated_peer.to_string(), serde_json::to_string(&trade).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-replay-trade");
    let mut order_book = OrderBook::new(temp_path("replay-trade-orders"));
    let mut trade_book = TradeBook::new(temp_path("replay-trade-trades"));
    let mut balance_book = BalanceBook::new(temp_path("replay-trade-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    // Add the trade directly first, then test replay
    trade_book.add_trade(trade);
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    // Replay the same trade message - should not create a duplicate
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
    drop(ctx);
    let count = trade_book.all_trades().iter().filter(|t| t.id == trade_id).count();
    assert_eq!(count, 1, "Replayed trade should not create a duplicate");
}

#[tokio::test]
async fn order_cancelled_replay_is_safe() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let authenticated_trader = authenticated_peer.to_string();
    let order = test_order(&authenticated_trader);
    let order_id = order.id;
    let cancellation = OrderCancelled { order_id, trader: authenticated_trader.clone() };
    let message = test_message(MessageType::OrderCancelled, &authenticated_trader, serde_json::to_string(&cancellation).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-replay-cancel");
    let mut order_book = OrderBook::new(temp_path("replay-cancel-orders"));
    let mut trade_book = TradeBook::new(temp_path("replay-cancel-trades"));
    let mut balance_book = BalanceBook::new(temp_path("replay-cancel-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    order_book.upsert_order(order);
    order_book.save_orders().unwrap();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result1 = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result1.is_ok());
    drop(ctx);
    assert!(!order_book.is_order_active(&order_id), "Order should be cancelled");
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result2 = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result2.is_ok());
    drop(ctx);
    assert!(!order_book.is_order_active(&order_id), "Replayed cancellation should not cause issues");
}

#[tokio::test]
async fn trade_state_changed_replay_is_safe() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let authenticated_trader = authenticated_peer.to_string();
    let mut trade = test_trade(&authenticated_trader, "seller");
    trade.state = TradeState::Created;
    let change = TradeStateChanged { trade_id: trade.id, state: TradeState::Accepted, timestamp: 2 };
    let message = test_message(MessageType::TradeStateChanged, &authenticated_trader, serde_json::to_string(&change).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-replay-state");
    let mut order_book = OrderBook::new(temp_path("replay-state-orders"));
    let mut trade_book = TradeBook::new(temp_path("replay-state-trades"));
    let mut balance_book = BalanceBook::new(temp_path("replay-state-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    trade_book.add_trade(trade);
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result1 = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result1.is_ok());
    let result2 = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result2.is_ok());
}

#[tokio::test]
async fn state_response_replay_prevents_regression() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let (_, buyer) = peer();
    let (_, seller) = peer();
    let mut settled_trade = test_trade(&buyer, &seller);
    settled_trade.state = TradeState::Settled;
    settled_trade.updated_at = 100;
    let trade_id = settled_trade.id;
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-replay-snapshot");
    let mut order_book = OrderBook::new(temp_path("replay-snapshot-orders"));
    let mut trade_book = TradeBook::new(temp_path("replay-snapshot-trades"));
    let mut balance_book = BalanceBook::new(temp_path("replay-snapshot-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    trade_book.add_trade(settled_trade);
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let mut stale_trade = test_trade(&buyer, &seller);
    stale_trade.id = trade_id;
    stale_trade.state = TradeState::Created;
    stale_trade.updated_at = 1;
    let stale_snapshot = StateSnapshot {
        version: 1, orders: Vec::new(), trades: vec![stale_trade],
        removed_orders: HashSet::new(), applied_trades: HashSet::new(),
    };
    let message = test_message(MessageType::StateResponse, &authenticated_peer.to_string(), serde_json::to_string(&stale_snapshot).unwrap());
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    // The handler may reject invalid snapshots with an error, which is also acceptable
    // The key test is that the trade state is not regressed
    drop(ctx);
    let trade = trade_book.get_trade(trade_id).unwrap();
    assert_eq!(trade.state, TradeState::Settled, "Replayed stale snapshot must not regress trade state");
}

// ==============================
// 1.3 Race Condition Attacks
// ==============================

#[tokio::test]
async fn trade_state_change_for_unknown_trade() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let change = TradeStateChanged { trade_id: Uuid::new_v4(), state: TradeState::Accepted, timestamp: 1 };
    let message = test_message(MessageType::TradeStateChanged, &authenticated_peer.to_string(), serde_json::to_string(&change).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-race-unknown");
    let mut order_book = OrderBook::new(temp_path("race-unknown-orders"));
    let mut trade_book = TradeBook::new(temp_path("race-unknown-trades"));
    let mut balance_book = BalanceBook::new(temp_path("race-unknown-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
}

#[tokio::test]
async fn cancel_during_trade_creation() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let authenticated_trader = authenticated_peer.to_string();
    let order = test_order(&authenticated_trader);
    let order_id = order.id;
    let cancellation = OrderCancelled { order_id, trader: authenticated_trader.clone() };
    let cancel_message = test_message(MessageType::OrderCancelled, &authenticated_trader, serde_json::to_string(&cancellation).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-race-cancel");
    let mut order_book = OrderBook::new(temp_path("race-cancel-orders"));
    let mut trade_book = TradeBook::new(temp_path("race-cancel-trades"));
    let mut balance_book = BalanceBook::new(temp_path("race-cancel-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    order_book.upsert_order(order);
    order_book.save_orders().unwrap();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&cancel_message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
    drop(ctx);
    assert!(!order_book.is_order_active(&order_id), "Order should be cancelled");
}

#[tokio::test]
async fn rapid_state_changes() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let authenticated_trader = authenticated_peer.to_string();
    let trade = test_trade(&authenticated_trader, "seller");
    let trade_id = trade.id;
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-rapid-state");
    let mut order_book = OrderBook::new(temp_path("rapid-state-orders"));
    let mut trade_book = TradeBook::new(temp_path("rapid-state-trades"));
    let mut balance_book = BalanceBook::new(temp_path("rapid-state-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    trade_book.add_trade(trade);
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let states = vec![TradeState::Accepted, TradeState::EscrowFunding, TradeState::EscrowFunded];
    for (i, state) in states.into_iter().enumerate() {
        let change = TradeStateChanged { trade_id, state: state.clone(), timestamp: (i + 2) as u64 };
        let message = test_message(MessageType::TradeStateChanged, &authenticated_trader, serde_json::to_string(&change).unwrap());
        let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
        assert!(result.is_ok(), "State change {} should succeed", i);
    }
    let final_trade = trade_book.get_trade(trade_id).unwrap();
    assert_eq!(final_trade.state, TradeState::EscrowFunded, "Trade should reach final state after rapid changes");
}

// ==============================
// 1.4 Peer Identity Spoofing
// ==============================

#[tokio::test]
async fn sender_field_ignored_for_authorization() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let order = test_order(&authenticated_peer.to_string());
    let message = test_message(MessageType::OrderCreated, "fake_attacker_sender", serde_json::to_string(&order).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-sender-spoof");
    let mut order_book = OrderBook::new(temp_path("sender-spoof-orders"));
    let mut trade_book = TradeBook::new(temp_path("sender-spoof-trades"));
    let mut balance_book = BalanceBook::new(temp_path("sender-spoof-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
}

#[test]
fn peer_announcement_identity_check() {
    let (peer_a, _) = peer();
    let (peer_b, _) = peer();
    assert_ne!(peer_a.to_string(), peer_b.to_string(), "Two peers should have different identities");
    assert!(is_authenticated_sender(&peer_a.to_string(), &peer_a));
    assert!(!is_authenticated_sender(&peer_b.to_string(), &peer_a));
}

#[test]
fn peer_id_derived_from_public_key() {
    let key1 = identity::Keypair::generate_ed25519();
    let key2 = identity::Keypair::generate_ed25519();
    let peer_id1 = PeerId::from(key1.public());
    let peer_id2 = PeerId::from(key2.public());
    assert_ne!(peer_id1, peer_id2, "Different keys produce different peer IDs");
    let peer_id1_again = PeerId::from(key1.public());
    assert_eq!(peer_id1, peer_id1_again, "Same key produces same peer ID");
}

// ==============================
// Message Validation Edge Cases
// ==============================

#[tokio::test]
async fn rejects_timestamp_zero() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let order = test_order(&authenticated_peer.to_string());
    let message = Message {
        message_type: MessageType::OrderCreated, sender: authenticated_peer.to_string(),
        payload: serde_json::to_string(&order).unwrap(), timestamp: 0,
        schema_version: protocol::message::SCHEMA_VERSION,
    };
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-ts-zero");
    let mut order_book = OrderBook::new(temp_path("ts-zero-orders"));
    let mut trade_book = TradeBook::new(temp_path("ts-zero-trades"));
    let mut balance_book = BalanceBook::new(temp_path("ts-zero-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
    assert!(order_book.active_orders().is_empty(), "timestamp 0 should be rejected as stale");
}

#[tokio::test]
async fn rejects_timestamp_max() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let order = test_order(&authenticated_peer.to_string());
    let message = Message {
        message_type: MessageType::OrderCreated, sender: authenticated_peer.to_string(),
        payload: serde_json::to_string(&order).unwrap(), timestamp: u64::MAX,
        schema_version: protocol::message::SCHEMA_VERSION,
    };
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-ts-max");
    let mut order_book = OrderBook::new(temp_path("ts-max-orders"));
    let mut trade_book = TradeBook::new(temp_path("ts-max-trades"));
    let mut balance_book = BalanceBook::new(temp_path("ts-max-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
    assert!(order_book.active_orders().is_empty(), "timestamp u64::MAX should be rejected as future");
}

#[tokio::test]
async fn handles_empty_message_data() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-empty-data");
    let mut order_book = OrderBook::new(temp_path("empty-data-orders"));
    let mut trade_book = TradeBook::new(temp_path("empty-data-trades"));
    let mut balance_book = BalanceBook::new(temp_path("empty-data-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(b"", &authenticated_peer, &mut ctx);
    assert!(result.is_ok(), "Empty data should not cause an error");
}

#[tokio::test]
async fn handles_null_bytes_in_payload() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let message = test_message(MessageType::OrderCreated, &authenticated_peer.to_string(), "test\x00payload".to_string());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-null-bytes");
    let mut order_book = OrderBook::new(temp_path("null-bytes-orders"));
    let mut trade_book = TradeBook::new(temp_path("null-bytes-trades"));
    let mut balance_book = BalanceBook::new(temp_path("null-bytes-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
    assert!(order_book.active_orders().is_empty(), "Null bytes in payload should not create an order");
}

#[tokio::test]
async fn handles_unicode_in_sender() {
    let key = identity::Keypair::generate_ed25519();
    let authenticated_peer = PeerId::from(key.public());
    let order = test_order(&authenticated_peer.to_string());
    let message = test_message(MessageType::OrderCreated, "тестирование🔐测试", serde_json::to_string(&order).unwrap());
    let mut swarm = build_swarm(key).unwrap();
    let topic = gossipsub::IdentTopic::new("opendex-unicode-sender");
    let mut order_book = OrderBook::new(temp_path("unicode-sender-orders"));
    let mut trade_book = TradeBook::new(temp_path("unicode-sender-trades"));
    let mut balance_book = BalanceBook::new(temp_path("unicode-sender-balances"));
    let mut pending_trade_states = HashMap::new();
    let mut p2p_offers = P2POfferBook::new();
    let mut ctx = HandlerContext {
        swarm: &mut swarm, topic: &topic, local_peer_id: &authenticated_peer,
        order_book: &mut order_book, trade_book: &mut trade_book, balance_book: &mut balance_book,
        pending_trade_states: &mut pending_trade_states, p2p_offer_book: &mut p2p_offers,
    };
    let result = handle_gossipsub_message(&serde_json::to_vec(&message).unwrap(), &authenticated_peer, &mut ctx);
    assert!(result.is_ok());
}
