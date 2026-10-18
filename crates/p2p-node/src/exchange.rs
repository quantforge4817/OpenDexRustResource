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

    fn reconcile_trade_reservation(
        &mut self,
        trade: &Trade,
        buy_order: Option<Order>,
        sell_order: Option<Order>,
    ) -> Result<()> {
        if let Some(order) = buy_order
            && self.balance_book.order_reservation(&order.id).is_some()
        {
            let remaining_amount = order.amount.saturating_sub(trade.amount);

            let remaining_order = Order {
                amount: remaining_amount,
                ..order.clone()
            };

            let remaining_required =
                self.balance_book.required_reservation(&remaining_order)?;

            self.balance_book
                .reconcile_order_reservation(&order, remaining_required)?;
        }

        if let Some(order) = sell_order
            && self.balance_book.order_reservation(&order.id).is_some()
        {
            let remaining_amount = order.amount.saturating_sub(trade.amount);

            self.balance_book
                .reconcile_order_reservation(&order, remaining_amount)?;
        }

        Ok(())
    }

    fn apply_trade_and_reconcile(
        &mut self,
        trade: &Trade,
        buy_order: Option<Order>,
        sell_order: Option<Order>,
    ) -> Result<bool> {
        if !self.order_book.apply_trade(trade) {
            return Ok(false);
        }

        self.reconcile_trade_reservation(trade, buy_order, sell_order)?;

        self.order_book.remove_pending_trade(trade.id);

        Ok(true)
    }

    fn add_order_and_reconcile_pending(&mut self, order: Order) -> Result<bool> {
        let incoming_order = order.clone();
        let pending_trades = self.order_book.upsert_order(order);
        let had_pending_trades = !pending_trades.is_empty();

        for trade in pending_trades {
            let buy_order = self
                .order_book
                .get_order(&trade.buy_order)
                .cloned()
                .or_else(|| {
                    if trade.buy_order == incoming_order.id {
                        Some(incoming_order.clone())
                    } else {
                        None
                    }
                });

            let sell_order = self
                .order_book
                .get_order(&trade.sell_order)
                .cloned()
                .or_else(|| {
                    if trade.sell_order == incoming_order.id {
                        Some(incoming_order.clone())
                    } else {
                        None
                    }
                });

            self.apply_trade_and_reconcile(&trade, buy_order, sell_order)?;
        }

        Ok(had_pending_trades)
    }

    pub fn submit_order(&mut self, operation: SubmitOrder) -> Result<Vec<Trade>> {
        if operation.actor != operation.order.trader {
            anyhow::bail!(
                "submit order actor is not the order owner: actor={}, trader={}",
                operation.actor,
                operation.order.trader
            );
        }

        // Reject orders with zero price or amount to prevent meaningless trades
        if operation.order.price == 0 {
            anyhow::bail!("order price must be greater than zero");
        }

        if operation.order.amount == 0 {
            anyhow::bail!("order amount must be greater than zero");
        }

        let order = operation.order;

        // Materialize the order and reconcile any remote trades that arrived
        // before the order itself. A pending remote trade already represents
        // the match for this arrival, so do not immediately create a second
        // local match from the remaining quantity.
        let had_pending_trades = self.add_order_and_reconcile_pending(order.clone())?;

        if had_pending_trades {
            return Ok(Vec::new());
        }

        let live_order = self.order_book.get_order(&order.id).cloned();

        let trades = live_order
            .map(|live_order| self.order_book.match_order(&live_order))
            .unwrap_or_default();

        for trade in &trades {
            self.order_book.validate_trade_structure(trade)?;
            self.order_book.validate_trade_identity(trade)?;

            if !self.trade_book.add_trade(trade.clone()) {
                continue;
            }

            // Capture the original orders before apply_trade() can remove
            // them after a full fill.
            let buy_order = self.order_book.get_order(&trade.buy_order).cloned();

            let sell_order = self.order_book.get_order(&trade.sell_order).cloned();

            self.apply_trade_and_reconcile(trade, buy_order, sell_order)?;
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

        // Capture the orders before apply_trade() can remove them after
        // a full fill.
        let buy_order = self.order_book.get_order(&trade.buy_order).cloned();

        let sell_order = self.order_book.get_order(&trade.sell_order).cloned();

        self.apply_trade_and_reconcile(&trade, buy_order, sell_order)?;

        Ok(true)
    }
}
