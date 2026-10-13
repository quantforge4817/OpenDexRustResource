use std::sync::Arc;
use iced::widget::{Column, Row};
use iced::{Length, Task, Element};
use crate::model::{GuiModel, NodeUpdate, PeerView};
use crate::view::{order_book, trade_form, trades, wallet, peers, status_bar};
use protocol::message::{Asset, Market, Side, TradeState};
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
    WalletBalance { available: f64, unlocked: f64 },
    WalletConnected { connected: bool, address: Option<String> },
    BtcWalletAddress(String),
    BtcWalletBalance { balance: f64 },
    BtcWalletConnected { connected: bool, address: Option<String> },
}

#[derive(Debug, Clone)]
pub enum Message {
    SideSelected(String),
    MarketSelected(String),
    PriceChanged(String),
    AmountChanged(String),
    SubmitOrder,
    PeerInputChanged(String),
    ConnectPeer,
    FundAssetSelected(String),
    FundAmountChanged(String),
    FundSubmit,
    RefreshBalances,
    RefreshOrderBook,
    RefreshTrades,
    AdvanceTradeState { trade_id: Uuid, state: TradeState },
    NodeUpdate(NodeUpdate),
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
    let model = GuiModel::new(
        flags.cmd_tx.clone(),
        flags.peer_id,
        flags.listen_addr,
    );
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
    let model = if let Some(m) = app.model.as_mut() { m } else { return Task::none() };

    match message {
        Message::SideSelected(label) => {
            model.trade_form.side = if label == "Buy" { Side::Buy } else { Side::Sell };
        }
        Message::MarketSelected(label) => {
            model.trade_form.market = if label == "BTC/XMR" {
                Market { base: Asset::BTC, quote: Asset::XMR }
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
            let market = model.trade_form.market.clone();
            let cmd = GuiCommand::PlaceOrder {
                side: model.trade_form.side.clone(),
                market,
                price,
                amount,
            };
            let tx = model.command_tx.clone();
            return Task::perform(
                async move { let _ = tx.send(cmd); },
                |_| Message::NodeUpdate(NodeUpdate::LogMessage("Order submitted".to_string())),
            );
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
                async move { let _ = tx.send(cmd); },
                |_| Message::NodeUpdate(NodeUpdate::LogMessage("Connect command sent".to_string())),
            );
        }
        Message::FundAssetSelected(label) => {
            model.fund_asset = if label == "BTC" { Asset::BTC } else { Asset::XMR };
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
            let cmd = GuiCommand::Fund { asset: model.fund_asset.clone(), amount };
            let tx = model.command_tx.clone();
            return Task::perform(
                async move { let _ = tx.send(cmd); },
                |_| Message::NodeUpdate(NodeUpdate::LogMessage("Fund command sent".to_string())),
            );
        }
        Message::RefreshBalances => {
            let tx = model.command_tx.clone();
            return Task::perform(
                async move { let _ = tx.send(GuiCommand::RequestBalances); },
                |_| Message::NodeUpdate(NodeUpdate::LogMessage("Refreshing balances...".to_string())),
            );
        }
        Message::RefreshOrderBook => {
            let tx = model.command_tx.clone();
            return Task::perform(
                async move { let _ = tx.send(GuiCommand::RequestOrderBook); },
                |_| Message::NodeUpdate(NodeUpdate::LogMessage("Refreshing order book...".to_string())),
            );
        }
        Message::RefreshTrades => {
            let tx = model.command_tx.clone();
            return Task::perform(
                async move { let _ = tx.send(GuiCommand::RequestTrades); },
                |_| Message::NodeUpdate(NodeUpdate::LogMessage("Refreshing trades...".to_string())),
            );
        }
        Message::AdvanceTradeState { trade_id, state } => {
            let cmd = GuiCommand::AdvanceTradeState { trade_id, state };
            let tx = model.command_tx.clone();
            return Task::perform(
                async move { let _ = tx.send(cmd); },
                |_| Message::NodeUpdate(NodeUpdate::LogMessage("Trade state advance sent".to_string())),
            );
        }
        Message::NodeUpdate(update) => {
            match update {
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
                        model.peers.push(PeerView { address: addr.clone(), connected: true });
                    }
                    model.node_status.peer_count = model.peers.len();
                }
                NodeUpdate::PeerDisconnected(addr) => {
                    model.peers.retain(|p| p.address != addr);
                    model.node_status.peer_count = model.peers.len();
                }
                NodeUpdate::OrderResult { order_id, success, error } => {
                    if success {
                        model.status_message = Some(format!("Order {} placed", &order_id.to_string()[..8]));
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
                NodeUpdate::WalletBalance { available, unlocked } => {
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
                    model.status_message = Some(format!(
                        "BTC wallet balance: {:.8} BTC",
                        balance
                    ));
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
            }
        }
    }
    Task::none()
}

fn view(app: &App) -> Element<'_, Message> {
    let model = if let Some(m) = &app.model { m } else {
        return iced::widget::text("Loading").into();
    };

    let top_bar = status_bar::status_bar(model);
    let left_column = Column::new()
        .spacing(10)
        .push(order_book::order_book_panel(&model.order_book))
        .push(trade_form::trade_form_panel(model));
    let right_column = Column::new()
        .spacing(10)
        .push(trades::trades_panel(&model.trades))
        .push(peers::peers_panel(model));
    let main_content = Row::new()
        .spacing(10)
        .push(left_column.width(Length::Fill))
        .push(right_column.width(Length::Fill));
    let bottom_bar = wallet::wallet_panel(model);

    Column::new()
        .spacing(10)
        .padding(10)
        .push(top_bar)
        .push(main_content.height(Length::Fill))
        .push(bottom_bar)
        .into()
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
        let flags = Arc::new(Flags { cmd_tx, update_rx, peer_id, listen_addr });
        let _ = iced::application({
            let flags = flags.clone();
            move || boot((*flags).clone())
        }, update, view)
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
    let flags = Arc::new(Flags { cmd_tx, update_rx, peer_id: String::new(), listen_addr: String::new() });
    let _ = iced::application({
        let flags = flags.clone();
        move || boot((*flags).clone())
    }, update, view)
        .window(iced::window::Settings {
            size: iced::Size::new(1200.0, 800.0),
            ..Default::default()
        })
        .run();
}
