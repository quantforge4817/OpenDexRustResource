
use p2p_node::balances::{Balance, BalanceBook};
use p2p_node::exchange::{Exchange, RecordTrade,  SubmitOrder};
use p2p_node::orderbook::OrderBook;
use p2p_node::trades::TradeBook;
use protocol::message::{Asset, Market, Order, Side, Trade, TradeState};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn market() -> Market {
    Market {
        base: Asset::BTC,
        quote: Asset::XMR,
    }
}

fn deterministic_trade_id(
    buy_order: Uuid,
    sell_order: Uuid,
    price: u64,
    amount: u64,
    fill_sequence: u64,
) -> Uuid {
    let trade_input = format!(
        "{}:{}:{}:{}:{}",
        buy_order,
        sell_order,
        price,
        amount,
        fill_sequence,
    );

    let hash = Sha256::digest(trade_input.as_bytes());
    Uuid::from_slice(&hash[..16]).unwrap()
}

fn books() -> (OrderBook, TradeBook, BalanceBook) {
    let suffix = Uuid::new_v4();

    (
        OrderBook::new(
            std::env::temp_dir().join(format!(
                "opendex-test-exchange-orders-{suffix}.json"
            )),
        ),
        TradeBook::new(
            std::env::temp_dir().join(format!(
                "opendex-test-exchange-trades-{suffix}.json"
            )),
        ),
        BalanceBook::new(
            std::env::temp_dir().join(format!(
                "opendex-test-exchange-balances-{suffix}.json"
            )),
        ),
    )
}

#[test]
fn record_trade_rejects_invalid_deterministic_trade_id() {
    let (mut order_book, mut trade_book, mut balance_book) = books();

    let buy = Order {
        id: Uuid::new_v4(),
        trader: "buyer".to_string(),
        market: market(),
        side: Side::Buy,
        price: 100,
        amount: 10,
    };

    let sell = Order {
        id: Uuid::new_v4(),
        trader: "seller".to_string(),
        market: market(),
        side: Side::Sell,
        price: 100,
        amount: 10,
    };

    order_book.orders.push(buy.clone());
    order_book.orders.push(sell.clone());

    let trade = Trade {
        id: Uuid::new_v4(),
        buy_order: buy.id,
        sell_order: sell.id,
        maker: "seller".to_string(),
        taker: "buyer".to_string(),
        buyer: "buyer".to_string(),
        seller: "seller".to_string(),
        market: market(),
        price: 100,
        amount: 5,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
};

    let mut exchange = Exchange::new(
        &mut order_book,
        &mut trade_book,
        &mut balance_book,
    );

    let result = exchange.record_trade(RecordTrade {
        actor: "seller".to_string(),
        trade,
    });

    assert!(
        result.is_err(),
        "trade with forged deterministic ID must be rejected"
    );

    assert!(
        trade_book.trades.is_empty(),
        "invalid trade must not be persisted"
    );

    assert!(
        order_book.pending_trades.is_empty(),
        "invalid trade must not become pending"
    );
}

#[test]
fn record_trade_reconciles_buy_reservation() {
    let (mut order_book, mut trade_book, mut balance_book) = books();

    let price = 100 * protocol::message::PRICE_SCALE;
    let amount = 10 * protocol::message::AMOUNT_SCALE;
    let trade_amount = 5 * protocol::message::AMOUNT_SCALE;

    let buy = Order {
        id: Uuid::new_v4(),
        trader: "buyer".to_string(),
        market: market(),
        side: Side::Buy,
        price,
        amount,
    };

    let sell = Order {
        id: Uuid::new_v4(),
        trader: "seller".to_string(),
        market: market(),
        side: Side::Sell,
        price,
        amount,
    };

    balance_book
        .credit(
            "buyer",
            Asset::XMR,
            1000 * protocol::message::AMOUNT_SCALE,
        )
        .unwrap();

    balance_book.lock_for_order(&buy).unwrap();

    order_book.orders.push(buy.clone());
    order_book.orders.push(sell.clone());

    let trade_id = deterministic_trade_id(
        buy.id,
        sell.id,
        price,
        trade_amount,
        10,
    );

    let trade = Trade {
        id: trade_id,
        buy_order: buy.id,
        sell_order: sell.id,
        maker: "seller".to_string(),
        taker: "buyer".to_string(),
        buyer: "buyer".to_string(),
        seller: "seller".to_string(),
        market: market(),
        price,
        amount: trade_amount,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };

    {
        let mut exchange = Exchange::new(
            &mut order_book,
            &mut trade_book,
            &mut balance_book,
        );

        let result = exchange
            .record_trade(RecordTrade {
                actor: "seller".to_string(),
                trade: trade.clone(),
            })
            .unwrap();

        assert!(result);

        let duplicate = exchange.record_trade(RecordTrade {
            actor: "seller".to_string(),
            trade,
        });

        assert_eq!(duplicate.unwrap(), false);
    }

    assert_eq!(
        order_book
            .orders
            .iter()
            .find(|order| order.id == buy.id)
            .unwrap()
            .amount,
        5 * protocol::message::AMOUNT_SCALE
    );

    assert_eq!(
        balance_book.order_reservation(&buy.id),
        Some(500 * protocol::message::AMOUNT_SCALE)
    );

    assert_eq!(
        balance_book.balance("buyer", &Asset::XMR),
        Balance {
            available: 500 * protocol::message::AMOUNT_SCALE,
            locked: 500 * protocol::message::AMOUNT_SCALE,
        }
    );
}
#[test]
fn pending_remote_trade_reconciles_reservation_when_orders_arrive() {
    let (mut order_book, mut trade_book, mut balance_book) = books();

    let price = 100 * protocol::message::PRICE_SCALE;
    let amount = 10 * protocol::message::AMOUNT_SCALE;
    let trade_amount = 5 * protocol::message::AMOUNT_SCALE;

    let buy = Order {
        id: Uuid::new_v4(),
        trader: "buyer".to_string(),
        market: market(),
        side: Side::Buy,
        price,
        amount,
    };

    let sell = Order {
        id: Uuid::new_v4(),
        trader: "seller".to_string(),
        market: market(),
        side: Side::Sell,
        price,
        amount,
    };

    balance_book
        .credit(
            "buyer",
            Asset::XMR,
            1000 * protocol::message::AMOUNT_SCALE,
        )
        .unwrap();

    balance_book.lock_for_order(&buy).unwrap();

    let trade_id = deterministic_trade_id(
        buy.id,
        sell.id,
        price,
        trade_amount,
        10,
    );

    let trade = Trade {
        id: trade_id,
        buy_order: buy.id,
        sell_order: sell.id,
        maker: "seller".to_string(),
        taker: "buyer".to_string(),
        buyer: "buyer".to_string(),
        seller: "seller".to_string(),
        market: market(),
        price,
        amount: trade_amount,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };

    // The trade arrives before either order.
    {
        let mut exchange = Exchange::new(
            &mut order_book,
            &mut trade_book,
            &mut balance_book,
        );

        let result = exchange
            .record_trade(RecordTrade {
                actor: "seller".to_string(),
                trade: trade.clone(),
            })
            .unwrap();

        assert!(result);
    }

    // Trade should be pending, and reservation must still be untouched.
    assert_eq!(order_book.pending_trades.len(), 1);

    assert_eq!(
        balance_book.order_reservation(&buy.id),
        Some(1000 * protocol::message::AMOUNT_SCALE)
    );

    assert_eq!(
        balance_book.balance("buyer", &Asset::XMR),
        Balance {
            available: 0,
            locked: 1000 * protocol::message::AMOUNT_SCALE,
        }
    );

    // Now deliver the orders.
     {
    let mut exchange = Exchange::new(
        &mut order_book,
        &mut trade_book,
        &mut balance_book,
    );

    exchange
        .add_order(SubmitOrder {
            actor: "buyer".to_string(),
            order: buy.clone(),
        })
        .unwrap();

    exchange
        .add_order(SubmitOrder {
            actor: "seller".to_string(),
            order: sell.clone(),
        })
        .unwrap();
}

    // The pending trade should have applied to the order book.
    assert_eq!(order_book.pending_trades.len(), 0);

    assert_eq!(
        order_book
            .orders
            .iter()
            .find(|order| order.id == buy.id)
            .map(|order| order.amount),
        Some(5 * protocol::message::AMOUNT_SCALE)
    );

    // This is the important assertion:
    // applying the pending trade must also reconcile the reservation.
    assert_eq!(
        balance_book.order_reservation(&buy.id),
        Some(500 * protocol::message::AMOUNT_SCALE)
    );

    assert_eq!(
        balance_book.balance("buyer", &Asset::XMR),
        Balance {
            available: 500 * protocol::message::AMOUNT_SCALE,
            locked: 500 * protocol::message::AMOUNT_SCALE,
        }
    );
}
#[test]
fn submit_order_reconciles_local_buy_reservation() {
    let (mut order_book, mut trade_book, mut balance_book) = books();

    let price = 10 * protocol::message::PRICE_SCALE;
    let amount = 10 * protocol::message::AMOUNT_SCALE;
    let fill_amount = 5 * protocol::message::AMOUNT_SCALE;

    let sell = Order {
        id: Uuid::new_v4(),
        trader: "seller".to_string(),
        market: market(),
        side: Side::Sell,
        price,
        amount: fill_amount,
    };

    let buy = Order {
        id: Uuid::new_v4(),
        trader: "buyer".to_string(),
        market: market(),
        side: Side::Buy,
        price,
        amount,
    };

    balance_book
        .credit(
            "buyer",
            Asset::XMR,
            100 * protocol::message::AMOUNT_SCALE,
        )
        .unwrap();

    balance_book.lock_for_order(&buy).unwrap();

    // The seller is already resting on the book.
    order_book.orders.push(sell);


    let trades = {
        let mut exchange = Exchange::new(
        &mut order_book,
        &mut trade_book,
        &mut balance_book,
    );
      exchange
        .submit_order(SubmitOrder {
            actor: "buyer".to_string(),
            order: buy.clone(),
        })
        .unwrap()
};
    assert_eq!(trades.len(), 1);

    // 5 BTC of the 10 BTC buy order was filled.
    assert_eq!(
        balance_book.order_reservation(&buy.id),
        Some(50 * protocol::message::AMOUNT_SCALE)
    );

    assert_eq!(
        balance_book.balance("buyer", &Asset::XMR),
        Balance {
            available: 50 * protocol::message::AMOUNT_SCALE,
            locked: 50 * protocol::message::AMOUNT_SCALE,
        }
    );
}

#[test]
fn record_trade_reconciles_sell_reservation() {
    let (mut order_book, mut trade_book, mut balance_book) = books();

    let price = 10 * protocol::message::PRICE_SCALE;
    let amount = 10 * protocol::message::AMOUNT_SCALE;
    let trade_amount = 5 * protocol::message::AMOUNT_SCALE;

    let buy = Order {
        id: Uuid::new_v4(),
        trader: "buyer".to_string(),
        market: market(),
        side: Side::Buy,
        price,
        amount,
    };

    let sell = Order {
        id: Uuid::new_v4(),
        trader: "seller".to_string(),
        market: market(),
        side: Side::Sell,
        price,
        amount,
    };

    balance_book
        .credit(
            "seller",
            Asset::BTC,
            10 * protocol::message::AMOUNT_SCALE,
        )
        .unwrap();

    balance_book.lock_for_order(&sell).unwrap();

    order_book.orders.push(buy.clone());
    order_book.orders.push(sell.clone());

    let trade_id = deterministic_trade_id(
        buy.id,
        sell.id,
        price,
        trade_amount,
        10,
    );

    let trade = Trade {
        id: trade_id,
        buy_order: buy.id,
        sell_order: sell.id,
        maker: "seller".to_string(),
        taker: "buyer".to_string(),
        buyer: "buyer".to_string(),
        seller: "seller".to_string(),
        market: market(),
        price,
        amount: trade_amount,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };

    let mut exchange = Exchange::new(
        &mut order_book,
        &mut trade_book,
        &mut balance_book,
    );

    exchange
        .record_trade(RecordTrade {
            actor: "seller".to_string(),
            trade,
        })
        .unwrap();

    assert_eq!(
        balance_book.order_reservation(&sell.id),
        Some(5 * protocol::message::AMOUNT_SCALE)
    );

    assert_eq!(
        balance_book.balance("seller", &Asset::BTC),
        Balance {
            available: 5 * protocol::message::AMOUNT_SCALE,
            locked: 5 * protocol::message::AMOUNT_SCALE,
        }
    );
}
#[test]
fn record_trade_multiple_fills_progressively_reduce_sell_reservation() {
    let (mut order_book, mut trade_book, mut balance_book) = books();

    let price = 10 * protocol::message::PRICE_SCALE;
    let amount = 10 * protocol::message::AMOUNT_SCALE;

    let buy = Order {
        id: Uuid::new_v4(),
        trader: "buyer".to_string(),
        market: market(),
        side: Side::Buy,
        price,
        amount,
    };

    let sell = Order {
        id: Uuid::new_v4(),
        trader: "seller".to_string(),
        market: market(),
        side: Side::Sell,
        price,
        amount,
    };

    balance_book
        .credit("seller", Asset::BTC, amount)
        .unwrap();

    balance_book.lock_for_order(&sell).unwrap();

    order_book.orders.push(buy.clone());
    order_book.orders.push(sell.clone());

    {
        let mut exchange = Exchange::new(
            &mut order_book,
            &mut trade_book,
            &mut balance_book,
        );

        for (index, fill_amount) in [
            3 * protocol::message::AMOUNT_SCALE,
            2 * protocol::message::AMOUNT_SCALE,
            5 * protocol::message::AMOUNT_SCALE,
        ]
        .into_iter()
        .enumerate()
        {
            let fill_sequence = index as u64;

            let trade_id = deterministic_trade_id(
                buy.id,
                sell.id,
                price,
                fill_amount,
                fill_sequence,
            );

            let trade = Trade {
                id: trade_id,
                buy_order: buy.id,
                sell_order: sell.id,
                maker: "seller".to_string(),
                taker: "buyer".to_string(),
                buyer: "buyer".to_string(),
                seller: "seller".to_string(),
                market: market(),
                price,
                amount: fill_amount,
                fill_sequence,
                state: TradeState::Created,
                created_at: index as u64 + 1,
                updated_at: index as u64 + 1,
            };

            exchange
                .record_trade(RecordTrade {
                    actor: "seller".to_string(),
                    trade,
                })
                .unwrap();
        }
    }

    assert_eq!(
        balance_book.order_reservation(&sell.id),
        None
    );

    assert_eq!(
        balance_book.balance("seller", &Asset::BTC),
        Balance {
            available: 10 * protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );
}

#[test]
fn partial_fill_then_cancel_releases_only_remaining_sell_reservation() {
    let (mut order_book, mut trade_book, mut balance_book) = books();

    let price = 10 * protocol::message::PRICE_SCALE;
    let amount = 10 * protocol::message::AMOUNT_SCALE;
    let trade_amount = 4 * protocol::message::AMOUNT_SCALE;

    let buy = Order {
        id: Uuid::new_v4(),
        trader: "buyer".to_string(),
        market: market(),
        side: Side::Buy,
        price,
        amount,
    };

    let sell = Order {
        id: Uuid::new_v4(),
        trader: "seller".to_string(),
        market: market(),
        side: Side::Sell,
        price,
        amount,
    };

    balance_book
        .credit("seller", Asset::BTC, amount)
        .unwrap();

    balance_book.lock_for_order(&sell).unwrap();

    order_book.orders.push(buy.clone());
    order_book.orders.push(sell.clone());

    let trade_id = deterministic_trade_id(
        buy.id,
        sell.id,
        price,
        trade_amount,
        10,
    );

    let trade = Trade {
        id: trade_id,
        buy_order: buy.id,
        sell_order: sell.id,
        maker: "seller".to_string(),
        taker: "buyer".to_string(),
        buyer: "buyer".to_string(),
        seller: "seller".to_string(),
        market: market(),
        price,
        amount: trade_amount,
        fill_sequence: 10,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };

    {
        let mut exchange = Exchange::new(
            &mut order_book,
            &mut trade_book,
            &mut balance_book,
        );

        exchange
            .record_trade(RecordTrade {
                actor: "seller".to_string(),
                trade,
            })
            .unwrap();
    }

    // 4 BTC traded, so only 6 BTC remains reserved.
    assert_eq!(
        balance_book.order_reservation(&sell.id),
        Some(6 * protocol::message::AMOUNT_SCALE)
    );

    assert_eq!(
        balance_book.balance("seller", &Asset::BTC),
        Balance {
            available: 4 * protocol::message::AMOUNT_SCALE,
            locked: 6 * protocol::message::AMOUNT_SCALE,
        }
    );

    // Cancel the remaining order.
    balance_book.unlock_for_order(&sell).unwrap();

    assert_eq!(balance_book.order_reservation(&sell.id), None);

    assert_eq!(
        balance_book.balance("seller", &Asset::BTC),
        Balance {
            available: 10 * protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );
}
