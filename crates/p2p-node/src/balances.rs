use anyhow::{Result, anyhow};
use protocol::message::{Asset, Order, PRICE_SCALE, Side, Trade, TradeState};
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

    pub fn debit_available(&mut self, trader: &str, asset: Asset, amount: u64) -> Result<()> {
        let balances = self.balances.entry(trader.to_string()).or_default();

        let balance = balances.entry(asset).or_default();

        if balance.available < amount {
            return Err(anyhow!("insufficient available balance"));
        }

        balance.available -= amount;

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

        // A trade must only settle once locally.
        if self.settled_trades.contains(&trade_id) {
            return Ok(());
        }

        // Settlement is only valid for a completed trade.
        if trade.state != TradeState::Settled {
            return Err(anyhow!(
                "cannot settle trade {} while state is {:?}",
                trade.id,
                trade.state
            ));
        }

        let quote_amount = quote_amount(trade.price, trade.amount)?;

        if local_trader == trade.buyer {
            // Buyer spends the quote asset that was locked for the order.
            {
                let balances = self.balances.entry(local_trader.to_string()).or_default();

                let quote_balance = balances.entry(trade.market.quote.clone()).or_default();

                if quote_balance.locked < quote_amount {
                    return Err(anyhow!(
                        "insufficient locked quote balance for trade {}",
                        trade.id
                    ));
                }

                quote_balance.locked -= quote_amount;
            }

            // Buyer receives the base asset.
            self.credit(local_trader, trade.market.base.clone(), trade.amount)?;
        } else if local_trader == trade.seller {
            // Seller spends the base asset that was locked for the order.
            {
                let balances = self.balances.entry(local_trader.to_string()).or_default();

                let base_balance = balances.entry(trade.market.base.clone()).or_default();

                if base_balance.locked < trade.amount {
                    return Err(anyhow!(
                        "insufficient locked base balance for trade {}",
                        trade.id
                    ));
                }

                base_balance.locked -= trade.amount;
            }

            // Seller receives the quote asset.
            self.credit(local_trader, trade.market.quote.clone(), quote_amount)?;
        } else {
            // This node does not own either side of the trade.
            return Ok(());
        }

        // Record settlement only after local settlement succeeded.
        self.settled_trades.push(trade_id);

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use protocol::message::{Asset, Market, TradeState};

    #[test]
    fn credit_adds_available_balance() {
        let path = std::env::temp_dir().join("opendex-test-balances-credit.json");

        let mut book = BalanceBook::new(path);

        book.credit("alice", Asset::BTC, 10).unwrap();

        assert_eq!(
            book.balance("alice", &Asset::BTC),
            Balance {
                available: 10,
                locked: 0,
            }
        );
    }

    #[test]
    fn lock_moves_available_to_locked() {
        let path = std::env::temp_dir().join("opendex-test-balances-lock.json");

        let mut book = BalanceBook::new(path);

        book.credit("alice", Asset::BTC, 10).unwrap();
        book.lock("alice", Asset::BTC, 4).unwrap();

        assert_eq!(
            book.balance("alice", &Asset::BTC),
            Balance {
                available: 6,
                locked: 4,
            }
        );
    }

    #[test]
    fn unlock_moves_locked_back_to_available() {
        let path = std::env::temp_dir().join("opendex-test-balances-unlock.json");

        let mut book = BalanceBook::new(path);

        book.credit("alice", Asset::BTC, 10).unwrap();
        book.lock("alice", Asset::BTC, 4).unwrap();
        book.unlock("alice", Asset::BTC, 3).unwrap();

        assert_eq!(
            book.balance("alice", &Asset::BTC),
            Balance {
                available: 9,
                locked: 1,
            }
        );
    }

    #[test]
    fn cannot_lock_more_than_available() {
        let path = std::env::temp_dir().join("opendex-test-balances-insufficient.json");

        let mut book = BalanceBook::new(path);

        book.credit("alice", Asset::BTC, 5).unwrap();

        assert!(book.lock("alice", Asset::BTC, 6).is_err());

        assert_eq!(
            book.balance("alice", &Asset::BTC),
            Balance {
                available: 5,
                locked: 0,
            }
        );
    }

    #[test]
    fn locked_transfer_moves_funds_between_traders() {
        let path = std::env::temp_dir().join("opendex-test-balances-transfer.json");

        let mut book = BalanceBook::new(path);

        book.credit("alice", Asset::BTC, 10).unwrap();
        book.lock("alice", Asset::BTC, 7).unwrap();

        book.transfer_locked("alice", "bob", Asset::BTC, 7).unwrap();

        assert_eq!(
            book.balance("alice", &Asset::BTC),
            Balance {
                available: 3,
                locked: 0,
            }
        );

        assert_eq!(
            book.balance("bob", &Asset::BTC),
            Balance {
                available: 7,
                locked: 0,
            }
        );
    }
    #[test]
    fn lock_for_order_reserves_correct_asset() {
        let path = std::env::temp_dir().join("opendex-test-lock-for-order.json");

        let mut book = BalanceBook::new(path);

        // Values in an Order are fixed-point scaled.
        // Buy 2 BTC at 100 XMR/BTC => lock 200 XMR.
        let price = 100 * protocol::message::PRICE_SCALE;
        let amount = 2 * protocol::message::AMOUNT_SCALE;

        book.credit("alice", Asset::XMR, 250 * protocol::message::AMOUNT_SCALE)
            .unwrap();

        let buy_order = Order {
            id: uuid::Uuid::new_v4(),
            trader: "alice".to_string(),
            market: protocol::message::Market {
                base: Asset::BTC,
                quote: Asset::XMR,
            },
            side: Side::Buy,
            price,
            amount,
        };

        book.lock_for_order(&buy_order).unwrap();

        assert_eq!(
            book.balance("alice", &Asset::XMR),
            Balance {
                available: 50 * protocol::message::AMOUNT_SCALE,
                locked: 200 * protocol::message::AMOUNT_SCALE,
            }
        );

        // Sell 3 BTC => lock 3 BTC.
        book.credit("alice", Asset::BTC, 5 * protocol::message::AMOUNT_SCALE)
            .unwrap();

        let sell_order = Order {
            id: uuid::Uuid::new_v4(),
            trader: "alice".to_string(),
            market: protocol::message::Market {
                base: Asset::BTC,
                quote: Asset::XMR,
            },
            side: Side::Sell,
            price,
            amount: 3 * protocol::message::AMOUNT_SCALE,
        };

        book.lock_for_order(&sell_order).unwrap();

        assert_eq!(
            book.balance("alice", &Asset::BTC),
            Balance {
                available: 2 * protocol::message::AMOUNT_SCALE,
                locked: 3 * protocol::message::AMOUNT_SCALE,
            }
        );
    }
    #[test]
    fn settle_trade_is_local_and_idempotent() {
        let alice_path = std::env::temp_dir().join(format!(
            "opendex-test-settle-alice-{}.json",
            uuid::Uuid::new_v4()
        ));

        let bob_path = std::env::temp_dir().join(format!(
            "opendex-test-settle-bob-{}.json",
            uuid::Uuid::new_v4()
        ));

        let mut alice_book = BalanceBook::new(alice_path);
        let mut bob_book = BalanceBook::new(bob_path);

        let price = 100 * protocol::message::PRICE_SCALE;
        let amount = 2 * protocol::message::AMOUNT_SCALE;

        // Alice is the buyer and locks the quote asset.
        alice_book
            .credit("alice", Asset::XMR, 300 * protocol::message::AMOUNT_SCALE)
            .unwrap();

        alice_book
            .lock("alice", Asset::XMR, 200 * protocol::message::AMOUNT_SCALE)
            .unwrap();

        // Bob is the seller and locks the base asset.
        bob_book
            .credit("bob", Asset::BTC, 3 * protocol::message::AMOUNT_SCALE)
            .unwrap();

        bob_book
            .lock("bob", Asset::BTC, 2 * protocol::message::AMOUNT_SCALE)
            .unwrap();

        let trade = Trade {
            id: uuid::Uuid::new_v4(),
            buy_order: uuid::Uuid::new_v4(),
            sell_order: uuid::Uuid::new_v4(),
            maker: "bob".to_string(),
            taker: "alice".to_string(),
            buyer: "alice".to_string(),
            seller: "bob".to_string(),
            market: Market {
                base: Asset::BTC,
                quote: Asset::XMR,
            },
            price,
            amount,
            state: TradeState::Settled,
            created_at: 1,
            updated_at: 1,
        };

        // Alice's node settles only Alice's side.
        alice_book.settle_trade("alice", &trade).unwrap();

        assert_eq!(
            alice_book.balance("alice", &Asset::BTC),
            Balance {
                available: 2 * protocol::message::AMOUNT_SCALE,
                locked: 0,
            }
        );

        assert_eq!(
            alice_book.balance("alice", &Asset::XMR),
            Balance {
                available: 100 * protocol::message::AMOUNT_SCALE,
                locked: 0,
            }
        );

        // Bob's node settles only Bob's side.
        bob_book.settle_trade("bob", &trade).unwrap();

        assert_eq!(
            bob_book.balance("bob", &Asset::BTC),
            Balance {
                available: protocol::message::AMOUNT_SCALE,
                locked: 0,
            }
        );

        assert_eq!(
            bob_book.balance("bob", &Asset::XMR),
            Balance {
                available: 200 * protocol::message::AMOUNT_SCALE,
                locked: 0,
            }
        );

        // Deliver the exact same trade to Alice again.
        alice_book.settle_trade("alice", &trade).unwrap();

        // Alice's balances must not change.
        assert_eq!(
            alice_book.balance("alice", &Asset::BTC),
            Balance {
                available: 2 * protocol::message::AMOUNT_SCALE,
                locked: 0,
            }
        );

        assert_eq!(
            alice_book.balance("alice", &Asset::XMR),
            Balance {
                available: 100 * protocol::message::AMOUNT_SCALE,
                locked: 0,
            }
        );

        assert_eq!(alice_book.settled_trades.len(), 1);
        assert_eq!(alice_book.settled_trades[0], trade.id.to_string());

        // Deliver the exact same trade to Bob again.
        bob_book.settle_trade("bob", &trade).unwrap();

        // Bob's balances must not change.
        assert_eq!(
            bob_book.balance("bob", &Asset::BTC),
            Balance {
                available: protocol::message::AMOUNT_SCALE,
                locked: 0,
            }
        );

        assert_eq!(
            bob_book.balance("bob", &Asset::XMR),
            Balance {
                available: 200 * protocol::message::AMOUNT_SCALE,
                locked: 0,
            }
        );

        assert_eq!(bob_book.settled_trades.len(), 1);
        assert_eq!(bob_book.settled_trades[0], trade.id.to_string());
    }
}
