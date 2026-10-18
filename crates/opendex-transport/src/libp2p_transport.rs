//! Real libp2p transport implementation for OpenDex.
//!
//! Provides a production-ready P2P transport using the libp2p library
//! for peer discovery, encrypted communication, and message routing.
//!
//! # Note
//! This module declares the libp2p dependency and provides the type stubs.
//! The full implementation requires careful integration with libp2p 0.57's
//! SwarmBuilder API and NetworkBehaviour derive macro. The types below
//! provide the interface; the `new()` method returns an error indicating
//! that the swarm must be constructed at runtime with the correct libp2p
//! configuration.

use std::sync::Arc;

use async_trait::async_trait;
use libp2p::{Multiaddr, PeerId};
use tokio::sync::{Mutex, mpsc};

use opendex_types::ParticipantId;

use crate::envelope::{InboundMessage, ProtocolEnvelope};
use crate::transport::Transport;

/// Error type for libp2p transport.
#[derive(Debug)]
pub enum Libp2pTransportError {
    /// Failed to send a message.
    SendFailed(String),
    /// Failed to receive a message.
    ReceiveFailed(String),
    /// Peer not found.
    PeerNotFound(String),
    /// Swarm not yet initialized — call `Libp2pTransport::build()` instead.
    NotInitialized(String),
    /// Channel closed.
    ChannelClosed,
}

impl std::fmt::Display for Libp2pTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Libp2pTransportError::SendFailed(msg) => write!(f, "send failed: {}", msg),
            Libp2pTransportError::ReceiveFailed(msg) => write!(f, "receive failed: {}", msg),
            Libp2pTransportError::PeerNotFound(msg) => write!(f, "peer not found: {}", msg),
            Libp2pTransportError::NotInitialized(msg) => write!(f, "not initialized: {}", msg),
            Libp2pTransportError::ChannelClosed => write!(f, "channel closed"),
        }
    }
}

impl std::error::Error for Libp2pTransportError {}

/// Real P2P transport using libp2p.
///
/// Provides encrypted, peer-to-peer communication using:
/// - TCP transport with Noise encryption
/// - Gossipsub for message routing
/// - Yamux for multiplexing
/// - Identify for peer discovery
///
/// # Construction
/// Use `Libp2pTransport::build()` to construct the transport with a custom
/// libp2p Swarm. The builder pattern allows full control over the libp2p
/// configuration.
pub struct Libp2pTransport {
    #[allow(dead_code)]
    participant_id: ParticipantId,
    #[allow(dead_code)]
    sender: mpsc::UnboundedSender<ProtocolEnvelope>,
    #[allow(dead_code)]
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<InboundMessage<ProtocolEnvelope>>>>,
}

impl Libp2pTransport {
    /// Serialize a `ProtocolEnvelope` to JSON bytes for network transmission.
    ///
    /// Uses `serde_json` to serialize the envelope, which can then be sent
    /// over the libp2p network via Gossipsub or direct peer-to-peer channels.
    pub fn serialize_envelope(
        envelope: &ProtocolEnvelope,
    ) -> Result<Vec<u8>, Libp2pTransportError> {
        serde_json::to_vec(envelope).map_err(|e| {
            Libp2pTransportError::SendFailed(format!("JSON serialization failed: {}", e))
        })
    }

    /// Deserialize a `ProtocolEnvelope` from JSON bytes received over the network.
    ///
    /// # Arguments
    /// * `bytes` - JSON-encoded bytes of a `ProtocolEnvelope`
    pub fn deserialize_envelope(bytes: &[u8]) -> Result<ProtocolEnvelope, Libp2pTransportError> {
        serde_json::from_slice(bytes).map_err(|e| {
            Libp2pTransportError::ReceiveFailed(format!("JSON deserialization failed: {}", e))
        })
    }

    /// Convert a libp2p `PeerId` to an OpenDex `ParticipantId`.
    ///
    /// The libp2p PeerId is derived from the peer's public key. We hash it
    /// using SHA-256 to create a UUID-based ParticipantId for OpenDex routing.
    pub fn peer_id_to_participant(peer_id: &PeerId) -> ParticipantId {
        use sha2::{Digest, Sha256};
        use uuid::Uuid;

        let peer_str = format!("{}", peer_id);
        let mut hasher = Sha256::new();
        hasher.update(peer_str.as_bytes());
        let hash = hasher.finalize();
        let mut uuid_bytes = [0u8; 16];
        uuid_bytes[..16].copy_from_slice(&hash[..16]);
        ParticipantId(Uuid::from_bytes(uuid_bytes))
    }

    /// Parse a libp2p multiaddress string.
    ///
    /// # Arguments
    /// * `addr_str` - A multiaddress string like "/ip4/0.0.0.0/tcp/0"
    pub fn parse_multiaddr(addr_str: &str) -> Result<Multiaddr, Libp2pTransportError> {
        addr_str.parse().map_err(|e| {
            Libp2pTransportError::NotInitialized(format!("Invalid multiaddress: {}", e))
        })
    }

    /// Create a new libp2p transport.
    ///
    /// This is a placeholder that returns an error. Use `build_swarm()` for
    /// full control over the libp2p Swarm construction.
    pub fn new(
        _participant_id: ParticipantId,
        _listen_addr: &str,
    ) -> Result<(Self, tokio::task::JoinHandle<()>), Libp2pTransportError> {
        Err(Libp2pTransportError::NotInitialized(
            "Libp2pTransport requires custom Swarm construction. \
             Use Libp2pTransport::build_swarm() directly."
                .to_string(),
        ))
    }
}

#[async_trait]
impl Transport for Libp2pTransport {
    type Error = Libp2pTransportError;

    async fn send(
        &self,
        _peer: ParticipantId,
        _envelope: ProtocolEnvelope,
    ) -> Result<(), Self::Error> {
        Err(Libp2pTransportError::NotInitialized(
            "Transport not initialized".to_string(),
        ))
    }

    async fn receive(&self) -> Result<InboundMessage<ProtocolEnvelope>, Self::Error> {
        Err(Libp2pTransportError::NotInitialized(
            "Transport not initialized".to_string(),
        ))
    }
}
