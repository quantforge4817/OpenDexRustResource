use std::collections::HashMap;

use network::publish_message;

use crate::cli::{Command, parse_command};

use trades::TradeBook;

use protocol::state::StateSnapshot;

use anyhow::Result;

use futures::StreamExt;

use protocol::message::{
    Asset, MessageType, OpenDexMessage, Order, OrderCancelled, TradeState, TradeStateChanged,
};

use balances::BalanceBook;

mod balances;

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

            let mut kademlia = kad::Behaviour::new(peer_id, store);
            kademlia.set_mode(Some(kad::Mode::Server));
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

                            balance_book.credit(&trader, asset.clone(), amount)?;
                            balance_book.save()?;

                            let balance = balance_book.balance(&trader, &asset);

                            println!(
                                "Funded {:?}: available={}, locked={}",
                                asset,
                                balance.available,
                                balance.locked
                            );
                        }

                        Command::ShowBalances => {
                            let trader = local_peer_id.to_string();

                            println!("=== Balances ===");

                            for asset in [Asset::BTC, Asset::XMR] {
                                let balance = balance_book.balance(&trader, &asset);

                                println!(
                                    "{:?}: available={}, locked={}",
                                    asset,
                                    balance.available,
                                    balance.locked
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
                                                                                           // Reserve the funds before publishing the order.
                                                                                           // An order must never enter the distributed order book
                                                                                           // unless its required funds are locally locked.
                                                                                           if let Err(e) = balance_book.lock_for_order(&order) {
                                                                                              eprintln!("Cannot create order {}: {}", order.id, e);
                                                                                               continue;
                                                                                           }

                                                                                           balance_book.save()?;

                                                                                           println!("Locked funds for order {}", order.id);

                                                        // Publish OrderCreated only after funds are reserved.
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

                                                                if let Err(unlock_error) = balance_book.unlock_for_order(&order) {
                                                                    eprintln!(
                                                                        "CRITICAL: failed to unlock funds for order {}: {}",
                                                                        order.id, unlock_error
                                                                    );
                                                                }

                                                                balance_book.save()?;

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

                                                                                                   println!("=== Trade Created ===");
                                        for trade in trades {
                                            println!("=== Trade Created ===");
                                            println!("{:?}", trade);

                                            if !trade_book.add_trade(trade.clone()) {
                                                eprintln!("Trade was already present: {}", trade.id);
                                                continue;
                                            }

                                            order_book.mark_trade_applied(&trade.id.to_string());

                                            order_book.save_orders()?;
                                            trade_book.save_trades()?;

                                            // First announce the creation of the trade.
                                            let timestamp = chrono::Utc::now().timestamp() as u64;

                                            let message = OpenDexMessage {
                                                message_type: MessageType::TradeCreated,
                                                sender: local_peer_id.to_string(),
                                                payload: serde_json::to_string(&trade)?,
                                                timestamp,
                                            };

                                            match publish_message(&mut swarm, &topic, &message) {
                                                Ok(_) => {
                                                    println!("Published TradeCreated: {}", trade.id);
                                                }
                                                Err(e) => {
                                                    eprintln!("Failed to publish TradeCreated {}: {:?}", trade.id, e);
                                                }
                                            }

                                            // The trade has now been accepted locally.
                                            transition_and_publish_trade(
                                                &mut trade_book,
                                                trade.id,
                                                TradeState::Accepted,
                                                chrono::Utc::now().timestamp() as u64,
                                                &mut swarm,
                                                &topic,
                                                &local_peer_id,
                                            )?;
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

                                                                           publish_message(&mut swarm, &topic, &message)?;
                                                                       }
                                                                   } else {
                                                                       println!("Order not found locally: {}", id);
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
                                                                                   println!("Adding address to Kademlia: {}", addr);

                                                                                   swarm
                                                                                       .behaviour_mut()
                                                                                       .kademlia
                                                                                       .add_address(&peer_id, addr);
                                                                               }

                                                                               // We already have a known peer/address.
                                                                               // Bootstrap only when the routing table has enough information.
                                                                               println!(
                                                                                   "Kademlia peer added: {:?}",
                                                                                   peer_id
                                                                               );

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

                                                                                       // Match the remote order against our local order book.
                                                                                       let trades = order_book.match_order(&order);

                                                                                       if trades.is_empty() {
                                                                                           // match_order() adds the remaining order automatically.
                                                                                           order_book.save_orders()?;
                                                                                           order_book.print_orders();

                                                                                           println!("Remote order added without matching");
                                                                                       } else {
                                                                                           for trade in trades {
                                                                                               println!("=== Trade Created From Remote Order ===");
                                                                                               println!("{:?}", trade);

                                                                                               if !trade_book.add_trade(trade.clone()) {
                                                                                                   println!("Trade already known: {}", trade.id);
                                                                                                   continue;
                                                                                               }

                                                                                               order_book.mark_trade_applied(&trade.id.to_string());

                                                                                               order_book.save_orders()?;
                                                                                               trade_book.save_trades()?;

                                                                                               let trade_message = OpenDexMessage {
                                                                                                   message_type: MessageType::TradeCreated,
                                                                                                   sender: local_peer_id.to_string(),
                                                                                                   payload: serde_json::to_string(&trade)?,
                                                                                                   timestamp: chrono::Utc::now().timestamp() as u64,
                                                                                               };

                                                                                               match publish_message(
                                                                                                   &mut swarm,
                                                                                                   &topic,
                                                                                                   &trade_message,
                                                                                               ) {
                                                                                                   Ok(_) => {
                                                                                                       println!(
                                                                                                           "Published TradeCreated: {}",
                                                                                                           trade.id
                                                                                                       );
                                                                                                   }
                                                                                                   Err(e) => {
                                                                                                       println!(
                                                                                                           "Failed to publish TradeCreated {}: {:?}",
                                                                                                           trade.id,
                                                                                                           e
                                                                                                       );
                                                                                                   }
                                                                                               }
                                                                                           }

                                                                                           order_book.print_orders();
                                                                                       }
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
                                                                                                                                                removed_orders: order_book.removed_orders.clone(),
                                                                                                                                                applied_trades: order_book.applied_trades.clone(),
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

                                            // 1. Merge tombstones first.
                                            // Removed orders must never be resurrected by an old snapshot.
                                            for order_id in state.removed_orders {
                                                if !order_book.removed_orders.contains(&order_id) {
                                                    order_book.removed_orders.push(order_id);
                                                }

                                                order_book.orders.retain(|order| order.id != order_id);
                                            }

                                            // 2. Snapshot orders contain CURRENT remaining amounts.
                                            // Do not replay trades against them.
                                            for order in state.orders {
                                                println!("Syncing order {}", order.id);
                                                order_book.update_order_from_snapshot(order);
                                            }

                                            // 3. Merge historical trades.
                                            // These trades are already reflected in the snapshot order amounts.
                                            for trade in state.trades {
                                                if trade_book.add_trade(trade.clone()) {
                                                    println!("Synced trade {}", trade.id);
                                                }
                                            }

                                            // 4. Merge applied-trade IDs.
                                            // These trades have already been accounted for in the snapshot.
                                            for trade_id in state.applied_trades {
                                                order_book.mark_trade_applied(&trade_id);

                                                // An applied trade cannot remain pending.
                                                order_book
                                                    .pending_trades
                                                    .retain(|trade| trade.id.to_string() != trade_id);
                                            }

                                            order_book.save_orders()?;
                                            trade_book.save_trades()?;

                                            order_book.print_orders();
                                            trade_book.print_trades();

                                            println!("=== State Sync Complete ===");
                                        }

                                        Err(e) => {
                                            eprintln!("Failed to decode StateResponse: {:?}", e);
                                        }
                                    }
                                }
                MessageType::TradeCreated => {
                    match serde_json::from_str::<protocol::message::Trade>(&msg.payload) {
                        Ok(trade) => {
                            println!("=== Trade Created Received ===");
                            println!("{:?}", trade);

                            if trade_book.add_trade(trade.clone()) {
                                println!("Added remote trade: {}", trade.id);

                                trade_book.save_trades()?;

                                println!(
                                    "Remote trade {} stored with state {:?}",
                                    trade.id,
                                    trade.state
                                );
        // TradeStateChanged may arrive before TradeCreated.
        if let Some(pending_change) = pending_trade_states.remove(&trade.id) {
            println!(
                "Applying pending state {:?} to trade {}",
                pending_change.state,
                trade.id
            );

            match trade_book.transition_trade(
                trade.id,
                pending_change.state.clone(),
                pending_change.timestamp,
            ) {
                Ok(true) => {
                    println!(
                        "Applied pending state {:?} to trade {}",
                        pending_change.state,
                        trade.id
                    );
                }

                Ok(false) => {
                    eprintln!(
                        "Trade {} disappeared before pending state could be applied",
                        trade.id
                    );
                }

                Err(e) => {
                    eprintln!(
                        "Failed to apply pending state to trade {}: {:?}",
                        trade.id,
                        e
                    );
                }
            }
        }


                                trade_book.save_trades()?;
                            } else {
                                println!("Remote trade already known: {}", trade.id);
                            }
                        }

                        Err(e) => {
                            eprintln!("Failed to decode TradeCreated: {:?}", e);
                        }
                    }
                }

                MessageType::TradeStateChanged => {
                    match serde_json::from_str::<TradeStateChanged>(&msg.payload) {
                        Ok(change) => {
                            println!("=== Trade State Change Received ===");
                            println!(
                                "Trade {} -> {:?}",
                                change.trade_id,
                                change.state
                            );

                            match trade_book.transition_trade(
                                change.trade_id,
                                change.state.clone(),
                                change.timestamp,
                            ) {
                                Ok(true) => {
                                    println!(
                                        "Trade {} transitioned to {:?}",
                                        change.trade_id,
                                        change.state
                                    );
                                }

                                Ok(false) => {
                                    println!(
                                        "Trade {} not found locally; storing pending state {:?}",
                                        change.trade_id,
                                        change.state
                                    );

                                      pending_trade_states.insert(change.trade_id, change);
                                    }

                                Err(e) => {
                                    eprintln!(
                                        "Trade {} state transition rejected: {:?}",
                                        change.trade_id,
                                        e
                                    );
                                }
                            }
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
                        println!("Received PeerAnnouncement from {}", msg.sender);
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
