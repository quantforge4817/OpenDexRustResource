use p2p_node::balances::*;

use protocol::message::{Asset, Market, Order, Side};

#[test]
fn credit_adds_available_balance() {
    let path = std::env::temp_dir().join("opendex-test-balances-credit.json");

    let mut book = BalanceBook::new(path);

    book.credit("alice", Asset::BTC, 10).unwrap();

    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 10,
            locked: 0,
        }
    );
}

#[test]
fn lock_moves_available_to_locked() {
    let path = std::env::temp_dir().join("opendex-test-balances-lock.json");

    let mut book = BalanceBook::new(path);

    book.credit("alice", Asset::BTC, 10).unwrap();
    book.lock("alice", Asset::BTC, 4).unwrap();

    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 6,
            locked: 4,
        }
    );
}

#[test]
fn unlock_moves_locked_back_to_available() {
    let path = std::env::temp_dir().join("opendex-test-balances-unlock.json");

    let mut book = BalanceBook::new(path);

    book.credit("alice", Asset::BTC, 10).unwrap();
    book.lock("alice", Asset::BTC, 4).unwrap();
    book.unlock("alice", Asset::BTC, 3).unwrap();

    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 9,
            locked: 1,
        }
    );
}

#[test]
fn cannot_lock_more_than_available() {
    let path = std::env::temp_dir().join("opendex-test-balances-insufficient.json");

    let mut book = BalanceBook::new(path);

    book.credit("alice", Asset::BTC, 5).unwrap();

    assert!(book.lock("alice", Asset::BTC, 6).is_err());

    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 5,
            locked: 0,
        }
    );
}

#[test]
fn locked_transfer_moves_funds_between_traders() {
    let path = std::env::temp_dir().join("opendex-test-balances-transfer.json");

    let mut book = BalanceBook::new(path);

    book.credit("alice", Asset::BTC, 10).unwrap();
    book.lock("alice", Asset::BTC, 7).unwrap();

    book.transfer_locked("alice", "bob", Asset::BTC, 7).unwrap();

    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 3,
            locked: 0,
        }
    );

    assert_eq!(
        book.balance("bob", &Asset::BTC),
        Balance {
            available: 7,
            locked: 0,
        }
    );
}

#[test]
fn lock_for_order_reserves_correct_asset() {
    let path = std::env::temp_dir().join("opendex-test-lock-for-order.json");

    let mut book = BalanceBook::new(path);

    // Values in an Order are fixed-point scaled.
    // Buy 2 BTC at 100 XMR/BTC => lock 200 XMR.
    let price = 100 * protocol::message::PRICE_SCALE;
    let amount = 2 * protocol::message::AMOUNT_SCALE;

    book.credit("alice", Asset::XMR, 250 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    let buy_order = Order {
        id: uuid::Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Buy,
        price,
        amount,
    };

    book.lock_for_order(&buy_order).unwrap();

    assert_eq!(
        book.balance("alice", &Asset::XMR),
        Balance {
            available: 50 * protocol::message::AMOUNT_SCALE,
            locked: 200 * protocol::message::AMOUNT_SCALE,
        }
    );

    // Sell 3 BTC => lock 3 BTC.
    book.credit("alice", Asset::BTC, 5 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    let sell_order = Order {
        id: uuid::Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Sell,
        price,
        amount: 3 * protocol::message::AMOUNT_SCALE,
    };

    book.lock_for_order(&sell_order).unwrap();

    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 2 * protocol::message::AMOUNT_SCALE,
            locked: 3 * protocol::message::AMOUNT_SCALE,
        }
    );
}
