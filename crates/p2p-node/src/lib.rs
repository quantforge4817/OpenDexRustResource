#![allow(unreachable_code)]

use zeroize::Zeroize;

pub mod balances;
pub mod btc_wallet_client;
pub mod cli;
pub mod escrow;
pub mod exchange;
pub mod message_handler;
pub mod network;
pub mod order_factory;
pub mod orderbook;
#[cfg(feature = "fiat")]
pub mod p2p_escrow;
#[cfg(feature = "fiat")]
pub mod p2p_offer_book;
pub mod persistence;
pub mod settlement;
pub mod sync;
pub mod trade_state;
pub mod trades;
pub mod wallet_client;

/// L31: Log redaction framework to prevent sensitive data in logs.
pub mod log_redact;

// Kani proofs disabled: the p2p-node crate has heavy dependencies (libp2p, chrono, reqwest)
// that cause state space explosion during symbolic execution (166K+ SAT variables).
// The orderbook_kani.rs file is kept for reference but not compiled.
// #[cfg(kani)]
// mod orderbook_kani;

/// Bridge types for GUI communication.
/// This module is public so external crates (e.g., desktop-gui) can construct
/// the channels without depending on internal main.rs details.
pub mod gui_bridge;

use libp2p::{gossipsub, identify, kad, ping, swarm::NetworkBehaviour};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "OpenDexBehaviourEvent")]
#[allow(clippy::large_enum_variant)]
pub struct OpenDexBehaviour {
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub ping: ping::Behaviour,
    pub identify: identify::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum OpenDexBehaviourEvent {
    Kademlia(kad::Event),
    Ping(ping::Event),
    Identify(identify::Event),
    Gossipsub(gossipsub::Event),
}

impl From<kad::Event> for OpenDexBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        Self::Kademlia(event)
    }
}

impl From<ping::Event> for OpenDexBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        Self::Ping(event)
    }
}

impl From<identify::Event> for OpenDexBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        Self::Identify(event)
    }
}

impl From<gossipsub::Event> for OpenDexBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        Self::Gossipsub(event)
    }
}

// Re-exports needed by external callers (e.g., desktop-gui)
pub use libp2p::{PeerId, identity};
pub use network::build_swarm;

/// Configuration for running the P2P node with a GUI.
#[derive(Debug, Clone)]
pub struct GuiNodeConfig {
    /// TCP port to listen on.
    pub port: u16,
    /// Optional peer multiaddress to dial on startup.
    pub peer: Option<String>,
    /// Generate a new identity instead of loading the saved one.
    pub new_identity: bool,
    /// Optional Monero wallet RPC URL.
    pub monero_rpc_url: Option<String>,
    /// Optional Monero wallet RPC username.
    pub monero_rpc_user: Option<String>,
    /// Optional Monero wallet RPC password (zeroized on drop).
    pub monero_rpc_pass: Option<String>,
    /// Optional Bitcoin wallet RPC URL.
    pub btc_rpc_url: Option<String>,
    /// Optional Bitcoin wallet RPC username.
    pub btc_rpc_user: Option<String>,
    /// Optional Bitcoin wallet RPC password (zeroized on drop).
    pub btc_rpc_pass: Option<String>,
}

impl Drop for GuiNodeConfig {
    fn drop(&mut self) {
        // Zeroize sensitive password fields on drop to prevent leakage in memory dumps
        if let Some(mut pass) = self.monero_rpc_pass.take() {
            pass.zeroize();
        }
        if let Some(mut pass) = self.btc_rpc_pass.take() {
            pass.zeroize();
        }
    }
}

impl Default for GuiNodeConfig {
    fn default() -> Self {
        Self {
            port: 4001,
            peer: None,
            new_identity: false,
            monero_rpc_url: None,
            monero_rpc_user: None,
            monero_rpc_pass: None,
            btc_rpc_url: None,
            btc_rpc_user: None,
            btc_rpc_pass: None,
        }
    }
}

// Note: GuiNodeConfig derives Clone, but the Drop impl zeroizes passwords.
// Cloning creates independent copies; each copy will zeroize its own passwords on drop.

/// Result of initializing the node for GUI mode.
pub struct GuiNodeHandle {
    /// Peer ID of this node.
    pub peer_id: String,
    /// Listen address as a string.
    pub listen_addr: String,
    /// Sender for GUI commands (GUI sends commands to the node via this).
    pub cmd_tx: flume::Sender<gui_bridge::GuiCommand>,
    /// Receiver for GUI updates (GUI receives updates from the node via this).
    pub update_rx: flume::Receiver<gui_bridge::GuiUpdate>,
}

/// Initialize and run the P2P node in GUI mode.
///
/// This function performs all node initialization (identity, swarm, wallets,
/// persistence) and spawns the node event loop. It returns a handle containing
/// the channels the GUI should use to communicate with the node.
///
/// The caller is responsible for launching the GUI with the returned channels.
pub async fn run_with_gui(mut config: GuiNodeConfig) -> anyhow::Result<GuiNodeHandle> {
    use std::fs;
    use uuid::Uuid;

    let port = config.port;
    let peer_addr = config.peer.clone();

    let node_dir = data_dir(&port);
    fs::create_dir_all(&node_dir)?;

    // Initialize structured logging
    init_audit_logging(&node_dir)?;

    let id_keys = if config.new_identity {
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

    let listen_addr: libp2p::Multiaddr = format!("/ip4/127.0.0.1/tcp/{}", port).parse()?;
    swarm.listen_on(listen_addr.clone())?;

    tracing::info!("Listening for peers...");
    tracing::info!(address = %format!("{}/p2p/{}", listen_addr, local_peer_id));

    if let Some(addr) = peer_addr {
        let peer: libp2p::Multiaddr = addr.parse()?;
        tracing::info!(peer = %peer, "Dialing peer");
        swarm.dial(peer)?;
    }

    let orders_path = node_dir.join("orders.json");
    let trades_path = node_dir.join("trades.json");
    let balances_path = node_dir.join("balances.json");

    let order_book = orderbook::OrderBook::load_orders(orders_path)?;
    let trade_book = trades::TradeBook::load_trades(trades_path)?;
    let balance_book = balances::BalanceBook::load(balances_path)?;

    let pending_trade_states: std::collections::HashMap<Uuid, message_handler::PendingTradeState> = std::collections::HashMap::new();
    #[cfg(feature = "fiat")]
    let p2p_offer_book = p2p_offer_book::P2POfferBook::load();

    // Initialize Monero wallet client if configured
    let monero_wallet = if config.monero_rpc_url.is_some()
        || config.monero_rpc_user.is_some()
        || config.monero_rpc_pass.is_some()
    {
        let wallet_config = wallet_client::MoneroWalletConfig {
            rpc_url: config
                .monero_rpc_url
                .take()
                .unwrap_or_else(|| "http://127.0.0.1:18082".to_string()),
            username: config.monero_rpc_user.take(),
            password: config.monero_rpc_pass.take(),
        };
        let mut wallet = wallet_client::MoneroWalletClient::new(wallet_config);
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
    let btc_wallet = if config.btc_rpc_url.is_some()
        || config.btc_rpc_user.is_some()
        || config.btc_rpc_pass.is_some()
    {
        let wallet_config = btc_wallet_client::BitcoinWalletConfig {
            rpc_url: config
                .btc_rpc_url
                .take()
                .unwrap_or_else(|| "http://127.0.0.1:8332".to_string()),
            username: config.btc_rpc_user.take(),
            password: config.btc_rpc_pass.take(),
            wallet_name: None,
        };
        let mut wallet = btc_wallet_client::BitcoinWalletClient::new(wallet_config);
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

    // Create channels for GUI communication
    // cmd_tx/cmd_rx: GUI → Node (commands)
    // update_tx/update_rx: Node → GUI (updates)
    let (cmd_tx, cmd_rx) = flume::unbounded::<gui_bridge::GuiCommand>();
    let (update_tx, update_rx) = flume::unbounded::<gui_bridge::GuiUpdate>();

    // Spawn the node event loop
    tokio::spawn(async move {
        #[cfg(feature = "fiat")]
        {
            run_node_loop_inner(
                swarm,
                topic,
                local_peer_id,
                order_book,
                trade_book,
                balance_book,
                pending_trade_states,
                p2p_offer_book,
                monero_wallet,
                btc_wallet,
                cmd_rx,
                update_tx,
            )
            .await;
        }
        #[cfg(not(feature = "fiat"))]
        {
            run_node_loop_inner(
                swarm,
                topic,
                local_peer_id,
                order_book,
                trade_book,
                balance_book,
                pending_trade_states,
                monero_wallet,
                btc_wallet,
                cmd_rx,
                update_tx,
            )
            .await;
        }
    });

    // Return the channels the GUI adapter needs:
    // - cmd_tx: GUI sends commands to the node via this
    // - update_rx: GUI receives updates from the node via this
    Ok(GuiNodeHandle {
        peer_id: local_peer_id.to_string(),
        listen_addr: listen_addr.to_string(),
        cmd_tx,
        update_rx,
    })
}

/// Internal node event loop for GUI mode (with fiat support).
#[cfg(feature = "fiat")]
async fn run_node_loop_inner(
    mut swarm: libp2p::Swarm<OpenDexBehaviour>,
    topic: gossipsub::IdentTopic,
    local_peer_id: PeerId,
    mut order_book: orderbook::OrderBook,
    mut trade_book: trades::TradeBook,
    mut balance_book: balances::BalanceBook,
    mut pending_trade_states: std::collections::HashMap<
        uuid::Uuid,
        message_handler::PendingTradeState,
    >,
    mut p2p_offer_book: p2p_offer_book::P2POfferBook,
    mut monero_wallet: Option<wallet_client::MoneroWalletClient>,
    mut btc_wallet: Option<btc_wallet_client::BitcoinWalletClient>,
    cmd_rx: flume::Receiver<gui_bridge::GuiCommand>,
    update_tx: flume::Sender<gui_bridge::GuiUpdate>,
) {
    use futures::StreamExt;
    loop {
        tokio::select! {
            biased;

            cmd = cmd_rx.recv_async() => {
                if let Ok(command) = cmd {
                    handle_gui_command_inner(
                        command,
                        &mut swarm,
                        &topic,
                        &local_peer_id,
                        &mut order_book,
                        &mut trade_book,
                        &mut balance_book,
                        &mut p2p_offer_book,
                        monero_wallet.as_mut(),
                        btc_wallet.as_mut(),
                        &update_tx,
                    ).await;
                }
            }

            event = swarm.next() => {
                if let Some(event) = &event {
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

/// Internal node event loop for GUI mode (without fiat support).
#[cfg(not(feature = "fiat"))]
#[allow(clippy::too_many_arguments)]
async fn run_node_loop_inner(
    mut swarm: libp2p::Swarm<OpenDexBehaviour>,
    topic: gossipsub::IdentTopic,
    local_peer_id: PeerId,
    mut order_book: orderbook::OrderBook,
    mut trade_book: trades::TradeBook,
    mut balance_book: balances::BalanceBook,
    mut pending_trade_states: std::collections::HashMap<
        uuid::Uuid,
        message_handler::PendingTradeState,
    >,
    mut monero_wallet: Option<wallet_client::MoneroWalletClient>,
    mut btc_wallet: Option<btc_wallet_client::BitcoinWalletClient>,
    cmd_rx: flume::Receiver<gui_bridge::GuiCommand>,
    update_tx: flume::Sender<gui_bridge::GuiUpdate>,
) {
    use futures::StreamExt;
    loop {
        tokio::select! {
            biased;

            cmd = cmd_rx.recv_async() => {
                if let Ok(command) = cmd {
                    handle_gui_command_inner(
                        command,
                        &mut swarm,
                        &topic,
                        &local_peer_id,
                        &mut order_book,
                        &mut trade_book,
                        &mut balance_book,
                        monero_wallet.as_mut(),
                        btc_wallet.as_mut(),
                        &update_tx,
                    ).await;
                }
            }

            event = swarm.next() => {
                if let Some(event) = &event {
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
                    };
                    if let Err(e) = message_handler::handle_swarm_event(event, &mut ctx) {
                        tracing::error!(error = %e, "Swarm event error");
                    }
                }
            }
        }
    }
}

/// Handle a GUI command within the node event loop (with fiat support).
#[cfg(feature = "fiat")]
async fn handle_gui_command_inner(
    command: gui_bridge::GuiCommand,
    swarm: &mut libp2p::Swarm<OpenDexBehaviour>,
    topic: &gossipsub::IdentTopic,
    local_peer_id: &PeerId,
    order_book: &mut orderbook::OrderBook,
    trade_book: &mut trades::TradeBook,
    balance_book: &mut balances::BalanceBook,
    p2p_offer_book: &mut p2p_offer_book::P2POfferBook,
    monero_wallet: Option<&mut wallet_client::MoneroWalletClient>,
    btc_wallet: Option<&mut btc_wallet_client::BitcoinWalletClient>,
    update_tx: &flume::Sender<gui_bridge::GuiUpdate>,
) {
    use gui_bridge::GuiUpdate;
    use protocol::message::AMOUNT_SCALE;

    match command {
        gui_bridge::GuiCommand::PlaceOrder {
            side,
            market,
            price,
            amount,
        } => {
            let order =
                order_factory::create_order(local_peer_id.to_string(), market, side, price, amount);
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
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                schema_version: protocol::message::SCHEMA_VERSION,
            };

            if let Err(e) = network::publish_message(swarm, topic, &order_message) {
                let _ = balance_book.unlock_for_order(&order);
                let _ = balance_book.save();
                let _ = update_tx.send(GuiUpdate::OrderResult {
                    order_id,
                    success: false,
                    error: Some(format!("Publish failed: {}", e)),
                });
                return;
            }

            order_book.upsert_order(order);
            let _ = order_book.save_orders();

            let _ = update_tx.send(GuiUpdate::OrderResult {
                order_id,
                success: true,
                error: None,
            });
        }
        gui_bridge::GuiCommand::CancelOrder { order_id } => {
            if let Some(order) = order_book.get_order(&order_id).cloned()
                && order_book.remove_order(order_id)
            {
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
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    schema_version: protocol::message::SCHEMA_VERSION,
                };
                let _ = network::publish_message(swarm, topic, &message);
            }
        }
        gui_bridge::GuiCommand::Fund { asset, amount } => {
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
            let _ = update_tx.send(GuiUpdate::LogMessage(format!(
                "Funded {} with {}",
                asset_label, amount
            )));
            let mut balances = Vec::new();
            for a in [protocol::message::Asset::BTC, protocol::message::Asset::XMR] {
                let b = balance_book.balance(&trader, &a);
                balances.push((trader.clone(), a, b));
            }
            let _ = update_tx.send(GuiUpdate::BalancesUpdated(balances));
        }
        gui_bridge::GuiCommand::ConnectPeer { multiaddr } => {
            match multiaddr.parse::<libp2p::Multiaddr>() {
                Ok(addr) => {
                    if let Err(e) = swarm.dial(addr) {
                        let _ = update_tx.send(GuiUpdate::Error(format!("Dial failed: {}", e)));
                    }
                }
                Err(e) => {
                    let _ = update_tx.send(GuiUpdate::Error(format!("Invalid address: {}", e)));
                }
            }
        }
        gui_bridge::GuiCommand::RequestBalances => {
            let trader = local_peer_id.to_string();
            let mut balances = Vec::new();
            for asset in [protocol::message::Asset::BTC, protocol::message::Asset::XMR] {
                let balance = balance_book.balance(&trader, &asset);
                balances.push((trader.clone(), asset, balance));
            }
            let _ = update_tx.send(GuiUpdate::BalancesUpdated(balances));
        }
        gui_bridge::GuiCommand::RequestOrderBook => {
            let orders = order_book.active_orders();
            let _ = update_tx.send(GuiUpdate::OrderBookSnapshot(orders));
        }
        gui_bridge::GuiCommand::RequestTrades => {
            for trade in trade_book.all_trades() {
                let _ = update_tx.send(GuiUpdate::TradeAdded(trade));
            }
        }
        gui_bridge::GuiCommand::AdvanceTradeState { trade_id, state } => {
            if trade_book.get_trade(trade_id).is_some() {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let _ = trade_book.transition_trade(trade_id, state.clone(), timestamp);
                let _ = update_tx.send(GuiUpdate::TradeStateChanged { trade_id, state });
            }
        }
        gui_bridge::GuiCommand::RequestWalletAddress => {
            if let Some(wallet) = monero_wallet {
                match wallet.get_address().await {
                    Ok(addr) => {
                        let _ = update_tx.send(GuiUpdate::WalletAddress(addr));
                    }
                    Err(e) => {
                        let _ = update_tx
                            .send(GuiUpdate::Error(format!("Wallet address error: {}", e)));
                    }
                }
            } else {
                let _ = update_tx.send(GuiUpdate::WalletConnected {
                    connected: false,
                    address: None,
                });
            }
        }
        gui_bridge::GuiCommand::RequestWalletBalance => {
            if let Some(wallet) = monero_wallet {
                match wallet.get_balance_xmr(0).await {
                    Ok((available, unlocked)) => {
                        let _ = update_tx.send(GuiUpdate::WalletBalance {
                            available,
                            unlocked,
                        });
                    }
                    Err(e) => {
                        let _ = update_tx
                            .send(GuiUpdate::Error(format!("Wallet balance error: {}", e)));
                    }
                }
            }
        }
        gui_bridge::GuiCommand::RequestBtcWalletAddress => {
            if let Some(wallet) = btc_wallet {
                match wallet.get_new_address(None).await {
                    Ok(addr) => {
                        let _ = update_tx.send(GuiUpdate::BtcWalletAddress(addr));
                    }
                    Err(e) => {
                        let _ = update_tx
                            .send(GuiUpdate::Error(format!("BTC wallet address error: {}", e)));
                    }
                }
            } else {
                let _ = update_tx.send(GuiUpdate::BtcWalletConnected {
                    connected: false,
                    address: None,
                });
            }
        }
        gui_bridge::GuiCommand::RequestBtcWalletBalance => {
            if let Some(wallet) = btc_wallet {
                match wallet.get_balance().await {
                    Ok(balance) => {
                        let _ = update_tx.send(GuiUpdate::BtcWalletBalance { balance });
                    }
                    Err(e) => {
                        let _ = update_tx
                            .send(GuiUpdate::Error(format!("BTC wallet balance error: {}", e)));
                    }
                }
            }
        }
    }
}

/// Handle a GUI command within the node event loop (without fiat support).
#[cfg(not(feature = "fiat"))]
#[allow(clippy::too_many_arguments)]
async fn handle_gui_command_inner(
    command: gui_bridge::GuiCommand,
    swarm: &mut libp2p::Swarm<OpenDexBehaviour>,
    topic: &gossipsub::IdentTopic,
    local_peer_id: &PeerId,
    order_book: &mut orderbook::OrderBook,
    trade_book: &mut trades::TradeBook,
    balance_book: &mut balances::BalanceBook,
    monero_wallet: Option<&mut wallet_client::MoneroWalletClient>,
    btc_wallet: Option<&mut btc_wallet_client::BitcoinWalletClient>,
    update_tx: &flume::Sender<gui_bridge::GuiUpdate>,
) {
    use gui_bridge::GuiUpdate;
    use protocol::message::AMOUNT_SCALE;

    match command {
        gui_bridge::GuiCommand::PlaceOrder {
            side,
            market,
            price,
            amount,
        } => {
            let order =
                order_factory::create_order(local_peer_id.to_string(), market, side, price, amount);
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
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                schema_version: protocol::message::SCHEMA_VERSION,
            };

            if let Err(e) = network::publish_message(swarm, topic, &order_message) {
                let _ = balance_book.unlock_for_order(&order);
                let _ = balance_book.save();
                let _ = update_tx.send(GuiUpdate::OrderResult {
                    order_id,
                    success: false,
                    error: Some(format!("Publish failed: {}", e)),
                });
                return;
            }

            order_book.upsert_order(order);
            let _ = order_book.save_orders();

            let _ = update_tx.send(GuiUpdate::OrderResult {
                order_id,
                success: true,
                error: None,
            });
        }
        gui_bridge::GuiCommand::CancelOrder { order_id } => {
            if let Some(order) = order_book.get_order(&order_id).cloned()
                && order_book.remove_order(order_id)
            {
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
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    schema_version: protocol::message::SCHEMA_VERSION,
                };
                let _ = network::publish_message(swarm, topic, &message);
            }
        }
        gui_bridge::GuiCommand::Fund { asset, amount } => {
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
            let _ = update_tx.send(GuiUpdate::LogMessage(format!(
                "Funded {} with {}",
                asset_label, amount
            )));
            let mut balances = Vec::new();
            for a in [protocol::message::Asset::BTC, protocol::message::Asset::XMR] {
                let b = balance_book.balance(&trader, &a);
                balances.push((trader.clone(), a, b));
            }
            let _ = update_tx.send(GuiUpdate::BalancesUpdated(balances));
        }
        gui_bridge::GuiCommand::ConnectPeer { multiaddr } => {
            match multiaddr.parse::<libp2p::Multiaddr>() {
                Ok(addr) => {
                    if let Err(e) = swarm.dial(addr) {
                        let _ = update_tx.send(GuiUpdate::Error(format!("Dial failed: {}", e)));
                    }
                }
                Err(e) => {
                    let _ = update_tx.send(GuiUpdate::Error(format!("Invalid address: {}", e)));
                }
            }
        }
        gui_bridge::GuiCommand::RequestBalances => {
            let trader = local_peer_id.to_string();
            let mut balances = Vec::new();
            for asset in [protocol::message::Asset::BTC, protocol::message::Asset::XMR] {
                let balance = balance_book.balance(&trader, &asset);
                balances.push((trader.clone(), asset, balance));
            }
            let _ = update_tx.send(GuiUpdate::BalancesUpdated(balances));
        }
        gui_bridge::GuiCommand::RequestOrderBook => {
            let orders = order_book.active_orders();
            let _ = update_tx.send(GuiUpdate::OrderBookSnapshot(orders));
        }
        gui_bridge::GuiCommand::RequestTrades => {
            for trade in trade_book.all_trades() {
                let _ = update_tx.send(GuiUpdate::TradeAdded(trade));
            }
        }
        gui_bridge::GuiCommand::AdvanceTradeState { trade_id, state } => {
            if trade_book.get_trade(trade_id).is_some() {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let _ = trade_book.transition_trade(trade_id, state.clone(), timestamp);
                let _ = update_tx.send(GuiUpdate::TradeStateChanged { trade_id, state });
            }
        }
        gui_bridge::GuiCommand::RequestWalletAddress => {
            if let Some(wallet) = monero_wallet {
                match wallet.get_address().await {
                    Ok(addr) => {
                        let _ = update_tx.send(GuiUpdate::WalletAddress(addr));
                    }
                    Err(e) => {
                        let _ = update_tx
                            .send(GuiUpdate::Error(format!("Wallet address error: {}", e)));
                    }
                }
            } else {
                let _ = update_tx.send(GuiUpdate::WalletConnected {
                    connected: false,
                    address: None,
                });
            }
        }
        gui_bridge::GuiCommand::RequestWalletBalance => {
            if let Some(wallet) = monero_wallet {
                match wallet.get_balance_xmr(0).await {
                    Ok((available, unlocked)) => {
                        let _ = update_tx.send(GuiUpdate::WalletBalance {
                            available,
                            unlocked,
                        });
                    }
                    Err(e) => {
                        let _ = update_tx
                            .send(GuiUpdate::Error(format!("Wallet balance error: {}", e)));
                    }
                }
            }
        }
        gui_bridge::GuiCommand::RequestBtcWalletAddress => {
            if let Some(wallet) = btc_wallet {
                match wallet.get_new_address(None).await {
                    Ok(addr) => {
                        let _ = update_tx.send(GuiUpdate::BtcWalletAddress(addr));
                    }
                    Err(e) => {
                        let _ = update_tx
                            .send(GuiUpdate::Error(format!("BTC wallet address error: {}", e)));
                    }
                }
            } else {
                let _ = update_tx.send(GuiUpdate::BtcWalletConnected {
                    connected: false,
                    address: None,
                });
            }
        }
        gui_bridge::GuiCommand::RequestBtcWalletBalance => {
            if let Some(wallet) = btc_wallet {
                match wallet.get_balance().await {
                    Ok(balance) => {
                        let _ = update_tx.send(GuiUpdate::BtcWalletBalance { balance });
                    }
                    Err(e) => {
                        let _ = update_tx
                            .send(GuiUpdate::Error(format!("BTC wallet balance error: {}", e)));
                    }
                }
            }
        }
    }
}

/// Internal helper functions extracted from main.rs for reuse.
mod internal {
    use std::path::PathBuf;

    /// Compute the identity file path for a given port.
    pub fn identity_path(port: &str) -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push(".opendex");
        path.push(format!("identity-{}.key", port));
        path
    }

    /// Compute the data directory for a given port.
    pub fn data_dir(port: &str) -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push(".opendex");
        path.push(format!("node-{}", port));
        path
    }

    /// Initialize structured logging with file-based audit output.
    /// L31: A log redaction layer masks sensitive field values (password, secret,
    /// token, key) so that accidental logging of credentials is prevented.
    pub fn init_audit_logging(node_dir: &std::path::Path) -> anyhow::Result<()> {
        use std::fs;
        use tracing_subscriber::prelude::*;

        let audit_dir = node_dir.join("audit");
        fs::create_dir_all(&audit_dir)?;

        let audit_log_path = audit_dir.join("audit.log");
        let error_log_path = audit_dir.join("error.log");

        let audit_file = fs::File::create(&audit_log_path)?;
        let audit_layer = tracing_subscriber::fmt::layer()
            .with_writer(audit_file)
            .with_ansi(false)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            );

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

        Ok(())
    }

    /// Load or create the node identity keypair.
    pub fn load_or_create_identity(port: &str) -> anyhow::Result<libp2p::identity::Keypair> {
        use std::fs;

        let path = identity_path(port);

        if path.exists() {
            let bytes = fs::read(&path)?;
            let key = libp2p::identity::Keypair::from_protobuf_encoding(&bytes)?;
            Ok(key)
        } else {
            let key = libp2p::identity::Keypair::generate_ed25519();
            let bytes = key.to_protobuf_encoding()?;

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(&path, &bytes)?;

            Ok(key)
        }
    }
}

/// Compute the data directory for a given port.
fn data_dir(port: &u16) -> std::path::PathBuf {
    internal::data_dir(&port.to_string())
}

/// Initialize structured logging with file-based audit output.
fn init_audit_logging(node_dir: &std::path::Path) -> anyhow::Result<()> {
    internal::init_audit_logging(node_dir)
}

/// Load or create the node identity keypair.
fn load_or_create_identity(port: &u16) -> anyhow::Result<libp2p::identity::Keypair> {
    internal::load_or_create_identity(&port.to_string())
}
