use anyhow::Result;
use protocol::message::{Order, Side, Trade, TradeState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

        let mut order_book: OrderBook = crate::persistence::read(&path)?;

        order_book.path = path;

        order_book.orders.dedup_by(|a, b| a.id == b.id);

        order_book.save_orders()?;

        Ok(order_book)
    }

    pub fn save_orders(&self) -> Result<()> {
        crate::persistence::write(&self.path, self)?;

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
    pub fn validate_trade(&self, trade: &Trade) -> Result<()> {
        if trade.amount == 0 {
            anyhow::bail!("trade amount must be greater than zero");
        }

        let buy_order = self
            .orders
            .iter()
            .find(|order| order.id == trade.buy_order)
            .ok_or_else(|| anyhow::anyhow!("buy order {} not found", trade.buy_order))?;

        let sell_order = self
            .orders
            .iter()
            .find(|order| order.id == trade.sell_order)
            .ok_or_else(|| anyhow::anyhow!("sell order {} not found", trade.sell_order))?;

        // The referenced orders must have the correct sides.
        if buy_order.side != Side::Buy {
            anyhow::bail!("buy_order does not reference a Buy order");
        }

        if sell_order.side != Side::Sell {
            anyhow::bail!("sell_order does not reference a Sell order");
        }

        // Trader identities must correspond to the referenced orders.
        if trade.buyer != buy_order.trader {
            anyhow::bail!("trade buyer does not match buy order trader");
        }

        if trade.seller != sell_order.trader {
            anyhow::bail!("trade seller does not match sell order trader");
        }

        // Both orders must belong to the same market as the trade.
        if trade.market != buy_order.market {
            anyhow::bail!("trade market does not match buy order market");
        }

        if trade.market != sell_order.market {
            anyhow::bail!("trade market does not match sell order market");
        }

        // The trade price must satisfy both limit orders.
        if trade.price > buy_order.price {
            anyhow::bail!("trade price exceeds buy order price");
        }

        if trade.price < sell_order.price {
            anyhow::bail!("trade price is below sell order price");
        }

        // A trade cannot consume more than either order has remaining.
        if trade.amount > buy_order.amount {
            anyhow::bail!("trade amount exceeds buy order remaining amount");
        }

        if trade.amount > sell_order.amount {
            anyhow::bail!("trade amount exceeds sell order remaining amount");
        }

        Ok(())
    }
    pub fn apply_trade(&mut self, trade: &Trade) -> bool {
        let trade_id = trade.id.to_string();

        // Idempotency: never apply the same trade twice.
        if self.applied_trades.contains(&trade_id) {
            return false;
        }
        // Reject malformed or forged trades before they can become pending
        // or mutate the order book.
        if let Err(error) = self.validate_trade_structure(trade) {
            eprintln!("Rejecting invalid trade {}: {}", trade.id, error);
            return false;
        }

        // A trade may arrive before one or both orders.
        // Keep it pending until the required orders arrive.
        let has_buy_order = self.orders.iter().any(|o| o.id == trade.buy_order);
        let has_sell_order = self.orders.iter().any(|o| o.id == trade.sell_order);

        if !has_buy_order || !has_sell_order {
            if !self.pending_trades.iter().any(|t| t.id == trade.id) {
                self.pending_trades.push(trade.clone());
            }

            return false;
        }

        // Both orders exist, so validate the trade before mutating
        // the order book.
        if let Err(error) = self.validate_trade(trade) {
            eprintln!("Rejecting invalid trade {}: {}", trade.id, error);

            return false;
        }

        // Mark the trade as applied before changing order quantities.
        self.applied_trades.push(trade_id);

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

        // Fully consumed orders are permanently removed.
        for id in fully_filled {
            if !self.removed_orders.contains(&id) {
                self.removed_orders.push(id);
            }
        }

        // The trade is no longer pending because it was successfully
        // validated and applied.
        self.pending_trades.retain(|pending| pending.id != trade.id);

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

        // Track the remaining quantity of resting orders without mutating
        // the actual order book. apply_trade() is the only function that
        // changes/removes orders.
        let mut available: std::collections::HashMap<Uuid, u64> = self
            .orders
            .iter()
            .map(|order| (order.id, order.amount))
            .collect();

        while remaining_amount > 0 {
            let best_index = self
                .orders
                .iter()
                .enumerate()
                .filter(|(_, existing)| {
                    let available_amount = available
                        .get(&existing.id)
                        .copied()
                        .unwrap_or(existing.amount);

                    available_amount > 0
                        && existing.market.base == incoming.market.base
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

            let existing_remaining = available
                .get(&existing.id)
                .copied()
                .unwrap_or(existing.amount);

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

            let amount = remaining_amount.min(existing_remaining);
            let trade_price = existing.price;

            let trade_input = format!("{}:{}:{}:{}", buy_order, sell_order, trade_price, amount);

            let hash = Sha256::digest(trade_input.as_bytes());

            let trade_id = Uuid::from_slice(&hash[..16]).expect("failed to create trade uuid");

            let now = chrono::Utc::now().timestamp() as u64;

            // The resting order is the maker.
            let maker = &existing.trader;

            // The incoming order is the taker.
            let taker = &incoming.trader;

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

            // Only update our temporary matching state.
            // The real order book is updated later by apply_trade().
            available.insert(existing.id, existing_remaining - amount);

            remaining_amount -= amount;
        }

        trades
    }
    pub fn validate_trade_structure(&self, trade: &Trade) -> Result<()> {
        if trade.amount == 0 {
            anyhow::bail!("trade amount must be greater than zero");
        }

        if trade.price == 0 {
            anyhow::bail!("trade price must be greater than zero");
        }

        if trade.buy_order == trade.sell_order {
            anyhow::bail!("trade cannot reference the same order twice");
        }

        if trade.buyer.is_empty() || trade.seller.is_empty() {
            anyhow::bail!("trade buyer/seller must not be empty");
        }

        if trade.maker.is_empty() || trade.taker.is_empty() {
            anyhow::bail!("trade maker/taker must not be empty");
        }

        if trade.maker != trade.seller && trade.maker != trade.buyer {
            anyhow::bail!("trade maker must be a participant");
        }

        if trade.taker != trade.seller && trade.taker != trade.buyer {
            anyhow::bail!("trade taker must be a participant");
        }

        if trade.market.base == trade.market.quote {}
        if trade.market.base == trade.market.quote {
            anyhow::bail!("trade has invalid market");
        }

        Ok(())
    }
}
