use p2p_node::exchange::{Exchange, RecordTrade};
use p2p_node::orderbook::OrderBook;
use p2p_node::trades::TradeBook;
use protocol::message::{Asset, Market, Order, Side, Trade, TradeState};
use uuid::Uuid;

fn market() -> Market {
    Market {
        base: Asset::BTC,
        quote: Asset::XMR,
    }
}

fn books() -> (OrderBook, TradeBook) {
    let suffix = Uuid::new_v4();

    (
        OrderBook::new(
            std::env::temp_dir().join(format!("opendex-test-exchange-orders-{suffix}.json")),
        ),
        TradeBook::new(
            std::env::temp_dir().join(format!("opendex-test-exchange-trades-{suffix}.json")),
        ),
    )
}
#[test]
fn record_trade_rejects_invalid_deterministic_trade_id() {
    let (mut order_book, mut trade_book) = books();

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
        // Deliberately incorrect. The real ID must be derived from
        // buy_order, sell_order, price, and amount.
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
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };

    let mut exchange = Exchange::new(&mut order_book, &mut trade_book);

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
