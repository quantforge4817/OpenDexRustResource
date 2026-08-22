use anyhow::Result;
use protocol::message::{Trade, TradeState};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeBook {
    pub trades: Vec<Trade>,

    #[serde(skip)]
    path: PathBuf,
}

impl TradeBook {
    pub fn new(path: PathBuf) -> Self {
        Self {
            trades: Vec::new(),
            path,
        }
    }
    pub fn transition_trade(
        &mut self,
        trade_id: uuid::Uuid,
        next_state: TradeState,
        timestamp: u64,
    ) -> Result<bool> {
        let Some(trade) = self.trades.iter_mut().find(|trade| trade.id == trade_id) else {
            return Ok(false);
        };

        trade.transition(next_state, timestamp)?;
        self.save_trades()?;

        Ok(true)
    }

    pub fn set_trade_state(
        &mut self,
        trade_id: uuid::Uuid,
        state: TradeState,
        timestamp: u64,
    ) -> Result<bool> {
        let Some(trade) = self.trades.iter_mut().find(|trade| trade.id == trade_id) else {
            return Ok(false);
        };

        trade.state = state;
        trade.updated_at = timestamp;

        self.save_trades()?;

        Ok(true)
    }

    pub fn load_trades(path: PathBuf) -> Result<Self> {
        if !Path::new(&path).exists() {
            return Ok(Self::new(path));
        }

        let data = fs::read_to_string(&path)?;
        let mut trade_book: TradeBook = serde_json::from_str(&data)?;

        trade_book.path = path;

        // Remove duplicate trade IDs even when they are
        // separated by other trades in the file.
        let mut seen = HashSet::new();

        trade_book
            .trades
            .retain(|trade| seen.insert(trade.id.clone()));

        trade_book.save_trades()?;

        Ok(trade_book)
    }

    pub fn save_trades(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&self.path, data)?;

        Ok(())
    }

    pub fn add_trade(&mut self, trade: Trade) -> bool {
        if self.trades.iter().any(|t| t.id == trade.id) {
            println!("Trade {} already exists, ignoring.", trade.id);
            return false;
        }

        println!("Adding trade {}", trade.id);
        self.trades.push(trade);

        true
    }

    pub fn get_trade(&self, trade_id: uuid::Uuid) -> Option<&Trade> {
        self.trades.iter().find(|trade| trade.id == trade_id)
    }

    pub fn print_trades(&self) {
        println!("=== OpenDEX Trades ===");

        for trade in &self.trades {
            println!("{:?}", trade);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::message::{AMOUNT_SCALE, Asset, Market, PRICE_SCALE, TradeState};
    use uuid::Uuid;

    fn test_trade() -> Trade {
        Trade {
            id: Uuid::new_v4(),
            buy_order: Uuid::new_v4(),
            sell_order: Uuid::new_v4(),
            maker: "maker".to_string(),
            taker: "taker".to_string(),
            buyer: "buyer".to_string(),
            seller: "seller".to_string(),
            market: Market {
                base: Asset::BTC,
                quote: Asset::XMR,
            },
            price: 100 * PRICE_SCALE,
            amount: AMOUNT_SCALE,
            state: TradeState::Created,
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn transition_trade_updates_state() {
        let path = std::env::temp_dir().join("opendex-test-trades-transition.json");

        let mut book = TradeBook::new(path);

        let trade = test_trade();
        let trade_id = trade.id;

        assert!(book.add_trade(trade));

        assert!(
            book.transition_trade(trade_id, TradeState::Accepted, 2,)
                .unwrap()
        );

        let stored = book
            .trades
            .iter()
            .find(|trade| trade.id == trade_id)
            .unwrap();

        assert_eq!(stored.state, TradeState::Accepted);
        assert_eq!(stored.updated_at, 2);
    }

    #[test]
    fn transition_trade_rejects_invalid_transition() {
        let path = std::env::temp_dir().join("opendex-test-trades-invalid-transition.json");

        let mut book = TradeBook::new(path);

        let trade = test_trade();
        let trade_id = trade.id;

        assert!(book.add_trade(trade));

        let result = book.transition_trade(trade_id, TradeState::Settled, 2);

        assert!(result.is_err());

        let stored = book
            .trades
            .iter()
            .find(|trade| trade.id == trade_id)
            .unwrap();

        assert_eq!(stored.state, TradeState::Created);
        assert_eq!(stored.updated_at, 1);
    }

    #[test]
    fn transition_trade_returns_false_for_unknown_trade() {
        let path = std::env::temp_dir().join("opendex-test-trades-unknown.json");

        let mut book = TradeBook::new(path);

        let result = book.transition_trade(Uuid::new_v4(), TradeState::Accepted, 2);

        assert!(!result.unwrap());
    }
}
