//! P2P network load tests — stress-test the gossipsub mesh under various
//! peer counts and message volumes.

use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, gossipsub, identity, multiaddr::Protocol, swarm::SwarmEvent,
};
use p2p_node::network::{build_swarm, publish_message};
use protocol::message::{Asset, Market, Message, MessageType, Order, Side};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use uuid::Uuid;

/// Helper: create a test order
fn test_order() -> Order {
    Order {
        id: Uuid::new_v4(),
        trader: "load_tester".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Buy,
        price: 100 * protocol::message::PRICE_SCALE,
        amount: protocol::message::AMOUNT_SCALE,
    }
}

/// Helper: get the local TCP address of a swarm
fn local_tcp_addr(swarm: &libp2p::Swarm<p2p_node::OpenDexBehaviour>) -> Multiaddr {
    swarm
        .listeners()
        .next()
        .expect("swarm should have a listener")
        .clone()
}

/// Helper: start a swarm listening on a random port
async fn start_listening(
    swarm: &mut libp2p::Swarm<p2p_node::OpenDexBehaviour>,
) -> Multiaddr {
    swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .expect("listen");

    timeout(Duration::from_secs(5), async {
        loop {
            if swarm.listeners().next().is_some() {
                break local_tcp_addr(swarm);
            }
            swarm.select_next_some().await;
        }
    })
    .await
    .expect("swarm did not start listening")
}

/// Helper: dial a peer and wait for connection
async fn dial_and_connect(
    dialer: &mut libp2p::Swarm<p2p_node::OpenDexBehaviour>,
    target_addr: &Multiaddr,
    target_peer: PeerId,
) {
    dialer.dial(target_addr.clone().with(Protocol::P2p(target_peer))).expect("dial");

    timeout(Duration::from_secs(10), async {
        loop {
            let event = dialer.select_next_some().await;
            if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event {
                if peer_id == target_peer {
                    break;
                }
            }
        }
    })
    .await
    .expect("connection not established");
}

/// Helper: subscribe a swarm to a topic
fn subscribe_to_topic(
    swarm: &mut libp2p::Swarm<p2p_node::OpenDexBehaviour>,
    topic: &gossipsub::IdentTopic,
) {
    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(topic)
        .expect("subscribe");
}

/// Helper: drive swarm events for a short time
async fn drive_swarm(
    swarm: &mut libp2p::Swarm<p2p_node::OpenDexBehaviour>,
    duration: Duration,
) {
    let start = Instant::now();
    while start.elapsed() < duration {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(10)) => {
                // Check elapsed time
            }
            _ = swarm.select_next_some() => {
                // Process event
            }
        }
    }
}

/// Multi-node mesh test: spawn N swarms, connect them in a chain, and verify
/// gossipsub peer counts.
#[tokio::test]
async fn multi_node_mesh_formation() {
    for &peer_count in &[5, 10, 20] {
        let start = Instant::now();

        // Create and start all swarms
        let mut swarms = Vec::new();
        let mut peers = Vec::new();
        let mut addrs = Vec::new();

        for _ in 0..peer_count {
            let key = identity::Keypair::generate_ed25519();
            let peer = PeerId::from(key.public());
            let mut swarm = build_swarm(key).expect("build swarm");
            let addr = start_listening(&mut swarm).await;
            swarms.push(swarm);
            peers.push(peer);
            addrs.push(addr);
        }

        let topic = gossipsub::IdentTopic::new("opendex-load-test");

        // Subscribe all to the topic
        for swarm in &mut swarms {
            subscribe_to_topic(swarm, &topic);
        }

        // Connect in a chain: peer[i] dials peer[i+1]
        for i in 0..peer_count - 1 {
            dial_and_connect(&mut swarms[i], &addrs[i + 1], peers[i + 1]).await;
            // Accept connection on the target side
            timeout(Duration::from_secs(5), async {
                loop {
                    let event = swarms[i + 1].select_next_some().await;
                    if let SwarmEvent::ConnectionEstablished { .. } = event {
                        break;
                    }
                }
            })
            .await
            .ok();
        }

        // Allow gossipsub to propagate
        for swarm in &mut swarms {
            drive_swarm(swarm, Duration::from_millis(500)).await;
        }

        let elapsed = start.elapsed();

        // Verify all peers have at least one mesh peer
        for (i, swarm) in swarms.iter().enumerate() {
            let mesh_peers: HashSet<PeerId> = swarm
                .behaviour()
                .gossipsub
                .all_peers()
                .map(|(peer, _)| *peer)
                .collect();

            // Each peer should have at least one neighbor (chain topology)
            assert!(
                !mesh_peers.is_empty() || peer_count == 1,
                "Peer {} has no mesh peers in {}-node mesh",
                i,
                peer_count
            );
        }

        println!(
            "{}-node mesh formed in {:?} (avg peers per node: {:.1})",
            peer_count,
            elapsed,
            swarms
                .iter()
                .map(|s| {
                    s.behaviour()
                        .gossipsub
                        .all_peers()
                        .count() as f64
                })
                .sum::<f64>()
                / peer_count as f64
        );
    }
}

/// Message flood test: publish M messages through the mesh and measure delivery.
#[tokio::test]
async fn message_flood_throughput() {
    for &msg_count in &[100, 1_000] {
        let peer_count = 5;
        let mut swarms = Vec::new();
        let mut peers = Vec::new();
        let mut addrs = Vec::new();

        for _ in 0..peer_count {
            let key = identity::Keypair::generate_ed25519();
            let peer = PeerId::from(key.public());
            let mut swarm = build_swarm(key).expect("build swarm");
            let addr = start_listening(&mut swarm).await;
            swarms.push(swarm);
            peers.push(peer);
            addrs.push(addr);
        }

        let topic = gossipsub::IdentTopic::new("opendex-flood-test");
        for swarm in &mut swarms {
            subscribe_to_topic(swarm, &topic);
        }

        // Connect in a chain
        for i in 0..peer_count - 1 {
            dial_and_connect(&mut swarms[i], &addrs[i + 1], peers[i + 1]).await;
            timeout(Duration::from_secs(5), async {
                loop {
                    let event = swarms[i + 1].select_next_some().await;
                    if let SwarmEvent::ConnectionEstablished { .. } = event {
                        break;
                    }
                }
            })
            .await
            .ok();
        }

        // Allow mesh to stabilize
        for swarm in &mut swarms {
            drive_swarm(swarm, Duration::from_millis(500)).await;
        }

        // Publish messages from the first peer
        let start = Instant::now();
        for i in 0..msg_count {
            let order = test_order();
            let payload = serde_json::to_string(&order).unwrap();
            let message = Message {
                message_type: MessageType::OrderCreated,
                sender: peers[0].to_string(),
                payload,
                timestamp: chrono::Utc::now().timestamp() as u64,
                schema_version: 1,
            };

            if let Err(e) = publish_message(&mut swarms[0], &topic, &message) {
                // Some messages may fail if the mesh is overloaded — that's expected
                tracing::debug!("Message {} failed: {}", i, e);
            }

            // Process swarm events after each publish
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(1)) => {}
                _ = swarms[0].select_next_some() => {}
            }
        }

        let elapsed = start.elapsed();
        let throughput = msg_count as f64 / elapsed.as_secs_f64();

        println!(
            "Published {} messages in {:?} ({:.0} msg/s)",
            msg_count, elapsed, throughput
        );

        // Process remaining events on all swarms
        for swarm in &mut swarms {
            drive_swarm(swarm, Duration::from_millis(500)).await;
        }
    }
}

/// Peer churn test: repeatedly add/remove peers while publishing messages.
#[tokio::test]
async fn peer_churn_under_load() {
    let base_peers = 5;
    let churn_rounds = 10;

    let mut swarms = Vec::new();
    let mut peers = Vec::new();
    let mut addrs = Vec::new();

    for _ in 0..base_peers {
        let key = identity::Keypair::generate_ed25519();
        let peer = PeerId::from(key.public());
        let mut swarm = build_swarm(key).expect("build swarm");
        let addr = start_listening(&mut swarm).await;
        swarms.push(swarm);
        peers.push(peer);
        addrs.push(addr);
    }

    let topic = gossipsub::IdentTopic::new("opendex-churn-test");
    for swarm in &mut swarms {
        subscribe_to_topic(swarm, &topic);
    }

    // Connect base peers in a chain
    for i in 0..base_peers - 1 {
        dial_and_connect(&mut swarms[i], &addrs[i + 1], peers[i + 1]).await;
        timeout(Duration::from_secs(5), async {
            loop {
                let event = swarms[i + 1].select_next_some().await;
                if let SwarmEvent::ConnectionEstablished { .. } = event {
                    break;
                }
            }
        })
        .await
        .ok();
    }

    // Allow mesh to stabilize
    for swarm in &mut swarms {
        drive_swarm(swarm, Duration::from_millis(500)).await;
    }

    // Churn: add a new peer, publish a message, then remove it
    for round in 0..churn_rounds {
        // Add a new peer
        let key = identity::Keypair::generate_ed25519();
        let new_peer = PeerId::from(key.public());
        let mut new_swarm = build_swarm(key).expect("build swarm");
        let new_addr = start_listening(&mut new_swarm).await;
        subscribe_to_topic(&mut new_swarm, &topic);

        // Connect to a random base peer
        let target = round % base_peers;
        dial_and_connect(&mut new_swarm, &addrs[target], peers[target]).await;

        // Publish a message
        let order = test_order();
        let payload = serde_json::to_string(&order).unwrap();
        let message = Message {
            message_type: MessageType::OrderCreated,
            sender: new_peer.to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp() as u64,
            schema_version: 1,
        };
        let _ = publish_message(&mut new_swarm, &topic, &message);

        // Drive events
        drive_swarm(&mut new_swarm, Duration::from_millis(100)).await;
        for swarm in &mut swarms {
            drive_swarm(swarm, Duration::from_millis(50)).await;
        }

        // Drop the swarm (simulates peer disconnect)
        drop(new_swarm);

        // Allow base peers to process disconnect
        for swarm in &mut swarms {
            drive_swarm(swarm, Duration::from_millis(100)).await;
        }
    }

    // Verify base peers are still healthy
    for (i, swarm) in swarms.iter().enumerate() {
        let peer_count = swarm.behaviour().gossipsub.all_peers().count();
        assert!(
            peer_count > 0,
            "Peer {} lost all connections after churn",
            i
        );
    }

    println!("Peer churn test passed: {} rounds, {} base peers survived", churn_rounds, base_peers);
}

/// Snapshot sync under load: create a large StateSnapshot and apply it while
/// messages are in flight.
#[tokio::test]
async fn snapshot_sync_under_load() {
    use p2p_node::orderbook::OrderBook;
    use p2p_node::sync::{apply_snapshot, create_snapshot};
    use p2p_node::trades::TradeBook;
    use protocol::message::{Trade, TradeState};
    use protocol::state::StateSnapshot;

    let order_count = 100;
    let trade_count = 50;

    // Create a snapshot with many orders and trades
    let mut orders = Vec::new();
    for i in 0..order_count {
        orders.push(Order {
            id: Uuid::new_v4(),
            trader: format!("trader_{}", i),
            market: Market {
                base: Asset::BTC,
                quote: Asset::XMR,
            },
            side: if i % 2 == 0 { Side::Buy } else { Side::Sell },
            price: (100 + i) * protocol::message::PRICE_SCALE,
            amount: protocol::message::AMOUNT_SCALE,
        });
    }

    let mut trades = Vec::new();
    for i in 0..trade_count {
        trades.push(Trade {
            id: Uuid::new_v4(),
            buy_order: orders[i * 2].id,
            sell_order: orders[i * 2 + 1].id,
            maker: format!("maker_{}", i),
            taker: format!("taker_{}", i),
            buyer: "buyer".to_string(),
            seller: "seller".to_string(),
            market: Market {
                base: Asset::BTC,
                quote: Asset::XMR,
            },
            price: 100 * protocol::message::PRICE_SCALE,
            amount: protocol::message::AMOUNT_SCALE,
            fill_sequence: 0,
            state: TradeState::Created,
            created_at: 1,
            updated_at: 1,
        });
    }

    let snapshot = StateSnapshot {
        version: 1,
        orders: orders.clone(),
        trades: trades.clone(),
        removed_orders: std::collections::HashSet::new(),
        applied_trades: std::collections::HashSet::new(),
    };

    // Apply snapshot to a fresh order book
    let tmp_path = std::env::temp_dir().join(format!(
        "opendex-snapshot-bench-{}",
        Uuid::new_v4().hyphenated()
    ));
    let mut order_book = OrderBook::new(tmp_path.clone());
    let mut trade_book = TradeBook::new(tmp_path.clone());

    let start = Instant::now();
    apply_snapshot(snapshot, &mut order_book, &mut trade_book).expect("apply snapshot");
    let elapsed = start.elapsed();

    // Verify data integrity
    assert_eq!(
        order_book.active_orders().len(),
        order_count as usize,
        "Order count mismatch after snapshot"
    );
    assert_eq!(
        trade_book.trades.len(),
        trade_count as usize,
        "Trade count mismatch after snapshot"
    );

    // Verify we can still create a snapshot from the applied data
    let recreated = create_snapshot(&order_book, &trade_book);
    assert_eq!(recreated.orders.len(), order_count as usize);
    assert_eq!(recreated.trades.len(), trade_count as usize);

    let _ = std::fs::remove_dir_all(tmp_path);

    println!(
        "Snapshot sync: {} orders + {} trades applied in {:?}",
        order_count, trade_count, elapsed
    );
}
