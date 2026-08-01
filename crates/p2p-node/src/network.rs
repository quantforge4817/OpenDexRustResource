use anyhow::Result;
use libp2p::{Swarm, gossipsub};

use crate::OpenDexBehaviour;
use protocol::message::OpenDexMessage;

pub fn publish_message(
    swarm: &mut Swarm<OpenDexBehaviour>,
    topic: &gossipsub::IdentTopic,
    message: &OpenDexMessage,
) -> Result<()> {
    let encoded = serde_json::to_vec(message)?;

    swarm
        .behaviour_mut()
        .gossipsub
        .publish(topic.clone(), encoded)?;

    Ok(())
}
