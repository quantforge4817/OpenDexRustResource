use crate::persistence;
use anyhow::Result;
use protocol::message::{Trade, TradeState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeBook {
    /// Trade records.  Production code should access through accessor methods
    /// (`all_trades`, `has_trade`, `get_trade`).  Direct access is allowed
    /// in tests for state inspection.
    pub trades: HashMap<uuid::Uuid, Trade>,

    #[serde(skip)]
    path: PathBuf,
}

impl TradeBook {
    pub fn new(path: PathBuf) -> Self {
        Self {
            trades: HashMap::new(),
            path,
        }
    }

    pub fn transition_trade(
        &mut self,
        trade_id: uuid::Uuid,
        next_state: TradeState,
        timestamp: u64,
    ) -> Result<bool> {
        let Some(trade) = self.trades.get_mut(&trade_id) else {
            return Ok(false);
        };

        trade.transition(next_state, timestamp)?;
        self.save_trades()?;

        Ok(true)
    }

    pub fn load_trades(path: PathBuf) -> Result<Self> {
        if !Path::new(&path).exists() {
            return Ok(Self::new(path));
        }

        let mut trade_book: TradeBook = persistence::read(&path)?;

        trade_book.path = path;

        trade_book.save_trades()?;

        Ok(trade_book)
    }

    pub fn save_trades(&self) -> Result<()> {
        persistence::write(&self.path, self)?;

        Ok(())
    }

    pub fn add_trade(&mut self, trade: Trade) -> bool {
        if self.trades.contains_key(&trade.id) {
            tracing::debug!(trade_id = %trade.id, "Trade already exists, ignoring");
            return false;
        }

        tracing::debug!(trade_id = %trade.id, "Adding trade");
        self.trades.insert(trade.id, trade);

        true
    }

    pub fn get_trade(&self, trade_id: uuid::Uuid) -> Option<&Trade> {
        self.trades.get(&trade_id)
    }

    pub fn print_trades(&self) {
        tracing::info!("=== OpenDEX Trades ===");

        for trade in self.trades.values() {
            tracing::info!("{:?}", trade);
        }
    }

    // -----------------------------------------------------------------
    // Accessors for sync.rs — provide controlled read access to internal
    // trade data without exposing the trades HashMap directly.
    // -----------------------------------------------------------------

    /// Returns all trades as a cloned vector.
    pub fn all_trades(&self) -> Vec<Trade> {
        self.trades.values().cloned().collect()
    }

    /// Check whether a trade ID is known.
    pub fn has_trade(&self, trade_id: &uuid::Uuid) -> bool {
        self.trades.contains_key(trade_id)
    }
}

// ---------------------------------------------------------------------
// Test-only accessors — allow test code to inspect internal state
// without exposing fields to production code.  These are prefixed with
// `_test_` to signal they are not part of the stable API.
// ---------------------------------------------------------------------
impl TradeBook {
    /// Returns a reference to the internal trades map (tests only).
    pub fn _test_trades(&self) -> &HashMap<uuid::Uuid, Trade> {
        &self.trades
    }

    /// Returns a mutable reference to the internal trades map (tests only).
    pub fn _test_trades_mut(&mut self) -> &mut HashMap<uuid::Uuid, Trade> {
        &mut self.trades
    }
}
