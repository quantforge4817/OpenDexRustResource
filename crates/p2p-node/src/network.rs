use anyhow::Result;
use libp2p::{Swarm, SwarmBuilder, gossipsub, identify, identity, kad, noise, ping, tcp, yamux};

use crate::OpenDexBehaviour;
use protocol::message::OpenDexMessage;

pub fn build_swarm(id_keys: identity::Keypair) -> Result<Swarm<OpenDexBehaviour>> {
    let swarm = SwarmBuilder::with_existing_identity(id_keys)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, || {
            yamux::Config::default()
        })?
        .with_behaviour(|key| {
            let peer_id = key.public().to_peer_id();
            let store = kad::store::MemoryStore::new(peer_id);

            let mut kademlia = kad::Behaviour::new(peer_id, store);
            kademlia.set_mode(Some(kad::Mode::Server));

            let config = gossipsub::ConfigBuilder::default()
                .build()
                .expect("valid gossipsub config");

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                config,
            )
            .expect("create gossipsub");

            Ok(OpenDexBehaviour {
                kademlia,
                ping: ping::Behaviour::new(
                    ping::Config::new().with_interval(std::time::Duration::from_secs(15)),
                ),
                identify: identify::Behaviour::new(identify::Config::new(
                    "/opendex/1.0.0".into(),
                    key.public(),
                )),
                gossipsub,
            })
        })?
        .with_swarm_config(|cfg| {
            cfg.with_idle_connection_timeout(std::time::Duration::from_secs(300))
        })
        .build();

    Ok(swarm)
}

pub fn publish_message(
    swarm: &mut Swarm<OpenDexBehaviour>,
    topic: &gossipsub::IdentTopic,
    message: &OpenDexMessage,
) -> Result<()> {
    let encoded = serde_json::to_vec(message)?;

    println!(
        "Gossipsub peers before publish: {:?}",
        swarm.behaviour().gossipsub.all_peers().collect::<Vec<_>>()
    );

    match swarm
        .behaviour_mut()
        .gossipsub
        .publish(topic.clone(), encoded)
    {
        Ok(_) => {
            println!("Gossipsub publish OK");
        }

        Err(e) => {
            println!("Gossipsub publish failed: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
