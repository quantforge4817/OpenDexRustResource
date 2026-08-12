use anyhow::Result;
use protocol::message::{Order, Side, Trade};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub orders: Vec<Order>,
    pub applied_trades: Vec<String>,

    #[serde(skip)]
    path: PathBuf,
}

impl OrderBook {
    pub fn new(path: PathBuf) -> Self {
        Self {
            orders: Vec::new(),
            applied_trades: Vec::new(),
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

    pub fn add_order(&mut self, order: Order) {
        if self.orders.iter().any(|existing| existing.id == order.id) {
            return;
        }

        self.orders.push(order);
    }

    pub fn remove_order(&mut self, id: Uuid) -> bool {
        let before = self.orders.len();

        self.orders.retain(|order| order.id != id);

        before != self.orders.len()
    }

    pub fn mark_trade_applied(&mut self, trade_id: &str) {
        if !self.applied_trades.contains(&trade_id.to_string()) {
            self.applied_trades.push(trade_id.to_string());
        }
    }

    pub fn apply_trade(&mut self, trade: &Trade) -> bool {
        if self.applied_trades.contains(&trade.id) {
            return false;
        }

        self.applied_trades.push(trade.id.clone());

        self.orders.retain_mut(|order| {
            if order.id == trade.buy_order || order.id == trade.sell_order {
                if order.amount > trade.amount {
                    order.amount -= trade.amount;
                    true
                } else {
                    false
                }
            } else {
                true
            }
        });

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

            let trade_id = Uuid::from_slice(&hash[..16])
                .expect("failed to create trade uuid")
                .to_string();

            let trade = Trade {
                id: trade_id,
                buy_order,
                sell_order,
                market: incoming.market.clone(),
                price: trade_price,
                amount,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };

            println!("Match found:");
            println!("{:?}", trade);

            trades.push(trade);

            if existing.amount > amount {
                self.orders[index].amount -= amount;
            } else {
                self.orders.remove(index);
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
    use protocol::message::{Asset, Market};

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
}
