use futures::StreamExt;
use libp2p::{Multiaddr, PeerId, gossipsub, identity, multiaddr::Protocol, swarm::SwarmEvent};
use p2p_node::network::build_swarm;
use protocol::message::{Asset, Market, MessageType, Message, Order, Side};
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

fn test_order() -> Order {
    Order {
        id: Uuid::new_v4(),
        trader: "alice".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side: Side::Buy,
        price: 100 * protocol::message::PRICE_SCALE,
        amount: protocol::message::AMOUNT_SCALE,
    }
}

fn local_tcp_addr(swarm: &libp2p::Swarm<p2p_node::OpenDexBehaviour>) -> Multiaddr {
    swarm
        .listeners()
        .next()
        .expect("swarm should have a listener")
        .clone()
}

fn peer_id_from_addr(addr: &Multiaddr) -> PeerId {
    match addr
        .iter()
        .last()
        .expect("address should contain a protocol")
    {
        Protocol::P2p(peer_id) => peer_id,
        protocol => panic!("expected /p2p address, got {protocol:?}"),
    }
}

#[tokio::test]
async fn two_nodes_connect_and_exchange_gossipsub_messages() {
    let key_a = identity::Keypair::generate_ed25519();
    let key_b = identity::Keypair::generate_ed25519();

    let peer_a = PeerId::from(key_a.public());
    let peer_b = PeerId::from(key_b.public());

    let mut swarm_a = build_swarm(key_a).expect("build swarm A");
    let mut swarm_b = build_swarm(key_b).expect("build swarm B");

    let topic = gossipsub::IdentTopic::new("opendex-orders");

    swarm_a
        .behaviour_mut()
        .gossipsub
        .subscribe(&topic)
        .expect("subscribe A");

    swarm_b
        .behaviour_mut()
        .gossipsub
        .subscribe(&topic)
        .expect("subscribe B");

    swarm_a
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .expect("listen A");

    swarm_b
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .expect("listen B");

    let listen_a = timeout(Duration::from_secs(5), async {
        loop {
            if swarm_a.listeners().next().is_some() {
                break local_tcp_addr(&swarm_a);
            }

            swarm_a.select_next_some().await;
        }
    })
    .await
    .expect("A did not start listening");

    let listen_b = timeout(Duration::from_secs(5), async {
        loop {
            if swarm_b.listeners().next().is_some() {
                break local_tcp_addr(&swarm_b);
            }

            swarm_b.select_next_some().await;
        }
    })
    .await
    .expect("B did not start listening");

    let dial_addr = listen_a.clone().with(Protocol::P2p(peer_a));

    swarm_b.dial(dial_addr).expect("B should dial A");

    timeout(Duration::from_secs(10), async {
        let mut connected_a = false;
        let mut connected_b = false;

        loop {
            tokio::select! {
                event = swarm_a.select_next_some() => {
                    if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event
                        && peer_id == peer_b
                    {
                        connected_a = true;
                    }
                }

                event = swarm_b.select_next_some() => {
                    if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event
                        && peer_id == peer_a
                    {
                        connected_b = true;
                    }
                }
            }

            if connected_a && connected_b {
                break;
            }
        }
    })
    .await
    .expect("nodes did not connect");

    // Wait until A observes B's gossipsub subscription.
    timeout(Duration::from_secs(10), async {
        loop {
            if let SwarmEvent::Behaviour(p2p_node::OpenDexBehaviourEvent::Gossipsub(
                gossipsub::Event::Subscribed {
                    peer_id,
                    topic: subscribed_topic,
                },
            )) = swarm_a.select_next_some().await
                && peer_id == peer_b
                && subscribed_topic == topic.hash()
            {
                break;
            }
        }
    })
    .await
    .expect("B did not subscribe to the gossipsub topic");

    let order = test_order();

    let message = Message {
        message_type: MessageType::OrderCreated,
        sender: peer_a.to_string(),
        payload: serde_json::to_string(&order).expect("serialize order"),
        timestamp: chrono::Utc::now().timestamp() as u64,
        schema_version: protocol::message::SCHEMA_VERSION,
    };

    let encoded = serde_json::to_vec(&message).expect("serialize message");

    swarm_a
        .behaviour_mut()
        .gossipsub
        .publish(topic.clone(), encoded)
        .expect("publish should succeed");

    let received_order = timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                event = swarm_a.select_next_some() => {
                    if let SwarmEvent::Behaviour(
                        p2p_node::OpenDexBehaviourEvent::Gossipsub(
                            gossipsub::Event::Message { message, .. },
                        ),
                    ) = event
                    {
                        let decoded: Message =
                            serde_json::from_slice(&message.data)
                                .expect("received invalid Message");

                        if decoded.message_type == MessageType::OrderCreated {
                            let received_order: Order =
                                serde_json::from_str(&decoded.payload)
                                    .expect("received invalid Order");

                            break received_order;
                        }
                    }
                }

                event = swarm_b.select_next_some() => {
                    if let SwarmEvent::Behaviour(
                        p2p_node::OpenDexBehaviourEvent::Gossipsub(
                            gossipsub::Event::Message { message, .. },
                        ),
                    ) = event
                    {
                        let decoded: Message =
                            serde_json::from_slice(&message.data)
                                .expect("received invalid Message");

                        if decoded.message_type == MessageType::OrderCreated {
                            let received_order: Order =
                                serde_json::from_str(&decoded.payload)
                                    .expect("received invalid Order");

                            break received_order;
                        }
                    }
                }
            }
        }
    })
    .await
    .expect("timed out waiting for OrderCreated");

    assert_eq!(received_order.id, order.id);
    assert_eq!(received_order.trader, "alice");
    assert_eq!(received_order.market.base, Asset::BTC);
    assert_eq!(received_order.market.quote, Asset::XMR);
    assert_eq!(received_order.side, Side::Buy);
    assert_eq!(received_order.price, order.price);
    assert_eq!(received_order.amount, order.amount);

    // Keep the compiler honest about the helper/peer address being valid.
    assert_eq!(
        peer_id_from_addr(&listen_a.with(Protocol::P2p(peer_a))),
        peer_a
    );
    assert!(listen_b.to_string().contains("/ip4/127.0.0.1/tcp/"));
}
