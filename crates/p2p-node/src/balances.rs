use anyhow::{Result, anyhow};
use protocol::message::{Asset, Order, PRICE_SCALE, Side, Trade};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Balance {
    pub available: u64,
    pub locked: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceBook {
    pub balances: HashMap<String, HashMap<Asset, Balance>>,

    #[serde(default)]
    pub settled_trades: Vec<String>,

    #[serde(skip)]
    path: PathBuf,
}

fn quote_amount(price: u64, amount: u64) -> Result<u64> {
    let value = u128::from(price)
        .checked_mul(u128::from(amount))
        .ok_or_else(|| anyhow!("trade value overflow"))?;

    let value = value / u128::from(PRICE_SCALE);

    u64::try_from(value).map_err(|_| anyhow!("quote amount overflow"))
}

impl BalanceBook {
    pub fn new(path: PathBuf) -> Self {
        Self {
            balances: HashMap::new(),
            settled_trades: Vec::new(),
            path,
        }
    }

    pub fn load(path: PathBuf) -> Result<Self> {
        if !Path::new(&path).exists() {
            return Ok(Self::new(path));
        }

        let data = fs::read_to_string(&path)?;
        let mut book: BalanceBook = serde_json::from_str(&data)?;

        book.path = path;

        Ok(book)
    }

    pub fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&self.path, data)?;

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
    pub fn lock_for_order(&mut self, order: &Order) -> Result<()> {
        match order.side {
            Side::Buy => {
                let required = quote_amount(order.price, order.amount)?;

                self.lock(&order.trader, order.market.quote.clone(), required)?;
            }

            Side::Sell => {
                self.lock(&order.trader, order.market.base.clone(), order.amount)?;
            }
        }

        Ok(())
    }

    pub fn unlock_for_order(&mut self, order: &Order) -> Result<()> {
        match order.side {
            Side::Buy => {
                let amount = quote_amount(order.price, order.amount)?;

                self.unlock(&order.trader, order.market.quote.clone(), amount)?;
            }

            Side::Sell => {
                self.unlock(&order.trader, order.market.base.clone(), order.amount)?;
            }
        }

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

    pub fn transfer_locked(
        &mut self,
        from: &str,
        to: &str,
        asset: Asset,
        amount: u64,
    ) -> Result<()> {
        let from_balances = self.balances.entry(from.to_string()).or_default();

        let from_balance = from_balances.entry(asset.clone()).or_default();

        if from_balance.locked < amount {
            return Err(anyhow!("insufficient locked balance"));
        }

        from_balance.locked -= amount;

        let to_balances = self.balances.entry(to.to_string()).or_default();

        let to_balance = to_balances.entry(asset).or_default();

        to_balance.available = to_balance
            .available
            .checked_add(amount)
            .ok_or_else(|| anyhow!("balance overflow"))?;
        Ok(())
    }

    pub fn settle_trade(&mut self, local_trader: &str, trade: &Trade) -> Result<()> {
        let trade_id = trade.id.to_string();

        // Idempotency: a repeated Settled event must never move funds twice.
        if self.settled_trades.iter().any(|id| id == &trade_id) {
            return Ok(());
        }

        // Ignore trades where this node is neither participant.
        if local_trader != trade.buyer && local_trader != trade.seller {
            return Ok(());
        }

        let quote_amount = trade
            .price
            .checked_mul(trade.amount)
            .ok_or_else(|| anyhow!("trade quote amount overflow"))?
            / PRICE_SCALE;

        if local_trader == trade.buyer {
            // Buyer gives locked quote and receives base.
            self.consume_locked(local_trader, trade.market.quote.clone(), quote_amount)?;

            self.credit(local_trader, trade.market.base.clone(), trade.amount)?;
        }

        if local_trader == trade.seller {
            // Seller gives locked base and receives quote.
            self.consume_locked(local_trader, trade.market.base.clone(), trade.amount)?;

            self.credit(local_trader, trade.market.quote.clone(), quote_amount)?;
        }

        self.settled_trades.push(trade_id);

        Ok(())
    }
}
