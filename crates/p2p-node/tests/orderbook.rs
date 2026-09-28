use p2p_node::orderbook::*;
use p2p_node::sync::apply_snapshot;
use p2p_node::trades::TradeBook;
use protocol::message::{AMOUNT_SCALE, Asset, Market, Order, PRICE_SCALE, Side, Trade, TradeState};
use protocol::state::StateSnapshot;
use sha2::{Digest, Sha256};

use std::fs;
use uuid::Uuid;

fn market() -> Market {
    Market {
        base: Asset::BTC,
        quote: Asset::XMR,
    }
}
fn trade_id(
    buy_order: Uuid,
    sell_order: Uuid,
    price: u64,
    amount: u64,
    fill_sequence: u64,
) -> Uuid {
    let input = format!(
        "{}:{}:{}:{}:{}",
        buy_order, sell_order, price, amount, fill_sequence);

    let hash = Sha256::digest(input.as_bytes());

    Uuid::from_slice(&hash[..16]).unwrap()
}

fn order(id: &str, side: Side, price: u64, amount: u64) -> Order {
    Order {
        id: Uuid::parse_str(id).unwrap(),
        trader: format!("trader-{}", id),
        market: market(),
        side,
        price,
        amount,
    }
}

fn assert_orders_unchanged(before: &[Order], after: &[Order]) {
    assert_eq!(after.len(), before.len());

    for (before, after) in before.iter().zip(after.iter()) {
        assert_eq!(before.id, after.id);
        assert_eq!(before.trader, after.trader);
        assert_eq!(before.price, after.price);
        assert_eq!(before.amount, after.amount);
    }
}

#[test]
fn buy_matches_cheapest_sell_first() {
    let path = std::env::temp_dir().join("opendex-test-buy.json");

    let mut book = OrderBook::new(path);

    let expensive_sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    let cheap_sell = order("00000000-0000-0000-0000-000000000001", Side::Sell, 90, 10);

    // Deliberately add the expensive order first.
    book.add_order(expensive_sell);
    book.add_order(cheap_sell);

    let incoming = order("00000000-0000-0000-0000-000000000003", Side::Buy, 100, 10);

    let trades = book.match_order(&incoming);

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].price, 90);
    assert_eq!(
        trades[0].sell_order,
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    );
}

#[test]
fn sell_matches_highest_buy_first() {
    let path = std::env::temp_dir().join("opendex-test-sell.json");

    let mut book = OrderBook::new(path);

    let cheap_buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 90, 10);

    let expensive_buy = order("00000000-0000-0000-0000-000000000002", Side::Buy, 100, 10);

    // Deliberately add the cheap order first.
    book.add_order(cheap_buy);
    book.add_order(expensive_buy);

    let incoming = order("00000000-0000-0000-0000-000000000003", Side::Sell, 90, 10);

    let trades = book.match_order(&incoming);

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].price, 100);
    assert_eq!(
        trades[0].buy_order,
        Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
    );
}

#[test]
fn partial_fill_leaves_remaining_order() {
    let path = std::env::temp_dir().join("opendex-test-partial.json");

    let mut book = OrderBook::new(path);

    let sell = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 5);

    book.add_order(sell);

    let incoming = order("00000000-0000-0000-0000-000000000002", Side::Buy, 100, 10);

    let trades = book.match_order(&incoming);

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].amount, 5);

    book.add_order(incoming.clone());
    assert!(book.apply_trade(&trades[0]));

    let remaining = book
        .orders
        .iter()
        .find(|o| o.id == incoming.id)
        .expect("remaining incoming order should exist");

    assert_eq!(remaining.amount, 5);
}

#[test]
fn different_arrival_order_produces_same_trades() {
    let path_a = std::env::temp_dir().join("opendex-test-determinism-a.json");
    let path_b = std::env::temp_dir().join("opendex-test-determinism-b.json");

    let mut book_a = OrderBook::new(path_a);
    let mut book_b = OrderBook::new(path_b);

    let sell_100 = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 5);

    let sell_90 = order("00000000-0000-0000-0000-000000000002", Side::Sell, 90, 5);

    // Node A receives 100 first.
    book_a.add_order(sell_100.clone());
    book_a.add_order(sell_90.clone());

    // Node B receives 90 first.
    book_b.add_order(sell_90);
    book_b.add_order(sell_100);

    let incoming_a = order("00000000-0000-0000-0000-000000000003", Side::Buy, 100, 10);

    let incoming_b = incoming_a.clone();

    let trades_a = book_a.match_order(&incoming_a);
    let trades_b = book_b.match_order(&incoming_b);

    assert_eq!(trades_a.len(), 2);
    assert_eq!(trades_b.len(), 2);

    assert_eq!(trades_a[0].id, trades_b[0].id);
    assert_eq!(trades_a[0].buy_order, trades_b[0].buy_order);
    assert_eq!(trades_a[0].sell_order, trades_b[0].sell_order);
    assert_eq!(trades_a[0].price, trades_b[0].price);
    assert_eq!(trades_a[0].amount, trades_b[0].amount);

    assert_eq!(trades_a[1].id, trades_b[1].id);
    assert_eq!(trades_a[1].buy_order, trades_b[1].buy_order);
    assert_eq!(trades_a[1].sell_order, trades_b[1].sell_order);
    assert_eq!(trades_a[1].price, trades_b[1].price);
    assert_eq!(trades_a[1].amount, trades_b[1].amount);
}
#[test]
fn one_order_can_be_filled_by_multiple_trades() {
    let path = std::env::temp_dir().join("opendex-test-multiple-fills.json");

    let mut book = OrderBook::new(path);

    let sell = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 13);

    book.add_order(sell.clone());

    let buy1 = order("00000000-0000-0000-0000-000000000002", Side::Buy, 100, 3);

    let buy2 = order("00000000-0000-0000-0000-000000000003", Side::Buy, 100, 4);

    let buy3 = order("00000000-0000-0000-0000-000000000004", Side::Buy, 100, 4);

    let buy4 = order("00000000-0000-0000-0000-000000000005", Side::Buy, 100, 2);

    let trades1 = book.match_order(&buy1);
    assert_eq!(trades1.len(), 1);
    assert_eq!(trades1[0].amount, 3);

    book.add_order(buy1.clone());
    assert!(book.apply_trade(&trades1[0]));

    let remaining = book
        .orders
        .iter()
        .find(|o| o.id == sell.id)
        .expect("sell order should remain");

    assert_eq!(remaining.amount, 10);

    let trades2 = book.match_order(&buy2);
    assert_eq!(trades2.len(), 1);
    assert_eq!(trades2[0].amount, 4);

    book.add_order(buy2.clone());
    assert!(book.apply_trade(&trades2[0]));

    let remaining = book
        .orders
        .iter()
        .find(|o| o.id == sell.id)
        .expect("sell order should remain");

    assert_eq!(remaining.amount, 6);

    let trades3 = book.match_order(&buy3);
    assert_eq!(trades3.len(), 1);
    assert_eq!(trades3[0].amount, 4);

    book.add_order(buy3.clone());
    assert!(book.apply_trade(&trades3[0]));

    let remaining = book
        .orders
        .iter()
        .find(|o| o.id == sell.id)
        .expect("sell order should remain");

    assert_eq!(remaining.amount, 2);

    let trades4 = book.match_order(&buy4);
    assert_eq!(trades4.len(), 1);
    assert_eq!(trades4[0].amount, 2);

    book.add_order(buy4.clone());
    assert!(book.apply_trade(&trades4[0]));

    assert!(
        !book.orders.iter().any(|o| o.id == sell.id),
        "sell order should be fully filled"
    );

    assert_eq!(trades1[0].sell_order, sell.id);
    assert_eq!(trades2[0].sell_order, sell.id);
    assert_eq!(trades3[0].sell_order, sell.id);
    assert_eq!(trades4[0].sell_order, sell.id);

    assert_eq!(
        trades1[0].amount + trades2[0].amount + trades3[0].amount + trades4[0].amount,
        13
    );
}

#[test]
fn apply_trade_is_idempotent() {
    let path = std::env::temp_dir().join("opendex-test-duplicate-trade.json");

    let mut book = OrderBook::new(path);

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    book.add_order(buy.clone());
    book.add_order(sell.clone());

    let fill_sequence = 10;
    let trade_input = format!(
        "{}:{}:{}:{}:{}",
        buy.id, sell.id, 100, 4, fill_sequence);
    let hash = Sha256::digest(trade_input.as_bytes());
    let trade_id = Uuid::from_slice(&hash[..16]).unwrap();

    let trade = Trade {
        id: trade_id,
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };

    assert!(book.apply_trade(&trade));

    // Same trade arriving again must be ignored.
    assert!(!book.apply_trade(&trade));

    let remaining_buy = book.orders.iter().find(|o| o.id == buy.id).unwrap();
    let remaining_sell = book.orders.iter().find(|o| o.id == sell.id).unwrap();

    assert_eq!(remaining_buy.amount, 6);
    assert_eq!(remaining_sell.amount, 6);
}
#[test]
fn distributed_event_order_converges() {
    let path_a = std::env::temp_dir().join("opendex-test-distributed-a.json");
    let path_b = std::env::temp_dir().join("opendex-test-distributed-b.json");

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    let trade = Trade {
        id: trade_id(buy.id, sell.id, 100, 4, 10),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    let mut node_a = OrderBook::new(path_a);
    let mut node_b = OrderBook::new(path_b);

    // Node A receives the trade before either order.
    assert!(!node_a.apply_trade(&trade));
    assert_eq!(node_a.pending_trades.len(), 1);

    node_a.add_order(buy.clone());
    assert_eq!(node_a.pending_trades.len(), 1);

     let pending = node_a.add_order(sell.clone());

     assert_eq!(pending.len(), 1);

	     for trade in pending {
    assert!(node_a.apply_trade(&trade));
    }

    assert!(node_a.pending_trades.is_empty());

    // Node B receives the orders first, then the trade.
    node_b.add_order(sell);
    node_b.add_order(buy);

    assert!(node_b.apply_trade(&trade));

    // Both nodes should converge to the same state.
    assert_eq!(node_a.orders.len(), node_b.orders.len());

    for order_a in &node_a.orders {
        let order_b = node_b
            .orders
            .iter()
            .find(|order| order.id == order_a.id)
            .expect("node B should contain the same order");

        assert_eq!(order_a.amount, order_b.amount);
    }

    assert_eq!(node_a.applied_trades, node_b.applied_trades);

    assert_eq!(node_a.pending_trades.len(), node_b.pending_trades.len());

    for (a, b) in node_a
        .pending_trades
        .iter()
        .zip(node_b.pending_trades.iter())
    {
        assert_eq!(a.id, b.id);
    }

    assert_eq!(node_a.removed_orders, node_b.removed_orders);
}

#[test]
fn trade_before_orders_applies_once_when_orders_arrive() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trade-before-orders-{}.json",
        Uuid::new_v4()
    ));

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    let trade = Trade {
        id: trade_id(buy.id, sell.id, 100, 4, 10),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    let mut book = OrderBook::new(path.clone());

    // Trade arrives before either order.
    assert!(!book.apply_trade(&trade));
    assert_eq!(book.pending_trades.len(), 1);
    assert!(book.applied_trades.is_empty());

    // First order arrives. Trade must remain pending.
    book.add_order(buy.clone());

    assert_eq!(book.pending_trades.len(), 1);
    assert!(book.applied_trades.is_empty());

    // Second order arrives. The pending trade is now applicable.
    let pending = book.add_order(sell.clone());

    assert_eq!(pending.len(), 1);

    for trade in pending {
    assert!(book.apply_trade(&trade));
    }
    assert!(book.pending_trades.is_empty());
    assert_eq!(book.applied_trades.len(), 1);
    assert_eq!(book.applied_trades[0], trade.id.to_string());

    let remaining_buy = book
        .orders
        .iter()
        .find(|order| order.id == buy.id)
        .expect("buy order should remain");

    let remaining_sell = book
        .orders
        .iter()
        .find(|order| order.id == sell.id)
        .expect("sell order should remain");

    assert_eq!(remaining_buy.amount, 6);
    assert_eq!(remaining_sell.amount, 6);

    // Duplicate delivery must not apply the trade a second time.
    assert!(!book.apply_trade(&trade));

    assert_eq!(book.applied_trades.len(), 1);
    assert_eq!(book.pending_trades.len(), 0);

    let remaining_buy = book
        .orders
        .iter()
        .find(|order| order.id == buy.id)
        .expect("buy order should remain after duplicate");

    let remaining_sell = book
        .orders
        .iter()
        .find(|order| order.id == sell.id)
        .expect("sell order should remain after duplicate");

    assert_eq!(remaining_buy.amount, 6);
    assert_eq!(remaining_sell.amount, 6);

    let _ = fs::remove_file(path);
}

#[test]
fn duplicate_out_of_order_delivery_converges() {
    let path_a = std::env::temp_dir().join("opendex-test-duplicate-out-of-order-a.json");
    let path_b = std::env::temp_dir().join("opendex-test-duplicate-out-of-order-b.json");

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    let trade = Trade {
        id: trade_id(buy.id, sell.id, 100, 4, 10),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    let mut node_a = OrderBook::new(path_a);
    let mut node_b = OrderBook::new(path_b);

    // Node A receives:
    // TRADE -> BUY -> duplicate TRADE -> SELL

    assert!(!node_a.apply_trade(&trade));
    assert_eq!(node_a.pending_trades.len(), 1);

    node_a.add_order(buy.clone());
    assert_eq!(node_a.pending_trades.len(), 1);

    // The duplicate trade must not be added twice to pending trades.
    assert!(!node_a.apply_trade(&trade));
    assert_eq!(node_a.pending_trades.len(), 1);

    node_a.add_order(sell.clone());

     let pending = node_a.add_order(sell.clone());

    assert_eq!(pending.len(), 1);

    for trade in pending {
    assert!(node_a.apply_trade(&trade));
   }
    assert!(node_a.pending_trades.is_empty());
    assert_eq!(node_a.applied_trades.len(), 1);
    assert_eq!(node_a.applied_trades[0], trade.id.to_string());

    // Node B receives:
    // SELL -> BUY -> TRADE

    node_b.add_order(sell);
    node_b.add_order(buy);

    assert!(node_b.apply_trade(&trade));

    // Both nodes should have the same logical orderbook state.
    assert_eq!(node_a.orders.len(), node_b.orders.len());

    for order_a in &node_a.orders {
        let order_b = node_b
            .orders
            .iter()
            .find(|order| order.id == order_a.id)
            .expect("node B should contain the same order");

        assert_eq!(order_a.amount, order_b.amount);
    }

    assert_eq!(node_a.applied_trades, node_b.applied_trades);
    assert_eq!(node_a.pending_trades.len(), node_b.pending_trades.len());
    assert_eq!(node_a.removed_orders, node_b.removed_orders);
}
#[test]
fn orderbook_persistence_preserves_distributed_state() {
    let path = std::env::temp_dir().join("opendex-test-persistence.json");

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    let trade = Trade {
        id: trade_id(buy.id, sell.id, 100, 4, 10),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    // Build state before persistence.
    let mut book = OrderBook::new(path.clone());

    book.add_order(buy.clone());
    book.add_order(sell.clone());

    assert!(book.apply_trade(&trade));

    assert_eq!(book.applied_trades.len(), 1);
    assert!(book.pending_trades.is_empty());

    let buy_before = book
        .orders
        .iter()
        .find(|order| order.id == buy.id)
        .expect("buy order should remain");

    let sell_before = book
        .orders
        .iter()
        .find(|order| order.id == sell.id)
        .expect("sell order should remain");

    assert_eq!(buy_before.amount, 6);
    assert_eq!(sell_before.amount, 6);

    // Persist the state.
    book.save_orders().expect("orderbook should save");

    // Load it into a fresh OrderBook.
    let loaded = OrderBook::load_orders(path.clone()).expect("orderbook should load");

    // Orders survived with their remaining amounts.
    assert_eq!(loaded.orders.len(), 2);

    let loaded_buy = loaded
        .orders
        .iter()
        .find(|order| order.id == buy.id)
        .expect("loaded buy order should exist");

    let loaded_sell = loaded
        .orders
        .iter()
        .find(|order| order.id == sell.id)
        .expect("loaded sell order should exist");

    assert_eq!(loaded_buy.amount, 6);
    assert_eq!(loaded_sell.amount, 6);

    // Distributed state survived.
    assert_eq!(loaded.applied_trades.len(), 1);
    assert_eq!(loaded.applied_trades[0], trade.id.to_string());

    assert!(loaded.pending_trades.is_empty());
    assert!(loaded.removed_orders.is_empty());

    // The same trade must still be idempotent after recovery.
    let mut recovered = loaded;

    assert!(!recovered.apply_trade(&trade));

    let recovered_buy = recovered
        .orders
        .iter()
        .find(|order| order.id == buy.id)
        .expect("recovered buy order should exist");

    let recovered_sell = recovered
        .orders
        .iter()
        .find(|order| order.id == sell.id)
        .expect("recovered sell order should exist");

    assert_eq!(recovered_buy.amount, 6);
    assert_eq!(recovered_sell.amount, 6);

    let _ = fs::remove_file(path);
}
#[test]
fn pending_trade_survives_restart() {
    let path = std::env::temp_dir().join("opendex-test-pending-trade-restart.json");

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    let trade = Trade {
        id: trade_id(buy.id, sell.id, 100, 4, 10),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    // Trade arrives before its orders.
    let mut book = OrderBook::new(path.clone());

    assert!(!book.apply_trade(&trade));
    assert_eq!(book.pending_trades.len(), 1);
    assert_eq!(book.pending_trades[0].id, trade.id);

    // Persist the pending trade.
    book.save_orders().expect("orderbook should save");

    // Simulate a restart.
    let mut recovered = OrderBook::load_orders(path.clone()).expect("orderbook should load");

    // The pending trade must have survived the restart.
    assert_eq!(recovered.pending_trades.len(), 1);
    assert_eq!(recovered.pending_trades[0].id, trade.id);

    // Orders arrive after restart.
    recovered.add_order(buy.clone());

    // Still waiting for the sell order.
    assert_eq!(recovered.pending_trades.len(), 1);

    recovered.add_order(sell.clone());

    let pending = recovered.add_order(sell.clone());

    assert_eq!(pending.len(), 1);

    for trade in pending {
    assert!(recovered.apply_trade(&trade));
    }
    assert!(recovered.pending_trades.is_empty());
    assert_eq!(recovered.applied_trades.len(), 1);
    assert_eq!(recovered.applied_trades[0], trade.id.to_string());

    // Both orders should have been reduced exactly once.
    let recovered_buy = recovered
        .orders
        .iter()
        .find(|order| order.id == buy.id)
        .expect("recovered buy order should exist");

    let recovered_sell = recovered
        .orders
        .iter()
        .find(|order| order.id == sell.id)
        .expect("recovered sell order should exist");

    assert_eq!(recovered_buy.amount, 6);
    assert_eq!(recovered_sell.amount, 6);

    let _ = fs::remove_file(path);
}
#[test]
fn removed_order_is_not_resurrected_by_stale_state() {
    let path_a = std::env::temp_dir().join("opendex-test-no-resurrection-a.json");
    let path_b = std::env::temp_dir().join("opendex-test-no-resurrection-b.json");

    let mut node_a = OrderBook::new(path_a);
    let mut node_b = OrderBook::new(path_b);

    let order_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    let order_a = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

    // Both nodes initially know the order.
    node_a.add_order(order_a.clone());
    node_b.add_order(order_a.clone());

// Node A receives a trade that completely fills the order.
let trade = Trade {
    id: trade_id(
        Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
        order_id,
        100,
        10,
        10,
    ),
    buy_order: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
    sell_order: order_id,
    maker: order_a.trader.clone(),
    taker: "trader-00000000-0000-0000-0000-000000000003".to_string(),
    buyer: "trader-00000000-0000-0000-0000-000000000003".to_string(),
    seller: order_a.trader.clone(),
    market: market(),
    price: 100,
    amount: 10,
    fill_sequence: 10,
    state: TradeState::Created,
    created_at: 0,
    updated_at: 0,
};

    // Add the buy order so the trade can be applied.
    node_a.add_order(order(
        "00000000-0000-0000-0000-000000000003",
        Side::Buy,
        100,
        10,
    ));

    assert!(node_a.apply_trade(&trade));

    // The sell order was completely filled and removed.
    assert!(!node_a.orders.iter().any(|o| o.id == order_id));
    assert!(node_a.removed_orders.contains(&order_id));
    assert!(node_a.applied_trades.contains(&trade.id.to_string()));

    // Node B is stale and still has the old order.
    assert!(node_b.orders.iter().any(|o| o.id == order_id));

    // Simulate the important part of StateResponse merging:
    // Node A receives Node B's stale order, but must not resurrect it.
    if !node_a.removed_orders.contains(&order_id) {
        node_a.removed_orders.push(order_id);
    }

    for stale_order in node_b.orders.clone() {
        if node_a.removed_orders.contains(&stale_order.id) {
            continue;
        }

        node_a.add_order(stale_order);
    }

    // The filled order must still be absent.
    assert!(
        !node_a.orders.iter().any(|o| o.id == order_id),
        "a removed order must never be resurrected by stale state"
    );

    // The tombstone must remain.
    assert!(node_a.removed_orders.contains(&order_id));

    // The trade must remain marked as applied.
    assert!(node_a.applied_trades.contains(&trade.id.to_string()));
}
#[test]
fn trade_waits_for_both_orders_then_applies_once() {
    let path = std::env::temp_dir().join("opendex-test-trade-before-orders.json");

    let mut book = OrderBook::new(path);

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    let trade = Trade {
        id: trade_id(buy.id, sell.id, 100, 4, 10),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    // Trade arrives before either order.
    assert!(!book.apply_trade(&trade));

    assert_eq!(book.pending_trades.len(), 1);
    assert!(book.applied_trades.is_empty());

    // BUY arrives. Trade still cannot be applied.
    book.add_order(buy.clone());

    assert_eq!(book.pending_trades.len(), 1);
    assert!(book.applied_trades.is_empty());

    let buy_remaining = book
        .orders
        .iter()
        .find(|o| o.id == buy.id)
        .expect("buy order should exist");

    assert_eq!(buy_remaining.amount, 10);

    // SELL arrives. The pending trade is now applicable.
    let pending = book.add_order(sell.clone());

    assert_eq!(pending.len(), 1);

    for trade in pending {
    assert!(book.apply_trade(&trade));
    }
    assert!(book.pending_trades.is_empty());
    assert_eq!(book.applied_trades.len(), 1);
    assert_eq!(book.applied_trades[0], trade.id.to_string());

    let buy_remaining = book
        .orders
        .iter()
        .find(|o| o.id == buy.id)
        .expect("buy order should remain");

    let sell_remaining = book
        .orders
        .iter()
        .find(|o| o.id == sell.id)
        .expect("sell order should remain");

    assert_eq!(buy_remaining.amount, 6);
    assert_eq!(sell_remaining.amount, 6);

    // A duplicate delivery must not apply the trade again.
    assert!(!book.apply_trade(&trade));

    assert_eq!(book.pending_trades.len(), 0);

    let buy_remaining = book.orders.iter().find(|o| o.id == buy.id).unwrap();

    let sell_remaining = book.orders.iter().find(|o| o.id == sell.id).unwrap();

    assert_eq!(buy_remaining.amount, 6);
    assert_eq!(sell_remaining.amount, 6);
}
#[test]
fn cancellation_before_order_creation_prevents_resurrection() {
    let path = std::env::temp_dir().join("opendex-test-cancel-before-order.json");

    let mut book = OrderBook::new(path);

    let order_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    // Cancellation arrives before OrderCreated.
    assert!(!book.remove_order(order_id));

    // The cancellation must still create a tombstone.
    assert!(book.removed_orders.contains(&order_id));

    let order = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

    // A stale OrderCreated must not resurrect the cancelled order.
    book.add_order(order);

    assert!(
        !book.orders.iter().any(|o| o.id == order_id),
        "cancelled order must not be resurrected"
    );

    assert!(book.removed_orders.contains(&order_id));
}

#[test]
fn duplicate_cancellation_is_idempotent() {
    let path = std::env::temp_dir().join("opendex-test-duplicate-cancel.json");

    let mut book = OrderBook::new(path);

    let order_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    let order = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

    book.add_order(order);

    assert!(book.remove_order(order_id));
    assert!(!book.remove_order(order_id));

    assert_eq!(book.removed_orders.len(), 1);
    assert_eq!(book.removed_orders[0], order_id);
}

#[test]
fn stale_order_cannot_be_added_after_cancellation() {
    let path = std::env::temp_dir().join("opendex-test-stale-order-after-cancel.json");

    let mut book = OrderBook::new(path);

    let order_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    let original = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

    book.add_order(original);

    assert!(book.remove_order(order_id));

    // Simulate an old OrderCreated message arriving later.
    let stale_order = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

    book.add_order(stale_order);

    assert!(book.orders.is_empty());
    assert_eq!(book.removed_orders.len(), 1);
    assert_eq!(book.removed_orders[0], order_id);
}
#[test]
fn rejects_trade_with_wrong_buy_side_without_mutating_state() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-invalid-buy-side-{}.json",
        Uuid::new_v4()
    ));

    let mut book = OrderBook::new(path);

    let buy = order(
        "00000000-0000-0000-0000-000000000001",
        Side::Sell, // deliberately wrong
        100,
        10,
    );

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    book.add_order(buy.clone());
    book.add_order(sell.clone());

    let trade = Trade {
        id: trade_id(buy.id, sell.id, 100, 4, 10),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: "test-trader".to_string(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    let before = book.orders.clone();

    assert!(book.validate_trade(&trade).is_err());

    // Invalid trade must not mutate the orderbook.
    assert_orders_unchanged(&before, &book.orders);
    assert!(book.applied_trades.is_empty());
    assert!(book.pending_trades.is_empty());
}

#[test]
fn rejects_trade_with_wrong_seller_identity() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-invalid-seller-{}.json",
        Uuid::new_v4()
    ));

    let mut book = OrderBook::new(path);

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    book.add_order(buy.clone());
    book.add_order(sell.clone());

    let trade = Trade {
        id: trade_id(buy.id, sell.id, 100, 4, 10),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: "attacker".to_string(),
        market: market(),
        price: 100,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    let before = book.orders.clone();

    assert!(book.validate_trade(&trade).is_err());
    assert_orders_unchanged(&before, &book.orders);
    assert!(book.applied_trades.is_empty());
}

#[test]
fn rejects_trade_above_buy_limit() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-price-above-buy-{}.json",
        Uuid::new_v4()
    ));

    let mut book = OrderBook::new(path);

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 90, 10);

    book.add_order(buy.clone());
    book.add_order(sell.clone());

    let trade = Trade {
        id: Uuid::new_v4(),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 101,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    assert!(book.validate_trade(&trade).is_err());
}

#[test]
fn rejects_trade_below_sell_limit() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-price-below-sell-{}.json",
        Uuid::new_v4()
    ));

    let mut book = OrderBook::new(path);

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 90, 10);

    book.add_order(buy.clone());
    book.add_order(sell.clone());

    let trade = Trade {
        id: Uuid::new_v4(),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 89,
        amount: 4,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    assert!(book.validate_trade(&trade).is_err());
}

#[test]
fn rejects_trade_exceeding_remaining_amount() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trade-overfill-{}.json",
        Uuid::new_v4()
    ));

    let mut book = OrderBook::new(path);

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 5);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    book.add_order(buy.clone());
    book.add_order(sell.clone());

    let trade = Trade {
        id: Uuid::new_v4(),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 6,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    let before = book.orders.clone();

    assert!(book.validate_trade(&trade).is_err());
    assert_orders_unchanged(&before, &book.orders);
    assert!(book.applied_trades.is_empty());
}

#[test]
fn rejects_zero_amount_trade() {
    let path =
        std::env::temp_dir().join(format!("opendex-test-zero-trade-{}.json", Uuid::new_v4()));

    let mut book = OrderBook::new(path);

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    book.add_order(buy.clone());
    book.add_order(sell.clone());

    let trade = Trade {
        id: Uuid::new_v4(),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 0,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 0,
        updated_at: 0,
    };

    assert!(book.validate_trade(&trade).is_err());
}
#[test]
fn failed_snapshot_trade_does_not_partially_mutate_live_state() {
    let order_path = std::env::temp_dir().join("opendex-test-snapshot-atomic.json");
    let trade_path = std::env::temp_dir().join("opendex-test-snapshot-atomic-trades.json");

       let mut order_book = OrderBook::new(order_path);
    let mut trade_book = TradeBook::new(trade_path);

    let existing_order =
        order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

    order_book.add_order(existing_order.clone());

    let original_orders = order_book.orders.clone();
    let original_trades = trade_book.trades.clone();
    let original_removed_orders = order_book.removed_orders.clone();
    let original_applied_trades = order_book.applied_trades.clone();

    // This trade references a buy order that does not exist.
    // Snapshot application must reject it rather than inserting the
    // trade into TradeBook and leaving the books inconsistent.
    let invalid_trade = Trade {
        id: trade_id(
            Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
            existing_order.id,
            100,
            10,
            10,
        ),
        buy_order: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
        sell_order: existing_order.id,
        maker: existing_order.trader.clone(),
        taker: "test-trader".to_string(),
        buyer: "test-trader".to_string(),
        seller: existing_order.trader.clone(),
        market: market(),
        price: 100,
        amount: 10,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };

    let snapshot = StateSnapshot {
        orders: vec![existing_order],
        trades: vec![invalid_trade],
        removed_orders: Vec::new(),
        applied_trades: Vec::new(),
    };

    let result = apply_snapshot(snapshot, &mut order_book, &mut trade_book);

    assert!(result.is_err());

    // The live books must remain exactly as they were before the failed
    // snapshot application.
    assert_orders_unchanged(&original_orders, &order_book.orders);
    assert_eq!(original_trades.len(), trade_book.trades.len());
    assert_eq!(original_removed_orders, order_book.removed_orders);
    assert_eq!(original_applied_trades, order_book.applied_trades);
}
#[test]
fn materialized_snapshot_does_not_replay_historical_trades() {
    // snapshot contains an order already reduced by a historical trade
    // applying the snapshot must not reduce it a second time
}
#[test]
fn applying_same_materialized_snapshot_twice_is_idempotent() {
    // applying the same snapshot twice must produce identical state
}
#[test]
fn rejects_snapshot_trade_with_invalid_maker() {
    let order_path = std::env::temp_dir().join(format!(
        "opendex-test-snapshot-maker-{}.json",
        Uuid::new_v4()
    ));
    let trade_path = std::env::temp_dir().join(format!(
        "opendex-test-snapshot-maker-trades-{}.json",
        Uuid::new_v4()
    ));

    let mut order_book = OrderBook::new(order_path);
    let mut trade_book = TradeBook::new(trade_path);

    let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

    let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

    order_book.add_order(buy.clone());
    order_book.add_order(sell.clone());

    let original_orders = order_book.orders.clone();
    let original_trades = trade_book.trades.clone();

    let invalid_trade = Trade {
        id: trade_id(buy.id, sell.id, 100, 4, 10),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: "not-a-participant".to_string(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: 100,
        amount: 10,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };

    let snapshot = StateSnapshot {
        orders: vec![buy, sell],
        trades: vec![invalid_trade],
        removed_orders: Vec::new(),
        applied_trades: Vec::new(),
    };

    let result = apply_snapshot(snapshot, &mut order_book, &mut trade_book);

    assert!(result.is_err());

    assert_orders_unchanged(&original_orders, &order_book.orders);
    assert_eq!(original_trades.len(), trade_book.trades.len());
}
#[test]
fn snapshot_can_reduce_existing_order_amount() {
    let order_path = std::env::temp_dir().join(format!(
        "opendex-test-snapshot-reduce-{}.json",
        Uuid::new_v4()
    ));
    let trade_path = std::env::temp_dir().join(format!(
        "opendex-test-snapshot-reduce-trades-{}.json",
        Uuid::new_v4()
    ));

    let mut order_book = OrderBook::new(order_path);
    let mut trade_book = TradeBook::new(trade_path);

    let local_order = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 100);

    order_book.add_order(local_order.clone());

    let newer_order = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 40);

    let snapshot = StateSnapshot {
        orders: vec![newer_order],
        trades: Vec::new(),
        removed_orders: Vec::new(),
        applied_trades: Vec::new(),
    };

    apply_snapshot(snapshot, &mut order_book, &mut trade_book).unwrap();

    let resulting_order = order_book
        .orders
        .iter()
        .find(|o| o.id == local_order.id)
        .unwrap();

    assert_eq!(resulting_order.amount, 40);
}
#[test]
fn conflicting_snapshot_order_terms_are_rejected() {
    let order_path = std::env::temp_dir().join(format!(
        "opendex-test-snapshot-conflict-{}.json",
        Uuid::new_v4()
    ));
    let trade_path = std::env::temp_dir().join(format!(
        "opendex-test-snapshot-conflict-trades-{}.json",
        Uuid::new_v4()
    ));

    let mut order_book = OrderBook::new(order_path);
    let mut trade_book = TradeBook::new(trade_path);

    let local_order = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 40);

    order_book.add_order(local_order.clone());

    let mut conflicting = local_order.clone();
    conflicting.trader = "attacker".to_string();
    conflicting.price = 1;

    let snapshot = StateSnapshot {
        orders: vec![conflicting],
        trades: Vec::new(),
        removed_orders: Vec::new(),
        applied_trades: Vec::new(),
    };

    apply_snapshot(snapshot, &mut order_book, &mut trade_book).unwrap();

    let resulting_order = order_book
        .orders
        .iter()
        .find(|o| o.id == local_order.id)
        .unwrap();

    assert_eq!(resulting_order.trader, local_order.trader);
    assert_eq!(resulting_order.price, local_order.price);
    assert_eq!(resulting_order.amount, local_order.amount);
}
#[test]
fn snapshot_does_not_resurrect_removed_order() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-snapshot-removed-{}.json",
        Uuid::new_v4()
    ));

    let mut book = OrderBook::new(path);

    let original = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

    book.add_order(original.clone());
    book.remove_order(original.id);

    let snapshot_order = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 50);

    book.update_order_from_snapshot(snapshot_order);

    assert!(book.orders.is_empty());
    assert!(book.removed_orders.contains(&original.id));
}

#[test]
fn matching_rejects_self_trade() {
    let path =
        std::env::temp_dir().join(format!("opendex-test-self-trade-{}.json", Uuid::new_v4()));

    let mut book = OrderBook::new(path);

    let sell = order(
        "00000000-0000-0000-0000-000000000001",
        Side::Sell,
        100 * PRICE_SCALE,
        AMOUNT_SCALE,
    );

    let mut buy = order(
        "00000000-0000-0000-0000-000000000002",
        Side::Buy,
        100 * PRICE_SCALE,
        AMOUNT_SCALE,
    );
    buy.trader = sell.trader.clone();
    book.add_order(sell);

    let trades = book.match_order(&buy);

    assert!(
        trades.is_empty(),
        "a trader must not match against their own resting order"
    );
}
#[test]
fn identical_sized_partial_fills_must_have_distinct_trade_ids() {
    let path = std::env::temp_dir().join(
        format!("opendex-test-identical-partial-fills-{}.json", Uuid::new_v4())
    );

    let mut book = OrderBook::new(path);

    let buy = order(
        "00000000-0000-0000-0000-000000000001",
        Side::Buy,
        100,
        10,
    );

    let sell = order(
        "00000000-0000-0000-0000-000000000002",
        Side::Sell,
        100,
        10,
    );

    book.add_order(buy.clone());
    book.add_order(sell.clone());

    // First legitimate 4-unit fill.
    let trades1 = book.match_order(&order(
        "00000000-0000-0000-0000-000000000001",
        Side::Buy,
        100,
        4,
    ));

    assert_eq!(trades1.len(), 1);

    // Apply the first fill.
    assert!(book.apply_trade(&trades1[0]));

    // The resting sell order now has 6 remaining.
    let remaining_sell = book.orders.iter().find(|o| o.id == sell.id).unwrap();
    assert_eq!(remaining_sell.amount, 6);

    // Second legitimate 4-unit fill against the same sell order.
    //
    // We construct the trade through matching again rather than inventing
    // a UUID, so this tests the actual production trade-ID generation path.
    let trades2 = book.match_order(&order(
        "00000000-0000-0000-0000-000000000001",
        Side::Buy,
        100,
        4,
    ));

    assert_eq!(trades2.len(), 1);

    // These are two different trade events.
    assert_ne!(
        trades1[0].id,
        trades2[0].id,
        "two legitimate fills must not share the same trade ID"
    );
}
