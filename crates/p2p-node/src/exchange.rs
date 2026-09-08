use anyhow::Result;
use protocol::message::{Order, Trade};

use crate::orderbook::OrderBook;
use crate::trades::TradeBook;

pub struct Exchange<'a> {
    pub order_book: &'a mut OrderBook,
    pub trade_book: &'a mut TradeBook,
}

impl<'a> Exchange<'a> {
    pub fn new(order_book: &'a mut OrderBook, trade_book: &'a mut TradeBook) -> Self {
        Self {
            order_book,
            trade_book,
        }
    }

    pub fn submit_order(&mut self, order: Order) -> Result<Vec<Trade>> {
        let trades = self.order_book.match_order(&order);

        self.order_book.add_order(order);

        for trade in &trades {
            if !self.trade_book.add_trade(trade.clone()) {
                continue;
            }

            if !self.order_book.apply_trade(trade) {
                anyhow::bail!("trade {} could not be applied after matching", trade.id);
            }

            self.order_book
                .pending_trades
                .retain(|pending| pending.id != trade.id);
        }

        Ok(trades)
    }
}
