use libp2p::PeerId;
use p2p_node::balances::{Balance, BalanceBook};
use p2p_node::message_handler::{handle_trade_created, handle_trade_state_changed};
use p2p_node::orderbook::OrderBook;
use p2p_node::trades::*;
use protocol::message::{
    AMOUNT_SCALE, Asset, Market, Order, PRICE_SCALE, Side, Trade, TradeState, TradeStateChanged,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
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
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trades-transition-{}.json",
        Uuid::new_v4()
    ));

    let mut book = TradeBook::new(path);

    let trade = test_trade();
    let trade_id = trade.id;

    assert!(book.add_trade(trade));

    assert!(
        book.transition_trade(trade_id, TradeState::Accepted, 2)
            .unwrap()
    );

    let stored = book.get_trade(trade_id).unwrap();

    assert_eq!(stored.state, TradeState::Accepted);
    assert_eq!(stored.updated_at, 2);
}

#[test]
fn transition_trade_rejects_invalid_transition() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trades-invalid-transition-{}.json",
        Uuid::new_v4()
    ));

    let mut book = TradeBook::new(path);

    let trade = test_trade();
    let trade_id = trade.id;

    assert!(book.add_trade(trade));

    let result = book.transition_trade(trade_id, TradeState::Settled, 2);

    assert!(result.is_err());

    let stored = book.get_trade(trade_id).unwrap();

    assert_eq!(stored.state, TradeState::Created);
    assert_eq!(stored.updated_at, 1);
}

#[test]
fn transition_trade_returns_false_for_unknown_trade() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trades-unknown-{}.json",
        Uuid::new_v4()
    ));

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

#[test]
fn stale_trade_state_does_not_move_trade_backward() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trade-stale-state-{}.json",
        Uuid::new_v4()
    ));

    let mut book = TradeBook::new(path);

    let mut trade = test_trade();
    trade.state = TradeState::PaymentConfirmed;

    let trade_id = trade.id;

    assert!(book.add_trade(trade));

    assert!(
        p2p_node::trade_state::advance_trade_to_state(
            &mut book,
            trade_id,
            TradeState::PaymentPending,
            2,
        )
        .is_err()
    );

    assert_eq!(
        book.get_trade(trade_id).unwrap().state,
        TradeState::PaymentConfirmed
    );
}

#[test]
fn trade_state_can_jump_forward_when_intermediate_states_are_missing() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trade-state-jump-{}.json",
        Uuid::new_v4()
    ));

    let mut book = TradeBook::new(path);

    let trade = test_trade();
    let trade_id = trade.id;

    assert!(book.add_trade(trade));

    p2p_node::trade_state::advance_trade_to_state(&mut book, trade_id, TradeState::Settled, 2)
        .unwrap();

    assert_eq!(book.get_trade(trade_id).unwrap().state, TradeState::Settled);
}

#[test]
fn disputed_trade_can_recover_to_settled() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-trade-disputed-recovery-{}.json",
        Uuid::new_v4()
    ));

    let mut book = TradeBook::new(path);

    let mut trade = test_trade();
    trade.state = TradeState::Disputed;

    let trade_id = trade.id;

    assert!(book.add_trade(trade));

    p2p_node::trade_state::advance_trade_to_state(&mut book, trade_id, TradeState::Settled, 2)
        .unwrap();

    assert_eq!(book.get_trade(trade_id).unwrap().state, TradeState::Settled);
}

fn market() -> Market {
    Market {
        base: Asset::BTC,
        quote: Asset::XMR,
    }
}

fn buy_order(id: Uuid, trader: &str) -> Order {
    Order {
        id,
        trader: trader.to_string(),
        market: market(),
        side: Side::Buy,
        price: 100 * PRICE_SCALE,
        amount: 2 * AMOUNT_SCALE,
    }
}

fn sell_order(id: Uuid, trader: &str) -> Order {
    Order {
        id,
        trader: trader.to_string(),
        market: market(),
        side: Side::Sell,
        price: 100 * PRICE_SCALE,
        amount: 2 * AMOUNT_SCALE,
    }
}

fn trade(buy: &Order, sell: &Order) -> Trade {
    let trade_input = format!("{}:{}:{}:{}", buy.id, sell.id, buy.price, buy.amount);
    let hash = Sha256::digest(trade_input.as_bytes());
    let id = Uuid::from_slice(&hash[..16]).expect("failed to create trade uuid");

    Trade {
        id,
        buy_order: buy.id,
        sell_order: sell.id,
        maker: sell.trader.clone(),
        taker: buy.trader.clone(),
        buyer: buy.trader.clone(),
        seller: sell.trader.clone(),
        market: market(),
        price: buy.price,
        amount: buy.amount,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    }
}

fn test_paths(prefix: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let id = Uuid::new_v4();

    (
        std::env::temp_dir().join(format!("{prefix}-trade-{id}.json")),
        std::env::temp_dir().join(format!("{prefix}-balance-{id}.json")),
        std::env::temp_dir().join(format!("{prefix}-orders-{id}.json")),
    )
}

fn fund_buyer(balance_book: &mut BalanceBook, buyer: &str) {
    balance_book
        .credit(buyer, Asset::XMR, 300 * AMOUNT_SCALE)
        .unwrap();

    balance_book
        .lock(buyer, Asset::XMR, 200 * AMOUNT_SCALE)
        .unwrap();
}

fn assert_settled_buyer_balance(balance_book: &BalanceBook, buyer: &str) {
    assert_eq!(
        balance_book.balance(buyer, &Asset::BTC),
        Balance {
            available: 2 * AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        balance_book.balance(buyer, &Asset::XMR),
        Balance {
            available: 100 * AMOUNT_SCALE,
            locked: 0,
        }
    );
}

#[test]
fn trade_created_then_settled_updates_local_balances() {
    let (trade_path, balance_path, order_path) = test_paths("opendex-test-handler-trade");

    let mut trade_book = TradeBook::new(trade_path);
    let mut balance_book = BalanceBook::new(balance_path);
    let mut order_book = OrderBook::new(order_path);
    let mut pending_trade_states = HashMap::new();

    let local_peer_id = PeerId::random();
    let local_trader = local_peer_id.to_string();

    let buy = buy_order(Uuid::new_v4(), &local_trader);
    let sell = sell_order(Uuid::new_v4(), "seller");
    let trade = trade(&buy, &sell);

    fund_buyer(&mut balance_book, &local_trader);

    order_book.add_order(buy);
    order_book.add_order(sell);

    handle_trade_created(
        trade.clone(),
        &trade.maker,
        &mut trade_book,
        &mut order_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    assert_eq!(
        trade_book.get_trade(trade.id).unwrap().state,
        TradeState::Created
    );

    let change = TradeStateChanged {
        trade_id: trade.id,
        state: TradeState::Settled,
        timestamp: 2,
    };

    handle_trade_state_changed(
        change.clone(),
        "seller",
        &mut trade_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    assert_eq!(
        trade_book.get_trade(trade.id).unwrap().state,
        TradeState::Settled
    );

    assert_settled_buyer_balance(&balance_book, &local_trader);

    // A duplicate Settled message must not transfer the funds again.
    handle_trade_state_changed(
        change,
        "seller",
        &mut trade_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    assert_settled_buyer_balance(&balance_book, &local_trader);
}

#[test]
fn settled_state_before_trade_created_is_applied_when_trade_arrives() {
    let (trade_path, balance_path, order_path) = test_paths("opendex-test-handler-out-of-order");

    let mut trade_book = TradeBook::new(trade_path);
    let mut balance_book = BalanceBook::new(balance_path);
    let mut order_book = OrderBook::new(order_path);
    let mut pending_trade_states = HashMap::new();

    let local_peer_id = PeerId::random();
    let local_trader = local_peer_id.to_string();

    let buy = buy_order(Uuid::new_v4(), &local_trader);
    let sell = sell_order(Uuid::new_v4(), "seller");
    let trade = trade(&buy, &sell);

    fund_buyer(&mut balance_book, &local_trader);

    let change = TradeStateChanged {
        trade_id: trade.id,
        state: TradeState::Settled,
        timestamp: 2,
    };

    // The state arrives before the trade itself.
    handle_trade_state_changed(
        change,
        "seller",
        &mut trade_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    assert!(pending_trade_states.contains_key(&trade.id));
    assert!(trade_book.get_trade(trade.id).is_none());

    // The trade arrives afterwards and should consume the pending state.
    handle_trade_created(
        trade.clone(),
        &trade.maker,
        &mut trade_book,
        &mut order_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    assert!(!pending_trade_states.contains_key(&trade.id));

    assert_eq!(
        trade_book.get_trade(trade.id).unwrap().state,
        TradeState::Settled
    );

    assert_settled_buyer_balance(&balance_book, &local_trader);

    // Settlement must remain idempotent if the same state is received again.
    let duplicate_change = TradeStateChanged {
        trade_id: trade.id,
        state: TradeState::Settled,
        timestamp: 3,
    };

    handle_trade_state_changed(
        duplicate_change,
        "seller",
        &mut trade_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    assert_settled_buyer_balance(&balance_book, &local_trader);
}
#[test]
fn unauthorized_pending_state_is_rejected_when_trade_arrives() {
    let (trade_path, balance_path, order_path) =
        test_paths("opendex-test-handler-unauthorized-pending");

    let mut trade_book = TradeBook::new(trade_path);
    let mut balance_book = BalanceBook::new(balance_path);
    let mut order_book = OrderBook::new(order_path);
    let mut pending_trade_states = HashMap::new();

    let local_peer_id = PeerId::random();
    let local_trader = local_peer_id.to_string();

    let buy = buy_order(Uuid::new_v4(), &local_trader);
    let sell = sell_order(Uuid::new_v4(), "seller");
    let trade = trade(&buy, &sell);

    fund_buyer(&mut balance_book, &local_trader);

    let change = TradeStateChanged {
        trade_id: trade.id,
        state: TradeState::Settled,
        timestamp: 2,
    };

    // An unrelated peer sends the state before the trade exists locally.
    handle_trade_state_changed(
        change,
        "attacker",
        &mut trade_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    // The current implementation queues the state because the trade
    // does not exist yet.
    assert!(pending_trade_states.contains_key(&trade.id));

    // When the real trade arrives, the pending state must be rejected
    // because "attacker" is neither the buyer nor the seller.
    handle_trade_created(
        trade.clone(),
        &trade.maker,
        &mut trade_book,
        &mut order_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    assert_eq!(
        trade_book.get_trade(trade.id).unwrap().state,
        TradeState::Created
    );

    // The unauthorized pending state must not settle the buyer's funds.
    assert_eq!(
        balance_book.balance(&local_trader, &Asset::BTC),
        Balance {
            available: 0,
            locked: 0,
        }
    );

    assert_eq!(
        balance_book.balance(&local_trader, &Asset::XMR),
        Balance {
            available: 100 * AMOUNT_SCALE,
            locked: 200 * AMOUNT_SCALE,
        }
    );

    assert!(!pending_trade_states.contains_key(&trade.id));
}
#[test]
fn pending_trade_state_keeps_highest_happy_path_state() {
    let (trade_path, balance_path, order_path) = test_paths("opendex-test-handler-pending-highest");

    let mut trade_book = TradeBook::new(trade_path);
    let mut balance_book = BalanceBook::new(balance_path);
    let mut order_book = OrderBook::new(order_path);
    let mut pending_trade_states = HashMap::new();

    let local_peer_id = PeerId::random();
    let local_trader = local_peer_id.to_string();

    let buy = buy_order(Uuid::new_v4(), &local_trader);
    let sell = sell_order(Uuid::new_v4(), "seller");
    let trade = trade(&buy, &sell);

    fund_buyer(&mut balance_book, &local_trader);

    // Receive the later state first.
    handle_trade_state_changed(
        TradeStateChanged {
            trade_id: trade.id,
            state: TradeState::Settled,
            timestamp: 10,
        },
        "seller",
        &mut trade_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    // Then receive an older state.
    handle_trade_state_changed(
        TradeStateChanged {
            trade_id: trade.id,
            state: TradeState::Accepted,
            timestamp: 11,
        },
        "seller",
        &mut trade_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    // The pending state should still be Settled. We verify this through
    // the public handler rather than accessing PendingTradeState internals.
    handle_trade_created(
        trade.clone(),
        &trade.maker,
        &mut trade_book,
        &mut order_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    assert_eq!(
        trade_book.get_trade(trade.id).unwrap().state,
        TradeState::Settled
    );

    assert_settled_buyer_balance(&balance_book, &local_trader);
    assert!(!pending_trade_states.contains_key(&trade.id));
}

#[test]
fn pending_trade_state_advances_through_happy_path() {
    let (trade_path, balance_path, order_path) = test_paths("opendex-test-handler-pending-advance");

    let mut trade_book = TradeBook::new(trade_path);
    let mut balance_book = BalanceBook::new(balance_path);
    let mut order_book = OrderBook::new(order_path);
    let mut pending_trade_states = HashMap::new();

    let local_peer_id = PeerId::random();
    let local_trader = local_peer_id.to_string();

    let buy = buy_order(Uuid::new_v4(), &local_trader);
    let sell = sell_order(Uuid::new_v4(), "seller");
    let trade = trade(&buy, &sell);

    fund_buyer(&mut balance_book, &local_trader);

    // Receive an early state first.
    handle_trade_state_changed(
        TradeStateChanged {
            trade_id: trade.id,
            state: TradeState::Accepted,
            timestamp: 2,
        },
        "seller",
        &mut trade_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    // Then receive a later state while the trade is still unknown.
    handle_trade_state_changed(
        TradeStateChanged {
            trade_id: trade.id,
            state: TradeState::PaymentConfirmed,
            timestamp: 6,
        },
        "seller",
        &mut trade_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    handle_trade_created(
        trade.clone(),
        &trade.maker,
        &mut trade_book,
        &mut order_book,
        &mut pending_trade_states,
        &mut balance_book,
        &local_peer_id,
    )
    .unwrap();

    assert_eq!(
        trade_book.get_trade(trade.id).unwrap().state,
        TradeState::PaymentConfirmed
    );

    // PaymentConfirmed is not Settled, so no settlement should have happened.
    assert_eq!(
        balance_book.balance(&local_trader, &Asset::BTC),
        Balance {
            available: 0,
            locked: 0,
        }
    );

    assert_eq!(
        balance_book.balance(&local_trader, &Asset::XMR),
        Balance {
            available: 100 * AMOUNT_SCALE,
            locked: 200 * AMOUNT_SCALE,
        }
    );

    assert!(!pending_trade_states.contains_key(&trade.id));
}
