use std::collections::HashMap;

use uuid::Uuid;

use p2p_node::{OpenDexBehaviour, OpenDexBehaviourEvent};

use network::build_swarm;

use crate::cli::{CliAction, parse_args, parse_command};

use trades::TradeBook;

use anyhow::Result;

use futures::StreamExt;

use balances::BalanceBook;

use command_handler::handle_command;

mod settlement;

mod escrow;

mod balances;

mod cli;

mod trade_state;
mod trades;

mod orderbook;

mod order_factory;

mod network;

mod command_handler;

mod message_handler;

mod sync;

mod persistence;

mod exchange;

// GUI modules
mod gui_bridge;
mod gui_model;
mod gui_app;
mod gui_style;
mod gui_view;
mod wallet_client;
mod btc_wallet_client;
mod p2p_escrow;
mod p2p_offer_book;

use gui_bridge::GuiCommand;
use orderbook::OrderBook;
use p2p_offer_book::P2POfferBook;
use wallet_client::{MoneroWalletClient, MoneroWalletConfig};
use btc_wallet_client::{BitcoinWalletClient, BitcoinWalletConfig};

use tokio::io::{self, AsyncBufReadExt};

use libp2p::{Multiaddr, PeerId, gossipsub, identity};

use std::{fs, path::PathBuf};

/// Initialize structured logging with file-based audit output.
///
/// Routes logs to separate files based on log level:
/// - `audit.log` — all logs (info, debug, warn, error)
/// - `error.log` — warn and error level only
fn init_audit_logging(node_dir: &PathBuf) -> Result<()> {
    use tracing_subscriber::prelude::*;

    let audit_dir = node_dir.join("audit");
    fs::create_dir_all(&audit_dir)?;

    let audit_log_path = audit_dir.join("audit.log");
    let error_log_path = audit_dir.join("error.log");

    // Layer 1: All logs to audit.log
    let audit_file = fs::File::create(&audit_log_path)?;
    let audit_layer = tracing_subscriber::fmt::layer()
        .with_writer(audit_file)
        .with_ansi(false)
        .with_target(true)
        .with_file(true)
        .with_line_number(true);

    // Layer 2: Warnings and errors to error.log
    let error_file = fs::File::create(&error_log_path)?;
    let error_layer = tracing_subscriber::fmt::layer()
        .with_writer(error_file)
        .with_ansi(false)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        );

    // Layer 3: Stderr for interactive use
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        );

    tracing_subscriber::registry()
        .with(audit_layer)
        .with(error_layer)
        .with(stderr_layer)
        .init();

    tracing::info!(
        audit_dir = %audit_dir.display(),
        "Audit logging initialized"
    );

    Ok(())
}

fn identity_path(port: &str) -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| {
        eprintln!("Cannot find home directory, using /tmp");
        PathBuf::from("/tmp")
    });

    path.push(".opendex");
    path.push(format!("identity-{}.key", port));

    path
}

fn data_dir(port: &str) -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| {
        eprintln!("Cannot find home directory, using /tmp");
        PathBuf::from("/tmp")
    });

    path.push(".opendex");
    path.push(format!("node-{}", port));

    path
}

fn load_or_create_identity(port: &str) -> Result<identity::Keypair> {
    let path = identity_path(port);

    if path.exists() {
        let bytes = persistence::read_bytes(&path)?;

        let key = identity::Keypair::from_protobuf_encoding(&bytes)?;

        Ok(key)
    } else {
        let key = identity::Keypair::generate_ed25519();

        let bytes = key.to_protobuf_encoding()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        persistence::write_bytes(&path, &bytes)?;

        Ok(key)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let cli = match parse_args(&args)? {
        CliAction::Run(config) => config,
        CliAction::Help => {
            tracing::info!("{}", cli::CliArgs::usage());
            return Ok(());
        }
    };

    let port = cli.port.to_string();
    let peer_addr = cli.peer;
    let gui_mode = cli.gui;

    let node_dir = data_dir(&port);
    fs::create_dir_all(&node_dir)?;

    // Initialize structured logging with file-based audit output
    init_audit_logging(&node_dir)?;

    let id_keys = if args.iter().any(|arg| arg == "--new-identity") {
        identity::Keypair::generate_ed25519()
    } else {
        load_or_create_identity(&port)?
    };

    let local_peer_id = PeerId::from(id_keys.public());

    tracing::info!(peer_id = %local_peer_id, "Peer ID");

    let mut swarm = build_swarm(id_keys)?;

    let topic = gossipsub::IdentTopic::new("opendex-orders");

    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&topic)
        .map_err(|e| anyhow::anyhow!("Failed to subscribe to gossipsub topic: {}", e))?;

    let listen_addr: Multiaddr = format!("/ip4/127.0.0.1/tcp/{}", port).parse()?;

    swarm.listen_on(listen_addr.clone())?;

    tracing::info!("Listening for peers...");
    tracing::info!(address = %format!("{}/p2p/{}", listen_addr, local_peer_id));

    if let Some(addr) = peer_addr {
        let peer: Multiaddr = addr.parse()?;

        tracing::info!(peer = %peer, "Dialing peer");
        swarm.dial(peer)?;
    }

    let orders_path = node_dir.join("orders.json");
    let trades_path = node_dir.join("trades.json");
    let balances_path = node_dir.join("balances.json");

    let mut order_book = OrderBook::load_orders(orders_path)?;
    let mut trade_book = TradeBook::load_trades(trades_path)?;
    let mut balance_book = BalanceBook::load(balances_path)?;

    let mut pending_trade_states: HashMap<Uuid, message_handler::PendingTradeState> =
        HashMap::new();

    let mut p2p_offer_book = P2POfferBook::load();

    // Initialize Monero wallet client if configured
    let mut monero_wallet = if cli.monero_rpc_url.is_some()
        || cli.monero_rpc_user.is_some()
        || cli.monero_rpc_pass.is_some()
    {
        let config = MoneroWalletConfig {
            rpc_url: cli
                .monero_rpc_url
                .unwrap_or_else(|| "http://127.0.0.1:18082".to_string()),
            username: cli.monero_rpc_user,
            password: cli.monero_rpc_pass,
        };
        let mut wallet = MoneroWalletClient::new(config);
        match wallet.connect().await {
            Ok(()) => {
                tracing::info!("Monero wallet connected successfully");
                if let Ok(addr) = wallet.get_address().await {
                    tracing::info!(address = %addr, "Monero wallet address");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to connect Monero wallet — on-chain XMR operations disabled");
            }
        }
        Some(wallet)
    } else {
        tracing::info!("Monero wallet not configured — on-chain XMR operations disabled");
        None
    };

    // Initialize Bitcoin wallet client if configured
    let mut btc_wallet = if cli.btc_rpc_url.is_some()
        || cli.btc_rpc_user.is_some()
        || cli.btc_rpc_pass.is_some()
    {
        let config = BitcoinWalletConfig {
            rpc_url: cli
                .btc_rpc_url
                .unwrap_or_else(|| "http://127.0.0.1:8332".to_string()),
            username: cli.btc_rpc_user,
            password: cli.btc_rpc_pass,
            wallet_name: None,
        };
        let mut wallet = BitcoinWalletClient::new(config);
        match wallet.connect().await {
            Ok(()) => {
                tracing::info!("Bitcoin wallet connected successfully");
                if let Ok(addr) = wallet.get_new_address(None).await {
                    tracing::info!(address = %addr, "Bitcoin wallet address");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to connect Bitcoin wallet — on-chain BTC operations disabled");
            }
        }
        Some(wallet)
    } else {
        tracing::info!("Bitcoin wallet not configured — on-chain BTC operations disabled");
        None
    };

    if gui_mode {
        // Launch GUI mode
        tracing::info!("Starting GUI mode");
        let (cmd_tx, cmd_rx) = flume::unbounded();
        let (update_tx, update_rx) = flume::unbounded();

        // Spawn the node event loop
        tokio::spawn(async move {
            run_node_loop(
                swarm,
                topic,
                local_peer_id,
                order_book,
                trade_book,
                balance_book,
                pending_trade_states,
                p2p_offer_book,
                cmd_rx,
                update_tx,
            ).await;
        });

        // Launch the GUI application
        gui_app::run_gui(cmd_tx, update_rx, local_peer_id.to_string(), listen_addr.to_string()).await;
    } else {
        // CLI mode
        let stdin = io::BufReader::new(io::stdin());
        let mut lines = stdin.lines();

        loop {
            tokio::select! {
                        line = lines.next_line() => {
                            if let Some(input) = line? {
                                let command = parse_command(&input);

                                // Handle wallet commands directly
                                match &command {
                                    cli::Command::WalletAddress => {
                                        if let Some(ref wallet) = monero_wallet {
                                            match wallet.get_address().await {
                                                Ok(addr) => {
                                                    tracing::info!(address = %addr, "Monero wallet address");
                                                }
                                                Err(e) => {
                                                    tracing::error!(error = %e, "Failed to get wallet address");
                                                }
                                            }
                                        } else {
                                            tracing::info!("Monero wallet not configured");
                                        }
                                        continue;
                                    }
                                    cli::Command::WalletBalance => {
                                        if let Some(ref wallet) = monero_wallet {
                                            match wallet.get_balance_xmr(0).await {
                                                Ok((available, unlocked)) => {
                                                    tracing::info!(
                                                        available = available,
                                                        unlocked = unlocked,
                                                        "Monero wallet balance (XMR)"
                                                    );
                                                }
                                                Err(e) => {
                                                    tracing::error!(error = %e, "Failed to get wallet balance");
                                                }
                                            }
                                        } else {
                                            tracing::info!("Monero wallet not configured");
                                        }
                                        continue;
                                    }
                                    cli::Command::BtcWalletAddress => {
                                        if let Some(ref wallet) = btc_wallet {
                                            match wallet.get_new_address(None).await {
                                                Ok(addr) => {
                                                    tracing::info!(address = %addr, "Bitcoin wallet address");
                                                }
                                                Err(e) => {
                                                    tracing::error!(error = %e, "Failed to get wallet address");
                                                }
                                            }
                                        } else {
                                            tracing::info!("Bitcoin wallet not configured");
                                        }
                                        continue;
                                    }
                                    cli::Command::BtcWalletBalance => {
                                        if let Some(ref wallet) = btc_wallet {
                                            match wallet.get_balance().await {
                                                Ok(balance) => {
                                                    tracing::info!(
                                                        balance_btc = balance,
                                                        balance_satoshi = (balance * 100_000_000.0) as u64,
                                                        "Bitcoin wallet balance"
                                                    );
                                                }
                                                Err(e) => {
                                                    tracing::error!(error = %e, "Failed to get wallet balance");
                                                }
                                            }
                                        } else {
                                            tracing::info!("Bitcoin wallet not configured");
                                        }
                                        continue;
                                    }
                                    _ => {}
                                }

                                handle_command(
                                    command,
                                    &mut swarm,
                                    &topic,
                                    &local_peer_id,
                                    &mut order_book,
                                    &mut trade_book,
                                    &mut balance_book,
                                    &mut p2p_offer_book,
                                    monero_wallet.as_ref(),
                                    btc_wallet.as_ref(),
                                ).await?;
                            }
                        }

                        event = swarm.next() => {
                            if let Some(event) = event {
                               let mut ctx = message_handler::HandlerContext {
                swarm: &mut swarm,
                topic: &topic,
                local_peer_id: &local_peer_id,
                order_book: &mut order_book,
                trade_book: &mut trade_book,
                balance_book: &mut balance_book,
                pending_trade_states: &mut pending_trade_states,
                p2p_offer_book: &mut p2p_offer_book,
            };

            message_handler::handle_swarm_event(event, &mut ctx)?;

                            }
                        }
                    }
        }
    }

    Ok(())
}

/// Run the node event loop receiving commands from a channel instead of stdin
async fn run_node_loop(
    mut swarm: libp2p::Swarm<OpenDexBehaviour>,
    topic: gossipsub::IdentTopic,
    local_peer_id: PeerId,
    mut order_book: OrderBook,
    mut trade_book: TradeBook,
    mut balance_book: BalanceBook,
    mut pending_trade_states: HashMap<Uuid, message_handler::PendingTradeState>,
    mut p2p_offer_book: P2POfferBook,
    cmd_rx: flume::Receiver<gui_bridge::GuiCommand>,
    update_tx: flume::Sender<gui_bridge::GuiUpdate>,
) {
    use futures::StreamExt;
    loop {
        tokio::select! {
            biased;

            cmd = cmd_rx.recv_async() => {
                if let Ok(command) = cmd {
                    handle_gui_command(
                        command,
                        &mut swarm,
                        &topic,
                        &local_peer_id,
                        &mut order_book,
                        &mut trade_book,
                        &mut balance_book,
                        &update_tx,
                    ).await;
                }
            }

            event = swarm.next() => {
                if let Some(event) = &event {
                    // Broadcast peer connection/disconnection before consuming event
                    match event {
                        libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            let _ = update_tx.send(gui_bridge::GuiUpdate::PeerConnected(*peer_id));
                        }
                        libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            let _ = update_tx.send(gui_bridge::GuiUpdate::PeerDisconnected(*peer_id));
                        }
                        _ => {}
                    }
                }
                if let Some(event) = event {
                    let mut ctx = message_handler::HandlerContext {
                        swarm: &mut swarm,
                        topic: &topic,
                        local_peer_id: &local_peer_id,
                        order_book: &mut order_book,
                        trade_book: &mut trade_book,
                        balance_book: &mut balance_book,
                        pending_trade_states: &mut pending_trade_states,
                        p2p_offer_book: &mut p2p_offer_book,
                    };

                    if let Err(e) = message_handler::handle_swarm_event(event, &mut ctx) {
                        tracing::error!(error = %e, "Swarm event error");
                    }
                }
            }
        }
    }
}

async fn handle_gui_command(
    command: gui_bridge::GuiCommand,
    swarm: &mut libp2p::Swarm<OpenDexBehaviour>,
    topic: &gossipsub::IdentTopic,
    local_peer_id: &PeerId,
    order_book: &mut OrderBook,
    trade_book: &mut TradeBook,
    balance_book: &mut BalanceBook,
    update_tx: &flume::Sender<gui_bridge::GuiUpdate>,
) {
    use gui_bridge::GuiUpdate;
    use protocol::message::AMOUNT_SCALE;

    match command {
        GuiCommand::PlaceOrder { side, market, price, amount } => {
            let order = crate::order_factory::create_order(
                local_peer_id.to_string(),
                market,
                side,
                price,
                amount,
            );
            let order_id = order.id;

            if let Err(e) = balance_book.lock_for_order(&order) {
                let _ = update_tx.send(GuiUpdate::OrderResult {
                    order_id,
                    success: false,
                    error: Some(format!("Cannot lock funds: {}", e)),
                });
                return;
            }
            let _ = balance_book.save();

            let order_message = protocol::message::Message {
                message_type: protocol::message::MessageType::OrderCreated,
                sender: local_peer_id.to_string(),
                payload: serde_json::to_string(&order).unwrap_or_default(),
                timestamp: chrono::Utc::now().timestamp() as u64,
            };

            if let Err(e) = crate::network::publish_message(swarm, topic, &order_message) {
                let _ = balance_book.unlock_for_order(&order);
                let _ = balance_book.save();
                let _ = update_tx.send(GuiUpdate::OrderResult {
                    order_id,
                    success: false,
                    error: Some(format!("Publish failed: {}", e)),
                });
                return;
            }

            let mut exchange = crate::exchange::Exchange::new(order_book, trade_book, balance_book);
            match exchange.submit_order(crate::exchange::SubmitOrder {
                actor: local_peer_id.to_string(),
                order: order.clone(),
            }) {
                Ok(trades) => {
                    if trades.is_empty() {
                        let _ = order_book.save_orders();
                    } else {
                        for trade in trades {
                            let timestamp = chrono::Utc::now().timestamp() as u64;
                            let message = protocol::message::Message {
                                message_type: protocol::message::MessageType::TradeCreated,
                                sender: local_peer_id.to_string(),
                                payload: serde_json::to_string(&trade).unwrap_or_default(),
                                timestamp,
                            };
                            let _ = crate::network::publish_message(swarm, topic, &message);
                            let _ = update_tx.send(GuiUpdate::TradeAdded(trade));
                        }
                    }
                    let _ = update_tx.send(GuiUpdate::OrderResult {
                        order_id,
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    let _ = balance_book.unlock_for_order(&order);
                    let _ = balance_book.save();
                    let _ = update_tx.send(GuiUpdate::OrderResult {
                        order_id,
                        success: false,
                        error: Some(format!("Exchange error: {}", e)),
                    });
                }
            }
        }

        GuiCommand::CancelOrder { order_id } => {
            if let Some(order) = order_book.get_order(&order_id).cloned() {
                if order_book.remove_order(order_id) {
                    let _ = balance_book.unlock_for_order(&order);
                    let _ = balance_book.save();
                    let _ = order_book.save_orders();

                    let cancelled = protocol::message::OrderCancelled {
                        order_id,
                        trader: order.trader.clone(),
                    };
                    let message = protocol::message::Message {
                        message_type: protocol::message::MessageType::OrderCancelled,
                        sender: local_peer_id.to_string(),
                        payload: serde_json::to_string(&cancelled).unwrap_or_default(),
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    };
                    let _ = crate::network::publish_message(swarm, topic, &message);
                }
            }
        }

        GuiCommand::Fund { asset, amount } => {
            let trader = local_peer_id.to_string();
            let scaled_amount = (amount * AMOUNT_SCALE as f64) as u64;
            if let Err(e) = balance_book.credit(&trader, asset.clone(), scaled_amount) {
                let _ = update_tx.send(GuiUpdate::Error(format!("Fund error: {}", e)));
                return;
            }
            let _ = balance_book.save();
            let asset_label = match &asset {
                protocol::message::Asset::BTC => "BTC",
                protocol::message::Asset::XMR => "XMR",
            };
            let _ = update_tx.send(GuiUpdate::LogMessage(format!("Funded {} with {}", asset_label, amount)));
            // Send updated balances
            let mut balances = Vec::new();
            for a in [protocol::message::Asset::BTC, protocol::message::Asset::XMR] {
                let b = balance_book.balance(&trader, &a);
                balances.push((trader.clone(), a, b));
            }
            let _ = update_tx.send(GuiUpdate::BalancesUpdated(balances));
        }

        GuiCommand::ConnectPeer { multiaddr } => {
            match multiaddr.parse::<Multiaddr>() {
                Ok(addr) => {
                    if let Err(e) = swarm.dial(addr.clone()) {
                        let _ = update_tx.send(GuiUpdate::Error(format!("Dial error: {}", e)));
                    } else {
                        let _ = update_tx.send(GuiUpdate::LogMessage(format!("Dialing {}", addr)));
                    }
                }
                Err(e) => {
                    let _ = update_tx.send(GuiUpdate::Error(format!("Invalid address: {}", e)));
                }
            }
        }

        GuiCommand::RequestBalances => {
            let trader = local_peer_id.to_string();
            let mut balances = Vec::new();
            for asset in [protocol::message::Asset::BTC, protocol::message::Asset::XMR] {
                let balance = balance_book.balance(&trader, &asset);
                balances.push((trader.clone(), asset, balance));
            }
            let _ = update_tx.send(GuiUpdate::BalancesUpdated(balances));
        }

        GuiCommand::RequestOrderBook => {
            let orders = order_book.active_orders();
            let _ = update_tx.send(GuiUpdate::OrderBookSnapshot(orders));
        }

        GuiCommand::RequestTrades => {
            for trade in trade_book.all_trades() {
                let _ = update_tx.send(GuiUpdate::TradeAdded(trade));
            }
        }

        GuiCommand::AdvanceTradeState { trade_id, state } => {
            let timestamp = chrono::Utc::now().timestamp() as u64;
            tracing::info!(
                trade_id = %trade_id,
                state = ?state,
                "GUI: advancing trade state"
            );
            if let Err(e) = crate::trade_state::advance_trade_to_state(trade_book, trade_id, state.clone(), timestamp) {
                tracing::error!(trade_id = %trade_id, error = %e, "GUI: trade state transition failed");
                let _ = update_tx.send(GuiUpdate::Error(format!("Trade state error: {}", e)));
                return;
            }
            if let Some(trade) = trade_book.get_trade(trade_id) {
                match &state {
                    protocol::message::TradeState::EscrowFunded => {
                        tracing::info!(trade_id = %trade_id, "GUI: funding escrow");
                        if let Err(e) = crate::escrow::fund_local_escrow(balance_book, local_peer_id, trade) {
                            tracing::error!(trade_id = %trade_id, error = %e, "GUI: escrow funding failed");
                            let _ = update_tx.send(GuiUpdate::Error(format!("Escrow funding error: {}", e)));
                        }
                    }
                    protocol::message::TradeState::Settled => {
                        tracing::info!(trade_id = %trade_id, "GUI: releasing escrow and settling");
                        // Release escrow before normal settlement
                        if let Err(e) = crate::escrow::release_local_escrow(balance_book, local_peer_id, trade) {
                            tracing::error!(trade_id = %trade_id, error = %e, "GUI: escrow release failed");
                            let _ = update_tx.send(GuiUpdate::Error(format!("Escrow release error: {}", e)));
                        }
                        if let Err(e) = crate::settlement::settle_local_trade(balance_book, local_peer_id, trade) {
                            tracing::error!(trade_id = %trade_id, error = %e, "GUI: settlement failed");
                            let _ = update_tx.send(GuiUpdate::Error(format!("Settlement error: {}", e)));
                        }
                    }
                    protocol::message::TradeState::Cancelled => {
                        tracing::info!(trade_id = %trade_id, "GUI: returning escrow");
                        if let Err(e) = crate::escrow::return_local_escrow(balance_book, local_peer_id, trade) {
                            tracing::error!(trade_id = %trade_id, error = %e, "GUI: escrow return failed");
                            let _ = update_tx.send(GuiUpdate::Error(format!("Escrow return error: {}", e)));
                        }
                    }
                    _ => {}
                }
            }
            let change = protocol::message::TradeStateChanged {
                trade_id,
                state: state.clone(),
                timestamp,
            };
            let message = protocol::message::Message {
                message_type: protocol::message::MessageType::TradeStateChanged,
                sender: local_peer_id.to_string(),
                payload: serde_json::to_string(&change).unwrap_or_default(),
                timestamp,
            };
            let _ = crate::network::publish_message(swarm, topic, &message);
            let _ = update_tx.send(GuiUpdate::TradeStateChanged { trade_id, state });
        }

        GuiCommand::RequestWalletAddress => {
            // Handled by caller with wallet reference
            let _ = update_tx.send(GuiUpdate::LogMessage(
                "Wallet address request received".to_string(),
            ));
        }

        GuiCommand::RequestWalletBalance => {
            // Handled by caller with wallet reference
            let _ = update_tx.send(GuiUpdate::LogMessage(
                "Wallet balance request received".to_string(),
            ));
        }

        GuiCommand::RequestBtcWalletAddress => {
            let _ = update_tx.send(GuiUpdate::LogMessage(
                "BTC wallet address request received".to_string(),
            ));
        }

        GuiCommand::RequestBtcWalletBalance => {
            let _ = update_tx.send(GuiUpdate::LogMessage(
                "BTC wallet balance request received".to_string(),
            ));
        }
    }
}
