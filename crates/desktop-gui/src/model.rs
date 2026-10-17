use std::sync::{Arc, Mutex};

use protocol::message::{Asset, Order, Side, Trade, TradeState};
use uuid::Uuid;
use opendex_user::AuthService;

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

/// Order history entry
#[derive(Debug, Clone)]
pub struct OrderHistoryEntry {
    pub id: String,
    pub side: String,
    pub price: String,
    pub amount: String,
    pub timestamp: u64,
    pub status: String, // "Placed", "Filled", "Cancelled", "Failed"
}

/// Price alert configuration
#[derive(Debug, Clone)]
pub struct PriceAlert {
    pub target_price: f64,
    pub direction: AlertDirection,
    pub triggered: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlertDirection {
    Above,  // Alert when price goes above target
    Below,  // Alert when price goes below target
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

/// Authentication screen state
#[derive(Debug, Clone, PartialEq)]
pub enum AuthScreen {
    /// Show login form
    Login,
    /// Show registration form
    Register,
    /// Show email verification form
    VerifyEmail,
    /// Show profile page
    Profile,
    /// Show password reset form
    ResetPassword,
}

/// Reputation stats displayed in the GUI.
#[derive(Debug, Clone)]
pub struct ReputationView {
    pub tier: String,
    pub trust_score: u32,
    pub makes: u32,
    pub takes: u32,
    pub completed_trades: u32,
    pub disputes: u32,
    pub failures_to_respond: u32,
    pub average_rating: f64,
    pub rating_count: u32,
    pub response_rate: f64,
    pub review_count: usize,
}

impl ReputationView {
    pub fn empty() -> Self {
        Self {
            tier: "Newcomer".to_string(),
            trust_score: 0,
            makes: 0,
            takes: 0,
            completed_trades: 0,
            disputes: 0,
            failures_to_respond: 0,
            average_rating: 0.0,
            rating_count: 0,
            response_rate: 100.0,
            review_count: 0,
        }
    }
}

impl Default for ReputationView {
    fn default() -> Self {
        Self::empty()
    }
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

    // === Authentication State ===
    /// Shared auth service for user management
    pub auth_service: Arc<Mutex<AuthService>>,
    /// Current auth screen state
    pub auth_screen: AuthScreen,
    /// Login form fields
    pub login_username: String,
    pub login_password: String,
    /// Registration form fields
    pub reg_username: String,
    pub reg_password: String,
    pub reg_email: String,
    pub reg_btc_wallet: String,
    pub reg_xmr_wallet: String,
    /// Email verification code input
    pub verify_code: String,
    /// Auth status/error messages
    pub auth_message: Option<String>,
    /// Currently logged-in user ID (after successful login)
    pub logged_in_user_id: Option<opendex_user::UserId>,
    /// Current reputation stats for the logged-in user
    pub reputation: ReputationView,
    /// Order history log
    pub order_history: Vec<OrderHistoryEntry>,
    /// Show confirmation dialog before placing order
    pub show_confirm_dialog: bool,
    /// Price alerts
    pub price_alerts: Vec<PriceAlert>,
    /// Alert price input
    pub alert_price: String,
    /// Order book filter: min price (empty = no filter)
    pub filter_min_price: String,
    /// Order book filter: max price (empty = no filter)
    pub filter_max_price: String,
    /// Show depth visualization
    pub show_depth_chart: bool,
    /// Show profile panel
    pub show_profile: bool,
    /// Password reset fields
    pub reset_email: String,
    pub reset_code: String,
    pub reset_new_password: String,
    /// Theme preference
    pub theme_override: Option<ThemeMode>,
    /// Trading analytics for the current user
    pub analytics: TradingAnalytics,
    /// Network-wide statistics
    pub network_stats: NetworkStats,
    /// Show analytics panel
    pub show_analytics: bool,
    /// Show network stats panel
    pub show_network_stats: bool,
}

/// Theme mode setting
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
}

/// Trading analytics data for the current user
#[derive(Debug, Clone)]
pub struct TradingAnalytics {
    pub total_volume_btc: f64,
    pub total_volume_xmr: f64,
    pub total_trades: u32,
    pub completed_trades: u32,
    pub failed_trades: u32,
    pub win_rate: f64,       // percentage 0-100
    pub profit_loss_xmr: f64,
    pub average_trade_size: f64,
}

impl Default for TradingAnalytics {
    fn default() -> Self {
        Self {
            total_volume_btc: 0.0,
            total_volume_xmr: 0.0,
            total_trades: 0,
            completed_trades: 0,
            failed_trades: 0,
            win_rate: 0.0,
            profit_loss_xmr: 0.0,
            average_trade_size: 0.0,
        }
    }
}

/// Network statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    pub total_peers: usize,
    pub total_orders: usize,
    pub volume_24h_xmr: f64,
    pub average_trade_size: f64,
    pub active_trades: usize,
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
            // Auth state — persistent storage at ~/.opendex/users.json
            auth_service: Arc::new(Mutex::new(|| -> anyhow::Result<AuthService> {
                let home = std::env::var("HOME").unwrap_or_default();
                let dir = format!("{home}/.opendex");
                std::fs::create_dir_all(&dir).ok();
                let path = format!("{dir}/users.json");
                AuthService::with_storage(&path)
            }().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to load persistent user storage, using in-memory");
                AuthService::new()
            }))),
            auth_screen: AuthScreen::Login,
            login_username: String::new(),
            login_password: String::new(),
            reg_username: String::new(),
            reg_password: String::new(),
            reg_email: String::new(),
            reg_btc_wallet: String::new(),
            reg_xmr_wallet: String::new(),
            verify_code: String::new(),
            auth_message: None,
            logged_in_user_id: None,
            reputation: ReputationView::empty(),
            order_history: Vec::new(),
            show_confirm_dialog: false,
            price_alerts: Vec::new(),
            alert_price: String::new(),
            filter_min_price: String::new(),
            filter_max_price: String::new(),
            show_depth_chart: false,
            show_profile: false,
            reset_email: String::new(),
            reset_code: String::new(),
            reset_new_password: String::new(),
            theme_override: None,
            analytics: TradingAnalytics::default(),
            network_stats: NetworkStats::default(),
            show_analytics: false,
            show_network_stats: false,
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
