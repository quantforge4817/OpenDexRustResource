use crate::cli::{Command, parse_command};
use protocol::message::Trade;

use anyhow::Result;

use futures::StreamExt;

use protocol::message::{MessageType, OpenDexMessage, Order, OrderCancelled};

mod cli;

use order_factory::create_order;

mod orderbook;

mod order_factory;

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

    let mut order_book = OrderBook::new();

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

                                    let trade = order_book.match_order(&order);

                                    if let Some(trade) = &trade {
                                        println!("=== Trade Created ===");
                                        println!("{:?}", trade);

                                        let trade_message = OpenDexMessage {
                                            message_type: MessageType::TradeCreated,
                                            sender: local_peer_id.to_string(),
                                            payload: serde_json::to_string(trade)?,
                                            timestamp: chrono::Utc::now().timestamp() as u64,
                                        };

                                        let encoded = serde_json::to_vec(&trade_message)?;

                                        match swarm
                                            .behaviour_mut()
                                            .gossipsub
                                            .publish(topic.clone(), encoded)
                                        {
                                            Ok(msg_id) => {
                                                println!("Published trade: {:?}", msg_id);
                                            }
                                            Err(e) => {
                                                println!("Publish failed: {:?}", e);
                                            }
                                        }
                                    } else {
                                        order_book.add_order(order.clone());

                                        let message = OpenDexMessage {
                                            message_type: MessageType::OrderCreated,
                                            sender: local_peer_id.to_string(),
                                            payload: serde_json::to_string(&order)?,
                                            timestamp: chrono::Utc::now().timestamp() as u64,
                                        };

                                        let encoded = serde_json::to_vec(&message)?;

                                        match swarm
                                            .behaviour_mut()
                                            .gossipsub
                                            .publish(topic.clone(), encoded)
                                        {
                                            Ok(msg_id) => {
                                                println!("Published order: {:?}", msg_id);
                                            }
                                            Err(e) => {
                                                println!("Publish failed: {:?}", e);
                                            }
                                        }
                                    }
                                }

                                Command::ShowOrders => {
                                    order_book.print_orders();
                                }

                                Command::Cancel { id } => {
                                    if order_book.remove_order(id) {
                                        println!("Removed local order: {}", id);
                                        order_book.print_orders();
                                    } else {
                                        println!("Order not found locally: {}", id);
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

                                        OpenDexBehaviourEvent::Identify(event) => {
                                            println!("Identify event: {:?}", event);
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
                                                            match serde_json::from_str::<Order>(
                                                                &msg.payload,
                                                            ) {
                                                                Ok(order) => {
                                                                    match order_book
                                                                        .match_order(&order)
                                                                    {
                                                                        Some(trade) => {
            println!("=== Trade Created ===");
            println!("{:?}", trade);

            let trade_message = OpenDexMessage {
                message_type: MessageType::TradeCreated,
                sender: local_peer_id.to_string(),
                payload: serde_json::to_string(&trade)?,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };

            let encoded = serde_json::to_vec(&trade_message)?;

            match swarm
                .behaviour_mut()
                .gossipsub
                .publish(topic.clone(), encoded)
            {
                Ok(msg_id) => {
                    println!("Published trade: {:?}", msg_id);
                }

                Err(e) => {
                    println!("Failed to publish trade: {:?}", e);
                }
            }
        }









                                                                        None => {
                                                                            order_book
                                                                                .add_order(order);

                                                                            order_book
                                                                                .print_orders();
                                                                        }
                                                                    }
                                                                }

                                                                Err(e) => {
                                                                    println!(
                                                                        "Failed to decode order: {:?}",
                                                                        e
                                                                    );
                                                                }
                                                            }
                                                        }

                                                        MessageType::OrderCancelled => {
                                                            match serde_json::from_str::<OrderCancelled>(
                                                                &msg.payload,
                                                            ) {
                                                                Ok(cancelled) => {
                                                                    if order_book.remove_order(
                                                                        cancelled.order_id,
                                                                    ) {
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

                                                        MessageType::TradeCreated => {
                                                            match serde_json::from_str::<Trade>(
                                                                &msg.payload,
                                                            ) {
                                                                Ok(trade) => {
                                                                    println!(
                                                                        "=== Trade Received ==="
                                                                    );
                                                                    println!("{:?}", trade);
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
