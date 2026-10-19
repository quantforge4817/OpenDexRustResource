use crate::model::{
    AlertDirection, AuthScreen, GuiModel, NodeUpdate, OrderHistoryEntry, PeerView,
    PriceAlert, ThemeMode,
};
use crate::view::{auth, order_book, peers, reputation, status_bar, trade_form, trades, wallet};
use iced::widget::{Column, Row};
use iced::{Element, Length, Task};
use protocol::message::{Asset, Market, Side, TradeState};
use std::sync::Arc;
use uuid::Uuid;

/// Commands sent from the GUI to the P2P node.
/// This mirrors the GuiCommand enum in p2p-node's gui_bridge module.
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
    RequestWalletAddress,
    RequestWalletBalance,
    RequestBtcWalletAddress,
    RequestBtcWalletBalance,
}

/// A balance snapshot sent from the node to the GUI.
#[derive(Debug, Clone)]
pub struct BalanceUpdate {
    pub asset: Asset,
    pub available: u64,
    pub locked: u64,
    pub escrowed: u64,
}

/// Updates sent from the P2P node to the GUI.
/// This mirrors the GuiUpdate enum in p2p-node's gui_bridge module.
/// Uses self-contained types so the GUI crate does not depend on p2p-node.
#[derive(Debug, Clone)]
pub enum GuiUpdate {
    OrderBookSnapshot(Vec<protocol::message::Order>),
    TradeAdded(protocol::message::Trade),
    TradeStateChanged {
        trade_id: Uuid,
        state: TradeState,
    },
    BalancesUpdated(Vec<BalanceUpdate>),
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
    WalletBalance {
        available: f64,
        unlocked: f64,
    },
    WalletConnected {
        connected: bool,
        address: Option<String>,
    },
    BtcWalletAddress(String),
    BtcWalletBalance {
        balance: f64,
    },
    BtcWalletConnected {
        connected: bool,
        address: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    SideSelected(String),
    MarketSelected(String),
    PriceChanged(String),
    AmountChanged(String),
    SubmitOrder,
    ConfirmOrder,
    CancelConfirmOrder,
    PeerInputChanged(String),
    ConnectPeer,
    ConnectBootstrap(String),
    FundAssetSelected(String),
    FundAmountChanged(String),
    FundSubmit,
    RefreshBalances,
    RefreshOrderBook,
    RefreshTrades,
    AdvanceTradeState { trade_id: Uuid, state: TradeState },
    NodeUpdate(NodeUpdate),
    // Auth messages
    LoginUsernameChanged(String),
    LoginPasswordChanged(String),
    LoginSubmit,
    LoginSuccess(opendex_user::LoginResult),
    LoginError(String),
    SwitchToRegister,
    SwitchToLogin,
    RegUsernameChanged(String),
    RegPasswordChanged(String),
    RegEmailChanged(String),
    RegBtcWalletChanged(String),
    RegXmrWalletChanged(String),
    RegisterSubmit,
    RegisterSuccess(opendex_user::RegistrationResult),
    RegisterError(String),
    VerifyCodeChanged(String),
    VerifyEmailSubmit,
    VerifySuccess,
    VerifyError(String),
    ResendVerificationCode,
    // Price alerts
    AlertPriceChanged(String),
    AddPriceAlert,
    RemovePriceAlert(usize),
    // Order book filters
    FilterMinPriceChanged(String),
    FilterMaxPriceChanged(String),
    // Depth chart toggle
    ToggleDepthChart,
    // Profile
    ToggleProfile,
    ResetEmailChanged(String),
    ResetCodeChanged(String),
    ResetPasswordChanged(String),
    SendResetCode,
    ConfirmPasswordReset,
    // Theme
    ToggleTheme,
    // Logout
    Logout,
    // Keyboard shortcuts
    KeyboardShortcutNewOrder,
    KeyboardShortcutPeers,
    KeyboardShortcutWallet,
    KeyboardShortcutHistory,
    // Analytics
    ToggleAnalytics,
    ToggleNetworkStats,
    ExportTradeHistoryCsv,
    // Blockchain explorer links
    OpenBtcExplorer(Uuid),
    OpenXmrExplorer(Uuid),
}

#[derive(Clone)]
pub struct Flags {
    pub cmd_tx: flume::Sender<GuiCommand>,
    pub update_rx: flume::Receiver<GuiUpdate>,
    pub peer_id: String,
    pub listen_addr: String,
}

#[derive(Default)]
pub struct App {
    model: Option<GuiModel>,
}

fn boot(flags: Flags) -> (App, Task<Message>) {
    let model = GuiModel::new(flags.cmd_tx.clone(), flags.peer_id, flags.listen_addr);
    let app = App { model: Some(model) };

    let cmd_tx = flags.cmd_tx.clone();
    let init_cmd = Task::perform(
        async move {
            let _ = cmd_tx.send(GuiCommand::RequestBalances);
            let _ = cmd_tx.send(GuiCommand::RequestOrderBook);
            let _ = cmd_tx.send(GuiCommand::RequestTrades);
        },
        |_| Message::NodeUpdate(NodeUpdate::LogMessage("Initial data requested".to_string())),
    );

    (app, init_cmd)
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    let model = if let Some(m) = app.model.as_mut() {
        m
    } else {
        return Task::none();
    };

    match message {
        Message::SideSelected(label) => {
            model.trade_form.side = if label == "Buy" {
                Side::Buy
            } else {
                Side::Sell
            };
        }
        Message::MarketSelected(label) => {
            model.trade_form.market = if label == "BTC/XMR" {
                Market {
                    base: Asset::BTC,
                    quote: Asset::XMR,
                }
            } else {
                model.trade_form.market.clone()
            };
        }
        Message::PriceChanged(value) => {
            model.trade_form.price = value;
        }
        Message::AmountChanged(value) => {
            model.trade_form.amount = value;
        }
        Message::SubmitOrder => {
            let price: f64 = model.trade_form.price.parse().unwrap_or(0.0);
            let amount: f64 = model.trade_form.amount.parse().unwrap_or(0.0);
            if price <= 0.0 || amount <= 0.0 {
                model.status_message = Some("Invalid price or amount".to_string());
                return Task::none();
            }
            // Show confirmation dialog
            model.show_confirm_dialog = true;
        }
        Message::ConfirmOrder => {
            model.show_confirm_dialog = false;
            let price: f64 = model.trade_form.price.parse().unwrap_or(0.0);
            let amount: f64 = model.trade_form.amount.parse().unwrap_or(0.0);
            if price <= 0.0 || amount <= 0.0 {
                model.status_message = Some("Invalid price or amount".to_string());
                return Task::none();
            }
            let market = model.trade_form.market.clone();
            let side = model.trade_form.side.clone();
            let cmd = GuiCommand::PlaceOrder {
                side: model.trade_form.side.clone(),
                market,
                price,
                amount,
            };
            let tx = model.command_tx.clone();
            // Record in order history
            let entry = OrderHistoryEntry {
                id: format!("{}", uuid::Uuid::new_v4()),
                side: match side {
                    Side::Buy => "Buy".to_string(),
                    Side::Sell => "Sell".to_string(),
                },
                price: model.trade_form.price.clone(),
                amount: model.trade_form.amount.clone(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                status: "Placed".to_string(),
            };
            model.order_history.push(entry);
            return Task::perform(
                async move {
                    let _ = tx.send(cmd);
                },
                |_| {
                    Message::NodeUpdate(NodeUpdate::LogMessage(
                        "Order confirmed and submitted".to_string(),
                    ))
                },
            );
        }
        Message::CancelConfirmOrder => {
            model.show_confirm_dialog = false;
        }
        Message::PeerInputChanged(value) => {
            model.peer_input = value;
        }
        Message::ConnectPeer => {
            let addr = model.peer_input.clone();
            if addr.is_empty() {
                model.status_message = Some("Enter a peer address".to_string());
                return Task::none();
            }
            let cmd = GuiCommand::ConnectPeer { multiaddr: addr };
            let tx = model.command_tx.clone();
            return Task::perform(
                async move {
                    let _ = tx.send(cmd);
                },
                |_| Message::NodeUpdate(NodeUpdate::LogMessage("Connect command sent".to_string())),
            );
        }
        Message::ConnectBootstrap(addr) => {
            // Fill the peer input and connect
            model.peer_input = addr.clone();
            let cmd = GuiCommand::ConnectPeer { multiaddr: addr };
            let tx = model.command_tx.clone();
            return Task::perform(
                async move {
                    let _ = tx.send(cmd);
                },
                |_| {
                    Message::NodeUpdate(NodeUpdate::LogMessage(
                        "Connecting to bootstrap node...".to_string(),
                    ))
                },
            );
        }
        Message::FundAssetSelected(label) => {
            model.fund_asset = if label == "BTC" {
                Asset::BTC
            } else {
                Asset::XMR
            };
        }
        Message::FundAmountChanged(value) => {
            model.fund_amount = value;
        }
        Message::FundSubmit => {
            let amount: f64 = model.fund_amount.parse().unwrap_or(0.0);
            if amount <= 0.0 {
                model.status_message = Some("Invalid amount".to_string());
                return Task::none();
            }
            let cmd = GuiCommand::Fund {
                asset: model.fund_asset.clone(),
                amount,
            };
            let tx = model.command_tx.clone();
            return Task::perform(
                async move {
                    let _ = tx.send(cmd);
                },
                |_| Message::NodeUpdate(NodeUpdate::LogMessage("Fund command sent".to_string())),
            );
        }
        Message::RefreshBalances => {
            let tx = model.command_tx.clone();
            return Task::perform(
                async move {
                    let _ = tx.send(GuiCommand::RequestBalances);
                },
                |_| {
                    Message::NodeUpdate(NodeUpdate::LogMessage(
                        "Refreshing balances...".to_string(),
                    ))
                },
            );
        }
        Message::RefreshOrderBook => {
            let tx = model.command_tx.clone();
            return Task::perform(
                async move {
                    let _ = tx.send(GuiCommand::RequestOrderBook);
                },
                |_| {
                    Message::NodeUpdate(NodeUpdate::LogMessage(
                        "Refreshing order book...".to_string(),
                    ))
                },
            );
        }
        Message::RefreshTrades => {
            let tx = model.command_tx.clone();
            return Task::perform(
                async move {
                    let _ = tx.send(GuiCommand::RequestTrades);
                },
                |_| Message::NodeUpdate(NodeUpdate::LogMessage("Refreshing trades...".to_string())),
            );
        }
        Message::AdvanceTradeState { trade_id, state } => {
            let cmd = GuiCommand::AdvanceTradeState { trade_id, state };
            let tx = model.command_tx.clone();
            return Task::perform(
                async move {
                    let _ = tx.send(cmd);
                },
                |_| {
                    Message::NodeUpdate(NodeUpdate::LogMessage(
                        "Trade state advance sent".to_string(),
                    ))
                },
            );
        }
        Message::NodeUpdate(update) => match update {
            NodeUpdate::OrderBookUpdated(ob) => {
                model.order_book = ob;
            }
            NodeUpdate::TradeAdded(trade) => {
                if !model.trades.iter().any(|t| t.id == trade.id) {
                    model.trades.push(trade);
                }
            }
            NodeUpdate::TradeStateChanged { trade_id, state } => {
                if let Some(trade) = model.trades.iter_mut().find(|t| t.id == trade_id) {
                    trade.state = state;
                }
            }
            NodeUpdate::BalancesUpdated(balances) => {
                model.balances = balances;
            }
            NodeUpdate::PeerConnected(addr) => {
                if !model.peers.iter().any(|p| p.address == addr) {
                    model.peers.push(PeerView {
                        address: addr.clone(),
                        connected: true,
                    });
                }
                model.node_status.peer_count = model.peers.len();
            }
            NodeUpdate::PeerDisconnected(addr) => {
                model.peers.retain(|p| p.address != addr);
                model.node_status.peer_count = model.peers.len();
            }
            NodeUpdate::OrderResult {
                order_id,
                success,
                error,
            } => {
                if success {
                    model.status_message =
                        Some(format!("Order {} placed", &order_id.to_string()[..8]));
                    model.trade_form.price.clear();
                    model.trade_form.amount.clear();
                } else {
                    model.status_message = Some(format!("Order failed: {:?}", error));
                }
            }
            NodeUpdate::LogMessage(msg) => {
                model.logs.push(msg.clone());
                if model.logs.len() > 100 {
                    model.logs.drain(..10);
                }
                model.status_message = Some(msg);
            }
            NodeUpdate::Error(msg) => {
                model.logs.push(format!("ERROR: {}", msg));
                model.status_message = Some(format!("Error: {}", msg));
            }
            NodeUpdate::WalletAddress(addr) => {
                model.wallet_address = Some(addr.clone());
                model.wallet_connected = true;
                model.status_message = Some(format!("Wallet address: {}", &addr[..20]));
            }
            NodeUpdate::WalletBalance {
                available,
                unlocked,
            } => {
                model.wallet_balance_available = Some(available);
                model.wallet_balance_unlocked = Some(unlocked);
                model.status_message = Some(format!(
                    "Wallet balance: {:.4} XMR (unlocked: {:.4})",
                    available, unlocked
                ));
            }
            NodeUpdate::WalletConnected { connected, address } => {
                model.wallet_connected = connected;
                model.wallet_address = address;
                model.status_message = Some(if connected {
                    "Monero wallet connected".to_string()
                } else {
                    "Monero wallet disconnected".to_string()
                });
            }
            NodeUpdate::BtcWalletAddress(addr) => {
                model.btc_wallet_address = Some(addr.clone());
                model.btc_wallet_connected = true;
                model.status_message = Some(format!("BTC wallet address: {}", &addr[..20]));
            }
            NodeUpdate::BtcWalletBalance { balance } => {
                model.btc_wallet_balance = Some(balance);
                model.status_message = Some(format!("BTC wallet balance: {:.8} BTC", balance));
            }
            NodeUpdate::BtcWalletConnected { connected, address } => {
                model.btc_wallet_connected = connected;
                model.btc_wallet_address = address;
                model.status_message = Some(if connected {
                    "Bitcoin wallet connected".to_string()
                } else {
                    "Bitcoin wallet disconnected".to_string()
                });
            }
        },
        // === Auth Messages ===
        Message::LoginUsernameChanged(v) => {
            model.login_username = v;
            model.auth_message = None;
        }
        Message::LoginPasswordChanged(v) => {
            model.login_password = zeroize::Zeroizing::new(v);
            model.auth_message = None;
        }
        Message::LoginSubmit => {
            let username = model.login_username.clone();
            let password = model.login_password.clone();
            let auth = model.auth_service.clone();
            return Task::perform(
                async move {
                    let mut auth = auth.lock().unwrap();
                    auth.login(&username, &password)
                },
                move |result| match result {
                    Ok(login_result) => Message::LoginSuccess(login_result),
                    Err(e) => Message::LoginError(format!("{}", e)),
                },
            );
        }
        Message::SwitchToRegister => {
            model.auth_screen = AuthScreen::Register;
            model.auth_message = None;
        }
        Message::SwitchToLogin => {
            model.auth_screen = AuthScreen::Login;
            model.auth_message = None;
        }
        Message::RegUsernameChanged(v) => {
            model.reg_username = v;
            model.auth_message = None;
        }
        Message::RegPasswordChanged(v) => {
            model.reg_password = zeroize::Zeroizing::new(v);
            model.auth_message = None;
        }
        Message::RegEmailChanged(v) => {
            model.reg_email = v;
            model.auth_message = None;
        }
        Message::RegBtcWalletChanged(v) => {
            model.reg_btc_wallet = v;
            model.auth_message = None;
        }
        Message::RegXmrWalletChanged(v) => {
            model.reg_xmr_wallet = v;
            model.auth_message = None;
        }
        Message::RegisterSubmit => {
            let username = model.reg_username.clone();
            let password = model.reg_password.clone();
            let email = model.reg_email.clone();
            let btc_wallet = if model.reg_btc_wallet.is_empty() {
                None
            } else {
                Some(model.reg_btc_wallet.clone())
            };
            let xmr_wallet = if model.reg_xmr_wallet.is_empty() {
                None
            } else {
                Some(model.reg_xmr_wallet.clone())
            };
            let auth = model.auth_service.clone();
            return Task::perform(
                async move {
                    let mut auth = auth.lock().unwrap();
                    auth.register(username, password.to_string(), email, btc_wallet, xmr_wallet)
                },
                move |result| match result {
                    Ok(reg_result) => Message::RegisterSuccess(reg_result),
                    Err(e) => Message::RegisterError(format!("{}", e)),
                },
            );
        }
        Message::VerifyCodeChanged(v) => {
            model.verify_code = v;
            model.auth_message = None;
        }
        Message::VerifyEmailSubmit => {
            let code = model.verify_code.clone();
            let auth = model.auth_service.clone();
            return Task::perform(
                async move {
                    let user_id = {
                        let auth = auth.lock().unwrap();
                        auth.first_user_id()
                            .unwrap_or(opendex_user::UserId(uuid::Uuid::nil()))
                    };
                    let mut auth = auth.lock().unwrap();
                    auth.verify_email(user_id, &code)
                },
                move |result| match result {
                    Ok(()) => Message::VerifySuccess,
                    Err(e) => Message::VerifyError(format!("{}", e)),
                },
            );
        }
        Message::ResendVerificationCode => {
            model.auth_message = Some("Check console for new code".to_string());
        }
        // Auth result handlers
        Message::LoginSuccess(result) => {
            model.auth_message = Some(format!("Welcome, {}!", result.username));
            // Store the logged-in user ID and load reputation data
            let auth = model.auth_service.lock().unwrap();
            if let Some(user) = auth.get_user_by_username(&result.username) {
                model.logged_in_user_id = Some(user.id);
                // Load reputation stats
                let rep = &user.reputation;
                model.reputation = crate::model::ReputationView {
                    tier: user.reputation_tier().to_string(),
                    trust_score: user.trust_score(),
                    makes: rep.makes,
                    takes: rep.takes,
                    completed_trades: rep.completed_trades,
                    disputes: rep.disputes,
                    failures_to_respond: rep.failures_to_respond,
                    average_rating: rep.average_rating,
                    rating_count: rep.rating_count,
                    response_rate: rep.response_rate(),
                    review_count: rep.review_count(),
                };
            }
            model.login_username.clear();
            model.login_password.clear();
        }
        Message::LoginError(msg) => {
            model.auth_message = Some(format!("Login failed: {}", msg));
        }
        Message::RegisterSuccess(result) => {
            model.auth_message = Some("Account created! Verification code sent to your email.".to_string());
            model.auth_screen = AuthScreen::VerifyEmail;
            model.verify_code.clear();
            // Store the user ID for verification
            model.logged_in_user_id = Some(result.user_id);
            model.reg_username.clear();
            model.reg_password.clear();
            model.reg_email.clear();
            model.reg_btc_wallet.clear();
            model.reg_xmr_wallet.clear();
        }
        Message::RegisterError(msg) => {
            model.auth_message = Some(format!("Registration failed: {}", msg));
        }
        Message::VerifySuccess => {
            model.auth_message = Some("Email verified! You can now log in.".to_string());
            model.auth_screen = AuthScreen::Login;
            model.verify_code.clear();
        }
        Message::VerifyError(msg) => {
            model.auth_message = Some(format!("Verification failed: {}", msg));
        }
        Message::AlertPriceChanged(value) => {
            model.alert_price = value;
        }
        Message::AddPriceAlert => {
            let price: f64 = model.alert_price.parse().unwrap_or(0.0);
            if price > 0.0 {
                model.price_alerts.push(PriceAlert {
                    target_price: price,
                    direction: AlertDirection::Below,
                    triggered: false,
                });
                model.alert_price.clear();
                model.status_message = Some(format!("Price alert set: below {}", price));
            }
        }
        Message::RemovePriceAlert(index) => {
            if index < model.price_alerts.len() {
                model.price_alerts.remove(index);
            }
        }
        Message::FilterMinPriceChanged(value) => {
            model.filter_min_price = value;
        }
        Message::FilterMaxPriceChanged(value) => {
            model.filter_max_price = value;
        }
        Message::ToggleDepthChart => {
            model.show_depth_chart = !model.show_depth_chart;
        }
        Message::ToggleProfile => {
            model.show_profile = !model.show_profile;
        }
        Message::ResetEmailChanged(value) => {
            model.reset_email = value;
        }
        Message::ResetCodeChanged(value) => {
            model.reset_code = value;
        }
        Message::ResetPasswordChanged(value) => {
            model.reset_new_password = zeroize::Zeroizing::new(value);
        }
        Message::SendResetCode => {
            if let Some(user_id) = model.logged_in_user_id
                && let Ok(mut auth) = model.auth_service.lock()
            {
                match auth.resend_verification_code(user_id) {
                    Ok(_) => model.auth_message = Some("Reset code sent to your email.".to_string()),
                    Err(e) => model.auth_message = Some(format!("Reset failed: {}", e)),
                }
            }
        }
        Message::ConfirmPasswordReset => {
            if model.reset_new_password.len() < 8 {
                model.auth_message = Some("Password must be at least 8 characters".to_string());
                return Task::none();
            }
            model.auth_message =
                Some("Password reset functionality requires email backend".to_string());
        }
        Message::ToggleTheme => {
            model.theme_override = match model.theme_override {
                None | Some(ThemeMode::Dark) => Some(ThemeMode::Light),
                Some(ThemeMode::Light) => Some(ThemeMode::Dark),
            };
            model.status_message = Some(format!("Theme: {:?}", model.theme_override));
        }
        Message::Logout => {
            model.logged_in_user_id = None;
            model.auth_screen = AuthScreen::Login;
            model.login_username.clear();
            model.login_password.clear();
            model.show_profile = false;
        }
        // Keyboard shortcuts
        Message::KeyboardShortcutNewOrder => {
            model.status_message = Some("Keyboard shortcut: New order focused".to_string());
        }
        Message::KeyboardShortcutPeers => {
            model.status_message = Some("Keyboard shortcut: Peers panel".to_string());
        }
        Message::KeyboardShortcutWallet => {
            model.status_message = Some("Keyboard shortcut: Wallet panel".to_string());
        }
        Message::KeyboardShortcutHistory => {
            model.status_message = Some("Keyboard shortcut: Order history".to_string());
        }
        // Analytics
        Message::ToggleAnalytics => {
            model.show_analytics = !model.show_analytics;
            model.status_message = Some(format!(
                "Analytics: {}",
                if model.show_analytics {
                    "shown"
                } else {
                    "hidden"
                }
            ));
        }
        Message::ToggleNetworkStats => {
            model.show_network_stats = !model.show_network_stats;
            // Update network stats from current state
            model.network_stats.total_peers = model.peers.len();
            model.network_stats.total_orders =
                model.order_book.bids.len() + model.order_book.asks.len();
            model.network_stats.active_trades = model.trades.len();
            model.status_message = Some(format!(
                "Network stats: {}",
                if model.show_network_stats {
                    "shown"
                } else {
                    "hidden"
                }
            ));
        }
        Message::ExportTradeHistoryCsv => {
            let csv = generate_trade_history_csv(&model.order_history);
            let home = std::env::var("HOME").unwrap_or_default();
            let file_path = format!("{}/.opendex/trade_history.csv", home);
            match std::fs::write(&file_path, &csv) {
                Ok(_) => {
                    model.status_message = Some(format!("Trade history exported to {}", file_path));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {}", e));
                }
            }
        }
        // Blockchain explorer links
        Message::OpenBtcExplorer(trade_id) => {
            let url = format!("https://mempool.space/tx/{}", trade_id);
            model.status_message = Some(format!(
                "Opening Bitcoin explorer: {}",
                &trade_id.to_string()[..8]
            ));
            let _ = open::that(&url);
        }
        Message::OpenXmrExplorer(trade_id) => {
            let url = format!("https://xmrchain.net/search?value={}", trade_id);
            model.status_message = Some(format!(
                "Opening Monero explorer: {}",
                &trade_id.to_string()[..8]
            ));
            let _ = open::that(&url);
        }
    }
    Task::none()
}

fn view(app: &App) -> Element<'_, Message> {
    let model = if let Some(m) = &app.model {
        m
    } else {
        return iced::widget::text("Loading").into();
    };

    // Show auth screen if not logged in
    if model.logged_in_user_id.is_none() {
        return auth::auth_panel(model);
    }

    let top_bar = status_bar::status_bar(model);
    let left_column = Column::new()
        .spacing(10)
        .push(order_book::order_book_panel(model, &model.order_book))
        .push(trade_form::trade_form_panel(model));
    let right_column = Column::new()
        .spacing(10)
        .push(trades::trades_panel(model))
        .push(reputation::reputation_panel(model))
        .push(price_alerts_panel(model))
        .push(order_history_panel(model))
        .push(analytics_panel(model))
        .push(network_stats_panel(model))
        .push(peers::peers_panel(model));
    let main_content = Row::new()
        .spacing(10)
        .push(left_column.width(Length::Fill))
        .push(right_column.width(Length::Fill));
    let bottom_bar = wallet::wallet_panel(model);

    // Build the base layout
    let base = Column::new()
        .spacing(10)
        .padding(10)
        .push(top_bar)
        .push(main_content.height(Length::Fill))
        .push(bottom_bar);

    // Overlay confirmation dialog if needed
    if model.show_confirm_dialog {
        let dialog = confirm_order_dialog(model);
        return iced::widget::Stack::new().push(base).push(dialog).into();
    }

    // Overlay profile panel if needed
    if model.show_profile {
        let dialog = profile_panel(model);
        return iced::widget::Stack::new().push(base).push(dialog).into();
    }

    base.into()
}

/// Keyboard subscription for shortcuts
fn subscription(app: &App) -> iced::Subscription<Message> {
    let _ = app; // subscription is active regardless of model state
    iced::event::listen_with(|event, _status, _window_id| {
        if let iced::event::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key,
            modifiers,
            text,
            ..
        }) = &event
        {
            if modifiers.control() {
                // Use the character produced by the key press for reliable detection
                let c = text.as_ref().map(|t| t.as_str()).unwrap_or("");
                match c {
                    "n" | "N" => return Some(Message::KeyboardShortcutNewOrder),
                    "p" | "P" => return Some(Message::KeyboardShortcutPeers),
                    "w" | "W" => return Some(Message::KeyboardShortcutWallet),
                    "h" | "H" => return Some(Message::KeyboardShortcutHistory),
                    _ => {}
                }
                // Fallback: check named keys via debug format
                if let iced::keyboard::Key::Named(_) = key {
                    let key_debug = format!("{:?}", key);
                    if key_debug.contains("KeyN") {
                        return Some(Message::KeyboardShortcutNewOrder);
                    } else if key_debug.contains("KeyP") {
                        return Some(Message::KeyboardShortcutPeers);
                    } else if key_debug.contains("KeyW") {
                        return Some(Message::KeyboardShortcutWallet);
                    } else if key_debug.contains("KeyH") {
                        return Some(Message::KeyboardShortcutHistory);
                    }
                }
            }
            None
        } else {
            None
        }
    })
}

/// Run the GUI application on a dedicated OS thread so iced can run its own event loop without
/// conflicting with the tokio multi-thread runtime used by the P2P node.
pub fn launch_gui(
    cmd_tx: flume::Sender<GuiCommand>,
    update_rx: flume::Receiver<GuiUpdate>,
    peer_id: String,
    listen_addr: String,
) {
    std::thread::spawn(move || {
        let flags = Arc::new(Flags {
            cmd_tx,
            update_rx,
            peer_id,
            listen_addr,
        });
        let _ = iced::application(
            {
                let flags = flags.clone();
                move || boot((*flags).clone())
            },
            update,
            view,
        )
        .subscription(subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1200.0, 800.0),
            ..Default::default()
        })
        .run();
    });
}

/// Run the GUI application on the current thread (must be the main thread for winit compatibility).
/// The P2P node runs on a separate background thread.
pub fn launch_gui_on_main(
    cmd_tx: flume::Sender<GuiCommand>,
    update_rx: flume::Receiver<GuiUpdate>,
) {
    let flags = Arc::new(Flags {
        cmd_tx,
        update_rx,
        peer_id: String::new(),
        listen_addr: String::new(),
    });
    let _ = iced::application(
        {
            let flags = flags.clone();
            move || boot((*flags).clone())
        },
        update,
        view,
    )
    .subscription(subscription)
    .window(iced::window::Settings {
        size: iced::Size::new(1200.0, 800.0),
        ..Default::default()
    })
    .run();
}

/// Order history panel showing past orders
fn order_history_panel(model: &GuiModel) -> Element<'_, Message> {
    use crate::style::{BG_PANEL, TEXT_SECONDARY};
    use iced::widget::{Column, Container, Row, Scrollable, Text};

    let title = Container::new(Text::new("Order History").size(14))
        .padding(iced::Padding::new(8.0).left(10).bottom(4));

    let header = Row::new()
        .spacing(6)
        .padding(iced::Padding::new(4.0).left(10).right(10))
        .push(
            Text::new("Side")
                .size(11)
                .width(Length::Fixed(35.0))
                .color(TEXT_SECONDARY),
        )
        .push(
            Text::new("Price")
                .size(11)
                .width(Length::Fixed(60.0))
                .color(TEXT_SECONDARY),
        )
        .push(
            Text::new("Amount")
                .size(11)
                .width(Length::Fixed(55.0))
                .color(TEXT_SECONDARY),
        )
        .push(
            Text::new("Status")
                .size(11)
                .width(Length::Fixed(50.0))
                .color(TEXT_SECONDARY),
        )
        .push(
            Text::new("Time")
                .size(11)
                .width(Length::Fill)
                .color(TEXT_SECONDARY),
        );

    let list = model
        .order_history
        .iter()
        .rev()
        .fold(Column::new().spacing(0), |col, entry| {
            let time = chrono::DateTime::<chrono::Utc>::from_timestamp(entry.timestamp as i64, 0)
                .map(|dt| format!("{}", dt.format("%H:%M:%S")))
                .unwrap_or_else(|| "??:??:??".to_string());
            let status_color = match entry.status.as_str() {
                "Placed" | "Filled" => crate::style::ACCENT_COLOR,
                "Failed" => iced::Color::from([1.0, 0.3, 0.3, 1.0]),
                _ => TEXT_SECONDARY,
            };
            col.push(
                Row::new()
                    .spacing(6)
                    .padding(iced::Padding::new(3.0).left(10).right(10))
                    .push(Text::new(&entry.side).size(10).width(Length::Fixed(35.0)))
                    .push(Text::new(&entry.price).size(10).width(Length::Fixed(60.0)))
                    .push(Text::new(&entry.amount).size(10).width(Length::Fixed(55.0)))
                    .push(
                        Text::new(&entry.status)
                            .size(10)
                            .width(Length::Fixed(50.0))
                            .color(status_color),
                    )
                    .push(
                        Text::new(time)
                            .size(10)
                            .width(Length::Fill)
                            .color(TEXT_SECONDARY),
                    ),
            )
        });

    let content = Column::new()
        .spacing(0)
        .push(title)
        .push(header)
        .push(Scrollable::new(list).height(Length::Fixed(100.0)));

    Container::new(content)
        .padding(0)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}

/// Confirmation dialog overlay before placing an order
fn confirm_order_dialog(model: &GuiModel) -> Element<'_, Message> {
    use crate::style::{ACCENT_COLOR, BG_PANEL, TEXT_SECONDARY};
    use iced::widget::{Button, Column, Container, Row, Text};

    let side_label = match model.trade_form.side {
        Side::Buy => "Buy",
        Side::Sell => "Sell",
    };

    let total = {
        let price: f64 = model.trade_form.price.parse().unwrap_or(0.0);
        let amount: f64 = model.trade_form.amount.parse().unwrap_or(0.0);
        format!("{:.4}", price * amount)
    };

    let confirm_btn = Button::new(Text::new("Confirm").size(12))
        .on_press(Message::ConfirmOrder)
        .padding(iced::Padding::new(12.0).top(8).bottom(8));

    let cancel_btn = Button::new(Text::new("Cancel").size(12))
        .on_press(Message::CancelConfirmOrder)
        .padding(iced::Padding::new(12.0).top(8).bottom(8));

    let dialog = Container::new(
        Column::new()
            .spacing(12)
            .padding(24)
            .push(Text::new("Confirm Order").size(18))
            .push(
                Text::new("Review your order details before submitting.")
                    .size(12)
                    .color(TEXT_SECONDARY),
            )
            .push(Text::new(format!("Side: {}", side_label)).size(14))
            .push(Text::new(format!("Price: {}", model.trade_form.price)).size(14))
            .push(Text::new(format!("Amount: {}", model.trade_form.amount)).size(14))
            .push(
                Text::new(format!("Total (est.): {} XMR", total))
                    .size(14)
                    .color(ACCENT_COLOR),
            )
            .push(
                Text::new(format!("Market: {}", model.trade_form.market))
                    .size(12)
                    .color(TEXT_SECONDARY),
            )
            .push(Row::new().spacing(12).push(confirm_btn).push(cancel_btn)),
    )
    .width(Length::Fixed(350.0))
    .style(|_theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(BG_PANEL)),
        ..Default::default()
    });

    // Centered overlay
    let overlay = Container::new(dialog)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::Alignment::Center)
        .align_y(iced::Alignment::Center);

    overlay.into()
}

/// Profile panel showing user info, wallet addresses, and settings
fn profile_panel(model: &GuiModel) -> Element<'_, Message> {
    use crate::style::{ACCENT_COLOR, BG_PANEL, TEXT_SECONDARY};
    use iced::widget::{Button, Column, Container, Row, Text};

    let username = if let Some(user_id) = model.logged_in_user_id {
        if let Ok(auth) = model.auth_service.lock() {
            auth.get_user(user_id)
                .map(|u| u.username.to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        }
    } else {
        "Unknown".to_string()
    };

    let email = if let Some(user_id) = model.logged_in_user_id {
        if let Ok(auth) = model.auth_service.lock() {
            auth.get_user(user_id)
                .map(|u| u.email.to_string())
                .unwrap_or_else(|| "Not set".to_string())
        } else {
            "Not set".to_string()
        }
    } else {
        "Not set".to_string()
    };

    let btc_wallet = model
        .btc_wallet_address
        .clone()
        .unwrap_or_else(|| "Not connected".to_string());
    let xmr_wallet = model
        .wallet_address
        .clone()
        .unwrap_or_else(|| "Not connected".to_string());

    let theme_label = match &model.theme_override {
        None => "Auto (System)",
        Some(ThemeMode::Dark) => "Dark",
        Some(ThemeMode::Light) => "Light",
    };

    let overlay = Container::new(
        Column::new()
            .spacing(12)
            .padding(24)
            .push(Text::new("User Profile").size(18))
            .push(Text::new(format!("Username: {}", username)).size(14))
            .push(Text::new(format!("Email: {}", email)).size(14))
            .push(
                Text::new(format!("BTC Wallet: {}", btc_wallet))
                    .size(12)
                    .color(TEXT_SECONDARY),
            )
            .push(
                Text::new(format!("XMR Wallet: {}", xmr_wallet))
                    .size(12)
                    .color(TEXT_SECONDARY),
            )
            .push(
                Text::new(format!("Theme: {}", theme_label))
                    .size(12)
                    .color(TEXT_SECONDARY),
            )
            .push(
                Text::new(format!(
                    "Reputation: {} ({})",
                    model.reputation.tier, model.reputation.trust_score
                ))
                .size(14)
                .color(ACCENT_COLOR),
            )
            .push(
                Row::new().spacing(12).push(
                    Button::new(Text::new("Close").size(12))
                        .on_press(Message::ToggleProfile)
                        .padding(iced::Padding::new(12.0).top(8).bottom(8)),
                ),
            ),
    )
    .width(Length::Fixed(400.0))
    .style(|_theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(BG_PANEL)),
        ..Default::default()
    });

    let centered = Container::new(overlay)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::Alignment::Center)
        .align_y(iced::Alignment::Center);

    centered.into()
}

/// Price alerts panel
fn price_alerts_panel(model: &GuiModel) -> Element<'_, Message> {
    use crate::style::{ACCENT_COLOR, BG_PANEL};
    use iced::widget::{Button, Column, Container, Row, Scrollable, Text, TextInput};

    let title = Container::new(Text::new("Price Alerts").size(14))
        .padding(iced::Padding::new(8.0).left(10).bottom(4));

    let alert_input = TextInput::new("Target price", &model.alert_price)
        .on_input(Message::AlertPriceChanged)
        .padding(4);

    let add_btn = Button::new(Text::new("Add").size(11))
        .on_press(Message::AddPriceAlert)
        .padding(iced::Padding::new(8.0).top(4).bottom(4));

    let alerts_list =
        model
            .price_alerts
            .iter()
            .enumerate()
            .fold(Column::new().spacing(0), |col, (i, alert)| {
                let dir = match alert.direction {
                    AlertDirection::Above => "↑ Above",
                    AlertDirection::Below => "↓ Below",
                };
                let status = if alert.triggered { " 🔔" } else { "" };
                let row = Row::new()
                    .spacing(6)
                    .padding(iced::Padding::new(3.0).left(10).right(10))
                    .push(
                        Text::new(format!("{} {:.2}{}", dir, alert.target_price, status))
                            .size(10)
                            .width(Length::Fill)
                            .color(ACCENT_COLOR),
                    )
                    .push(
                        Button::new(Text::new("✕").size(10))
                            .on_press(Message::RemovePriceAlert(i))
                            .padding(iced::Padding::new(4.0)),
                    );
                col.push(row)
            });

    let content = Column::new()
        .spacing(4)
        .push(title)
        .push(
            Row::new()
                .spacing(6)
                .padding(iced::Padding::new(4.0).left(10).right(10))
                .push(alert_input.width(Length::Fill))
                .push(add_btn),
        )
        .push(Scrollable::new(alerts_list).height(Length::Fixed(60.0)));

    Container::new(content)
        .padding(0)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}

/// Generate CSV content from order history
fn generate_trade_history_csv(history: &[OrderHistoryEntry]) -> String {
    let mut csv = String::from("ID,Side,Price,Amount,Timestamp,Status\n");
    for entry in history {
        let timestamp = chrono::DateTime::<chrono::Utc>::from_timestamp(entry.timestamp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            entry.id, entry.side, entry.price, entry.amount, timestamp, entry.status
        ));
    }
    csv
}

/// Analytics panel showing personal trading statistics
fn analytics_panel(model: &GuiModel) -> Element<'_, Message> {
    use crate::style::{ACCENT_COLOR, BG_PANEL, TEXT_SECONDARY};
    use iced::widget::{Button, Column, Container, Row, Text};

    let a = &model.analytics;
    let toggle_btn = Button::new(
        Text::new(if model.show_analytics {
            "▼ Hide"
        } else {
            "▶ Show"
        })
        .size(11),
    )
    .on_press(Message::ToggleAnalytics)
    .padding(iced::Padding::new(4.0).left(10).right(10));

    let export_btn = Button::new(Text::new("📥 Export CSV").size(10))
        .on_press(Message::ExportTradeHistoryCsv)
        .padding(iced::Padding::new(4.0).left(6).right(6));

    let title = Row::new()
        .spacing(6)
        .padding(iced::Padding::new(8.0).left(10).right(10))
        .push(Text::new("📊 Trading Analytics").size(13))
        .push(toggle_btn)
        .push(iced::widget::Space::new().width(Length::Fill))
        .push(export_btn);

    let content = if model.show_analytics {
        Column::new()
            .spacing(4)
            .padding(iced::Padding::new(4.0).left(10).right(10).bottom(8))
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Total Trades:").size(10).color(TEXT_SECONDARY))
                    .push(Text::new(format!("{}", a.total_trades)).size(11)),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Completed:").size(10).color(TEXT_SECONDARY))
                    .push(
                        Text::new(format!("{}", a.completed_trades))
                            .size(11)
                            .color(ACCENT_COLOR),
                    ),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Failed:").size(10).color(TEXT_SECONDARY))
                    .push(
                        Text::new(format!("{}", a.failed_trades))
                            .size(11)
                            .color(crate::style::WARNING_COLOR),
                    ),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Win Rate:").size(10).color(TEXT_SECONDARY))
                    .push(Text::new(format!("{:.1}%", a.win_rate)).size(11)),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Volume (BTC):").size(10).color(TEXT_SECONDARY))
                    .push(Text::new(format!("{:.6}", a.total_volume_btc)).size(11)),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Volume (XMR):").size(10).color(TEXT_SECONDARY))
                    .push(Text::new(format!("{:.4}", a.total_volume_xmr)).size(11)),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("P/L (XMR):").size(10).color(TEXT_SECONDARY))
                    .push(
                        Text::new(format!("{:+.4}", a.profit_loss_xmr))
                            .size(11)
                            .color(if a.profit_loss_xmr >= 0.0 {
                                ACCENT_COLOR
                            } else {
                                crate::style::WARNING_COLOR
                            }),
                    ),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Avg Trade:").size(10).color(TEXT_SECONDARY))
                    .push(Text::new(format!("{:.4} XMR", a.average_trade_size)).size(11)),
            )
    } else {
        Column::new()
    };

    let all = Column::new().spacing(0).push(title).push(content);

    Container::new(all)
        .padding(0)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}

/// Network statistics dashboard panel
fn network_stats_panel(model: &GuiModel) -> Element<'_, Message> {
    use crate::style::{ACCENT_COLOR, BG_PANEL, TEXT_SECONDARY};
    use iced::widget::{Button, Column, Container, Row, Text};

    let ns = &model.network_stats;
    let toggle_btn = Button::new(
        Text::new(if model.show_network_stats {
            "▼ Hide"
        } else {
            "▶ Show"
        })
        .size(11),
    )
    .on_press(Message::ToggleNetworkStats)
    .padding(iced::Padding::new(4.0).left(10).right(10));

    let title = Row::new()
        .spacing(6)
        .padding(iced::Padding::new(8.0).left(10).right(10))
        .push(Text::new("🌐 Network Statistics").size(13))
        .push(toggle_btn);

    let content = if model.show_network_stats {
        Column::new()
            .spacing(4)
            .padding(iced::Padding::new(4.0).left(10).right(10).bottom(8))
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Total Peers:").size(10).color(TEXT_SECONDARY))
                    .push(
                        Text::new(format!("{}", ns.total_peers))
                            .size(11)
                            .color(ACCENT_COLOR),
                    ),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Active Trades:").size(10).color(TEXT_SECONDARY))
                    .push(Text::new(format!("{}", ns.active_trades)).size(11)),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Total Orders:").size(10).color(TEXT_SECONDARY))
                    .push(Text::new(format!("{}", ns.total_orders)).size(11)),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("24h Volume:").size(10).color(TEXT_SECONDARY))
                    .push(Text::new(format!("{:.2} XMR", ns.volume_24h_xmr)).size(11)),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Avg Trade:").size(10).color(TEXT_SECONDARY))
                    .push(Text::new(format!("{:.4} XMR", ns.average_trade_size)).size(11)),
            )
    } else {
        Column::new()
    };

    let all = Column::new().spacing(0).push(title).push(content);

    Container::new(all)
        .padding(0)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}
