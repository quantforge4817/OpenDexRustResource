use p2p_node::trades::*;
use protocol::message::{AMOUNT_SCALE, Asset, Market, PRICE_SCALE, Trade, TradeState};
use uuid::Uuid;

fn test_trade() -> Trade {
    Trade {
        id: Uuid::new_v4(),
        buy_order: Uuid::new_v4(),
        sell_order: Uuid::new_v4(),
        maker: "maker".to_string(),
        taker: "taker".to_string(),
        buyer: "buyer".to_string(),
        seller: "seller".to_string(),
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

#[test]
fn transition_trade_updates_state() {
    let path = std::env::temp_dir().join("opendex-test-trades-transition.json");

    let mut book = TradeBook::new(path);

    let trade = test_trade();
    let trade_id = trade.id;

    assert!(book.add_trade(trade));

    assert!(
        book.transition_trade(trade_id, TradeState::Accepted, 2,)
            .unwrap()
    );

    let stored = book
        .trades
        .iter()
        .find(|trade| trade.id == trade_id)
        .unwrap();

    assert_eq!(stored.state, TradeState::Accepted);
    assert_eq!(stored.updated_at, 2);
}

#[test]
fn transition_trade_rejects_invalid_transition() {
    let path = std::env::temp_dir().join("opendex-test-trades-invalid-transition.json");

    let mut book = TradeBook::new(path);

    let trade = test_trade();
    let trade_id = trade.id;

    assert!(book.add_trade(trade));

    let result = book.transition_trade(trade_id, TradeState::Settled, 2);

    assert!(result.is_err());

    let stored = book
        .trades
        .iter()
        .find(|trade| trade.id == trade_id)
        .unwrap();

    assert_eq!(stored.state, TradeState::Created);
    assert_eq!(stored.updated_at, 1);
}

#[test]
fn transition_trade_returns_false_for_unknown_trade() {
    let path = std::env::temp_dir().join("opendex-test-trades-unknown.json");

    let mut book = TradeBook::new(path);

    let result = book.transition_trade(Uuid::new_v4(), TradeState::Accepted, 2);

    assert!(!result.unwrap());
}

#[test]
fn transition_trade_allows_disputed_from_escrow_funding() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trade-disputed-{}.json",
        Uuid::new_v4()
    ));

    let mut book = TradeBook::new(path);

    let mut trade = test_trade();
    trade.state = TradeState::EscrowFunding;

    let trade_id = trade.id;
    assert!(book.add_trade(trade));

    assert!(
        book.transition_trade(trade_id, TradeState::Disputed, 2)
            .unwrap()
    );

    assert_eq!(
        book.get_trade(trade_id).unwrap().state,
        TradeState::Disputed
    );
}

#[test]
fn transition_trade_allows_settled_from_disputed() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trade-disputed-settled-{}.json",
        Uuid::new_v4()
    ));

    let mut book = TradeBook::new(path);

    let mut trade = test_trade();
    trade.state = TradeState::Disputed;

    let trade_id = trade.id;
    assert!(book.add_trade(trade));

    assert!(
        book.transition_trade(trade_id, TradeState::Settled, 2)
            .unwrap()
    );

    assert_eq!(book.get_trade(trade_id).unwrap().state, TradeState::Settled);
}

#[test]
fn transition_trade_allows_cancelled_from_disputed() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trade-disputed-cancelled-{}.json",
        Uuid::new_v4()
    ));

    let mut book = TradeBook::new(path);

    let mut trade = test_trade();
    trade.state = TradeState::Disputed;

    let trade_id = trade.id;
    assert!(book.add_trade(trade));

    assert!(
        book.transition_trade(trade_id, TradeState::Cancelled, 2)
            .unwrap()
    );

    assert_eq!(
        book.get_trade(trade_id).unwrap().state,
        TradeState::Cancelled
    );
}
