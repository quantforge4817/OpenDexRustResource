use std::collections::HashMap;

use network::{build_swarm, publish_message};

use crate::cli::{Command, parse_command};

use trades::TradeBook;

use protocol::state::StateSnapshot;

use anyhow::Result;

use futures::StreamExt;

use protocol::message::{
    AMOUNT_SCALE, Asset, MessageType, OpenDexMessage, Order, OrderCancelled, Trade, TradeState,
    TradeStateChanged,
};

use balances::BalanceBook;

use message_handler::{
    handle_order_cancelled, handle_order_created, handle_state_request, handle_state_response,
    handle_trade_created, handle_trade_state_changed,
};

mod settlement;

mod balances;

mod cli;

use order_factory::create_order;

mod trade_state;
mod trades;

mod orderbook;

mod order_factory;

mod network;

mod message_handler;

mod sync;

use orderbook::OrderBook;

use tokio::io::{self, AsyncBufReadExt};

use libp2p::{
    Multiaddr, PeerId, gossipsub, identify, identity, kad, ping, swarm::NetworkBehaviour,
};

use std::{fs, path::PathBuf};

use tracing_subscriber;

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

fn format_amount(amount: u64) -> String {
    let whole = amount / AMOUNT_SCALE;
    let fractional = amount % AMOUNT_SCALE;

    if fractional == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fractional:08}")
            .trim_end_matches('0')
            .to_string()
    }
}

fn transition_and_publish_trade(
    trade_book: &mut TradeBook,
    trade_id: uuid::Uuid,
    next_state: TradeState,
    timestamp: u64,
    swarm: &mut libp2p::Swarm<OpenDexBehaviour>,
    topic: &libp2p::gossipsub::IdentTopic,
    local_peer_id: &PeerId,
) -> Result<bool> {
    let changed = trade_book.transition_trade(trade_id, next_state.clone(), timestamp)?;

    if !changed {
        return Ok(false);
    }
    let change = TradeStateChanged {
        trade_id,
        state: next_state,
        timestamp,
    };

    let message = OpenDexMessage {
        message_type: MessageType::TradeStateChanged,
        sender: local_peer_id.to_string(),
        payload: serde_json::to_string(&change)?,
        timestamp,
    };

    network::publish_message(swarm, topic, &message)?;

    println!("Published TradeStateChanged: {}", trade_id);

    Ok(true)
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

#[derive(NetworkBehaviour)]
struct OpenDexBehaviour {
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
    gossipsub: gossipsub::Behaviour,
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

    let mut pending_trade_states: HashMap<uuid::Uuid, TradeStateChanged> = HashMap::new();

    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    loop {
        tokio::select! {
                                                                    line = lines.next_line() => {
                                                                        if let Some(input) = line? {
                                                                            match parse_command(&input) {
                                                                                Command::Fund { asset, amount } => {
                                                                                    let trader = local_peer_id.to_string();

                                                                                    let scaled_amount = amount
                                                                                        .checked_mul(AMOUNT_SCALE)
                                                                                        .ok_or_else(|| anyhow::anyhow!("fund amount overflow"))?;

                                                                                    balance_book.credit(
                                                                                        &trader,
                                                                                        asset.clone(),
                                                                                        scaled_amount,
                                                                                    )?;

                                                                                    balance_book.save()?;

                                                                                    let balance = balance_book.balance(&trader, &asset);

                                                                                    println!(
                                                                                        "Funded {:?}: available={}, locked={}",
                                                                                        asset,
                                                                                        format_amount(balance.available),
                                                                                        format_amount(balance.locked)
                                                                                    );
                                                                                }

                                                                                Command::ShowBalances => {
                                                                                    let trader = local_peer_id.to_string();

                                                                                    println!("=== Balances ===");

                                                                                    for asset in [Asset::BTC, Asset::XMR] {
                                                                                        let balance =
                                                                                            balance_book.balance(&trader, &asset);

                                                                                        println!(
                                                                                            "{:?}: available={}, locked={}",
                                                                                            asset,
                                                                                            format_amount(balance.available),
                                                                                            format_amount(balance.locked)
                                                                                        );
                                                                                    }
                                                                                }

                                                                                Command::Order {
                                                                                    side,
                                                                                    market,
                                                                                    price,
                                                                                    amount,
                                                                                } => {
                                                                                    let order = create_order(
                                                                                        local_peer_id.to_string(),
                                                                                        market,
                                                                                        side,
                                                                                        price,
                                                                                        amount,
                                                                                    );

                                                                                    println!("Created order: {}", order.id);

                                                                                    if let Err(e) =
                                                                                        balance_book.lock_for_order(&order)
                                                                                    {
                                                                                        eprintln!(
                                                                                            "Cannot create order {}: {}",
                                                                                            order.id, e
                                                                                        );
                                                                                        continue;
                                                                                    }

                                                                                    balance_book.save()?;

                                                                                    println!(
                                                                                        "Locked funds for order {}",
                                                                                        order.id
                                                                                    );

                                                                                    let order_message = OpenDexMessage {
                                                                                        message_type: MessageType::OrderCreated,
                                                                                        sender: local_peer_id.to_string(),
                                                                                        payload: serde_json::to_string(&order)?,
                                                                                        timestamp: chrono::Utc::now()
                                                                                            .timestamp() as u64,
                                                                                    };

                                                                                    match publish_message(
                                                                                        &mut swarm,
                                                                                        &topic,
                                                                                        &order_message,
                                                                                    ) {
                                                                                        Ok(_) => {
                                                                                            println!("Published order");
                                                                                        }

                                                                                        Err(e) => {
                                                                                            println!(
                                                                                                "Publish failed: {:?}",
                                                                                                e
                                                                                            );

                                                                                            if let Err(unlock_error) =
                                                                                                balance_book.unlock_for_order(&order)
                                                                                            {
                                                                                                eprintln!(
                                                                                                    "CRITICAL: failed to unlock funds for order {}: {}",
                                                                                                    order.id,
                                                                                                    unlock_error
                                                                                                );
                                                                                            }

                                                                                            balance_book.save()?;

                                                                                            println!(
                                                                                                "Order {} was not created",
                                                                                                order.id
                                                                                            );

                                                                                            continue;
                                                                                        }
                                                                                    }

                                                                                    let trades =
                                                                                        order_book.match_order(&order);

                                                                                    order_book.add_order(order.clone());

                                                                                    if trades.is_empty() {
                                                                                        order_book.save_orders()?;
                                                                                    } else {
                                                                                        for trade in trades {
                                                                                            println!("=== Trade Created ===");
                                                                                            println!("{:?}", trade);

                                                                                            if !trade_book.add_trade(trade.clone()) {
                                                                                                eprintln!(
                                                                                                    "Trade was already present: {}",
                                                                                                    trade.id
                                                                                                );
                                                                                                continue;
                                                                                            }

                                                                                            if !order_book.apply_trade(&trade) {
                                                                                                eprintln!(
                                                                                                    "Trade {} could not be applied to the order book",
                                                                                                    trade.id
                                                                                                );
                                                                                                continue;
                                                                                            }

                                                                                            order_book
                                                                                                .pending_trades
                                                                                                .retain(|pending| pending.id != trade.id);

                                                                                            order_book.save_orders()?;
                                                                                            trade_book.save_trades()?;

                                                                                            let timestamp =
                                                                                                chrono::Utc::now()
                                                                                                    .timestamp() as u64;

                                                                                            let message = OpenDexMessage {
                                                                                                message_type:
                                                                                                    MessageType::TradeCreated,
                                                                                                sender:
                                                                                                    local_peer_id.to_string(),
                                                                                                payload:
                                                                                                    serde_json::to_string(&trade)?,
                                                                                                timestamp,
                                                                                            };

                                                                                            match publish_message(
                                                                                                &mut swarm,
                                                                                                &topic,
                                                                                                &message,
                                                                                            ) {
                                                                                                Ok(_) => {
                                                                                                    println!(
                                                                                                        "Published TradeCreated: {}",
                                                                                                        trade.id
                                                                                                    );
                                                                                                }

                                                                                                Err(e) => {
                                                                                                    eprintln!(
                                                                                                        "Failed to publish TradeCreated {}: {:?}",
                                                                                                        trade.id,
                                                                                                        e
                                                                                                    );
                                                                                                }
                                                                                            }

                                                                                            transition_and_publish_trade(
                                                                                                &mut trade_book,
                                                                                                trade.id,
                                                                                                TradeState::Accepted,
                                                                                                chrono::Utc::now()
                                                                                                    .timestamp() as u64,
                                                                                                &mut swarm,
                                                                                                &topic,
                                                                                                &local_peer_id,
                                                                                            )?;

                                                                                            order_book.save_orders()?;
                                                                                            trade_book.save_trades()?;
                                                                                            balance_book.save()?;
                                                                                        }
                                                                                    }
                                                                                }

                                                                                Command::ShowOrders => {
                                                                                    order_book.print_orders();
                                                                                }

                                                                                Command::ShowTrades => {
                                                                                    trade_book.print_trades();
                                                                                }

                                                                                Command::Cancel { id } => {
                                                                                    if let Some(order) = order_book
                                                                                        .orders
                                                                                        .iter()
                                                                                        .find(|order| order.id == id)
                                                                                        .cloned()
                                                                                    {
                                                                                        if order_book.remove_order(id) {
                                                                                            balance_book.unlock_for_order(&order)?;
                                                                                            balance_book.save()?;
                                                                                            order_book.save_orders()?;

                                                                                            println!(
                                                                                                "Removed local order: {}",
                                                                                                id
                                                                                            );

                                                                                            let cancelled = OrderCancelled {
                                                                                                order_id: id,
                                                                                            };

                                                                                            let message = OpenDexMessage {
                                                                                                message_type:
                                                                                                    MessageType::OrderCancelled,
                                                                                                sender:
                                                                                                    local_peer_id.to_string(),
                                                                                                payload:
                                                                                                    serde_json::to_string(
                                                                                                        &cancelled,
                                                                                                    )?,
                                                                                                timestamp:
                                                                                                    chrono::Utc::now()
                                                                                                        .timestamp() as u64,
                                                                                            };

                                                                                            publish_message(
                                                                                                &mut swarm,
                                                                                                &topic,
                                                                                                &message,
                                                                                            )?;
                                                                                        }
                                                                                    } else {
                                                                                        println!(
                                                                                            "Order not found locally: {}",
                                                                                            id
                                                                                        );
                                                                                    }
                                                                                }

                                                                                Command::Peers => {
                                                                                    println!("=== Connected Peers ===");

                                                                                    for peer in swarm.connected_peers() {
                                                                                        println!("{}", peer);
                                                                                    }
                                                                                }

                                                                                Command::Unknown => {
                                                                                    println!("Unknown command");
                                                                                    println!(
                                                                                        "Example: buy BTC XMR 12.5 0.1"
                                                                                    );
                                                                                }
                                                                            }
                                                                        }
                                                                    }

                                                                    event = swarm.next() => {
                                                                        if let Some(event) = event {
                                                                            match event {
                                                                                libp2p::swarm::SwarmEvent::ConnectionEstablished {
                                                                                    peer_id,
                                                                                    num_established,
                                                                                    ..
                                                                                } => {
                                                                                    println!(
                                                                                        "Connected to peer: {:?}, total connections: {}",
                                                                                        peer_id,
                                                                                        num_established
                                                                                    );
                                                                                }

                                                                                libp2p::swarm::SwarmEvent::ConnectionClosed {
                                                                                    peer_id,
                                                                                    cause,
                                                                                    ..
                                                                                } => {
                                                                                    println!(
                                                                                        "Connection closed with {:?}, cause: {:?}",
                                                                                        peer_id,
                                                                                        cause
                                                                                    );
                                                                                }

                                                                                libp2p::swarm::SwarmEvent::OutgoingConnectionError {
                                                                                    peer_id,
                                                                                    error,
                                                                                    ..
                                                                                } => {
                                                                                    println!(
                                                                                        "Outgoing connection error to {:?}: {:?}",
                                                                                        peer_id,
                                                                                        error
                                                                                    );
                                                                                }

                                                                                libp2p::swarm::SwarmEvent::Behaviour(event) => {
                                                                                    match event {
                                                                                        OpenDexBehaviourEvent::Identify(
                                                                                            identify::Event::Received {
                                                                                                peer_id,
                                                                                                info,
                                                                                                ..
                                                                                            },
                                                                                        ) => {
                                                                                            println!(
                                                                                                "Identify received from peer: {:?}",
                                                                                                peer_id
                                                                                            );

                                                                                            for addr in info.listen_addrs {
                                                                                                println!(
                                                                                                    "Adding address to Kademlia: {}",
                                                                                                    addr
                                                                                                );

                                                                                                swarm
                                                                                                    .behaviour_mut()
                                                                                                    .kademlia
                                                                                                    .add_address(
                                                                                                        &peer_id,
                                                                                                        addr,
                                                                                                    );
                                                                                            }

                                                                                            println!(
                                                                                                "Kademlia peer added: {:?}",
                                                                                                peer_id
                                                                                            );
                                                                                        }

                                                                                        OpenDexBehaviourEvent::Gossipsub(
                                                                                            gossipsub::Event::Subscribed {
                                                                                                peer_id,
                                                                                                topic: subscribed_topic,
                                                                                            },
                                                                                        ) => {
                                                                                            println!(
                                                                                                "Peer {:?} subscribed to {:?}",
                                                                                                peer_id,
                                                                                                subscribed_topic
                                                                                            );

                                                                                            let request = OpenDexMessage {
                                                                                                message_type:
                                                                                                    MessageType::StateRequest,
                                                                                                sender:
                                                                                                    local_peer_id.to_string(),
                                                                                                payload: "{}".to_string(),
                                                                                                timestamp:
                                                                                                    chrono::Utc::now()
                                                                                                        .timestamp() as u64,
                                                                                            };

                                                                                            publish_message(
                                                                                                &mut swarm,
                                                                                                &topic,
                                                                                                &request,
                                                                                            )?;
                                                                                        }

                                                                                        OpenDexBehaviourEvent::Gossipsub(
                                                                                            gossipsub::Event::Message {
                                                                                                message,
                                                                                                ..
                                                                                            },
                                                                                        ) => {
                                                                                            match serde_json::from_slice::<OpenDexMessage>(
                                                                                                &message.data,
                                                                                            ) {
                                                                                                Ok(msg) => {
                                                                                                    println!(
                                                                                                        "Received OpenDEX message:"
                                                                                                    );
                                                                                                    println!("{:?}", msg);

                                                                                                    match msg.message_type {

                                        MessageType::OrderCreated => {
                                            match serde_json::from_str::<Order>(&msg.payload) {
                                                Ok(order) => {
                                                    handle_order_created(
                                                        order,
                                                        &mut order_book,
                                                        &mut trade_book,
                                                        &mut swarm,
                                                        &topic,
                                                        &local_peer_id,
                                                    )?;
                                                }

                                                Err(e) => {
                                                    println!("Failed to decode order: {:?}", e);
                                                }
                                            }
                                        }



                MessageType::OrderCancelled => {
                    match serde_json::from_str::<OrderCancelled>(&msg.payload) {
                        Ok(cancelled) => {
                            handle_order_cancelled(
                                cancelled,
                                &mut order_book,
                            )?;
                        }

                        Err(e) => {
                            println!("Failed to decode cancellation: {:?}", e);
                        }
                    }
                }



                MessageType::StateRequest => {
                    handle_state_request(
                        &order_book,
                        &trade_book,
                        &mut swarm,
                        &topic,
                        &local_peer_id,
                    )?;
                }

        MessageType::StateResponse => {
            match serde_json::from_str::<StateSnapshot>(&msg.payload) {
                Ok(state) => {
                    handle_state_response(
                        state,
                        &mut order_book,
                        &mut trade_book,
                    )?;
                }

                Err(e) => {
                    eprintln!(
                        "Failed to decode StateResponse: {:?}",
                        e
                    );
                }
            }
        }


                                                        MessageType::TradeCreated => {
                                                            match serde_json::from_str::<Trade>(&msg.payload) {
                                                                Ok(trade) => {
                                                                    handle_trade_created(
                                                                        trade,
                                                                        &mut trade_book,
                                                                        &mut order_book,
                                                                        &mut pending_trade_states,
                                                                        &mut balance_book,
                                                                        &local_peer_id,
                                                                    )?;
                                                                }

                                                                Err(e) => {
                                                                    eprintln!(
                                                                        "Failed to decode TradeCreated: {:?}",
                                                                        e
                                                                    );
                                                                }
                                                            }
                                                        }



                                                MessageType::TradeStateChanged => {
                                                    match serde_json::from_str::<TradeStateChanged>(&msg.payload) {
                                                        Ok(change) => {
                                                            handle_trade_state_changed(
                                                                change,
                                                                &mut trade_book,
                                                                &mut pending_trade_states,
                                                                &mut balance_book,
                                                                &local_peer_id,
                                                            )?;
                                                        }

                                                        Err(e) => {
                                                            eprintln!(
                                                                "Failed to decode TradeStateChanged: {:?}",
                                                                e
                                                            );
                                                        }
                                                    }
                                                }


                                                                                                        MessageType::PeerAnnouncement => {
                                                                                                            println!(
                                                                                                                "Received PeerAnnouncement from {}",
                                                                                                                msg.sender
                                                                                                            );
                                                                                                        }
                                                                                                    }
                                                                                                }

                                                                                                Err(e) => {
                                                                                                    println!(
                                                                                                        "Invalid OpenDEX message: {:?}",
                                                                                                        e
                                                                                                    );
                                                                                                }
                                                                                            }
                                                                                        }

                                                                                        OpenDexBehaviourEvent::Kademlia(event) => {
                                                                                            println!(
                                                                                                "Kademlia event: {:?}",
                                                                                                event
                                                                                            );
                                                                                        }

                                                                                        OpenDexBehaviourEvent::Ping(_) => {}

                                                                                        _ => {}
                                                                                    }
                                                                                }

                                                                                _ => {}
                                                                            }
                                                                        }
                                                                    }
                                                                }
    }
}
