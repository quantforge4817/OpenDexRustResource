use anyhow::Result;
use futures::StreamExt;
use libp2p::ping;

use libp2p::{
    Multiaddr, Swarm, SwarmBuilder, identify, identity, kad, noise, swarm::NetworkBehaviour, tcp,
    yamux,
};

use std::{fs, path::PathBuf};

use tracing_subscriber;

fn identity_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("Cannot find home directory");

    path.push(".opendex");
    path.push("identity.key");

    path
}

fn load_or_create_identity() -> Result<identity::Keypair> {
    let path = identity_path();

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
    identify: identify::Behaviour,
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    ping: ping::Behaviour,
}



fn save_bootstrap_peer(addr: &Multiaddr) -> Result<()> {
    // Ignore addresses without a PeerId.
    // These are usually temporary connection addresses like /tcp/38232.
    if !addr
        .iter()
        .any(|p| matches!(p, libp2p::multiaddr::Protocol::P2p(_)))
    {
        return Ok(());
    }

    let path = dirs::home_dir().unwrap().join(".opendex/bootstrap.peers");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut peers = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };

    let addr_string = addr.to_string();

    if !peers.contains(&addr_string) {
        peers.push_str(&addr_string);
        peers.push('\n');

        fs::write(&path, peers)?;
    }

    Ok(())
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

    let id_keys = if args.contains(&"--new-identity".to_string()) {
        identity::Keypair::generate_ed25519()
    } else {
	        load_or_create_identity()?
    };

    let peer_id = id_keys.public().to_peer_id();

    println!("================================");
    println!(" OpenDEX Node Started");
    println!("================================");

    println!("Peer ID:");
    println!("{}", peer_id);

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

            Ok(OpenDexBehaviour {
                identify: identify::Behaviour::new(identify::Config::new(
                    "/opendex/1.0.0".into(),
                    key.public(),
                )),

                kademlia,

                ping: ping::Behaviour::new(ping::Config::new()),
            })
        })?
        .build();

    swarm
        .behaviour_mut()
        .kademlia
        .set_mode(Some(kad::Mode::Server));

    let listen_addr: Multiaddr = format!("/ip4/127.0.0.1/tcp/{}", port).parse()?;

    swarm.listen_on(listen_addr.clone())?;

    println!("Listening for peers...");
    println!("Address:");
    println!("{}/p2p/{}", listen_addr, peer_id);

    if let Some(addr) = peer_addr {
        let peer: Multiaddr = addr.parse()?;

        println!("Dialing peer: {}", peer);

        swarm.dial(peer)?;
    }

    if port == "4001" {
    for peer in load_bootstrap_peers()? {
        println!("Dialing bootstrap peer: {:?}", peer);

        if let Err(e) = swarm.dial(peer.clone()) {
            eprintln!("Failed dialing bootstrap peer {:?}: {}", peer, e);
        }
    }
}

    loop {
        if let Some(event) = swarm.next().await {
            match event {
                libp2p::swarm::SwarmEvent::ConnectionEstablished {
    peer_id,
    ..
} => {
    println!("Connected to peer: {:?}", peer_id);
}

                              libp2p::swarm::SwarmEvent::Behaviour(event) => match event {
                    OpenDexBehaviourEvent::Identify(identify::Event::Received {
                             peer_id,
                             info,
                              ..
                    }) => {
    for addr in info.listen_addrs {
        let addr = addr.with(libp2p::multiaddr::Protocol::P2p(peer_id.into()));

        swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, addr.clone());

        println!("Kademlia learned address: {:?}", addr);
    }
}

                    OpenDexBehaviourEvent::Ping(event) => {
                        println!("Ping event: {:?}", event);
                    }

                    OpenDexBehaviourEvent::Kademlia(event) => {
                        match event {
                            kad::Event::RoutingUpdated {
                             peer,
                             addresses,
                             ..
                            } => {
    println!("Routing updated: {:?}", peer);

    for addr in addresses.iter() {
        if let Err(e) = save_bootstrap_peer(addr) {
            eprintln!("Failed saving peer: {}", e);
        } else{

        println!("Saved stable peer: {:?}", addr);
        }
      }
    }

                            kad::Event::OutboundQueryProgressed { result, .. } => {
                                if matches!(result, kad::QueryResult::Bootstrap(_)) {
                                    println!("Bootstrap progress: {:?}", result);
                                }
                            }

                            _ => {}
                        }
                    }

                    other => {
                        println!("Other behaviour event: {:?}", other);
                    }
                },

                _ => {}
            }
        }
    }
}
fn load_bootstrap_peers() -> Result<Vec<Multiaddr>> {
    let path = dirs::home_dir()
        .unwrap()
        .join(".opendex/bootstrap.peers");

    if !path.exists() {
        return Ok(vec![]);
    }

    let data = fs::read_to_string(path)?;

    let peers = data
        .lines()
        .filter_map(|line| line.parse().ok())
        .collect();

    Ok(peers)
}
