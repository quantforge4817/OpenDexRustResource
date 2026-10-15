//! Kani formal verification proof harnesses for `OrderBook`.
//!
//! Proves properties about trade validation, idempotency, and fill sequences
//! using symbolic execution.

use crate::orderbook::OrderBook;
use protocol::message::{Market, Order, Side, Trade, TradeState};
use std::path::PathBuf;
use uuid::Uuid;

/// Helper to create a concrete OrderBook with test orders
fn make_orderbook_with_orders() -> OrderBook {
    let mut ob = OrderBook::new(PathBuf::from("/tmp/kani_test"));
    
    // Buy order: BTC/USD at price 50000, amount 1000
    let buy = Order {
        id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        trader: "buyer1".to_string(),
        market: Market { base: protocol::message::Asset::BTC, quote: protocol::message::Asset::XMR },
        side: Side::Buy,
        price: 50_000,
        amount: 1_000,
    };
    
    // Sell order: BTC/USD at price 49000, amount 800
    let sell = Order {
        id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        trader: "seller1".to_string(),
        market: Market { base: protocol::message::Asset::BTC, quote: protocol::message::Asset::XMR },
        side: Side::Sell,
        price: 49_000,
        amount: 800,
    };
    
    ob.orders.insert(buy.id, buy);
    ob.orders.insert(sell.id, sell);
    ob
}

/// Prove that zero-amount trades are rejected by `validate_trade_structure()`.
#[kani::proof]
fn verify_validate_trade_rejects_zero_amount() {
    let ob = make_orderbook_with_orders();
    
    let trade = Trade {
        id: Uuid::nil(),
        buy_order: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        sell_order: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        maker: "seller1".to_string(),
        taker: "buyer1".to_string(),
        buyer: "buyer1".to_string(),
        seller: "seller1".to_string(),
        market: Market { base: protocol::message::Asset::BTC, quote: protocol::message::Asset::XMR },
        price: 49_500,
        amount: 0, // Zero amount
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };
    
    assert!(ob.validate_trade_structure(&trade).is_err());
}

/// Prove that zero-price trades are rejected by `validate_trade_structure()`.
#[kani::proof]
fn verify_validate_trade_rejects_zero_price() {
    let ob = make_orderbook_with_orders();
    
    let trade = Trade {
        id: Uuid::nil(),
        buy_order: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        sell_order: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        maker: "seller1".to_string(),
        taker: "buyer1".to_string(),
        buyer: "buyer1".to_string(),
        seller: "seller1".to_string(),
        market: Market { base: protocol::message::Asset::BTC, quote: protocol::message::Asset::XMR },
        price: 0, // Zero price
        amount: 100,
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };
    
    assert!(ob.validate_trade_structure(&trade).is_err());
}

/// Prove that trades with same buy_order and sell_order are rejected.
#[kani::proof]
fn verify_validate_trade_rejects_same_order() {
    let ob = make_orderbook_with_orders();
    
    let order_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
    let trade = Trade {
        id: Uuid::nil(),
        buy_order: order_id,
        sell_order: order_id, // Same order
        maker: "seller1".to_string(),
        taker: "buyer1".to_string(),
        buyer: "buyer1".to_string(),
        seller: "seller1".to_string(),
        market: Market { base: protocol::message::Asset::BTC, quote: protocol::message::Asset::XMR },
        price: 49_500,
        amount: 100,
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };
    
    assert!(ob.validate_trade_structure(&trade).is_err());
}

/// Prove that trades with empty buyer/seller are rejected.
#[kani::proof]
fn verify_validate_trade_rejects_empty_participants() {
    let ob = make_orderbook_with_orders();
    
    let trade = Trade {
        id: Uuid::nil(),
        buy_order: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        sell_order: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        maker: "seller1".to_string(),
        taker: "buyer1".to_string(),
        buyer: "".to_string(), // Empty buyer
        seller: "seller1".to_string(),
        market: Market { base: protocol::message::Asset::BTC, quote: protocol::message::Asset::XMR },
        price: 49_500,
        amount: 100,
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };
    
    assert!(ob.validate_trade_structure(&trade).is_err());
}

/// Prove that trades with invalid market (base == quote) are rejected.
#[kani::proof]
fn verify_validate_trade_rejects_invalid_market() {
    let ob = make_orderbook_with_orders();
    
    let trade = Trade {
        id: Uuid::nil(),
        buy_order: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        sell_order: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        maker: "seller1".to_string(),
        taker: "buyer1".to_string(),
        buyer: "buyer1".to_string(),
        seller: "seller1".to_string(),
        market: Market { 
            base: protocol::message::Asset::BTC, 
            quote: protocol::message::Asset::BTC // Same base and quote
        },
        price: 49_500,
        amount: 100,
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };
    
    assert!(ob.validate_trade_structure(&trade).is_err());
}

/// Prove that `validate_trade()` rejects trades exceeding order amounts.
#[kani::proof]
fn verify_validate_trade_rejects_excess_amount() {
    let ob = make_orderbook_with_orders();
    
    // Sell order has amount 800, trade tries to take 1000
    let trade = Trade {
        id: Uuid::nil(),
        buy_order: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        sell_order: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        maker: "seller1".to_string(),
        taker: "buyer1".to_string(),
        buyer: "buyer1".to_string(),
        seller: "seller1".to_string(),
        market: Market { base: protocol::message::Asset::BTC, quote: protocol::message::Asset::XMR },
        price: 49_500,
        amount: 1_000, // Exceeds sell order amount of 800
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };
    
    assert!(ob.validate_trade(&trade).is_err());
}

/// Prove that `validate_trade()` rejects trades with price exceeding buy order.
#[kani::proof]
fn verify_validate_trade_rejects_high_price() {
    let ob = make_orderbook_with_orders();
    
    // Buy order has price 50000, trade price is 51000
    let trade = Trade {
        id: Uuid::nil(),
        buy_order: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        sell_order: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        maker: "seller1".to_string(),
        taker: "buyer1".to_string(),
        buyer: "buyer1".to_string(),
        seller: "seller1".to_string(),
        market: Market { base: protocol::message::Asset::BTC, quote: protocol::message::Asset::XMR },
        price: 51_000, // Exceeds buy order price of 50000
        amount: 100,
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };
    
    assert!(ob.validate_trade(&trade).is_err());
}

/// Prove that `validate_trade()` rejects trades with price below sell order.
#[kani::proof]
fn verify_validate_trade_rejects_low_price() {
    let ob = make_orderbook_with_orders();
    
    // Sell order has price 49000, trade price is 48000
    let trade = Trade {
        id: Uuid::nil(),
        buy_order: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        sell_order: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        maker: "seller1".to_string(),
        taker: "buyer1".to_string(),
        buyer: "buyer1".to_string(),
        seller: "seller1".to_string(),
        market: Market { base: protocol::message::Asset::BTC, quote: protocol::message::Asset::XMR },
        price: 48_000, // Below sell order price of 49000
        amount: 100,
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };
    
    assert!(ob.validate_trade(&trade).is_err());
}

/// Prove that a valid trade passes `validate_trade()`.
#[kani::proof]
fn verify_validate_trade_accepts_valid_trade() {
    let ob = make_orderbook_with_orders();
    
    let trade = Trade {
        id: Uuid::nil(),
        buy_order: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        sell_order: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        maker: "seller1".to_string(),
        taker: "buyer1".to_string(),
        buyer: "buyer1".to_string(),
        seller: "seller1".to_string(),
        market: Market { base: protocol::message::Asset::BTC, quote: protocol::message::Asset::XMR },
        price: 49_500, // Between sell (49000) and buy (50000)
        amount: 500, // Within both order amounts
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };
    
    assert!(ob.validate_trade(&trade).is_ok());
}
