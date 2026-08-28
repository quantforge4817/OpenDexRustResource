use p2p_node::balances::*;
use protocol::message::{Asset, Market, Order, Side, Trade, TradeState};

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

#[test]
fn settle_trade_is_local_and_idempotent() {
    let alice_path = std::env::temp_dir().join(format!(
        "opendex-test-settle-alice-{}.json",
        uuid::Uuid::new_v4()
    ));

    let bob_path = std::env::temp_dir().join(format!(
        "opendex-test-settle-bob-{}.json",
        uuid::Uuid::new_v4()
    ));

    let mut alice_book = BalanceBook::new(alice_path);
    let mut bob_book = BalanceBook::new(bob_path);

    let price = 100 * protocol::message::PRICE_SCALE;
    let amount = 2 * protocol::message::AMOUNT_SCALE;

    // Alice is the buyer and locks the quote asset.
    alice_book
        .credit("alice", Asset::XMR, 300 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    alice_book
        .lock("alice", Asset::XMR, 200 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    // Bob is the seller and locks the base asset.
    bob_book
        .credit("bob", Asset::BTC, 3 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    bob_book
        .lock("bob", Asset::BTC, 2 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    let trade = Trade {
        id: uuid::Uuid::new_v4(),
        buy_order: uuid::Uuid::new_v4(),
        sell_order: uuid::Uuid::new_v4(),
        maker: "bob".to_string(),
        taker: "alice".to_string(),
        buyer: "alice".to_string(),
        seller: "bob".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        price,
        amount,
        state: TradeState::Settled,
        created_at: 1,
        updated_at: 1,
    };

    // Alice's node settles only Alice's side.
    alice_book.settle_trade("alice", &trade).unwrap();

    assert_eq!(
        alice_book.balance("alice", &Asset::BTC),
        Balance {
            available: 2 * protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        alice_book.balance("alice", &Asset::XMR),
        Balance {
            available: 100 * protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );

    // Bob's node settles only Bob's side.
    bob_book.settle_trade("bob", &trade).unwrap();

    assert_eq!(
        bob_book.balance("bob", &Asset::BTC),
        Balance {
            available: protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        bob_book.balance("bob", &Asset::XMR),
        Balance {
            available: 200 * protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );

    // Deliver the exact same trade to Alice again.
    alice_book.settle_trade("alice", &trade).unwrap();

    // Alice's balances must not change.
    assert_eq!(
        alice_book.balance("alice", &Asset::BTC),
        Balance {
            available: 2 * protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        alice_book.balance("alice", &Asset::XMR),
        Balance {
            available: 100 * protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(alice_book.settled_trades.len(), 1);
    assert_eq!(alice_book.settled_trades[0], trade.id.to_string());

    // Deliver the exact same trade to Bob again.
    bob_book.settle_trade("bob", &trade).unwrap();

    // Bob's balances must not change.
    assert_eq!(
        bob_book.balance("bob", &Asset::BTC),
        Balance {
            available: protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        bob_book.balance("bob", &Asset::XMR),
        Balance {
            available: 200 * protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(bob_book.settled_trades.len(), 1);
    assert_eq!(bob_book.settled_trades[0], trade.id.to_string());
}
#[test]
fn settle_trade_does_nothing_for_non_participant() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-settle-nonparticipant-{}.json",
        uuid::Uuid::new_v4()
    ));

    let mut book = BalanceBook::new(path);

    book.credit("charlie", Asset::BTC, 10 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    let trade = Trade {
        id: uuid::Uuid::new_v4(),
        buy_order: uuid::Uuid::new_v4(),
        sell_order: uuid::Uuid::new_v4(),
        maker: "bob".to_string(),
        taker: "alice".to_string(),
        buyer: "alice".to_string(),
        seller: "bob".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        price: 100 * protocol::message::PRICE_SCALE,
        amount: 2 * protocol::message::AMOUNT_SCALE,
        state: TradeState::Settled,
        created_at: 1,
        updated_at: 1,
    };

    book.settle_trade("charlie", &trade).unwrap();

    assert_eq!(
        book.balance("charlie", &Asset::BTC),
        Balance {
            available: 10 * protocol::message::AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        book.balance("charlie", &Asset::XMR),
        Balance {
            available: 0,
            locked: 0,
        }
    );

    assert!(book.settled_trades.is_empty());
}
#[test]
fn settle_trade_rejects_quote_overflow_without_mutating_balance() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-settle-overflow-{}.json",
        uuid::Uuid::new_v4()
    ));

    let mut book = BalanceBook::new(path);

    book.credit("alice", Asset::XMR, 100 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    book.lock("alice", Asset::XMR, 100 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    let trade = Trade {
        id: uuid::Uuid::new_v4(),
        buy_order: uuid::Uuid::new_v4(),
        sell_order: uuid::Uuid::new_v4(),
        maker: "bob".to_string(),
        taker: "alice".to_string(),
        buyer: "alice".to_string(),
        seller: "bob".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        price: u64::MAX,
        amount: u64::MAX,
        state: TradeState::Settled,
        created_at: 1,
        updated_at: 1,
    };

    assert!(book.settle_trade("alice", &trade).is_err());

    assert_eq!(
        book.balance("alice", &Asset::XMR),
        Balance {
            available: 0,
            locked: 100 * protocol::message::AMOUNT_SCALE,
        }
    );

    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: 0,
            locked: 0,
        }
    );

    assert!(book.settled_trades.is_empty());
}
#[test]
fn settle_partial_trade_consumes_only_traded_locked_amount() {
    let path = std::env::temp_dir().join(format!(
        "opendex-test-settle-partial-{}.json",
        uuid::Uuid::new_v4()
    ));

    let mut book = BalanceBook::new(path);

    let price = 100 * protocol::message::PRICE_SCALE;

    // Alice has 300 XMR and locks 200 XMR for a 2 BTC order.
    book.credit("alice", Asset::XMR, 300 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    book.lock("alice", Asset::XMR, 200 * protocol::message::AMOUNT_SCALE)
        .unwrap();

    // Only 0.5 BTC of the order trades at 100 XMR/BTC.
    let trade = Trade {
        id: uuid::Uuid::new_v4(),
        buy_order: uuid::Uuid::new_v4(),
        sell_order: uuid::Uuid::new_v4(),
        maker: "bob".to_string(),
        taker: "alice".to_string(),
        buyer: "alice".to_string(),
        seller: "bob".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        price,
        amount: protocol::message::AMOUNT_SCALE / 2, // 0.5 BTC
        state: TradeState::Settled,
        created_at: 1,
        updated_at: 1,
    };

    book.settle_trade("alice", &trade).unwrap();

    // 50 XMR was consumed, not unlocked.
    assert_eq!(
        book.balance("alice", &Asset::XMR),
        Balance {
            available: 100 * protocol::message::AMOUNT_SCALE,
            locked: 150 * protocol::message::AMOUNT_SCALE,
        }
    );

    // Alice receives the 0.5 BTC.
    assert_eq!(
        book.balance("alice", &Asset::BTC),
        Balance {
            available: protocol::message::AMOUNT_SCALE / 2,
            locked: 0,
        }
    );
}
