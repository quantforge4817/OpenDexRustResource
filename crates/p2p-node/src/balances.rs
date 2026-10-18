use crate::persistence;
use anyhow::{Result, anyhow};
use protocol::message::{Asset, Order, PRICE_SCALE, Side};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Balance {
    pub available: u64,
    pub locked: u64,
    #[serde(default)]
    pub escrowed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceBook {
    pub balances: HashMap<String, HashMap<Asset, Balance>>,

    #[serde(default)]
    pub settled_trades: HashSet<Uuid>,

    #[serde(default)]
    pub escrowed_trades: HashSet<Uuid>,

    #[serde(default)]
    pub order_reservations: HashMap<Uuid, u64>,

    #[serde(skip)]
    path: PathBuf,
}

/// Compute the quote amount for a trade using round-half-up fixed-point arithmetic.
///
/// Rounding policy (AUDIT-007):
/// - The result is `price * amount / PRICE_SCALE` rounded to the nearest integer.
/// - When the remainder is exactly half the divisor, round up (round-half-up).
/// - This avoids systematic truncation bias that would favor one trading side
///   over many repeated fractional trades.
/// - The remainder (dust) is discarded after rounding; no dust accumulation
///   mechanism is needed because rounding is unbiased in expectation.
pub fn quote_amount(price: u64, amount: u64) -> Result<u64> {
    let numerator = u128::from(price)
        .checked_mul(u128::from(amount))
        .ok_or_else(|| anyhow!("trade value overflow"))?;

    let divisor = u128::from(PRICE_SCALE);

    // Round-half-up: (numerator + divisor/2) / divisor
    let rounded = (numerator + divisor / 2) / divisor;

    u64::try_from(rounded).map_err(|_| anyhow!("quote amount overflow"))
}

impl BalanceBook {
    pub fn new(path: PathBuf) -> Self {
        Self {
            balances: HashMap::new(),
            settled_trades: HashSet::new(),
            escrowed_trades: HashSet::new(),
            order_reservations: HashMap::new(),
            path,
        }
    }
    #[allow(dead_code)]
    pub fn order_reservation(&self, order_id: &Uuid) -> Option<u64> {
        self.order_reservations.get(order_id).copied()
    }

    pub fn load(path: PathBuf) -> Result<Self> {
        if !Path::new(&path).exists() {
            return Ok(Self::new(path));
        }
        let mut book: BalanceBook = persistence::read(&path)?;

        book.path = path;

        Ok(book)
    }

    pub fn save(&self) -> Result<()> {
        persistence::write(&self.path, self)?;

        Ok(())
    }

    pub fn balance(&self, trader: &str, asset: &Asset) -> Balance {
        self.balances
            .get(trader)
            .and_then(|balances| balances.get(asset))
            .copied()
            .unwrap_or_default()
    }

    pub fn credit(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()> {
        let balances = self.balances.entry(trader.to_string()).or_default();

        let balance = balances.entry(asset).or_default();

        balance.available = balance
            .available
            .checked_add(amount)
            .ok_or_else(|| anyhow!("balance overflow"))?;

        Ok(())
    }

    pub fn lock(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()> {
        let balances = self.balances.entry(trader.to_string()).or_default();

        let balance = balances.entry(asset).or_default();

        if balance.available < amount {
            return Err(anyhow!("insufficient available balance"));
        }

        balance.available -= amount;

        balance.locked = balance
            .locked
            .checked_add(amount)
            .ok_or_else(|| anyhow!("locked balance overflow"))?;

        Ok(())
    }
    pub fn required_reservation(&self, order: &Order) -> Result<u64> {
        match order.side {
            Side::Buy => quote_amount(order.price, order.amount),
            Side::Sell => Ok(order.amount),
        }
    }
    pub fn lock_for_order(&mut self, order: &Order) -> Result<()> {
        let required = self.required_reservation(order)?;

        if self.order_reservations.contains_key(&order.id) {
            return Err(anyhow!("order {} already has a reservation", order.id));
        }

        let asset = match order.side {
            Side::Buy => order.market.quote.clone(),
            Side::Sell => order.market.base.clone(),
        };

        self.lock(&order.trader, asset, required)?;
        self.order_reservations.insert(order.id, required);

        Ok(())
    }

    pub fn unlock_for_order(&mut self, order: &Order) -> Result<()> {
        let amount = self
            .order_reservations
            .get(&order.id)
            .copied()
            .ok_or_else(|| anyhow!("no reservation found for order {}", order.id))?;

        let asset = match order.side {
            Side::Buy => order.market.quote.clone(),
            Side::Sell => order.market.base.clone(),
        };

        self.unlock(&order.trader, asset, amount)?;
        self.order_reservations.remove(&order.id);

        Ok(())
    }

    pub fn unlock(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()> {
        let balances = self.balances.entry(trader.to_string()).or_default();

        let balance = balances.entry(asset).or_default();

        if balance.locked < amount {
            return Err(anyhow!("insufficient locked balance"));
        }

        balance.locked -= amount;

        balance.available = balance
            .available
            .checked_add(amount)
            .ok_or_else(|| anyhow!("balance overflow"))?;

        Ok(())
    }
    pub fn consume_locked(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()> {
        let balances = self.balances.entry(trader.to_string()).or_default();

        let balance = balances.entry(asset).or_default();

        if balance.locked < amount {
            return Err(anyhow!("insufficient locked balance"));
        }

        balance.locked -= amount;

        Ok(())
    }

    /// Consumes locked funds from a specific order's reservation.
    ///
    /// This is the per-order counterpart to `consume_locked`: instead of
    /// drawing from the aggregate locked pool, it verifies that the order's
    /// reservation covers the requested amount, reduces the reservation, and
    /// consumes the corresponding locked balance.
    ///
    /// After consumption the reservation is updated to reflect the remaining
    /// reserved amount.  When the reservation reaches zero it is removed.
    pub fn consume_locked_for_order(
        &mut self,
        order_id: &Uuid,
        trader: &str,
        asset: Asset,
        amount: u64,
    ) -> Result<()> {
        let current = self
            .order_reservations
            .get(order_id)
            .copied()
            .ok_or_else(|| anyhow!("no reservation found for order {}", order_id))?;

        if current < amount {
            return Err(anyhow!(
                "order {} reservation ({}) insufficient for consumption ({})",
                order_id,
                current,
                amount
            ));
        }

        let remaining = current - amount;

        // Consume from the aggregate locked balance.
        self.consume_locked(trader, asset, amount)?;

        // Update the per-order reservation.
        if remaining == 0 {
            self.order_reservations.remove(order_id);
        } else {
            self.order_reservations.insert(*order_id, remaining);
        }

        Ok(())
    }

    /// Moves funds from locked to escrowed.
    pub fn escrow_lock(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()> {
        let balances = self.balances.entry(trader.to_string()).or_default();

        let balance = balances.entry(asset).or_default();

        if balance.locked < amount {
            return Err(anyhow!("insufficient locked balance for escrow"));
        }

        balance.locked -= amount;

        balance.escrowed = balance
            .escrowed
            .checked_add(amount)
            .ok_or_else(|| anyhow!("escrowed balance overflow"))?;

        Ok(())
    }

    /// Moves funds from an order's locked reservation to escrowed.
    ///
    /// Verifies that the order's reservation covers the requested amount,
    /// reduces the reservation, and moves the corresponding locked balance
    /// to escrowed.
    pub fn escrow_lock_for_order(
        &mut self,
        order_id: &Uuid,
        trader: &str,
        asset: Asset,
        amount: u64,
    ) -> Result<()> {
        let current = self
            .order_reservations
            .get(order_id)
            .copied()
            .ok_or_else(|| anyhow!("no reservation found for order {}", order_id))?;

        if current < amount {
            return Err(anyhow!(
                "order {} reservation ({}) insufficient for escrow ({})",
                order_id,
                current,
                amount
            ));
        }

        let remaining = current - amount;

        // Move from locked to escrowed.
        self.escrow_lock(trader, asset, amount)?;

        // Update the per-order reservation.
        if remaining == 0 {
            self.order_reservations.remove(order_id);
        } else {
            self.order_reservations.insert(*order_id, remaining);
        }

        Ok(())
    }

    /// Moves funds from escrowed to available.
    pub fn escrow_release(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()> {
        let balances = self.balances.entry(trader.to_string()).or_default();

        let balance = balances.entry(asset).or_default();

        if balance.escrowed < amount {
            return Err(anyhow!("insufficient escrowed balance for release"));
        }

        balance.escrowed -= amount;

        balance.available = balance
            .available
            .checked_add(amount)
            .ok_or_else(|| anyhow!("available balance overflow"))?;

        Ok(())
    }

    /// Moves funds from escrowed back to locked.
    pub fn escrow_return(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()> {
        let balances = self.balances.entry(trader.to_string()).or_default();

        let balance = balances.entry(asset).or_default();

        if balance.escrowed < amount {
            return Err(anyhow!("insufficient escrowed balance for return"));
        }

        balance.escrowed -= amount;

        balance.locked = balance
            .locked
            .checked_add(amount)
            .ok_or_else(|| anyhow!("locked balance overflow"))?;

        Ok(())
    }
    pub fn reconcile_order_reservation(
        &mut self,
        order: &Order,
        remaining_required: u64,
    ) -> Result<()> {
        let current = self
            .order_reservations
            .get(&order.id)
            .copied()
            .ok_or_else(|| anyhow!("no reservation found for order {}", order.id))?;

        if remaining_required > current {
            return Err(anyhow!(
                "remaining reservation exceeds current reservation for order {}",
                order.id
            ));
        }

        // Update the per-order reservation to reflect the remaining required
        // amount.  The locked funds are not released here — they remain locked
        // until settlement or escrow consumes them via
        // `consume_locked_for_order` / `escrow_lock_for_order`, or until the
        // order is cancelled via `unlock_for_order`.
        if remaining_required == 0 {
            self.order_reservations.remove(&order.id);
        } else {
            self.order_reservations.insert(order.id, remaining_required);
        }
        Ok(())
    }
}
