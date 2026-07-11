use anyhow::Result;
use futures::StreamExt;

use libp2p::{
    identify,
    kad,
    identity,
    noise,
    tcp,
    yamux,
    Swarm,
    SwarmBuilder,
    Multiaddr,
    swarm::NetworkBehaviour,
};

use std::{
    fs,
    path::PathBuf,
};

use tracing_subscriber;


fn identity_path() -> PathBuf {
    let mut path =
        dirs::home_dir()
            .expect("Cannot find home directory");

    path.push(".opendex");
    path.push("identity.key");

    path
}


fn load_or_create_identity() -> Result<identity::Keypair> {
    let path = identity_path();

    if path.exists() {
        let bytes = fs::read(&path)?;

        let key =
            identity::Keypair::from_protobuf_encoding(&bytes)?;

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
}


fn load_bootstrap_peers() -> Vec<Multiaddr> {
    let path = dirs::home_dir()
        .unwrap()
        .join(".opendex/bootstrap.peers");

    if !path.exists() {
        return vec![];
    }

    let data = fs::read_to_string(path)
        .unwrap();

    data.lines()
        .filter_map(|line| line.parse().ok())
        .collect()
}


#[tokio::main]
async fn main() -> Result<()> {

    let args: Vec<String> = std::env::args().collect();

    let port = args
        .iter()
    .find(|arg| arg.parse::<u16>().is_ok())
        .cloned()
        .unwrap_or_else(|| "4001".to_string());

    tracing_subscriber::fmt::init();


    let id_keys = if args.contains(&"--new-identity".to_string()) {

        identity::Keypair::generate_ed25519()

    } else {

        load_or_create_identity()?

    };


    let peer_id =
        id_keys.public().to_peer_id();


    println!("================================");
    println!(" OpenDEX Node Started");
    println!("================================");

    println!("Peer ID:");
    println!("{}", peer_id);


    let mut swarm: Swarm<OpenDexBehaviour> =
        SwarmBuilder::with_existing_identity(id_keys)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {

                let peer_id = key.public().to_peer_id();

                let store =
                    kad::store::MemoryStore::new(peer_id);

                let kademlia =
                    kad::Behaviour::new(
                        peer_id,
                        store,
                    );


                OpenDexBehaviour {

                    identify: identify::Behaviour::new(
                        identify::Config::new(
                            "/opendex/1.0.0".into(),
                            key.public(),
                        ),
                    ),

                    kademlia,

                }

            })?
            .build();


    let listen_addr: Multiaddr =
        format!("/ip4/127.0.0.1/tcp/{}", port)
            .parse()?;

    swarm.listen_on(listen_addr.clone())?;

         println!("Listening for peers...");
         println!("Address:");
         println!("{}/p2p/{}", listen_addr, peer_id);

   


    if port == "4001" {

        for peer in load_bootstrap_peers() {

            println!(
                "Dialing bootstrap peer: {:?}",
                peer
            );

            swarm.dial(peer)?;
        }
    }


    loop {

        if let Some(event) = swarm.next().await {

            match event {

                libp2p::swarm::SwarmEvent::ConnectionEstablished {
                    peer_id,
                    endpoint,
                    ..
                } => {

                    println!("Connected to peer: {:?}", peer_id);

                    match endpoint {

                        libp2p::core::ConnectedPoint::Dialer { address, .. } => {

                            swarm
                                .behaviour_mut()
                                .kademlia
                                .add_address(
                                    &peer_id,
                                    address.clone()
                                );

                        }

                        libp2p::core::ConnectedPoint::Listener { send_back_addr, .. } => {

                            swarm
                                .behaviour_mut()
                                .kademlia
                                .add_address(
                                    &peer_id,
                                    send_back_addr.clone()
                                );

                        }
                    }


                    swarm
                        .behaviour_mut()
                        .kademlia
                        .bootstrap()
                        .ok();

                }


                libp2p::swarm::SwarmEvent::Behaviour(event) => {
                    println!("Behaviour event: {:?}", event);
                }


                other => {
                    println!("{:?}", other);
                }

            }
        }
    }
}
