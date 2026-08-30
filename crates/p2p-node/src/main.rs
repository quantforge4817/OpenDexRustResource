use std::collections::HashMap;

use uuid::Uuid;

use network::build_swarm;

use crate::cli::parse_command;

use p2p_node::{OpenDexBehaviour, OpenDexBehaviourEvent};

use trades::TradeBook;

use anyhow::Result;

use futures::StreamExt;

use balances::BalanceBook;

use command_handler::handle_command;

mod settlement;

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

use orderbook::OrderBook;

use tokio::io::{self, AsyncBufReadExt};

use libp2p::{Multiaddr, PeerId, gossipsub, identity};

use std::{fs, path::PathBuf};

fn identity_path(port: &str) -> PathBuf {
    let mut path = dirs::home_dir().expect("Cannot find home directory");

    path.push(".opendex");
    path.push(format!("identity-{}.key", port));

    path
}

fn data_dir(port: &str) -> PathBuf {
    let mut path = dirs::home_dir().expect("Cannot find home directory");

    path.push(".opendex");
    path.push(format!("node-{}", port));

    path
}

fn load_or_create_identity(port: &str) -> Result<identity::Keypair> {
    let path = identity_path(port);

    if path.exists() {
        let bytes = fs::read(&path)?;

        let key = identity::Keypair::from_protobuf_encoding(&bytes)?;

        Ok(key)
    } else {
        let key = identity::Keypair::generate_ed25519();

        let bytes = key.to_protobuf_encoding()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, bytes)?;

        Ok(key)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let port = args
        .iter()
        .find(|arg| arg.parse::<u16>().is_ok())
        .cloned()
        .unwrap_or_else(|| "4001".to_string());

    let peer_addr = args
        .iter()
        .position(|arg| arg == "--peer")
        .and_then(|i| args.get(i + 1))
        .cloned();

    tracing_subscriber::fmt::init();

    let id_keys = if args.iter().any(|arg| arg == "--new-identity") {
        identity::Keypair::generate_ed25519()
    } else {
        load_or_create_identity(&port)?
    };

    let local_peer_id = PeerId::from(id_keys.public());

    println!("Peer ID:");
    println!("{}", local_peer_id);

    let mut swarm = build_swarm(id_keys)?;

    let topic = gossipsub::IdentTopic::new("opendex-orders");

    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&topic)
        .expect("subscribe failed");

    let listen_addr: Multiaddr = format!("/ip4/127.0.0.1/tcp/{}", port).parse()?;

    swarm.listen_on(listen_addr.clone())?;

    println!("Listening for peers...");
    println!("Address:");
    println!("{}/p2p/{}", listen_addr, local_peer_id);

    if let Some(addr) = peer_addr {
        let peer: Multiaddr = addr.parse()?;

        println!("Dialing peer: {}", peer);
        swarm.dial(peer)?;
    }

    let node_dir = data_dir(&port);
    fs::create_dir_all(&node_dir)?;

    let orders_path = node_dir.join("orders.json");
    let trades_path = node_dir.join("trades.json");
    let balances_path = node_dir.join("balances.json");

    let mut order_book = OrderBook::load_orders(orders_path)?;
    let mut trade_book = TradeBook::load_trades(trades_path)?;
    let mut balance_book = BalanceBook::load(balances_path)?;

    let mut pending_trade_states: HashMap<Uuid, message_handler::PendingTradeState> =
        HashMap::new();

    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    loop {
        tokio::select! {
                    line = lines.next_line() => {
                        if let Some(input) = line? {
                            let command = parse_command(&input);

                            handle_command(
                                command,
                                &mut swarm,
                                &topic,
                                &local_peer_id,
                                &mut order_book,
                                &mut trade_book,
                                &mut balance_book,
                            )?;
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
        };

        message_handler::handle_swarm_event(event, &mut ctx)?;

                        }
                    }
                }
    }
}
