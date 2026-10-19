//! OpenDex Desktop Application
//!
//! Standalone desktop application that embeds both the P2P node and the GUI.
//! This binary runs as a single process on Linux, Windows, and macOS.
//!
//! ## Usage
//!
//! ```bash
//! # Run with default settings
//! cargo run -p desktop-gui
//!
//! # Run with custom port
//! cargo run -p desktop-gui -- --port 4001
//!
//! # Connect to a peer on startup
//! cargo run -p desktop-gui -- --peer /ip4/127.0.0.1/tcp/4001/p2p/PEER_ID
//! ```

use anyhow::Result;
use desktop_gui::app::{BalanceUpdate, GuiCommand, GuiUpdate};
use p2p_node::gui_bridge::{GuiCommand as BridgeCmd, GuiUpdate as BridgeUpd};
use p2p_node::{GuiNodeConfig, run_with_gui};

/// Parse command-line arguments for the desktop application.
/// RPC passwords are read from environment variables, not CLI args, to avoid
/// exposure in process listings (ps, /proc, etc.).
struct DesktopArgs {
    port: u16,
    peer: Option<String>,
    new_identity: bool,
    monero_rpc_url: Option<String>,
    monero_rpc_user: Option<String>,
    btc_rpc_url: Option<String>,
    btc_rpc_user: Option<String>,
}

impl Default for DesktopArgs {
    fn default() -> Self {
        Self {
            port: 4001,
            peer: None,
            new_identity: false,
            monero_rpc_url: None,
            monero_rpc_user: None,
            btc_rpc_url: None,
            btc_rpc_user: None,
        }
    }
}

impl DesktopArgs {
    fn parse(args: &[String]) -> Self {
        let mut config = Self::default();
        let mut i = 0;

        while i < args.len() {
            match args[i].as_str() {
                "--port" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        config.port = val.parse().unwrap_or(4001);
                    }
                }
                "--peer" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        config.peer = Some(val.clone());
                    }
                }
                "--new-identity" => {
                    config.new_identity = true;
                }
                "--monero-rpc" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        config.monero_rpc_url = Some(val.clone());
                    }
                }
                "--monero-rpc-user" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        config.monero_rpc_user = Some(val.clone());
                    }
                }
                "--btc-rpc" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        config.btc_rpc_url = Some(val.clone());
                    }
                }
                "--btc-rpc-user" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        config.btc_rpc_user = Some(val.clone());
                    }
                }
                "-h" | "--help" => {
                    println!(
                        "OpenDex Desktop Application

USAGE:
    opendex-desktop [OPTIONS]

OPTIONS:
    --port <PORT>              Listening TCP port [default: 4001]
    --peer <MULTIADDR>         Dial a peer on startup
    --new-identity             Generate a new identity
    --monero-rpc <URL>         Monero wallet RPC URL
    --monero-rpc-user <U>      Monero wallet RPC username
    --btc-rpc <URL>            Bitcoin wallet RPC URL
    --btc-rpc-user <U>         Bitcoin wallet RPC username

ENVIRONMENT VARIABLES:
    OPENDEX_MONERO_RPC_PASS    Monero wallet RPC password
    OPENDEX_BTC_RPC_PASS       Bitcoin wallet RPC password
    -h, --help                 Print this help message
"
                    );
                    std::process::exit(0);
                }
                _ => {}
            }
            i += 1;
        }

        config
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let desktop_args = DesktopArgs::parse(&args);

    // Read RPC passwords from environment variables to avoid exposure in process listings
    let monero_rpc_pass = std::env::var("OPENDEX_MONERO_RPC_PASS").ok();
    let btc_rpc_pass = std::env::var("OPENDEX_BTC_RPC_PASS").ok();

    // Build the node configuration
    let node_config = GuiNodeConfig {
        port: desktop_args.port,
        peer: desktop_args.peer,
        new_identity: desktop_args.new_identity,
        monero_rpc_url: desktop_args.monero_rpc_url,
        monero_rpc_user: desktop_args.monero_rpc_user,
        monero_rpc_pass,
        btc_rpc_url: desktop_args.btc_rpc_url,
        btc_rpc_user: desktop_args.btc_rpc_user,
        btc_rpc_pass,
    };

    // Create adapter channels between desktop-gui types and p2p-node bridge types
    let (gui_cmd_tx, gui_cmd_rx) = flume::unbounded::<GuiCommand>();
    let (gui_upd_tx, gui_upd_rx) = flume::unbounded::<GuiUpdate>();

    // Run the P2P node on a background thread so the GUI can own the main thread
    let node_config_clone = node_config.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            if let Err(e) = node_main(node_config_clone, gui_cmd_rx, gui_upd_tx).await {
                eprintln!("Node error: {}", e);
            }
        });
        // Keep the thread alive so the process doesn't exit when the node future completes
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    // Launch the GUI on the main thread (required by winit)
    desktop_gui::launch_gui_on_main(gui_cmd_tx, gui_upd_rx);

    Ok(())
}

async fn node_main(
    node_config: GuiNodeConfig,
    gui_cmd_rx: flume::Receiver<GuiCommand>,
    gui_upd_tx: flume::Sender<GuiUpdate>,
) -> Result<()> {
    // Initialize the P2P node (this also sets up tracing)
    let handle = run_with_gui(node_config).await?;

    let peer_id = handle.peer_id;
    let listen_addr = handle.listen_addr;
    let cmd_tx = handle.cmd_tx;
    let update_rx = handle.update_rx;

    tracing::info!(
        peer_id = %peer_id,
        listen_addr = %listen_addr,
        "Node initialized, GUI should be launching"
    );

    // Forward commands: GUI → Node bridge
    tokio::spawn(async move {
        while let Ok(gui_cmd) = gui_cmd_rx.recv_async().await {
            let bridge_cmd = match gui_cmd {
                GuiCommand::PlaceOrder {
                    side,
                    market,
                    price,
                    amount,
                } => BridgeCmd::PlaceOrder {
                    side,
                    market,
                    price,
                    amount,
                },
                GuiCommand::CancelOrder { order_id } => BridgeCmd::CancelOrder { order_id },
                GuiCommand::Fund { asset, amount } => BridgeCmd::Fund { asset, amount },
                GuiCommand::ConnectPeer { multiaddr } => BridgeCmd::ConnectPeer { multiaddr },
                GuiCommand::RequestBalances => BridgeCmd::RequestBalances,
                GuiCommand::RequestOrderBook => BridgeCmd::RequestOrderBook,
                GuiCommand::RequestTrades => BridgeCmd::RequestTrades,
                GuiCommand::AdvanceTradeState { trade_id, state } => {
                    BridgeCmd::AdvanceTradeState { trade_id, state }
                }
                GuiCommand::RequestWalletAddress => BridgeCmd::RequestWalletAddress,
                GuiCommand::RequestWalletBalance => BridgeCmd::RequestWalletBalance,
                GuiCommand::RequestBtcWalletAddress => BridgeCmd::RequestBtcWalletAddress,
                GuiCommand::RequestBtcWalletBalance => BridgeCmd::RequestBtcWalletBalance,
            };
            let _ = cmd_tx.send(bridge_cmd);
        }
    });

    // Forward updates: Node bridge → GUI
    tokio::spawn(async move {
        while let Ok(bridge_upd) = update_rx.recv_async().await {
            let gui_upd = match bridge_upd {
                BridgeUpd::OrderBookSnapshot(orders) => GuiUpdate::OrderBookSnapshot(orders),
                BridgeUpd::TradeAdded(trade) => GuiUpdate::TradeAdded(trade),
                BridgeUpd::TradeStateChanged { trade_id, state } => {
                    GuiUpdate::TradeStateChanged { trade_id, state }
                }
                BridgeUpd::BalancesUpdated(balances) => {
                    let converted: Vec<BalanceUpdate> = balances
                        .into_iter()
                        .map(|(_, asset, bal)| BalanceUpdate {
                            asset,
                            available: bal.available,
                            locked: bal.locked,
                            escrowed: bal.escrowed,
                        })
                        .collect();
                    GuiUpdate::BalancesUpdated(converted)
                }
                BridgeUpd::PeerConnected(peer_id) => GuiUpdate::PeerConnected(peer_id.to_string()),
                BridgeUpd::PeerDisconnected(peer_id) => {
                    GuiUpdate::PeerDisconnected(peer_id.to_string())
                }
                BridgeUpd::OrderResult {
                    order_id,
                    success,
                    error,
                } => GuiUpdate::OrderResult {
                    order_id,
                    success,
                    error,
                },
                BridgeUpd::LogMessage(msg) => GuiUpdate::LogMessage(msg),
                BridgeUpd::Error(msg) => GuiUpdate::Error(msg),
                BridgeUpd::WalletAddress(addr) => GuiUpdate::WalletAddress(addr),
                BridgeUpd::WalletBalance {
                    available,
                    unlocked,
                } => GuiUpdate::WalletBalance {
                    available,
                    unlocked,
                },
                BridgeUpd::WalletConnected { connected, address } => {
                    GuiUpdate::WalletConnected { connected, address }
                }
                BridgeUpd::BtcWalletAddress(addr) => GuiUpdate::BtcWalletAddress(addr),
                BridgeUpd::BtcWalletBalance { balance } => GuiUpdate::BtcWalletBalance { balance },
                BridgeUpd::BtcWalletConnected { connected, address } => {
                    GuiUpdate::BtcWalletConnected { connected, address }
                }
            };
            let _ = gui_upd_tx.send(gui_upd);
        }
    });

    Ok(())
}
