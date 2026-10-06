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
            escrowed: 0,
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
            escrowed: 0,
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
            escrowed: 0,
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
            escrowed: 0,
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
            escrowed: 0,
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
            escrowed: 0,
        }
    );
}
#[test]
fn buy_order_reservation_tracks_order_id() {
    let path = std::env::temp_dir().join("opendex-test-buy-reservation.json");

    let mut book = BalanceBook::new(path);

    let order = Order {
        id: uuid::Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Buy,
        price: 100 * protocol::message::PRICE_SCALE,
        amount: 2 * protocol::message::AMOUNT_SCALE,
    };

    book.credit("alice", Asset::XMR, 250 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    book.lock_for_order(&order).unwrap();

    assert_eq!(
        book.order_reservation(&order.id),
        Some(200 * protocol::message::AMOUNT_SCALE)
    );

    assert_eq!(
        book.balance("alice", &Asset::XMR),
        Balance {
            available: 50 * protocol::message::AMOUNT_SCALE,
            locked: 200 * protocol::message::AMOUNT_SCALE,
            escrowed: 0,
        }
    );
}

#[test]
fn cancelling_buy_order_releases_remaining_reservation() {
    let path = std::env::temp_dir().join("opendex-test-buy-cancel-reservation.json");

    let mut book = BalanceBook::new(path);

    let order = Order {
        id: uuid::Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Buy,
        price: 100 * protocol::message::PRICE_SCALE,
        amount: 2 * protocol::message::AMOUNT_SCALE,
    };

    book.credit("alice", Asset::XMR, 250 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    book.lock_for_order(&order).unwrap();
    book.unlock_for_order(&order).unwrap();

    assert_eq!(
        book.balance("alice", &Asset::XMR),
        Balance {
            available: 250 * protocol::message::AMOUNT_SCALE,
            locked: 0,
            escrowed: 0,
        }
    );
}

#[test]
fn duplicate_order_reservation_does_not_double_lock() {
    let path = std::env::temp_dir().join("opendex-test-duplicate-reservation.json");

    let mut book = BalanceBook::new(path);

    let order = Order {
        id: uuid::Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Buy,
        price: 100 * protocol::message::PRICE_SCALE,
        amount: 2 * protocol::message::AMOUNT_SCALE,
    };

    book.credit("alice", Asset::XMR, 500 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    book.lock_for_order(&order).unwrap();

    assert!(book.lock_for_order(&order).is_err());

    assert_eq!(
        book.balance("alice", &Asset::XMR),
        Balance {
            available: 300 * protocol::message::AMOUNT_SCALE,
            locked: 200 * protocol::message::AMOUNT_SCALE,
            escrowed: 0,
        }
    );
}
#[test]
fn partial_buy_fill_releases_remaining_quote_reservation() {
    let path = std::env::temp_dir().join("opendex-test-partial-buy-reconciliation.json");

    let mut book = BalanceBook::new(path);

    let order = Order {
        id: uuid::Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Buy,
        price: 10 * protocol::message::PRICE_SCALE,
        amount: 10 * protocol::message::AMOUNT_SCALE,
    };

    book.credit("alice", Asset::XMR, 100 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    book.lock_for_order(&order).unwrap();

    assert_eq!(
        book.order_reservation(&order.id),
        Some(100 * protocol::message::AMOUNT_SCALE)
    );

    // Fill 5 BTC. The remaining order requires 50 XMR.
    let remaining_order = Order {
        amount: 5 * protocol::message::AMOUNT_SCALE,
        ..order.clone()
    };

    let remaining_required = book.required_reservation(&remaining_order).unwrap();

    book.reconcile_order_reservation(&order, remaining_required)
        .unwrap();

    assert_eq!(
        book.order_reservation(&order.id),
        Some(50 * protocol::message::AMOUNT_SCALE)
    );

    // Reconciliation only updates tracking; locked funds remain locked
    // until settlement/escrow/cancel consumes them.
    assert_eq!(
        book.balance("alice", &Asset::XMR),
        Balance {
            available: 0,
            locked: 100 * protocol::message::AMOUNT_SCALE,
            escrowed: 0,
        }
    );

    // Fill the remaining 5 BTC.
    book.reconcile_order_reservation(&order, 0).unwrap();

    assert_eq!(book.order_reservation(&order.id), None);

    // Locked funds still remain; they are released by settlement/cancel.
    assert_eq!(
        book.balance("alice", &Asset::XMR),
        Balance {
            available: 0,
            locked: 100 * protocol::message::AMOUNT_SCALE,
            escrowed: 0,
        }
    );
}

#[test]
fn partial_sell_fill_releases_remaining_base_reservation() {
    let path = std::env::temp_dir().join("opendex-test-partial-sell-reconciliation.json");

    let mut book = BalanceBook::new(path);

    let order = Order {
        id: uuid::Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Sell,
        price: 10 * protocol::message::PRICE_SCALE,
        amount: 10 * protocol::message::AMOUNT_SCALE,
    };

    book.credit("alice", Asset::BTC, 10 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    book.lock_for_order(&order).unwrap();

    assert_eq!(
        book.order_reservation(&order.id),
        Some(10 * protocol::message::AMOUNT_SCALE)
    );

    // Fill 5 BTC. 5 BTC remains reserved.
    book.reconcile_order_reservation(&order, 5 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    assert_eq!(
        book.order_reservation(&order.id),
        Some(5 * protocol::message::AMOUNT_SCALE)
    );

    // Reconciliation only updates tracking; locked funds remain locked.
    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 0,
            locked: 10 * protocol::message::AMOUNT_SCALE,
            escrowed: 0,
        }
    );

    // Fill the remaining 5 BTC.
    book.reconcile_order_reservation(&order, 0).unwrap();

    assert_eq!(book.order_reservation(&order.id), None);

    // Locked funds still remain; released by settlement/cancel.
    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 0,
            locked: 10 * protocol::message::AMOUNT_SCALE,
            escrowed: 0,
        }
    );
}

#[test]
fn multiple_partial_fills_progressively_reduce_reservation() {
    let path = std::env::temp_dir().join("opendex-test-multiple-reconciliation.json");

    let mut book = BalanceBook::new(path);

    let order = Order {
        id: uuid::Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Sell,
        price: 10 * protocol::message::PRICE_SCALE,
        amount: 10 * protocol::message::AMOUNT_SCALE,
    };

    book.credit("alice", Asset::BTC, 10 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    book.lock_for_order(&order).unwrap();

    // 10 BTC → 7 BTC.
    book.reconcile_order_reservation(&order, 7 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    assert_eq!(
        book.order_reservation(&order.id),
        Some(7 * protocol::message::AMOUNT_SCALE)
    );

    // 7 BTC → 5 BTC.
    book.reconcile_order_reservation(&order, 5 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    assert_eq!(
        book.order_reservation(&order.id),
        Some(5 * protocol::message::AMOUNT_SCALE)
    );

    // 5 BTC → 0 BTC.
    book.reconcile_order_reservation(&order, 0).unwrap();

    assert_eq!(book.order_reservation(&order.id), None);

    // Locked funds still remain; released by settlement/cancel.
    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 0,
            locked: 10 * protocol::message::AMOUNT_SCALE,
            escrowed: 0,
        }
    );
}

#[test]
fn full_fill_removes_reservation_entirely() {
    let path = std::env::temp_dir().join("opendex-test-full-fill-reconciliation.json");

    let mut book = BalanceBook::new(path);

    let order = Order {
        id: uuid::Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Buy,
        price: 10 * protocol::message::PRICE_SCALE,
        amount: 10 * protocol::message::AMOUNT_SCALE,
    };

    book.credit("alice", Asset::XMR, 100 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    book.lock_for_order(&order).unwrap();

    book.reconcile_order_reservation(&order, 0).unwrap();

    assert_eq!(book.order_reservation(&order.id), None);

    // Locked funds still remain; released by settlement/cancel.
    assert_eq!(
        book.balance("alice", &Asset::XMR),
        Balance {
            available: 0,
            locked: 100 * protocol::message::AMOUNT_SCALE,
            escrowed: 0,
        }
    );
}

#[test]
fn partial_fill_then_cancel_releases_only_remaining_reservation() {
    let path = std::env::temp_dir().join("opendex-test-partial-fill-cancel.json");

    let mut book = BalanceBook::new(path);

    let order = Order {
        id: uuid::Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Sell,
        price: 10 * protocol::message::PRICE_SCALE,
        amount: 10 * protocol::message::AMOUNT_SCALE,
    };

    book.credit("alice", Asset::BTC, 10 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    book.lock_for_order(&order).unwrap();

    // Trade 4 BTC. Reconciliation only updates tracking; locked funds
    // remain locked until settlement/escrow/cancel consumes them.
    book.reconcile_order_reservation(&order, 6 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    assert_eq!(
        book.order_reservation(&order.id),
        Some(6 * protocol::message::AMOUNT_SCALE)
    );

    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 0,
            locked: 10 * protocol::message::AMOUNT_SCALE,
            escrowed: 0,
        }
    );

    // Cancel the partially filled order. Unlock releases only the
    // remaining reservation (6 BTC). The 4 BTC consumed by settlement
    // remains locked until explicitly consumed.
    book.unlock_for_order(&order).unwrap();

    assert_eq!(book.order_reservation(&order.id), None);

    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 6 * protocol::message::AMOUNT_SCALE,
            locked: 4 * protocol::message::AMOUNT_SCALE,
            escrowed: 0,
        }
    );
}
