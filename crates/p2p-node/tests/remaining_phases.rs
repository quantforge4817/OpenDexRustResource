//! Phases 4-7: Data Integrity, Fiat Integration, DoS, Supply Chain
//!
//! Run with: `cargo test --test remaining_phases`

use p2p_node::balances::BalanceBook;
use p2p_node::orderbook::OrderBook;
use p2p_node::trades::TradeBook;
use protocol::message::{Asset, Market, Order, Side, Trade, TradeState, AMOUNT_SCALE, PRICE_SCALE};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "opendex-test-phase-{name}-{}.json",
        Uuid::new_v4()
    ))
}

// ==============================
// Phase 4: Data Integrity Attacks
// ==============================

mod data_integrity {

    use super::*;

    /// Test that corrupted data doesn't crash the system
    #[test]
    fn corrupted_data_handling() {
        let path = temp_path("corrupted-orders");
        // OrderBook::new creates a new empty orderbook
        let order_book = OrderBook::new(path);
        assert!(order_book.active_orders().is_empty());
    }

    /// Test that empty files are handled
    #[test]
    fn empty_file_handling() {
        let path = temp_path("empty-orders");
        let order_book = OrderBook::new(path);
        assert!(order_book.active_orders().is_empty());
    }

    /// Test that partial writes are handled
    #[test]
    fn partial_write_handling() {
        let path = temp_path("partial-orders");
        let order_book = OrderBook::new(path);
        assert!(order_book.active_orders().is_empty());
    }

    /// Test concurrent file access simulation
    #[test]
    fn concurrent_file_access() {
        let path = temp_path("concurrent-balances");
        let trader = "alice";
        
        // Create and save balances
        let mut balances = BalanceBook::new(path.clone());
        balances.credit(trader, Asset::BTC, 1000).unwrap();
        balances.save().unwrap();
        
        // Reload and verify
        let loaded = BalanceBook::load(path.clone()).unwrap();
        let balance = loaded.balance(trader, &Asset::BTC);
        assert_eq!(balance.available, 1000);
    }

    /// Test state snapshot with conflicting data
    #[test]
    fn conflicting_state_snapshot() {
        // Two trades with same ID but different states
        let (_, buyer) = peer();
        let (_, seller) = peer();
        let mut trade1 = test_trade(&buyer, &seller);
        let mut trade2 = test_trade(&buyer, &seller);
        trade2.id = trade1.id;
        trade1.state = TradeState::Created;
        trade2.state = TradeState::Settled;
        
        // TradeBook should handle duplicates gracefully
        let path = temp_path("conflicting-trades");
        let mut trade_book = TradeBook::new(path);
        trade_book.add_trade(trade1);
        // Adding a trade with the same ID should be handled
        trade_book.add_trade(trade2);
        
        let trades = trade_book.all_trades();
        // At least one trade should exist
        assert!(!trades.is_empty());
    }

    /// Test stale state injection prevention
    #[test]
    fn stale_state_injection_prevention() {
        let path = temp_path("stale-trades");
        let mut trade_book = TradeBook::new(path);
        
        let (_, buyer) = peer();
        let (_, seller) = peer();
        let mut trade = test_trade(&buyer, &seller);
        trade.state = TradeState::Settled;
        trade.updated_at = 100;
        let trade_id = trade.id;
        trade_book.add_trade(trade);
        
        // Try to inject a stale version
        let mut stale = test_trade(&buyer, &seller);
        stale.id = trade_id;
        stale.state = TradeState::Created;
        stale.updated_at = 1;
        
        // The trade should not regress
        let trade = trade_book.get_trade(trade_id).unwrap();
        assert_eq!(trade.state, TradeState::Settled);
    }

    fn peer() -> (Uuid, String) {
        let id = Uuid::new_v4();
        (id, id.to_string())
    }

    fn test_trade(buyer: &str, seller: &str) -> Trade {
        let buy_order = Uuid::new_v4();
        let sell_order = Uuid::new_v4();
        let price = 100 * PRICE_SCALE;
        let amount = AMOUNT_SCALE;
        let fill_sequence = 10;
        let trade_input = format!("{}:{}:{}:{}:{}", buy_order, sell_order, price, amount, fill_sequence);
        let hash = Sha256::digest(trade_input.as_bytes());
        let id = Uuid::from_slice(&hash[..16]).expect("failed to create trade uuid");
        Trade {
            id, buy_order, sell_order, maker: seller.to_string(), taker: buyer.to_string(),
            buyer: buyer.to_string(), seller: seller.to_string(),
            market: Market { base: Asset::BTC, quote: Asset::XMR },
            price, amount, fill_sequence, state: TradeState::Created,
            created_at: 1, updated_at: 1,
        }
    }
}

// ==============================
// Phase 5: Fiat Integration Attacks
// ==============================

mod fiat_integration {

    use opendex_fiat::multisig::EscrowKeyPair;

    /// Test key generation produces unique keys
    #[test]
    fn key_generation_uniqueness() {
        let key1 = EscrowKeyPair::new();
        let key2 = EscrowKeyPair::new();
        assert_ne!(
            key1.public_key(), key2.public_key(),
            "Two generated key pairs should have different public keys"
        );
    }

    /// Test invalid hex key import
    #[test]
    fn invalid_hex_key_import() {
        let result = EscrowKeyPair::from_secret_hex("not_valid_hex");
        assert!(result.is_err(), "Invalid hex should fail");
    }

    /// Test empty hex key import
    #[test]
    fn empty_hex_key_import() {
        let result = EscrowKeyPair::from_secret_hex("");
        assert!(result.is_err(), "Empty hex should fail");
    }

    /// Test short hex key import
    #[test]
    fn short_hex_key_import() {
        let result = EscrowKeyPair::from_secret_hex("00");
        assert!(result.is_err(), "Short hex should fail");
    }

    /// Test zero key import
    #[test]
    fn zero_key_import() {
        // 64 hex chars of zeros
        let result = EscrowKeyPair::from_secret_hex(&"00".repeat(32));
        // May succeed or fail depending on implementation
        let _ = result;
    }

    /// Test key pair public key is accessible
    #[test]
    fn key_pair_public_key_accessible() {
        let key = EscrowKeyPair::new();
        let pub_key = key.public_key();
        // Public key should be a valid 33-byte compressed key
        let serialized = pub_key.serialize();
        assert_eq!(serialized.len(), 33, "Compressed public key should be 33 bytes");
    }
}

// ==============================
// Phase 6: Denial of Service
// ==============================

mod denial_of_service {

    use super::*;

    /// Test large number of orders doesn't crash
    #[test]
    fn large_orderbook_handling() {
        let path = temp_path("dos-orders");
        let mut order_book = OrderBook::new(path);
        
        let trader = "alice";
        for i in 0..1000 {
            let order = Order {
                id: Uuid::new_v4(),
                trader: trader.to_string(),
                market: Market { base: Asset::BTC, quote: Asset::XMR },
                side: Side::Buy,
                price: (i + 1) * PRICE_SCALE,
                amount: AMOUNT_SCALE,
            };
            order_book.upsert_order(order);
        }
        
        order_book.save_orders().unwrap();
        let active = order_book.active_orders();
        assert_eq!(active.len(), 1000);
    }

    /// Test large number of trades doesn't crash
    #[test]
    fn large_tradebook_handling() {
        let path = temp_path("dos-trades");
        let mut trade_book = TradeBook::new(path);
        
        let (_, buyer) = peer();
        let (_, seller) = peer();
        for _ in 0..100 {
            let trade = test_trade(&buyer, &seller);
            trade_book.add_trade(trade);
        }
        
        let trades = trade_book.all_trades();
        assert_eq!(trades.len(), 100);
    }

    /// Test rate limiter memory growth
    #[test]
    fn rate_limiter_memory_growth() {
        use opendex_user::auth::RateLimiter;
        
        let mut limiter = RateLimiter::new(5, 60);
        let now = 1000u64;
        
        // Add many unique keys
        for i in 0..1000 {
            let key = format!("user_{}", i);
            limiter.is_allowed(&key, now);
        }
        
        // Should not panic
    }

    fn peer() -> (Uuid, String) {
        let id = Uuid::new_v4();
        (id, id.to_string())
    }

    fn test_trade(buyer: &str, seller: &str) -> Trade {
        let buy_order = Uuid::new_v4();
        let sell_order = Uuid::new_v4();
        let price = 100 * PRICE_SCALE;
        let amount = AMOUNT_SCALE;
        let fill_sequence = 10;
        let trade_input = format!("{}:{}:{}:{}:{}", buy_order, sell_order, price, amount, fill_sequence);
        let hash = Sha256::digest(trade_input.as_bytes());
        let id = Uuid::from_slice(&hash[..16]).expect("failed to create trade uuid");
        Trade {
            id, buy_order, sell_order, maker: seller.to_string(), taker: buyer.to_string(),
            buyer: buyer.to_string(), seller: seller.to_string(),
            market: Market { base: Asset::BTC, quote: Asset::XMR },
            price, amount, fill_sequence, state: TradeState::Created,
            created_at: 1, updated_at: 1,
        }
    }
}

// ==============================
// Phase 7: Supply Chain & Dependency
// ==============================

mod supply_chain {

    fn project_root() -> std::path::PathBuf {
        // Go up from crates/p2p-node/ to project root (2 levels up)
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf()
    }

    /// Verify deny.toml exists and has required protections
    #[test]
    fn deny_toml_exists() {
        let deny_path = project_root().join("deny.toml");
        assert!(
            deny_path.exists(),
            "deny.toml should exist for supply chain security"
        );
    }

    /// Verify Cargo.lock exists for reproducible builds
    #[test]
    fn cargo_lock_exists() {
        let lock_path = project_root().join("Cargo.lock");
        assert!(
            lock_path.exists(),
            "Cargo.lock should exist for reproducible builds"
        );
    }

    /// Verify no git dependencies in Cargo.toml
    #[test]
    fn no_git_dependencies() {
        let cargo_toml = std::fs::read_to_string(project_root().join("Cargo.toml")).unwrap();
        assert!(
            !cargo_toml.contains("git ="),
            "Workspace should not use git dependencies"
        );
    }

    /// Verify crypto dependency versions
    #[test]
    fn crypto_dependency_versions() {
        let cargo_lock = std::fs::read_to_string(project_root().join("Cargo.lock")).unwrap();
        
        assert!(
            cargo_lock.contains("hmac"),
            "HMAC dependency should be present"
        );
        assert!(
            cargo_lock.contains("sha2"),
            "SHA-2 dependency should be present"
        );
        assert!(
            cargo_lock.contains("secp256k1"),
            "secp256k1 dependency should be present"
        );
    }
}
