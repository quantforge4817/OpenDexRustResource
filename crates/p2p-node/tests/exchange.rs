use p2p_node::exchange::{Exchange, SubmitOrder};
use p2p_node::orderbook::OrderBook;
use p2p_node::trades::TradeBook;
use protocol::message::{Asset, Market, Order, Side};
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
