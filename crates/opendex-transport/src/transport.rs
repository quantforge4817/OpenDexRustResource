//! Transport trait for network-agnostic message delivery.
//!
//! The `Transport` trait decouples the swap state machine from any specific
//! network implementation. Real implementations may use libp2p, Tor, or
//! direct TCP; the mock implementation uses in-memory channels.

use async_trait::async_trait;

use opendex_types::ParticipantId;

use crate::envelope::{InboundMessage, ProtocolEnvelope};

/// Asynchronous network transport for protocol messages.
///
/// Implementations may use any underlying network technology (libp2p, Tor,
/// direct TCP, relay). The trait is generic over the error type to allow
/// each implementation to return its own error.
#[async_trait]
pub trait Transport: Send + Sync {
    /// The error type returned by this transport.
    type Error: std::error::Error + Send + Sync;

    /// Send a protocol envelope to a specific peer.
    async fn send(&self, peer: ParticipantId, envelope: ProtocolEnvelope) -> Result<(), Self::Error>;

    /// Receive the next inbound message from any peer.
    /// Blocks until a message is available or an error occurs.
    async fn receive(&self) -> Result<InboundMessage<ProtocolEnvelope>, Self::Error>;
}

