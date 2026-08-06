use anyhow::Result;
use protocol::message::Trade;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeBook {
    pub trades: Vec<Trade>,
}

impl TradeBook {
    pub fn new() -> Self {
        Self { trades: Vec::new() }
    }

    pub fn load_trades() -> Result<Self> {
        let path = "trades.json";

        if !Path::new(path).exists() {
            return Ok(Self::new());
        }

        let data = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save_trades(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write("trades.json", data)?;
        Ok(())
    }
    pub fn add_trade(&mut self, trade: Trade) -> Result<()> {
        if self.trades.iter().any(|t| t.id == trade.id) {
            println!("Trade {} already exists, ignoring.", trade.id);
            return Ok(());
        }

        println!("Adding trade {}", trade.id);

        self.trades.push(trade);

        self.save_trades()?;

        Ok(())
    }

    pub fn print_trades(&self) {
        println!("=== OpenDEX Trades ===");

        for trade in &self.trades {
            println!("{:?}", trade);
        }
    }
}
