use p2p_node::exchange::{Exchange, RecordTrade, SubmitOrder};
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

fn order(trader: &str) -> Order {
    Order {
        id: Uuid::new_v4(),
        trader: trader.to_string(),
        market: market(),
        side: Side::Buy,
        price: 100,
        amount: 10,
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
fn submit_order_accepts_order_owned_by_actor() {
    let (mut order_book, mut trade_book) = books();
    let order = order("alice");
    let order_id = order.id;

    let mut exchange = Exchange::new(&mut order_book, &mut trade_book);

    let trades = exchange
        .submit_order(SubmitOrder {
            actor: "alice".to_string(),
            order,
        })
        .unwrap();

    assert!(trades.is_empty());
    assert!(order_book.orders.iter().any(|o| o.id == order_id));
}

#[test]
fn submit_order_rejects_actor_who_is_not_order_owner() {
    let (mut order_book, mut trade_book) = books();
    let order = order("alice");

    let mut exchange = Exchange::new(&mut order_book, &mut trade_book);

    let result = exchange.submit_order(SubmitOrder {
        actor: "mallory".to_string(),
        order,
    });

    assert!(result.is_err());
    assert!(order_book.orders.is_empty());
}
#[test]
fn record_trade_rejects_invalid_structure_before_persistence() {
    let (mut order_book, mut trade_book) = books();

    let trade = Trade {
        id: Uuid::new_v4(),
        buy_order: Uuid::new_v4(),
        sell_order: Uuid::new_v4(),
        maker: "seller".to_string(),
        taker: "buyer".to_string(),
        buyer: "buyer".to_string(),
        seller: "seller".to_string(),
        market: market(),
        price: 100,
        amount: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };

    let mut exchange = Exchange::new(&mut order_book, &mut trade_book);

    let result = exchange.record_trade(RecordTrade {
        actor: "seller".to_string(),
        trade,
    });

    assert!(result.is_err());
    assert!(
        trade_book.trades.is_empty(),
        "invalid trade must not be persisted"
    );
    assert!(
        order_book.pending_trades.is_empty(),
        "invalid trade must not become pending"
    );
}
