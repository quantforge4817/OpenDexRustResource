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
