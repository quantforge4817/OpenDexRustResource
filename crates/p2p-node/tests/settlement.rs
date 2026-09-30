use libp2p::{PeerId, identity};
use p2p_node::balances::{Balance, BalanceBook};
use p2p_node::settlement::settle_local_trade;
use protocol::message::{AMOUNT_SCALE, Asset, Market, PRICE_SCALE, Trade, TradeState};

use std::path::PathBuf;
use uuid::Uuid;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("opendex-test-{name}-{}.json", Uuid::new_v4()))
}

fn peer() -> (PeerId, String) {
    let key = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(key.public());
    let trader = peer_id.to_string();

    (peer_id, trader)
}

fn test_trade(buyer: &str, seller: &str, price: u64, amount: u64, fill_sequence: u64) -> Trade {
    Trade {
        id: Uuid::new_v4(),
        buy_order: Uuid::new_v4(),
        sell_order: Uuid::new_v4(),
        maker: seller.to_string(),
        taker: buyer.to_string(),
        buyer: buyer.to_string(),
        seller: seller.to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        price,
        amount,
        fill_sequence,
        state: TradeState::Settled,
        created_at: 1,
        updated_at: 1,
    }
}

#[test]
fn settles_local_buyer_and_is_idempotent() {
    let (alice_peer, alice) = peer();
    let (_, bob) = peer();

    let mut book = BalanceBook::new(temp_path("settlement-buyer"));

    book.credit(&alice, Asset::XMR, 300 * AMOUNT_SCALE).unwrap();

    book.lock(&alice, Asset::XMR, 200 * AMOUNT_SCALE).unwrap();

    let trade = test_trade(&alice, &bob, 100 * PRICE_SCALE, 2 * AMOUNT_SCALE, 10);

    settle_local_trade(&mut book, &alice_peer, &trade).unwrap();

    assert_eq!(
        book.balance(&alice, &Asset::BTC),
        Balance {
            available: 2 * AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        book.balance(&alice, &Asset::XMR),
        Balance {
            available: 100 * AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(book.settled_trades.len(), 1);

    // The same settlement must not move funds twice.
    settle_local_trade(&mut book, &alice_peer, &trade).unwrap();

    assert_eq!(
        book.balance(&alice, &Asset::BTC),
        Balance {
            available: 2 * AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        book.balance(&alice, &Asset::XMR),
        Balance {
            available: 100 * AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(book.settled_trades.len(), 1);
}

#[test]
fn settles_local_seller_and_is_idempotent() {
    let (bob_peer, bob) = peer();
    let (_, alice) = peer();

    let mut book = BalanceBook::new(temp_path("settlement-seller"));

    book.credit(&bob, Asset::BTC, 3 * AMOUNT_SCALE).unwrap();

    book.lock(&bob, Asset::BTC, 2 * AMOUNT_SCALE).unwrap();

    let trade = test_trade(&alice, &bob, 100 * PRICE_SCALE, 2 * AMOUNT_SCALE, 10);

    settle_local_trade(&mut book, &bob_peer, &trade).unwrap();

    assert_eq!(
        book.balance(&bob, &Asset::BTC),
        Balance {
            available: AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        book.balance(&bob, &Asset::XMR),
        Balance {
            available: 200 * AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(book.settled_trades.len(), 1);

    settle_local_trade(&mut book, &bob_peer, &trade).unwrap();

    assert_eq!(
        book.balance(&bob, &Asset::BTC),
        Balance {
            available: AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        book.balance(&bob, &Asset::XMR),
        Balance {
            available: 200 * AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(book.settled_trades.len(), 1);
}

#[test]
fn ignores_non_participant() {
    let (charlie_peer, charlie) = peer();
    let (_, alice) = peer();
    let (_, bob) = peer();

    let mut book = BalanceBook::new(temp_path("settlement-nonparticipant"));

    book.credit(&charlie, Asset::BTC, 10 * AMOUNT_SCALE)
        .unwrap();

    let trade = test_trade(&alice, &bob, 100 * PRICE_SCALE, 2 * AMOUNT_SCALE, 10);

    settle_local_trade(&mut book, &charlie_peer, &trade).unwrap();

    assert_eq!(
        book.balance(&charlie, &Asset::BTC),
        Balance {
            available: 10 * AMOUNT_SCALE,
            locked: 0,
        }
    );

    assert_eq!(
        book.balance(&charlie, &Asset::XMR),
        Balance {
            available: 0,
            locked: 0,
        }
    );

    assert!(book.settled_trades.is_empty());
}

#[test]
fn rejects_quote_overflow_without_mutating_balance() {
    let (alice_peer, alice) = peer();
    let (_, bob) = peer();

    let mut book = BalanceBook::new(temp_path("settlement-overflow"));

    book.credit(&alice, Asset::XMR, 100 * AMOUNT_SCALE).unwrap();

    book.lock(&alice, Asset::XMR, 100 * AMOUNT_SCALE).unwrap();

    let trade = test_trade(&alice, &bob, u64::MAX, u64::MAX, 10);

    assert!(settle_local_trade(&mut book, &alice_peer, &trade).is_err());

    assert_eq!(
        book.balance(&alice, &Asset::XMR),
        Balance {
            available: 0,
            locked: 100 * AMOUNT_SCALE,
        }
    );

    assert_eq!(
        book.balance(&alice, &Asset::BTC),
        Balance {
            available: 0,
            locked: 0,
        }
    );

    assert!(book.settled_trades.is_empty());
}

#[test]
fn partial_trade_consumes_only_traded_locked_amount() {
    let (alice_peer, alice) = peer();
    let (_, bob) = peer();

    let mut book = BalanceBook::new(temp_path("settlement-partial"));

    book.credit(&alice, Asset::XMR, 300 * AMOUNT_SCALE).unwrap();

    book.lock(&alice, Asset::XMR, 200 * AMOUNT_SCALE).unwrap();

    let trade = test_trade(&alice, &bob, 100 * PRICE_SCALE, AMOUNT_SCALE / 2, 10);

    settle_local_trade(&mut book, &alice_peer, &trade).unwrap();

    // 0.5 BTC traded for 50 XMR.
    assert_eq!(
        book.balance(&alice, &Asset::BTC),
        Balance {
            available: AMOUNT_SCALE / 2,
            locked: 0,
        }
    );

    assert_eq!(
        book.balance(&alice, &Asset::XMR),
        Balance {
            available: 100 * AMOUNT_SCALE,
            locked: 150 * AMOUNT_SCALE,
        }
    );
}
#[test]
fn settlement_failure_does_not_partially_mutate_balances() {
    let (alice_peer, alice) = peer();
    let (_, bob) = peer();

    let mut book = BalanceBook::new(temp_path("settlement-atomicity"));

    // Give Alice exactly enough locked XMR for the trade.
    book.credit(&alice, Asset::XMR, 100 * AMOUNT_SCALE).unwrap();
    book.lock(&alice, Asset::XMR, 100 * AMOUNT_SCALE).unwrap();

    // Force the BTC credit to overflow.
    book.credit(&alice, Asset::BTC, u64::MAX).unwrap();

    let trade = test_trade(&alice, &bob, 100 * PRICE_SCALE, AMOUNT_SCALE, 10);

    assert!(settle_local_trade(&mut book, &alice_peer, &trade).is_err());

    // XMR must still be locked because settlement failed.
    assert_eq!(
        book.balance(&alice, &Asset::XMR),
        Balance {
            available: 0,
            locked: 100 * AMOUNT_SCALE,
        }
    );

    // BTC must be completely unchanged.
    assert_eq!(
        book.balance(&alice, &Asset::BTC),
        Balance {
            available: u64::MAX,
            locked: 0,
        }
    );

    // Failed settlement must not become permanently marked as settled.
    assert!(book.settled_trades.is_empty());
}
#[test]
fn insufficient_locked_base_rejects_settlement_without_mutation() {
    let (bob_peer, bob) = peer();
    let (_, alice) = peer();

    let mut book = BalanceBook::new(temp_path("settlement-insufficient-locked-base"));

    // Bob has enough total BTC, but not enough locked BTC for this trade.
    book.credit(&bob, Asset::BTC, 3 * AMOUNT_SCALE).unwrap();
    book.lock(&bob, Asset::BTC, AMOUNT_SCALE).unwrap();

    let trade = test_trade(&alice, &bob, 100 * PRICE_SCALE, 2 * AMOUNT_SCALE, 10);

    // The trade requires 2 BTC, but only 1 BTC is locked.
    assert!(settle_local_trade(&mut book, &bob_peer, &trade).is_err());

    // Neither available nor locked BTC may change.
    assert_eq!(
        book.balance(&bob, &Asset::BTC),
        Balance {
            available: 2 * AMOUNT_SCALE,
            locked: AMOUNT_SCALE,
        }
    );

    // No XMR may be created by a failed settlement.
    assert_eq!(
        book.balance(&bob, &Asset::XMR),
        Balance {
            available: 0,
            locked: 0,
        }
    );

    // The failed settlement must remain retryable.
    assert!(book.settled_trades.is_empty());
}

#[test]
fn insufficient_locked_quote_rejects_settlement_without_mutation() {
    let (alice_peer, alice) = peer();
    let (_, bob) = peer();

    let mut book = BalanceBook::new(temp_path("settlement-insufficient-locked-quote"));

    // Alice has enough total XMR, but not enough locked XMR for this trade.
    book.credit(&alice, Asset::XMR, 200 * AMOUNT_SCALE).unwrap();
    book.lock(&alice, Asset::XMR, 50 * AMOUNT_SCALE).unwrap();

    let trade = test_trade(&alice, &bob, 100 * PRICE_SCALE, 2 * AMOUNT_SCALE, 10);

    // The trade requires 200 XMR, but only 50 XMR is locked.
    assert!(settle_local_trade(&mut book, &alice_peer, &trade).is_err());

    // Available funds must not be touched.
    assert_eq!(
        book.balance(&alice, &Asset::XMR),
        Balance {
            available: 150 * AMOUNT_SCALE,
            locked: 50 * AMOUNT_SCALE,
        }
    );

    // No BTC may be created by a failed settlement.
    assert_eq!(
        book.balance(&alice, &Asset::BTC),
        Balance {
            available: 0,
            locked: 0,
        }
    );

    // The failed settlement must remain retryable.
    assert!(book.settled_trades.is_empty());
}

#[test]
fn settlement_accepts_u128_intermediate_product_that_fits_after_scaling() {
    let (alice_peer, alice) = peer();
    let (_, bob) = peer();

    let mut book = BalanceBook::new(temp_path("settlement-u128-intermediate"));

    let price = u64::MAX;
    let amount = 2;

    // The multiplication itself cannot fit in u64.
    let product = u128::from(price) * u128::from(amount);
    assert!(product > u128::from(u64::MAX));

    // But after applying PRICE_SCALE, the final quote fits in u64.
    let expected_quote = u64::try_from(product / u128::from(PRICE_SCALE)).unwrap();

    book.credit(&alice, Asset::XMR, expected_quote).unwrap();
    book.lock(&alice, Asset::XMR, expected_quote).unwrap();

    let trade = test_trade(&alice, &bob, price, amount, 10);

    settle_local_trade(&mut book, &alice_peer, &trade).unwrap();

    assert_eq!(
        book.balance(&alice, &Asset::XMR),
        Balance {
            available: 0,
            locked: 0,
        }
    );

    assert_eq!(
        book.balance(&alice, &Asset::BTC),
        Balance {
            available: amount,
            locked: 0,
        }
    );

    assert_eq!(book.settled_trades.len(), 1);
}
#[test]
fn settlement_save_failure_does_not_mutate_in_memory_state() {
    let (alice_peer, alice) = peer();
    let (_, bob) = peer();

    let temp_dir = std::env::temp_dir().join("opendex-settlement-save-failure");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let mut book = BalanceBook::new(temp_dir.clone());

    let quote = 100 * AMOUNT_SCALE;
    book.credit(&alice, Asset::XMR, quote).unwrap();
    book.lock(&alice, Asset::XMR, quote).unwrap();

    let trade = test_trade(&alice, &bob, 100 * PRICE_SCALE, AMOUNT_SCALE, 10);

    let result = settle_local_trade(&mut book, &alice_peer, &trade);

    assert!(result.is_err());

    assert_eq!(
        book.balance(&alice, &Asset::XMR),
        Balance {
            available: 0,
            locked: quote,
        }
    );

    assert_eq!(
        book.balance(&alice, &Asset::BTC),
        Balance {
            available: 0,
            locked: 0,
        }
    );

    assert!(book.settled_trades.is_empty());

    std::fs::remove_dir_all(temp_dir).unwrap();
}
#[test]
fn settlement_planning_does_not_mutate_balance_book() {
    let (alice_peer, alice) = peer();
    let (_, bob) = peer();

    let mut book = BalanceBook::new(temp_path("settlement-plan-pure"));

    let quote = 100 * AMOUNT_SCALE;

    book.credit(&alice, Asset::XMR, quote).unwrap();
    book.lock(&alice, Asset::XMR, quote).unwrap();

    let before = book.clone();

    let trade = test_trade(&alice, &bob, 100 * PRICE_SCALE, AMOUNT_SCALE, 10);

    let plan = p2p_node::settlement::plan_local_settlement(&book, &alice_peer, &trade)
        .unwrap()
        .expect("buyer should receive a settlement plan");

    assert_eq!(plan.debit_asset, Asset::XMR);
    assert_eq!(plan.debit_amount, quote);
    assert_eq!(plan.credit_asset, Asset::BTC);
    assert_eq!(plan.credit_amount, AMOUNT_SCALE);

    assert_eq!(
        book.balance(&alice, &Asset::XMR),
        before.balance(&alice, &Asset::XMR)
    );

    assert_eq!(
        book.balance(&alice, &Asset::BTC),
        before.balance(&alice, &Asset::BTC)
    );

    assert_eq!(book.settled_trades, before.settled_trades);
}
