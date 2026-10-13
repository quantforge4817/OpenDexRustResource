use protocol::message::{Asset, Order, Side, Trade, TradeState};
use uuid::Uuid;

/// View model for the order book
#[derive(Debug, Clone, Default)]
pub struct OrderBookView {
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
}

/// View model for a single trade
#[derive(Debug, Clone)]
pub struct TradeView {
    pub id: Uuid,
    pub market: String,
    pub price: f64,
    pub amount: f64,
    pub state: TradeState,
    pub timestamp: u64,
}

impl From<Trade> for TradeView {
    fn from(trade: Trade) -> Self {
        Self {
            id: trade.id,
            market: trade.market.to_string(),
            price: trade.price as f64 / protocol::message::PRICE_SCALE as f64,
            amount: trade.amount as f64 / protocol::message::AMOUNT_SCALE as f64,
            state: trade.state,
            timestamp: trade.created_at,
        }
    }
}

/// View model for a balance
#[derive(Debug, Clone)]
pub struct BalanceView {
    pub asset: Asset,
    pub available: u64,
    pub locked: u64,
    pub escrowed: u64,
}

/// View model for a connected peer
#[derive(Debug, Clone)]
pub struct PeerView {
    pub address: String,
    pub connected: bool,
}

/// Trade form state
#[derive(Debug, Clone)]
pub struct TradeForm {
    pub side: Side,
    pub market: protocol::message::Market,
    pub price: String,
    pub amount: String,
}

impl Default for TradeForm {
    fn default() -> Self {
        Self {
            side: Side::Buy,
            market: protocol::message::Market {
                base: protocol::message::Asset::BTC,
                quote: protocol::message::Asset::XMR,
            },
            price: String::new(),
            amount: String::new(),
        }
    }
}

/// Node status information
#[derive(Debug, Clone, Default)]
pub struct NodeStatus {
    pub peer_id: String,
    pub listen_addr: String,
    pub peer_count: usize,
}

/// Updates from the node
#[derive(Debug, Clone)]
pub enum NodeUpdate {
    OrderBookUpdated(OrderBookView),
    TradeAdded(TradeView),
    TradeStateChanged { trade_id: Uuid, state: TradeState },
    BalancesUpdated(Vec<BalanceView>),
    PeerConnected(String),
    PeerDisconnected(String),
    OrderResult {
        order_id: Uuid,
        success: bool,
        error: Option<String>,
    },
    LogMessage(String),
    Error(String),
    WalletAddress(String),
    WalletBalance { available: f64, unlocked: f64 },
    WalletConnected { connected: bool, address: Option<String> },
    BtcWalletAddress(String),
    BtcWalletBalance { balance: f64 },
    BtcWalletConnected { connected: bool, address: Option<String> },
}

/// Main GUI model
#[derive(Clone)]
pub struct GuiModel {
    pub order_book: OrderBookView,
    pub trades: Vec<TradeView>,
    pub balances: Vec<BalanceView>,
    pub peers: Vec<PeerView>,
    pub trade_form: TradeForm,
    pub node_status: NodeStatus,
    pub logs: Vec<String>,
    /// Channel for sending commands to the P2P node
    pub command_tx: flume::Sender<crate::app::GuiCommand>,
    pub peer_input: String,
    pub fund_asset: Asset,
    pub fund_amount: String,
    pub status_message: Option<String>,
    /// Monero wallet connection status
    pub wallet_connected: bool,
    /// Monero wallet address
    pub wallet_address: Option<String>,
    /// Monero wallet available balance in XMR
    pub wallet_balance_available: Option<f64>,
    /// Monero wallet unlocked balance in XMR
    pub wallet_balance_unlocked: Option<f64>,
    /// Bitcoin wallet connection status
    pub btc_wallet_connected: bool,
    /// Bitcoin wallet address
    pub btc_wallet_address: Option<String>,
    /// Bitcoin wallet balance in BTC
    pub btc_wallet_balance: Option<f64>,
}

impl GuiModel {
    pub fn new(
        command_tx: flume::Sender<crate::app::GuiCommand>,
        peer_id: String,
        listen_addr: String,
    ) -> Self {
        Self {
            order_book: OrderBookView::default(),
            trades: Vec::new(),
            balances: Vec::new(),
            peers: Vec::new(),
            trade_form: TradeForm::default(),
            node_status: NodeStatus {
                peer_id,
                listen_addr,
                peer_count: 0,
            },
            logs: Vec::new(),
            command_tx,
            peer_input: String::new(),
            fund_asset: Asset::BTC,
            fund_amount: String::new(),
            status_message: None,
            wallet_connected: false,
            wallet_address: None,
            wallet_balance_available: None,
            wallet_balance_unlocked: None,
            btc_wallet_connected: false,
            btc_wallet_address: None,
            btc_wallet_balance: None,
        }
    }

    pub fn format_amount(amount: u64) -> String {
        let scale = protocol::message::AMOUNT_SCALE;
        let whole = amount / scale;
        let fractional = amount % scale;
        if fractional == 0 {
            whole.to_string()
        } else {
            format!("{whole}.{fractional:08}").trim_end_matches('0').to_string()
        }
    }

    pub fn format_price(price: u64) -> String {
        let scale = protocol::message::PRICE_SCALE;
        let whole = price / scale;
        let fractional = price % scale;
        if fractional == 0 {
            whole.to_string()
        } else {
            format!("{whole}.{fractional:08}").trim_end_matches('0').to_string()
        }
    }
}
