use anyhow::Result;
use protocol::message::{Order, Side, Trade};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderBook {
    pub orders: Vec<Order>,
    pub applied_trades: Vec<String>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            orders: Vec::new(),
            applied_trades: Vec::new(),
        }
    }

    pub fn load_orders() -> Result<Self> {
        let path = "orders.json";

        if !Path::new(path).exists() {
            return Ok(Self::new());
        }

        let data = fs::read_to_string(path)?;
        let mut order_book: OrderBook = serde_json::from_str(&data)?;

        order_book.orders.dedup_by(|a, b| a.id == b.id);

        order_book.save_orders()?;

        Ok(order_book)
    }

    pub fn save_orders(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write("orders.json", data)?;

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

    pub fn apply_trade(&mut self, trade: &Trade) {
        if self.applied_trades.contains(&trade.id) {
            return;
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

        let mut index = 0;

        while index < self.orders.len() && remaining_amount > 0.0 {
            let existing = self.orders[index].clone();

            if existing.market.base == incoming.market.base
                && existing.market.quote == incoming.market.quote
                && existing.side != incoming.side
                && match incoming.side {
                    Side::Buy => existing.price <= incoming.price,
                    Side::Sell => existing.price >= incoming.price,
                }
            {
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

                let trade_input =
                    format!("{}{}{}{}", buy_order, sell_order, incoming.price, amount);

                let hash = Sha256::digest(trade_input.as_bytes());

                let trade_id = Uuid::from_slice(&hash[..16])
                    .expect("failed to create trade uuid")
                    .to_string();

                let trade = Trade {
                    id: trade_id,
                    buy_order,
                    sell_order,
                    market: incoming.market.clone(),
                    price: incoming.price,
                    amount,
                };

                println!("Match found:");
                println!("{:?}", trade);

                trades.push(trade);

                if existing.amount > amount {
                    self.orders[index].amount -= amount;
                    index += 1;
                } else {
                    self.orders.remove(index);
                }

                remaining_amount -= amount;
            } else {
                index += 1;
            }
        }

        trades
    }
}
