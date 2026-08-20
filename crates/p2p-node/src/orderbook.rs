use anyhow::Result;
use protocol::message::{Order, Side, Trade, TradeState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub orders: Vec<Order>,
    pub applied_trades: Vec<String>,

    #[serde(default)]
    pub pending_trades: Vec<Trade>,

    #[serde(default)]
    pub removed_orders: Vec<Uuid>,

    #[serde(skip)]
    path: PathBuf,
}

impl OrderBook {
    pub fn new(path: PathBuf) -> Self {
        Self {
            orders: Vec::new(),
            applied_trades: Vec::new(),
            removed_orders: Vec::new(),
            pending_trades: Vec::new(),
            path,
        }
    }

    pub fn load_orders(path: PathBuf) -> Result<Self> {
        if !Path::new(&path).exists() {
            return Ok(Self::new(path));
        }

        let data = fs::read_to_string(&path)?;
        let mut order_book: OrderBook = serde_json::from_str(&data)?;

        order_book.path = path;

        order_book.orders.dedup_by(|a, b| a.id == b.id);

        order_book.save_orders()?;

        Ok(order_book)
    }

    pub fn save_orders(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&self.path, data)?;

        Ok(())
    }

    pub fn add_order(&mut self, order: Order) -> Vec<Trade> {
        self.upsert_order(order)
    }

    pub fn upsert_order(&mut self, order: Order) -> Vec<Trade> {
        let mut applied_pending_trades = Vec::new();

        if self.removed_orders.contains(&order.id) {
            return applied_pending_trades;
        }

        let order_id = order.id;

        if let Some(index) = self
            .orders
            .iter()
            .position(|existing| existing.id == order_id)
        {
            println!(
                "Updating order {} amount: {} -> {}",
                order_id, self.orders[index].amount, order.amount
            );

            self.orders[index] = order;
        } else {
            println!(
                "Adding new order {} with remaining amount {}",
                order_id, order.amount
            );

            self.orders.push(order);
        }

        // Re-check pending trades involving this order.
        let pending_trades: Vec<Trade> = self
            .pending_trades
            .iter()
            .filter(|trade| trade.buy_order == order_id || trade.sell_order == order_id)
            .cloned()
            .collect();

        for trade in pending_trades {
            if self.apply_trade(&trade) {
                self.pending_trades.retain(|t| t.id != trade.id);
                applied_pending_trades.push(trade);
            }
        }

        applied_pending_trades
    }
    pub fn update_order_from_snapshot(&mut self, order: Order) {
        if self.removed_orders.contains(&order.id) {
            println!("Ignoring removed order during snapshot sync: {}", order.id);
            return;
        }

        self.upsert_order(order);

        // Snapshot orders may satisfy a trade that arrived before
        // the corresponding order.
        //
        // upsert_order() already re-checks pending trades.
    }

    pub fn remove_order(&mut self, id: Uuid) -> bool {
        let before = self.orders.len();

        self.orders.retain(|order| order.id != id);

        if !self.removed_orders.contains(&id) {
            self.removed_orders.push(id);
        }

        before != self.orders.len()
    }

    pub fn mark_trade_applied(&mut self, trade_id: &str) {
        if !self.applied_trades.iter().any(|id| id == trade_id) {
            self.applied_trades.push(trade_id.to_string());
        }
    }

    pub fn apply_trade(&mut self, trade: &Trade) -> bool {
        let trade_id = trade.id.to_string();

        if self.applied_trades.contains(&trade_id) {
            return false;
        }

        let has_buy_order = self.orders.iter().any(|o| o.id == trade.buy_order);
        let has_sell_order = self.orders.iter().any(|o| o.id == trade.sell_order);

        if !has_buy_order || !has_sell_order {
            if !self.pending_trades.iter().any(|t| t.id == trade.id) {
                self.pending_trades.push(trade.clone());
            }

            return false;
        }

        self.applied_trades.push(trade.id.to_string());

        let mut fully_filled = Vec::new();

        self.orders.retain_mut(|order| {
            if order.id == trade.buy_order || order.id == trade.sell_order {
                if order.amount > trade.amount {
                    order.amount -= trade.amount;
                    true
                } else {
                    fully_filled.push(order.id);
                    false
                }
            } else {
                true
            }
        });

        for id in fully_filled {
            if !self.removed_orders.contains(&id) {
                self.removed_orders.push(id);
            }
        }

        true
    }

    pub fn print_orders(&self) {
        println!("=== OpenDEX OrderBook ===");

        for order in &self.orders {
            println!("{:?}", order);
        }
    }

    pub fn match_order(&mut self, incoming: &Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut remaining_amount = incoming.amount;

        while remaining_amount > 0 {
            let best_index = self
                .orders
                .iter()
                .enumerate()
                .filter(|(_, existing)| {
                    existing.market.base == incoming.market.base
                        && existing.market.quote == incoming.market.quote
                        && existing.side != incoming.side
                        && match incoming.side {
                            Side::Buy => existing.price <= incoming.price,
                            Side::Sell => existing.price >= incoming.price,
                        }
                })
                .min_by(|(_, a), (_, b)| {
                    let price_cmp = match incoming.side {
                        Side::Buy => a.price.cmp(&b.price),
                        Side::Sell => b.price.cmp(&a.price),
                    };

                    if price_cmp == std::cmp::Ordering::Equal {
                        a.id.cmp(&b.id)
                    } else {
                        price_cmp
                    }
                })
                .map(|(index, _)| index);

            let Some(index) = best_index else {
                break;
            };

            let existing = self.orders[index].clone();

            let buy_order = if matches!(incoming.side, Side::Buy) {
                incoming.id
            } else {
                existing.id
            };

            let sell_order = if matches!(incoming.side, Side::Sell) {
                incoming.id
            } else {
                existing.id
            };

            let amount = remaining_amount.min(existing.amount);
            let trade_price = existing.price;

            let trade_input = format!("{}:{}:{}:{}", buy_order, sell_order, trade_price, amount);

            let hash = Sha256::digest(trade_input.as_bytes());

            let trade_id = Uuid::from_slice(&hash[..16]).expect("failed to create trade uuid");

            let now = chrono::Utc::now().timestamp() as u64;

            // The resting order is the maker.
            // The incoming order is the taker.
            let maker = &existing.trader;
            let taker = &incoming.trader;

            // Determine the actual buyer and seller.
            // This must happen here because both orders are available
            // before match_order() potentially removes a fully-filled order.
            let buyer = if matches!(incoming.side, Side::Buy) {
                incoming.trader.clone()
            } else {
                existing.trader.clone()
            };

            let seller = if matches!(incoming.side, Side::Sell) {
                incoming.trader.clone()
            } else {
                existing.trader.clone()
            };

            let trade = Trade {
                id: trade_id,
                buy_order,
                sell_order,
                maker: maker.clone(),
                taker: taker.clone(),
                buyer,
                seller,
                market: incoming.market.clone(),
                price: trade_price,
                amount,
                state: TradeState::Created,
                created_at: now,
                updated_at: now,
            };

            println!("Match found:");
            println!("{:?}", trade);

            trades.push(trade);

            if existing.amount > amount {
                self.orders[index].amount -= amount;
            } else {
                let removed = self.orders.remove(index);

                if !self.removed_orders.contains(&removed.id) {
                    self.removed_orders.push(removed.id);
                }
            }

            remaining_amount -= amount;
        }

        if remaining_amount > 0 {
            let mut remaining_order = incoming.clone();
            remaining_order.amount = remaining_amount;
            self.add_order(remaining_order);
        }

        trades
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::message::{Asset, Market, TradeState};
    fn market() -> Market {
        Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        }
    }

    fn order(id: &str, side: Side, price: u64, amount: u64) -> Order {
        Order {
            id: Uuid::parse_str(id).unwrap(),
            trader: "test-trader".to_string(),
            market: market(),
            side,
            price,
            amount,
        }
    }

    #[test]
    fn buy_matches_cheapest_sell_first() {
        let path = std::env::temp_dir().join("opendex-test-buy.json");

        let mut book = OrderBook::new(path);

        let expensive_sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

        let cheap_sell = order("00000000-0000-0000-0000-000000000001", Side::Sell, 90, 10);

        // Deliberately add the expensive order first.
        book.add_order(expensive_sell);
        book.add_order(cheap_sell);

        let incoming = order("00000000-0000-0000-0000-000000000003", Side::Buy, 100, 10);

        let trades = book.match_order(&incoming);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, 90);
        assert_eq!(
            trades[0].sell_order,
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
        );
    }

    #[test]
    fn sell_matches_highest_buy_first() {
        let path = std::env::temp_dir().join("opendex-test-sell.json");

        let mut book = OrderBook::new(path);

        let cheap_buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 90, 10);

        let expensive_buy = order("00000000-0000-0000-0000-000000000002", Side::Buy, 100, 10);

        // Deliberately add the cheap order first.
        book.add_order(cheap_buy);
        book.add_order(expensive_buy);

        let incoming = order("00000000-0000-0000-0000-000000000003", Side::Sell, 90, 10);

        let trades = book.match_order(&incoming);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, 100);
        assert_eq!(
            trades[0].buy_order,
            Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
        );
    }

    #[test]
    fn partial_fill_leaves_remaining_order() {
        let path = std::env::temp_dir().join("opendex-test-partial.json");

        let mut book = OrderBook::new(path);

        let sell = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 5);

        book.add_order(sell);

        let incoming = order("00000000-0000-0000-0000-000000000002", Side::Buy, 100, 10);

        let trades = book.match_order(&incoming);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].amount, 5);

        let remaining = book
            .orders
            .iter()
            .find(|o| o.id == incoming.id)
            .expect("remaining incoming order should exist");

        assert_eq!(remaining.amount, 5);
    }

    #[test]
    fn different_arrival_order_produces_same_trades() {
        let path_a = std::env::temp_dir().join("opendex-test-determinism-a.json");
        let path_b = std::env::temp_dir().join("opendex-test-determinism-b.json");

        let mut book_a = OrderBook::new(path_a);
        let mut book_b = OrderBook::new(path_b);

        let sell_100 = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 5);

        let sell_90 = order("00000000-0000-0000-0000-000000000002", Side::Sell, 90, 5);

        // Node A receives 100 first.
        book_a.add_order(sell_100.clone());
        book_a.add_order(sell_90.clone());

        // Node B receives 90 first.
        book_b.add_order(sell_90);
        book_b.add_order(sell_100);

        let incoming_a = order("00000000-0000-0000-0000-000000000003", Side::Buy, 100, 10);

        let incoming_b = incoming_a.clone();

        let trades_a = book_a.match_order(&incoming_a);
        let trades_b = book_b.match_order(&incoming_b);

        assert_eq!(trades_a.len(), 2);
        assert_eq!(trades_b.len(), 2);

        assert_eq!(trades_a[0].id, trades_b[0].id);
        assert_eq!(trades_a[0].buy_order, trades_b[0].buy_order);
        assert_eq!(trades_a[0].sell_order, trades_b[0].sell_order);
        assert_eq!(trades_a[0].price, trades_b[0].price);
        assert_eq!(trades_a[0].amount, trades_b[0].amount);

        assert_eq!(trades_a[1].id, trades_b[1].id);
        assert_eq!(trades_a[1].buy_order, trades_b[1].buy_order);
        assert_eq!(trades_a[1].sell_order, trades_b[1].sell_order);
        assert_eq!(trades_a[1].price, trades_b[1].price);
        assert_eq!(trades_a[1].amount, trades_b[1].amount);
    }
    #[test]
    fn one_order_can_be_filled_by_multiple_trades() {
        let path = std::env::temp_dir().join("opendex-test-multiple-fills.json");

        let mut book = OrderBook::new(path);

        let sell = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 13);

        book.add_order(sell.clone());

        let buy1 = order("00000000-0000-0000-0000-000000000002", Side::Buy, 100, 3);

        let buy2 = order("00000000-0000-0000-0000-000000000003", Side::Buy, 100, 4);

        let buy3 = order("00000000-0000-0000-0000-000000000004", Side::Buy, 100, 4);

        let buy4 = order("00000000-0000-0000-0000-000000000005", Side::Buy, 100, 2);

        let trades1 = book.match_order(&buy1);
        assert_eq!(trades1.len(), 1);
        assert_eq!(trades1[0].amount, 3);

        let remaining = book
            .orders
            .iter()
            .find(|o| o.id == sell.id)
            .expect("sell order should remain");

        assert_eq!(remaining.amount, 10);

        let trades2 = book.match_order(&buy2);
        assert_eq!(trades2.len(), 1);
        assert_eq!(trades2[0].amount, 4);

        let remaining = book
            .orders
            .iter()
            .find(|o| o.id == sell.id)
            .expect("sell order should remain");

        assert_eq!(remaining.amount, 6);

        let trades3 = book.match_order(&buy3);
        assert_eq!(trades3.len(), 1);
        assert_eq!(trades3[0].amount, 4);

        let remaining = book
            .orders
            .iter()
            .find(|o| o.id == sell.id)
            .expect("sell order should remain");

        assert_eq!(remaining.amount, 2);

        let trades4 = book.match_order(&buy4);
        assert_eq!(trades4.len(), 1);
        assert_eq!(trades4[0].amount, 2);

        assert!(
            !book.orders.iter().any(|o| o.id == sell.id),
            "fully filled sell order should be removed"
        );

        assert_eq!(trades1[0].sell_order, sell.id);
        assert_eq!(trades2[0].sell_order, sell.id);
        assert_eq!(trades3[0].sell_order, sell.id);
        assert_eq!(trades4[0].sell_order, sell.id);

        assert_eq!(
            trades1[0].amount + trades2[0].amount + trades3[0].amount + trades4[0].amount,
            13
        );
    }

    #[test]
    fn apply_trade_is_idempotent() {
        let path = std::env::temp_dir().join("opendex-test-duplicate-trade.json");

        let mut book = OrderBook::new(path);

        let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

        let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

        book.add_order(buy.clone());
        book.add_order(sell.clone());

        let trade = Trade {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
            buy_order: buy.id,
            sell_order: sell.id,
            maker: sell.trader.clone(),
            taker: buy.trader.clone(),
            buyer: buy.trader.clone(),
            seller: sell.trader.clone(),
            market: market(),
            price: 100,
            amount: 4,
            state: TradeState::Created,
            created_at: 0,
            updated_at: 0,
        };

        assert!(book.apply_trade(&trade));

        // Same trade arriving again must be ignored.
        assert!(!book.apply_trade(&trade));

        let remaining_buy = book.orders.iter().find(|o| o.id == buy.id).unwrap();
        let remaining_sell = book.orders.iter().find(|o| o.id == sell.id).unwrap();

        assert_eq!(remaining_buy.amount, 6);
        assert_eq!(remaining_sell.amount, 6);
    }
    #[test]
    fn distributed_event_order_converges() {
        let path_a = std::env::temp_dir().join("opendex-test-distributed-a.json");
        let path_b = std::env::temp_dir().join("opendex-test-distributed-b.json");

        let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

        let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

        let trade = Trade {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
            buy_order: buy.id,
            sell_order: sell.id,
            maker: sell.trader.clone(),
            taker: buy.trader.clone(),
            buyer: buy.trader.clone(),
            seller: sell.trader.clone(),
            market: market(),
            price: 100,
            amount: 4,
            state: TradeState::Created,
            created_at: 0,
            updated_at: 0,
        };

        let mut node_a = OrderBook::new(path_a);
        let mut node_b = OrderBook::new(path_b);

        // Node A receives the trade before either order.
        assert!(!node_a.apply_trade(&trade));
        assert_eq!(node_a.pending_trades.len(), 1);

        node_a.add_order(buy.clone());
        assert_eq!(node_a.pending_trades.len(), 1);

        node_a.add_order(sell.clone());

        // The pending trade should now have been applied.
        assert!(node_a.pending_trades.is_empty());

        // Node B receives the orders first, then the trade.
        node_b.add_order(sell);
        node_b.add_order(buy);

        assert!(node_b.apply_trade(&trade));

        // Both nodes should converge to the same state.
        assert_eq!(node_a.orders.len(), node_b.orders.len());

        for order_a in &node_a.orders {
            let order_b = node_b
                .orders
                .iter()
                .find(|order| order.id == order_a.id)
                .expect("node B should contain the same order");

            assert_eq!(order_a.amount, order_b.amount);
        }

        assert_eq!(node_a.applied_trades, node_b.applied_trades);

        assert_eq!(node_a.pending_trades.len(), node_b.pending_trades.len());

        for (a, b) in node_a
            .pending_trades
            .iter()
            .zip(node_b.pending_trades.iter())
        {
            assert_eq!(a.id, b.id);
        }

        assert_eq!(node_a.removed_orders, node_b.removed_orders);
    }
    #[test]
    fn duplicate_out_of_order_delivery_converges() {
        let path_a = std::env::temp_dir().join("opendex-test-duplicate-out-of-order-a.json");
        let path_b = std::env::temp_dir().join("opendex-test-duplicate-out-of-order-b.json");

        let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

        let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

        let trade = Trade {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
            buy_order: buy.id,
            sell_order: sell.id,
            maker: sell.trader.clone(),
            taker: buy.trader.clone(),
            buyer: buy.trader.clone(),
            seller: sell.trader.clone(),
            market: market(),
            price: 100,
            amount: 4,
            state: TradeState::Created,
            created_at: 0,
            updated_at: 0,
        };

        let mut node_a = OrderBook::new(path_a);
        let mut node_b = OrderBook::new(path_b);

        // Node A receives:
        // TRADE -> BUY -> duplicate TRADE -> SELL

        assert!(!node_a.apply_trade(&trade));
        assert_eq!(node_a.pending_trades.len(), 1);

        node_a.add_order(buy.clone());
        assert_eq!(node_a.pending_trades.len(), 1);

        // The duplicate trade must not be added twice to pending trades.
        assert!(!node_a.apply_trade(&trade));
        assert_eq!(node_a.pending_trades.len(), 1);

        node_a.add_order(sell.clone());

        // The pending trade should now have been applied exactly once.
        assert!(node_a.pending_trades.is_empty());
        assert_eq!(node_a.applied_trades.len(), 1);
        assert_eq!(node_a.applied_trades[0], trade.id.to_string());

        // Node B receives:
        // SELL -> BUY -> TRADE

        node_b.add_order(sell);
        node_b.add_order(buy);

        assert!(node_b.apply_trade(&trade));

        // Both nodes should have the same logical orderbook state.
        assert_eq!(node_a.orders.len(), node_b.orders.len());

        for order_a in &node_a.orders {
            let order_b = node_b
                .orders
                .iter()
                .find(|order| order.id == order_a.id)
                .expect("node B should contain the same order");

            assert_eq!(order_a.amount, order_b.amount);
        }

        assert_eq!(node_a.applied_trades, node_b.applied_trades);
        assert_eq!(node_a.pending_trades.len(), node_b.pending_trades.len());
        assert_eq!(node_a.removed_orders, node_b.removed_orders);
    }
    #[test]
    fn orderbook_persistence_preserves_distributed_state() {
        let path = std::env::temp_dir().join("opendex-test-persistence.json");

        let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

        let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

        let trade = Trade {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
            buy_order: buy.id,
            sell_order: sell.id,
            maker: sell.trader.clone(),
            taker: buy.trader.clone(),
            buyer: buy.trader.clone(),
            seller: sell.trader.clone(),
            market: market(),
            price: 100,
            amount: 4,
            state: TradeState::Created,
            created_at: 0,
            updated_at: 0,
        };

        // Build state before persistence.
        let mut book = OrderBook::new(path.clone());

        book.add_order(buy.clone());
        book.add_order(sell.clone());

        assert!(book.apply_trade(&trade));

        assert_eq!(book.applied_trades.len(), 1);
        assert!(book.pending_trades.is_empty());

        let buy_before = book
            .orders
            .iter()
            .find(|order| order.id == buy.id)
            .expect("buy order should remain");

        let sell_before = book
            .orders
            .iter()
            .find(|order| order.id == sell.id)
            .expect("sell order should remain");

        assert_eq!(buy_before.amount, 6);
        assert_eq!(sell_before.amount, 6);

        // Persist the state.
        book.save_orders().expect("orderbook should save");

        // Load it into a fresh OrderBook.
        let loaded = OrderBook::load_orders(path.clone()).expect("orderbook should load");

        // Orders survived with their remaining amounts.
        assert_eq!(loaded.orders.len(), 2);

        let loaded_buy = loaded
            .orders
            .iter()
            .find(|order| order.id == buy.id)
            .expect("loaded buy order should exist");

        let loaded_sell = loaded
            .orders
            .iter()
            .find(|order| order.id == sell.id)
            .expect("loaded sell order should exist");

        assert_eq!(loaded_buy.amount, 6);
        assert_eq!(loaded_sell.amount, 6);

        // Distributed state survived.
        assert_eq!(loaded.applied_trades.len(), 1);
        assert_eq!(loaded.applied_trades[0], trade.id.to_string());

        assert!(loaded.pending_trades.is_empty());
        assert!(loaded.removed_orders.is_empty());

        // The same trade must still be idempotent after recovery.
        let mut recovered = loaded;

        assert!(!recovered.apply_trade(&trade));

        let recovered_buy = recovered
            .orders
            .iter()
            .find(|order| order.id == buy.id)
            .expect("recovered buy order should exist");

        let recovered_sell = recovered
            .orders
            .iter()
            .find(|order| order.id == sell.id)
            .expect("recovered sell order should exist");

        assert_eq!(recovered_buy.amount, 6);
        assert_eq!(recovered_sell.amount, 6);

        let _ = fs::remove_file(path);
    }
    #[test]
    fn pending_trade_survives_restart() {
        let path = std::env::temp_dir().join("opendex-test-pending-trade-restart.json");

        let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

        let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

        let trade = Trade {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
            buy_order: buy.id,
            sell_order: sell.id,
            maker: sell.trader.clone(),
            taker: buy.trader.clone(),
            buyer: buy.trader.clone(),
            seller: sell.trader.clone(),
            market: market(),
            price: 100,
            amount: 4,
            state: TradeState::Created,
            created_at: 0,
            updated_at: 0,
        };

        // Trade arrives before its orders.
        let mut book = OrderBook::new(path.clone());

        assert!(!book.apply_trade(&trade));
        assert_eq!(book.pending_trades.len(), 1);
        assert_eq!(book.pending_trades[0].id, trade.id);

        // Persist the pending trade.
        book.save_orders().expect("orderbook should save");

        // Simulate a restart.
        let mut recovered = OrderBook::load_orders(path.clone()).expect("orderbook should load");

        // The pending trade must have survived the restart.
        assert_eq!(recovered.pending_trades.len(), 1);
        assert_eq!(recovered.pending_trades[0].id, trade.id);

        // Orders arrive after restart.
        recovered.add_order(buy.clone());

        // Still waiting for the sell order.
        assert_eq!(recovered.pending_trades.len(), 1);

        recovered.add_order(sell.clone());

        // The pending trade should now be applied.
        assert!(recovered.pending_trades.is_empty());
        assert_eq!(recovered.applied_trades.len(), 1);
        assert_eq!(recovered.applied_trades[0], trade.id.to_string());

        // Both orders should have been reduced exactly once.
        let recovered_buy = recovered
            .orders
            .iter()
            .find(|order| order.id == buy.id)
            .expect("recovered buy order should exist");

        let recovered_sell = recovered
            .orders
            .iter()
            .find(|order| order.id == sell.id)
            .expect("recovered sell order should exist");

        assert_eq!(recovered_buy.amount, 6);
        assert_eq!(recovered_sell.amount, 6);

        let _ = fs::remove_file(path);
    }
    #[test]
    fn removed_order_is_not_resurrected_by_stale_state() {
        let path_a = std::env::temp_dir().join("opendex-test-no-resurrection-a.json");
        let path_b = std::env::temp_dir().join("opendex-test-no-resurrection-b.json");

        let mut node_a = OrderBook::new(path_a);
        let mut node_b = OrderBook::new(path_b);

        let order_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        let order_a = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

        // Both nodes initially know the order.
        node_a.add_order(order_a.clone());
        node_b.add_order(order_a.clone());

        // Node A receives a trade that completely fills the order.
        let trade = Trade {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            buy_order: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
            sell_order: order_id,
            maker: order_a.trader.clone(),
            taker: "test-trader".to_string(),
            buyer: "test-trader".to_string(),
            seller: order_a.trader.clone(),
            market: market(),
            price: 100,
            amount: 10,
            state: TradeState::Created,
            created_at: 0,
            updated_at: 0,
        };

        // Add the buy order so the trade can be applied.
        node_a.add_order(order(
            "00000000-0000-0000-0000-000000000003",
            Side::Buy,
            100,
            10,
        ));

        assert!(node_a.apply_trade(&trade));

        // The sell order was completely filled and removed.
        assert!(!node_a.orders.iter().any(|o| o.id == order_id));
        assert!(node_a.removed_orders.contains(&order_id));
        assert!(node_a.applied_trades.contains(&trade.id.to_string()));

        // Node B is stale and still has the old order.
        assert!(node_b.orders.iter().any(|o| o.id == order_id));

        // Simulate the important part of StateResponse merging:
        // Node A receives Node B's stale order, but must not resurrect it.
        if !node_a.removed_orders.contains(&order_id) {
            node_a.removed_orders.push(order_id);
        }

        for stale_order in node_b.orders.clone() {
            if node_a.removed_orders.contains(&stale_order.id) {
                continue;
            }

            node_a.add_order(stale_order);
        }

        // The filled order must still be absent.
        assert!(
            !node_a.orders.iter().any(|o| o.id == order_id),
            "a removed order must never be resurrected by stale state"
        );

        // The tombstone must remain.
        assert!(node_a.removed_orders.contains(&order_id));

        // The trade must remain marked as applied.
        assert!(node_a.applied_trades.contains(&trade.id.to_string()));
    }
    #[test]
    fn trade_waits_for_both_orders_then_applies_once() {
        let path = std::env::temp_dir().join("opendex-test-trade-before-orders.json");

        let mut book = OrderBook::new(path);

        let buy = order("00000000-0000-0000-0000-000000000001", Side::Buy, 100, 10);

        let sell = order("00000000-0000-0000-0000-000000000002", Side::Sell, 100, 10);

        let trade = Trade {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
            buy_order: buy.id,
            sell_order: sell.id,
            maker: sell.trader.clone(),
            taker: buy.trader.clone(),
            buyer: buy.trader.clone(),
            seller: sell.trader.clone(),
            market: market(),
            price: 100,
            amount: 4,
            state: TradeState::Created,
            created_at: 0,
            updated_at: 0,
        };

        // Trade arrives before either order.
        assert!(!book.apply_trade(&trade));

        assert_eq!(book.pending_trades.len(), 1);
        assert!(book.applied_trades.is_empty());

        // BUY arrives. Trade still cannot be applied.
        book.add_order(buy.clone());

        assert_eq!(book.pending_trades.len(), 1);
        assert!(book.applied_trades.is_empty());

        let buy_remaining = book
            .orders
            .iter()
            .find(|o| o.id == buy.id)
            .expect("buy order should exist");

        assert_eq!(buy_remaining.amount, 10);

        // SELL arrives. The pending trade should now apply automatically.
        book.add_order(sell.clone());

        assert!(book.pending_trades.is_empty());
        assert_eq!(book.applied_trades.len(), 1);
        assert_eq!(book.applied_trades[0], trade.id.to_string());

        let buy_remaining = book
            .orders
            .iter()
            .find(|o| o.id == buy.id)
            .expect("buy order should remain");

        let sell_remaining = book
            .orders
            .iter()
            .find(|o| o.id == sell.id)
            .expect("sell order should remain");

        assert_eq!(buy_remaining.amount, 6);
        assert_eq!(sell_remaining.amount, 6);

        // A duplicate delivery must not apply the trade again.
        assert!(!book.apply_trade(&trade));

        assert_eq!(book.pending_trades.len(), 0);

        let buy_remaining = book.orders.iter().find(|o| o.id == buy.id).unwrap();

        let sell_remaining = book.orders.iter().find(|o| o.id == sell.id).unwrap();

        assert_eq!(buy_remaining.amount, 6);
        assert_eq!(sell_remaining.amount, 6);
    }
    #[test]
    fn cancellation_before_order_creation_prevents_resurrection() {
        let path = std::env::temp_dir().join("opendex-test-cancel-before-order.json");

        let mut book = OrderBook::new(path);

        let order_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        // Cancellation arrives before OrderCreated.
        assert!(!book.remove_order(order_id));

        // The cancellation must still create a tombstone.
        assert!(book.removed_orders.contains(&order_id));

        let order = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

        // A stale OrderCreated must not resurrect the cancelled order.
        book.add_order(order);

        assert!(
            !book.orders.iter().any(|o| o.id == order_id),
            "cancelled order must not be resurrected"
        );

        assert!(book.removed_orders.contains(&order_id));
    }

    #[test]
    fn duplicate_cancellation_is_idempotent() {
        let path = std::env::temp_dir().join("opendex-test-duplicate-cancel.json");

        let mut book = OrderBook::new(path);

        let order_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        let order = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

        book.add_order(order);

        assert!(book.remove_order(order_id));
        assert!(!book.remove_order(order_id));

        assert_eq!(book.removed_orders.len(), 1);
        assert_eq!(book.removed_orders[0], order_id);
    }

    #[test]
    fn stale_order_cannot_be_added_after_cancellation() {
        let path = std::env::temp_dir().join("opendex-test-stale-order-after-cancel.json");

        let mut book = OrderBook::new(path);

        let order_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        let original = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

        book.add_order(original);

        assert!(book.remove_order(order_id));

        // Simulate an old OrderCreated message arriving later.
        let stale_order = order("00000000-0000-0000-0000-000000000001", Side::Sell, 100, 10);

        book.add_order(stale_order);

        assert!(book.orders.is_empty());
        assert_eq!(book.removed_orders.len(), 1);
        assert_eq!(book.removed_orders[0], order_id);
    }
} // mod test
