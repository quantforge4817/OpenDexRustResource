use network::publish_message;

use crate::cli::{Command, parse_command};

use trades::TradeBook;

use protocol::state::StateSnapshot;

use protocol::message::Trade;

use anyhow::Result;

use futures::StreamExt;

use protocol::message::{MessageType, OpenDexMessage, Order, OrderCancelled};

mod cli;

use order_factory::create_order;

mod trades;

mod orderbook;

mod order_factory;

mod network;

use orderbook::OrderBook;

use tokio::io::{self, AsyncBufReadExt};

use libp2p::{
    Multiaddr, PeerId, Swarm, SwarmBuilder, gossipsub, identify, identity, kad, noise, ping,
    swarm::NetworkBehaviour, tcp, yamux,
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

    println!("================================");
    println!(" OpenDEX Node Started");
    println!("================================");

    println!("Peer ID:");
    println!("{}", local_peer_id);

    let mut swarm: Swarm<OpenDexBehaviour> = SwarmBuilder::with_existing_identity(id_keys)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let peer_id = key.public().to_peer_id();

            let store = kad::store::MemoryStore::new(peer_id);

            let kademlia = kad::Behaviour::new(peer_id, store);
            let gossipsub = {
                let config = gossipsub::ConfigBuilder::default()
                    .build()
                    .expect("valid gossipsub config");

                gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    config,
                )
                .expect("create gossipsub")
            };

            Ok(OpenDexBehaviour {
                identify: identify::Behaviour::new(identify::Config::new(
                    "/opendex/1.0.0".into(),
                    key.public(),
                )),

                kademlia,

                ping: ping::Behaviour::new(
                    ping::Config::new().with_interval(std::time::Duration::from_secs(15)),
                ),
                gossipsub,
            })
        })?
        .with_swarm_config(|cfg| {
            cfg.with_idle_connection_timeout(std::time::Duration::from_secs(300))
        })
        .build();
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

    let mut order_book = OrderBook::load_orders(orders_path)?;
    let mut trade_book = TradeBook::load_trades(trades_path)?;

    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();
    loop {
        tokio::select! {
                            line = lines.next_line() => {
                                if let Some(input) = line? {
                                    match parse_command(&input) {
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

                    // Publish OrderCreated BEFORE mutating the local order book.
                    let order_message = OpenDexMessage {
                        message_type: MessageType::OrderCreated,
                        sender: local_peer_id.to_string(),
                        payload: serde_json::to_string(&order)?,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    };

                    match publish_message(&mut swarm, &topic, &order_message) {
                        Ok(_) => {
                            println!("Published order");
                        }
                        Err(e) => {
                            println!("Publish failed: {:?}", e);
                            println!("Order {} was not created", order.id);
                            continue;
                        }
                    }

                    // Publication succeeded, so the order is now allowed
                    // to affect the local order book.

                         let trades = order_book.match_order(&order);

                    if trades.is_empty() {
                        // No match: keep the new order in the local order book.
                        order_book.add_order(order);
                        order_book.save_orders()?;
                    } else {
                        // Match found: persist and broadcast each trade.
                        for trade in trades {
                            println!("=== Trade Created ===");
                            println!("{:?}", trade);

                            if let Err(e) = trade_book.add_trade(trade.clone()) {
                                eprintln!("Failed to save trade: {}", e);
                                continue;
                            }

                            // Mark this trade as applied locally.
                            order_book.mark_trade_applied(&trade.id);

                            order_book.save_orders()?;
                            trade_book.save_trades()?;

                            let message = OpenDexMessage {
                                message_type: MessageType::TradeCreated,
                                sender: local_peer_id.to_string(),
                                payload: serde_json::to_string(&trade)?,
                                timestamp: chrono::Utc::now().timestamp() as u64,
                            };

                            match publish_message(&mut swarm, &topic, &message) {
                                Ok(_) => {
                                    println!("Published trade");
                                }
                                Err(e) => {
                                    eprintln!("Publish failed: {:?}", e);
                                }
                            }
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
                                            if order_book.remove_order(id) {

                                                order_book.save_orders()?;

                                                println!("Removed local order: {}", id);

                                                let cancelled = OrderCancelled {
                                                    order_id: id,
                                                };

                                                let message = OpenDexMessage {
                                                    message_type: MessageType::OrderCancelled,
                                                    sender: local_peer_id.to_string(),
                                                    payload: serde_json::to_string(&cancelled)?,
                                                    timestamp: chrono::Utc::now().timestamp() as u64,
                                                };

                                                publish_message(
                                                    &mut swarm,
                                                    &topic,
                                                    &message,
                                                )?;

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
                                            println!("Example: buy BTC XMR 12.5 0.1");
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
                                                            .add_address(&peer_id, addr);
                                                    }

                                                    if let Err(e) =
                                                        swarm.behaviour_mut().kademlia.bootstrap()
                                                    {
                                                        println!(
                                                            "Bootstrap error: {:?}",
                                                            e
                                                        );
                                                    }
                                                }





                                                OpenDexBehaviourEvent::Gossipsub(
                                                    gossipsub::Event::Subscribed {
                                                        peer_id,
                                                        topic,
                                                    },
                                                ) => {
                                                    println!(
                                                        "Peer {:?} subscribed to {:?}",
                                                        peer_id,
                                                        topic
                                                    );

                                                    let request = OpenDexMessage {
                                                        message_type: MessageType::StateRequest,
                                                        sender: local_peer_id.to_string(),
                                                        payload: "{}".to_string(),
                                                        timestamp: chrono::Utc::now().timestamp() as u64,
                                                };

                                                 let order_topic = gossipsub::IdentTopic::new("opendex-orders");

                                                 publish_message(&mut swarm, &order_topic, &request)?;
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
                            if order_book
                                .orders
                                .iter()
                                .any(|o| o.id == order.id)
                            {
                                println!("Order already known: {}", order.id);
                            } else {
                                println!("=== Remote Order Received ===");
                                println!("{:?}", order);

                                // Do NOT match remote orders here.
                                // The node that created the order is responsible
                                // for matching it and broadcasting TradeCreated.
                                order_book.add_order(order);

                                order_book.save_orders()?;
                                order_book.print_orders();

                                println!("Remote order added without matching");
                            }
                        }

                        Err(e) => {
                            println!("Failed to decode order: {:?}", e);
                        }
                    }
                }

                                                                  MessageType::OrderCancelled => {
                                                                      match serde_json::from_str::<OrderCancelled>(
                                                                          &msg.payload,
                                                                      ) {
                                                                          Ok(cancelled) => {
                                                                              if order_book.remove_order(cancelled.order_id) {
                                                                                  println!(
                                                                                      "Removed cancelled order: {}",
                                                                                      cancelled.order_id
                                                                                  );
                                                                              } else {
                                                                                  println!(
                                                                                      "Cancelled order not found: {}",
                                                                                      cancelled.order_id
                                                                                  );
                                                                              }

                                                                              order_book.save_orders()?;
                                                                              order_book.print_orders();
                                                                          }

                                                                          Err(e) => {
                                                                              println!(
                                                                                  "Failed to decode cancellation: {:?}",
                                                                                  e
                                                                              );
                                                                          }
                                                                      }
                                                                  }

                                                                 MessageType::StateRequest => {
                                                                    let mut orders = order_book.orders.clone();

                                                                    orders.sort_by_key(|o| o.id);
                                                                    orders.dedup_by_key(|o| o.id);

                                                                    let state = StateSnapshot {
                                                                         orders,
                                                                         trades: trade_book.trades.clone(),
                                                                     };

                                                                     let message = OpenDexMessage {
                                                                         message_type: MessageType::StateResponse,
                                                                         sender: local_peer_id.to_string(),
                                                                         payload: serde_json::to_string(&state)?,
                                                                         timestamp: chrono::Utc::now().timestamp() as u64,
                                                                     };

                                                                     publish_message(&mut swarm, &topic, &message)?;
                                                                  }

                                                                MessageType::StateResponse => {
                                                                    match serde_json::from_str::<StateSnapshot>(&msg.payload) {
                                                                        Ok(state) => {
                                                                            println!("=== State Sync Received ===");

                                                                            let mut changed = false;

                                                                            // Merge trades from the remote snapshot.
                                                                            for trade in state.trades {
                                                                                if !trade_book
                                                                                    .trades
                                                                                    .iter()
                                                                                    .any(|t| t.id == trade.id)
                                                                                {
                                                                                    println!("Syncing trade {}", trade.id);

                                                                                    if let Err(e) =
                                                                                        trade_book.add_trade(trade)
                                                                                    {
                                                                                        eprintln!(
                                                                                            "Failed to save trade: {}",
                                                                                            e
                                                                                        );
                                                                                    }

                                                                                    changed = true;
                                                                                }
                                                                            }

                                                                            // Merge orders from the remote snapshot.
                                                                            //
                                                                            // Do not match these orders again and do not replay
                                                                            // historical trades against the current order book.
                                                                            for order in state.orders {
                                                                                if order_book
                                                                                    .orders
                                                                                    .iter()
                                                                                    .any(|o| o.id == order.id)
                                                                                {
                                                                                    continue;
                                                                                }

                                                                                if trade_book
                                                                                    .trades
                                                                                    .iter()
                                                                                    .any(|t| {
                                                                                        t.buy_order == order.id
                                                                                            || t.sell_order == order.id
                                                                                    })
                                                                                {
                                                                                    println!(
                                                                                        "Skipping traded order during state sync: {}",
                                                                                        order.id
                                                                                    );
                                                                                    continue;
                                                                                }

                                                                                println!("Syncing order {}", order.id);

                                                                                order_book.add_order(order);
                                                                                changed = true;
                                                                            }

                                                                            if changed {
                                                                                order_book.save_orders()?;
                                                                                trade_book.save_trades()?;
                                                                            }

                                                                            order_book.print_orders();
                                                                            trade_book.print_trades();
                                                                        }

                                                                        Err(e) => {
                                                                            println!(
                                                                                "Failed to decode state: {:?}",
                                                                                e
                                                                            );
                                                                        }
                                                                    }
                                                                }

                                                                 MessageType::TradeCreated => {
            match serde_json::from_str::<Trade>(&msg.payload) {
                Ok(trade) => {
                    println!("=== Trade Received ===");
                    println!("{:?}", trade);

                    // A trade is applied exactly once.
                    if order_book.applied_trades.contains(&trade.id) {
                        println!(
                            "Trade already applied, ignoring: {}",
                            trade.id
                        );
                        continue;
                    }

                    // If we already have the trade but it hasn't somehow
                    // been marked as applied, repair the state by applying it.
                    let trade_known = trade_book
                        .trades
                        .iter()
                        .any(|t| t.id == trade.id);

                    if !trade_known {
                        if let Err(e) = trade_book.add_trade(trade.clone()) {
                            eprintln!(
                                "Failed to save trade {}: {}",
                                trade.id, e
                            );
                            continue;
                        }

                        trade_book.save_trades()?;
                    }

                    // Apply the trade to the order book exactly once.

                    if order_book.apply_trade(&trade) {
                        order_book.save_orders()?;
                        println!("Trade applied successfully: {}", trade.id);
                   } else {
                       eprintln!("Trade was rejected: {}", trade.id);
                   }

                    println!(
                        "Trade applied successfully: {}",
                        trade.id
                    );

                    order_book.print_orders();
                    trade_book.print_trades();
                }

                Err(e) => {
                    println!(
                        "Failed to decode trade: {:?}",
                        e
                    );
                }
            }
        }


                                                                _ => {}
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
