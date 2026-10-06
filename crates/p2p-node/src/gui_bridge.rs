use flume::Sender;
use libp2p::PeerId;
use protocol::message::{Asset, Market, Order, Side, Trade, TradeState};
use std::collections::HashMap;
use uuid::Uuid;

use crate::balances::{Balance, BalanceBook};
use crate::orderbook::OrderBook;
use crate::trades::TradeBook;

/// Commands sent from the GUI to the P2P node
#[derive(Debug, Clone)]
pub enum GuiCommand {
    PlaceOrder {
        side: Side,
        market: Market,
        price: f64,
        amount: f64,
    },
    CancelOrder {
        order_id: Uuid,
    },
    Fund {
        asset: Asset,
        amount: f64,
    },
    ConnectPeer {
        multiaddr: String,
    },
    RequestBalances,
    RequestOrderBook,
    RequestTrades,
    AdvanceTradeState {
        trade_id: Uuid,
        state: TradeState,
    },
    /// Request the on-chain Monero wallet address
    RequestWalletAddress,
    /// Request the on-chain Monero wallet balance
    RequestWalletBalance,
    /// Request the on-chain Bitcoin wallet address
    RequestBtcWalletAddress,
    /// Request the on-chain Bitcoin wallet balance
    RequestBtcWalletBalance,
}

/// Updates sent from the P2P node to the GUI
#[derive(Debug, Clone)]
pub enum GuiUpdate {
    OrderBookSnapshot(Vec<Order>),
    TradeAdded(Trade),
    TradeStateChanged {
        trade_id: Uuid,
        state: TradeState,
    },
    BalancesUpdated(Vec<(String, Asset, Balance)>),
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    OrderResult {
        order_id: Uuid,
        success: bool,
        error: Option<String>,
    },
    LogMessage(String),
    Error(String),
    /// Monero wallet address retrieved
    WalletAddress(String),
    /// Monero wallet balance retrieved (available, unlocked in XMR)
    WalletBalance { available: f64, unlocked: f64 },
    /// Monero wallet connection status
    WalletConnected { connected: bool, address: Option<String> },
    /// Bitcoin wallet address retrieved
    BtcWalletAddress(String),
    /// Bitcoin wallet balance retrieved (in BTC)
    BtcWalletBalance { balance: f64 },
    /// Bitcoin wallet connection status
    BtcWalletConnected { connected: bool, address: Option<String> },
}

/// Holds the state needed for the node event loop
pub struct NodeState {
    pub order_book: OrderBook,
    pub trade_book: TradeBook,
    pub balance_book: BalanceBook,
    pub pending_trade_states: HashMap<Uuid, crate::message_handler::PendingTradeState>,
    pub local_peer_id: PeerId,
    pub update_tx: Sender<GuiUpdate>,
}

impl NodeState {
    pub fn new(
        order_book: OrderBook,
        trade_book: TradeBook,
        balance_book: BalanceBook,
        local_peer_id: PeerId,
        update_tx: Sender<GuiUpdate>,
    ) -> Self {
        Self {
            order_book,
            trade_book,
            balance_book,
            pending_trade_states: HashMap::new(),
            local_peer_id,
            update_tx,
        }
    }

    /// Send an update to the GUI, ignoring errors if the receiver is dropped
    pub fn send_update(&self, update: GuiUpdate) {
        let _ = self.update_tx.send(update);
    }

    /// Send a log message to the GUI
    pub fn log(&self, msg: impl std::fmt::Display) {
        self.send_update(GuiUpdate::LogMessage(msg.to_string()));
    }

    /// Send an error to the GUI
    pub fn error(&self, msg: impl std::fmt::Display) {
        self.send_update(GuiUpdate::Error(msg.to_string()));
    }

    /// Request current balances from the balance book
    pub fn request_balances(&self) {
        let trader = self.local_peer_id.to_string();
        let mut balances = Vec::new();
        for asset in [Asset::BTC, Asset::XMR] {
            let balance = self.balance_book.balance(&trader, &asset);
            balances.push((trader.clone(), asset, balance));
        }
        self.send_update(GuiUpdate::BalancesUpdated(balances));
    }

    /// Request current order book snapshot
    pub fn request_order_book(&self) {
        let orders = self.order_book.active_orders();
        self.send_update(GuiUpdate::OrderBookSnapshot(orders));
    }

    /// Request current trades
    pub fn request_trades(&self) {
        for trade in self.trade_book.all_trades() {
            self.send_update(GuiUpdate::TradeAdded(trade));
        }
    }
}
