use anyhow::Result;
use protocol::message::{Order, Trade};

use crate::balances::BalanceBook;
use crate::orderbook::OrderBook;
use crate::trades::TradeBook;

pub struct SubmitOrder {
    pub actor: String,
    pub order: Order,
}

pub struct RecordTrade {
    pub actor: String,
    pub trade: Trade,
}

pub struct Exchange<'a> {
    pub order_book: &'a mut OrderBook,
    pub trade_book: &'a mut TradeBook,
    pub balance_book: &'a mut BalanceBook,
}

impl<'a> Exchange<'a> {
    pub fn new(
        order_book: &'a mut OrderBook,
        trade_book: &'a mut TradeBook,
        balance_book: &'a mut BalanceBook,
    ) -> Self {
        Self {
            order_book,
            trade_book,
            balance_book,
        }
    }

    pub fn submit_order(&mut self, operation: SubmitOrder) -> Result<Vec<Trade>> {
        if operation.actor != operation.order.trader {
            anyhow::bail!(
                "submit order actor is not the order owner: actor={}, trader={}",
                operation.actor,
                operation.order.trader
            );
        }

        let order = operation.order;
        let trades = self.order_book.match_order(&order);

        self.order_book.add_order(order);

        for trade in &trades {
            self.order_book.validate_trade_structure(trade)?;
            self.order_book.validate_trade_identity(trade)?;

            if !self.trade_book.add_trade(trade.clone()) {
                continue;
            }

            let buy_order = self
                .order_book
                .orders
                .iter()
                .find(|order| order.id == trade.buy_order)
                .cloned();

            let sell_order = self
                .order_book
                .orders
                .iter()
                .find(|order| order.id == trade.sell_order)
                .cloned();

            if !self.order_book.apply_trade(trade) {
                anyhow::bail!("trade {} could not be applied after matching", trade.id);
            }

            if let Some(order) = buy_order {
                let remaining_amount = order.amount.saturating_sub(trade.amount);

                let remaining_order = Order {
                    amount: remaining_amount,
                    ..order.clone()
                };

                let remaining_required =
                    self.balance_book.required_reservation(&remaining_order)?;

                if self
                    .balance_book
                    .order_reservation(&order.id)
                    .is_some()
                {
                    self.balance_book
                        .reconcile_order_reservation(&order, remaining_required)?;
                }
            }

            if let Some(order) = sell_order {
                let remaining_amount = order.amount.saturating_sub(trade.amount);

                if self
                    .balance_book
                    .order_reservation(&order.id)
                    .is_some()
                {
                    self.balance_book
                        .reconcile_order_reservation(&order, remaining_amount)?;
                }
            }

            self.order_book
                .pending_trades
                .retain(|pending| pending.id != trade.id);
        }

        Ok(trades)
    }

    pub fn record_trade(&mut self, operation: RecordTrade) -> Result<bool> {
        let trade = operation.trade;

        if operation.actor != trade.maker && operation.actor != trade.taker {
            anyhow::bail!(
                "record trade actor is not a participant: actor={}, trade={}",
                operation.actor,
                trade.id
            );
        }

        self.order_book.validate_trade_structure(&trade)?;
        self.order_book.validate_trade_identity(&trade)?;

        if !self.trade_book.add_trade(trade.clone()) {
            return Ok(false);
        }

        if self.order_book.apply_trade(&trade) {
            self.order_book
                .pending_trades
                .retain(|pending| pending.id != trade.id);
        }

        Ok(true)
    }
}
