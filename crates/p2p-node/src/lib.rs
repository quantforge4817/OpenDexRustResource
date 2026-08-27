pub mod balances;
pub mod network;
pub mod order_factory;
pub mod orderbook;
pub mod settlement;
pub mod trade_state;
pub mod trades;

use libp2p::{gossipsub, identify, kad, ping, swarm::NetworkBehaviour};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "OpenDexBehaviourEvent")]
pub struct OpenDexBehaviour {
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub ping: ping::Behaviour,
    pub identify: identify::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
}

#[derive(Debug)]
pub enum OpenDexBehaviourEvent {
    Kademlia(kad::Event),
    Ping(ping::Event),
    Identify(identify::Event),
    Gossipsub(gossipsub::Event),
}

impl From<kad::Event> for OpenDexBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        Self::Kademlia(event)
    }
}

impl From<ping::Event> for OpenDexBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        Self::Ping(event)
    }
}

impl From<identify::Event> for OpenDexBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        Self::Identify(event)
    }
}

impl From<gossipsub::Event> for OpenDexBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        Self::Gossipsub(event)
    }
}
