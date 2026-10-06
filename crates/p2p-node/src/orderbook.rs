use anyhow::Result;
use protocol::message::{Order, Side, Trade, TradeState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn deterministic_trade_id(
    buy_order: Uuid,
    sell_order: Uuid,
    price: u64,
    amount: u64,
    fill_sequence: u64,
) -> Uuid {
    let trade_input = format!(
        "{}:{}:{}:{}:{}",
        buy_order, sell_order, price, amount, fill_sequence,
    );

    let hash = Sha256::digest(trade_input.as_bytes());

    Uuid::from_slice(&hash[..16]).unwrap_or_else(|_| {
        panic!("SHA-256 digest is 32 bytes, always enough for Uuid")
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    /// Active orders.  Production code should access this field only through
    /// the accessor methods (`get_order`, `is_order_active`, `active_orders`).
    /// Direct access is allowed in tests for state inspection.
    pub orders: HashMap<Uuid, Order>,

    /// Trades that have been applied to the order book.
    pub applied_trades: HashSet<Uuid>,

    /// Trades that arrived before their counterpart orders.
    #[serde(default)]
    pub pending_trades: Vec<Trade>,

    /// Orders that have been cancelled or fully filled.
    #[serde(default)]
    pub removed_orders: HashSet<Uuid>,

    /// Fill sequence counters per resting order.
    #[serde(default)]
    pub fill_sequences: HashMap<Uuid, u64>,

    /// Highest snapshot version accepted from a remote peer (AUDIT-010).
    /// Used to reject stale or replayed snapshots.
    #[serde(default)]
    pub last_accepted_version: u64,

    #[serde(skip)]
    path: PathBuf,
}

impl OrderBook {
    pub fn new(path: PathBuf) -> Self {
        Self {
            orders: HashMap::new(),
            applied_trades: HashSet::new(),
            removed_orders: HashSet::new(),
            pending_trades: Vec::new(),
            fill_sequences: HashMap::new(),
            last_accepted_version: 0,
            path,
        }
    }

    pub fn load_orders(path: PathBuf) -> Result<Self> {
        if !Path::new(&path).exists() {
            return Ok(Self::new(path));
        }

        let mut order_book: OrderBook = crate::persistence::read(&path)?;

        order_book.path = path;

        order_book.save_orders()?;

        Ok(order_book)
    }

    pub fn save_orders(&self) -> Result<()> {
        crate::persistence::write(&self.path, self)?;

        Ok(())
    }

    pub fn upsert_order(&mut self, order: Order) -> Vec<Trade> {
        if self.removed_orders.contains(&order.id) {
            return Vec::new();
        }

        let order_id = order.id;

        if let Some(existing) = self.orders.get(&order_id) {
            tracing::info!(
                order_id = %order_id,
                old_amount = existing.amount,
                new_amount = order.amount,
                "Updating order amount"
            );
        } else {
            tracing::info!(
                order_id = %order_id,
                amount = order.amount,
                "Adding new order"
            );
        }

        self.orders.insert(order_id, order);

        // Return pending trades involving this order to the caller.
        // The Exchange layer is responsible for applying the trade and
        // reconciling the associated balance reservation.
        let pending_trades: Vec<Trade> = self
            .pending_trades
            .iter()
            .filter(|trade| {
                if trade.buy_order != order_id && trade.sell_order != order_id {
                    return false;
                }

                let has_buy = self.orders.contains_key(&trade.buy_order);
                let has_sell = self.orders.contains_key(&trade.sell_order);

                has_buy && has_sell
            })
            .cloned()
            .collect();

        pending_trades
    }
    pub fn update_order_from_snapshot(&mut self, order: Order) {
        if self.removed_orders.contains(&order.id) {
            tracing::debug!(order_id = %order.id, "Ignoring removed order during snapshot sync");
            return;
        }

        if let Some(existing) = self.orders.get_mut(&order.id) {
            if existing.trader != order.trader
                || existing.market != order.market
                || existing.side != order.side
                || existing.price != order.price
            {
                tracing::debug!(order_id = %order.id, "Ignoring conflicting snapshot terms");
                return;
            }

            if order.amount > existing.amount {
                tracing::debug!(
                    order_id = %order.id,
                    old_amount = existing.amount,
                    new_amount = order.amount,
                    "Ignoring snapshot increase"
                );
                return;
            }

            tracing::debug!(
                order_id = %order.id,
                old_amount = existing.amount,
                new_amount = order.amount,
                "Updating snapshot order amount"
            );

            existing.amount = order.amount;
            return;
        }

        self.upsert_order(order);
    }

    pub fn remove_order(&mut self, id: Uuid) -> bool {
        let removed = self.orders.remove(&id).is_some();

        self.removed_orders.insert(id);

        removed
    }

    pub fn validate_trade(&self, trade: &Trade) -> Result<()> {
        if trade.amount == 0 {
            anyhow::bail!("trade amount must be greater than zero");
        }

        let buy_order = self
            .orders
            .get(&trade.buy_order)
            .ok_or_else(|| anyhow::anyhow!("buy order {} not found", trade.buy_order))?;

        let sell_order = self
            .orders
            .get(&trade.sell_order)
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
        // Idempotency: never apply the same trade twice.
        if self.applied_trades.contains(&trade.id) {
            return false;
        }
        // Reject malformed or forged trades before they can become pending
        // or mutate the order book.
        if let Err(error) = self.validate_trade_structure(trade) {
            tracing::error!(trade_id = %trade.id, error = %error, "Rejecting invalid trade");
            return false;
        }

        // Verify the trade ID matches its deterministic hash.
        // This guard protects against tampered or fabricated trades,
        // including those arriving via pending trade reconciliation.
        if let Err(error) = self.validate_trade_identity(trade) {
            tracing::error!(trade_id = %trade.id, error = %error, "Rejecting trade with invalid ID");
            return false;
        }

        // A trade may arrive before one or both orders.
        // Keep it pending until the required orders arrive.
        let has_buy_order = self.orders.contains_key(&trade.buy_order);
        let has_sell_order = self.orders.contains_key(&trade.sell_order);

        if !has_buy_order || !has_sell_order {
            if !self.pending_trades.iter().any(|t| t.id == trade.id) {
                self.pending_trades.push(trade.clone());
            }

            return false;
        }

        // Both orders exist, so validate the trade before mutating
        // the order book.
        if let Err(error) = self.validate_trade(trade) {
            tracing::error!(trade_id = %trade.id, error = %error, "Rejecting invalid trade");
            return false;
        }
        let next_sequence = trade.fill_sequence.saturating_add(1);

        let sequence_order_id = if trade.maker == trade.buyer {
            trade.buy_order
        } else if trade.maker == trade.seller {
            trade.sell_order
        } else {
            return false;
        };

        let current = self
            .fill_sequences
            .get(&sequence_order_id)
            .copied()
            .unwrap_or(0);

        if next_sequence > current {
            self.fill_sequences.insert(sequence_order_id, next_sequence);
        }

        // Mark the trade as applied before changing order quantities.
        self.applied_trades.insert(trade.id);

        let mut fully_filled = Vec::new();

        for order_id in [trade.buy_order, trade.sell_order] {
            if let Some(order) = self.orders.get(&order_id) {
                if order.amount > trade.amount {
                    if let Some(order) = self.orders.get_mut(&order_id) {
                        order.amount -= trade.amount;
                    }
                } else {
                    fully_filled.push(order_id);
                    self.orders.remove(&order_id);
                }
            }
        }

        // Fully consumed orders are permanently removed.
        for id in fully_filled {
            self.removed_orders.insert(id);
        }

        // The trade is no longer pending because it was successfully
        // validated and applied.
        self.pending_trades.retain(|pending| pending.id != trade.id);

        true
    }

    pub fn print_orders(&self) {
        tracing::info!("=== OpenDEX OrderBook ===");

        for order in self.orders.values() {
            tracing::info!("{:?}", order);
        }
    }

    pub fn match_order(&mut self, incoming: &Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut remaining_amount = incoming.amount;

        // Track the remaining quantity of resting orders without mutating
        // the actual order book. apply_trade() is the only function that
        // changes/removes orders.
        let mut available: HashMap<Uuid, u64> = self
            .orders
            .iter()
            .map(|(id, order)| (*id, order.amount))
            .collect();

        while remaining_amount > 0 {
            let best_order = self
                .orders
                .iter()
                .filter(|(id, existing)| {
                    let available_amount = available
                        .get(id)
                        .copied()
                        .unwrap_or(existing.amount);

                    available_amount > 0
                        && existing.market.base == incoming.market.base
                        && existing.market.quote == incoming.market.quote
                        && existing.side != incoming.side
                        && existing.trader != incoming.trader
                        && match incoming.side {
                            Side::Buy => existing.price <= incoming.price,
                            Side::Sell => existing.price >= incoming.price,
                        }
                })
                .min_by(|(a_id, a), (b_id, b)| {
                    let price_cmp = match incoming.side {
                        Side::Buy => a.price.cmp(&b.price),
                        Side::Sell => b.price.cmp(&a.price),
                    };

                    if price_cmp == std::cmp::Ordering::Equal {
                        a_id.cmp(b_id)
                    } else {
                        price_cmp
                    }
                });

            let Some((_, existing)) = best_order else {
                break;
            };

            let existing_id = existing.id;
            let existing = existing.clone();

            let existing_remaining = available
                .get(&existing_id)
                .copied()
                .unwrap_or(existing.amount);

            let buy_order = if matches!(incoming.side, Side::Buy) {
                incoming.id
            } else {
                existing_id
            };

            let sell_order = if matches!(incoming.side, Side::Sell) {
                incoming.id
            } else {
                existing_id
            };

            let amount = remaining_amount.min(existing_remaining);

            let trade_price = existing.price;

            let fill_sequence = self.fill_sequences.get(&existing_id).copied().unwrap_or(0);

            self.fill_sequences.insert(existing_id, fill_sequence + 1);

            let trade_id =
                deterministic_trade_id(buy_order, sell_order, trade_price, amount, fill_sequence);

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
                fill_sequence,
                state: TradeState::Created,
                created_at: now,
                updated_at: now,
            };

            tracing::info!(trade_id = %trade.id, "Match found");

            trades.push(trade);

            // Only update our temporary matching state.
            // The real order book is updated later by apply_trade().
            available.insert(existing_id, existing_remaining - amount);

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

        if trade.market.base == trade.market.quote {
            anyhow::bail!("trade has invalid market");
        }

        Ok(())
    }

    pub fn validate_trade_identity(&self, trade: &Trade) -> Result<()> {
        let expected_id = deterministic_trade_id(
            trade.buy_order,
            trade.sell_order,
            trade.price,
            trade.amount,
            trade.fill_sequence,
        );

        if trade.id != expected_id {
            anyhow::bail!(
                "trade {} has invalid deterministic ID; expected {}",
                trade.id,
                expected_id
            );
        }

        Ok(())
    }

    // -----------------------------------------------------------------
    // Accessors for sync.rs — provide controlled read access to internal
    // state without exposing fields directly.  These methods are used
    // during snapshot validation and application so that sync.rs cannot
    // bypass domain invariants.
    // -----------------------------------------------------------------

    /// Returns true if the order is currently active (not removed).
    pub fn is_order_active(&self, order_id: &Uuid) -> bool {
        self.orders.contains_key(order_id)
    }

    /// Returns true if the order has been removed (cancelled or fully filled).
    pub fn is_order_removed(&self, order_id: &Uuid) -> bool {
        self.removed_orders.contains(order_id)
    }

    /// Look up an active order by ID.
    pub fn get_order(&self, order_id: &Uuid) -> Option<&Order> {
        self.orders.get(order_id)
    }

    /// Returns all active orders as a cloned vector, sorted by ID.
    pub fn active_orders(&self) -> Vec<Order> {
        let mut orders: Vec<_> = self.orders.values().cloned().collect();
        orders.sort_by_key(|order| order.id);
        orders.dedup_by_key(|order| order.id);
        orders
    }

    /// Returns the set of removed order IDs.
    pub fn removed_order_ids(&self) -> HashSet<Uuid> {
        self.removed_orders.clone()
    }

    /// Returns the set of applied trade IDs.
    pub fn applied_trade_ids(&self) -> HashSet<Uuid> {
        self.applied_trades.clone()
    }

    /// Returns the highest snapshot version accepted from a remote peer.
    pub fn snapshot_version(&self) -> u64 {
        self.last_accepted_version
    }

    /// Advance the accepted snapshot version after a successful sync.
    pub fn accept_snapshot_version(&mut self, version: u64) {
        self.last_accepted_version = version;
    }

    /// Check whether a removed order ID is present in the removal set.
    pub fn contains_removed_order(&self, order_id: &Uuid) -> bool {
        self.removed_orders.contains(order_id)
    }

    /// Remove any pending trade matching the given ID.
    pub fn remove_pending_trade(&mut self, trade_id: Uuid) {
        self.pending_trades.retain(|pending| pending.id != trade_id);
    }

    /// Returns the number of pending trades.
    pub fn pending_trades_count(&self) -> usize {
        self.pending_trades.len()
    }
}

// ---------------------------------------------------------------------
// Test-only accessors — allow test code to inspect internal state
// without exposing fields to production code.  These are prefixed with
// `_test_` to signal they are not part of the stable API.
// ---------------------------------------------------------------------
impl OrderBook {
    /// Returns a reference to the internal orders map (tests only).
    pub fn _test_orders(&self) -> &HashMap<Uuid, Order> {
        &self.orders
    }

    /// Returns a mutable reference to the internal orders map (tests only).
    pub fn _test_orders_mut(&mut self) -> &mut HashMap<Uuid, Order> {
        &mut self.orders
    }

    /// Returns a reference to the internal removed_orders set (tests only).
    pub fn _test_removed_orders(&self) -> &HashSet<Uuid> {
        &self.removed_orders
    }

    /// Returns a reference to the internal applied_trades set (tests only).
    pub fn _test_applied_trades(&self) -> &HashSet<Uuid> {
        &self.applied_trades
    }

    /// Returns a reference to the internal pending_trades vec (tests only).
    pub fn _test_pending_trades(&self) -> &Vec<Trade> {
        &self.pending_trades
    }

    /// Returns a reference to the internal fill_sequences map (tests only).
    pub fn _test_fill_sequences(&self) -> &HashMap<Uuid, u64> {
        &self.fill_sequences
    }

    /// Returns the last accepted snapshot version (tests only).
    pub fn _test_last_accepted_version(&self) -> u64 {
        self.last_accepted_version
    }

    /// Sets the last accepted snapshot version directly (tests only).
    pub fn _test_set_last_accepted_version(&mut self, version: u64) {
        self.last_accepted_version = version;
    }
}
