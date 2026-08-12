use anyhow::Result;
use protocol::message::Trade;
use serde::{Deserialize, Serialize};
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

    pub fn load_trades(path: PathBuf) -> Result<Self> {
        if !Path::new(&path).exists() {
            return Ok(Self::new(path));
        }

        let data = fs::read_to_string(&path)?;
        let mut trade_book: TradeBook = serde_json::from_str(&data)?;

        trade_book.path = path;

        trade_book.trades.dedup_by(|a, b| a.id == b.id);

        trade_book.save_trades()?;

        Ok(trade_book)
    }

    pub fn save_trades(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&self.path, data)?;

        Ok(())
    }

    pub fn add_trade(&mut self, trade: Trade) -> Result<()> {
        if self.trades.iter().any(|t| t.id == trade.id) {
            println!("Trade {} already exists, ignoring.", trade.id);
            return Ok(());
        }

        println!("Adding trade {}", trade.id);

        self.trades.push(trade);

        Ok(())
    }

    pub fn print_trades(&self) {
        println!("=== OpenDEX Trades ===");

        for trade in &self.trades {
            println!("{:?}", trade);
        }
    }
}
