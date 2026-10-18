//! Protocol envelope and inbound message types.
//!
//! Every message sent over the transport is wrapped in a `ProtocolEnvelope`
//! that carries routing metadata: protocol version, swap ID, message ID,
//! sender, and sequence number.

use serde::{Deserialize, Serialize};

use opendex_types::{MessageId, ParticipantId, ProtocolVersion, SwapId};

/// A message received from the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage<T> {
    /// The peer that sent this message.
    pub peer: ParticipantId,
    /// The message payload.
    pub message: T,
    /// ISO 8601 timestamp when the message was received.
    pub received_at: String,
}

impl<T> InboundMessage<T> {
    pub fn new(peer: ParticipantId, message: T, received_at: String) -> Self {
        Self {
            peer,
            message,
            received_at,
        }
    }
}

/// Envelope that wraps protocol messages for transport-level routing.
///
/// Carries metadata required for message delivery, replay protection,
/// and ordering — independent of the underlying transport implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolEnvelope {
    /// The settlement protocol version this message belongs to.
    pub protocol: ProtocolVersion,
    /// The swap this message is associated with.
    pub swap_id: SwapId,
    /// Unique message identifier for replay protection.
    pub message_id: MessageId,
    /// The sender of this message.
    pub sender: ParticipantId,
    /// Monotonically increasing sequence number for ordering.
    pub sequence: u64,
    /// The serialized protocol message payload.
    pub payload: Vec<u8>,
}

impl ProtocolEnvelope {
    pub fn new(
        protocol: ProtocolVersion,
        swap_id: SwapId,
        message_id: MessageId,
        sender: ParticipantId,
        sequence: u64,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            protocol,
            swap_id,
            message_id,
            sender,
            sequence,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_swap_id() -> SwapId {
        SwapId(Uuid::new_v4())
    }

    fn test_message_id() -> MessageId {
        MessageId(Uuid::new_v4())
    }

    fn test_participant_id() -> ParticipantId {
        ParticipantId(Uuid::new_v4())
    }

    #[test]
    fn test_protocol_envelope_construction() {
        let swap_id = test_swap_id();
        let message_id = test_message_id();
        let sender = test_participant_id();
        let envelope = ProtocolEnvelope::new(
            ProtocolVersion::btc_xmr_v1(),
            swap_id,
            message_id,
            sender,
            1,
            vec![1, 2, 3],
        );
        assert_eq!(envelope.swap_id, swap_id);
        assert_eq!(envelope.message_id, message_id);
        assert_eq!(envelope.sender, sender);
        assert_eq!(envelope.sequence, 1);
        assert_eq!(envelope.payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_inbound_message_construction() {
        let peer = test_participant_id();
        let envelope = ProtocolEnvelope::new(
            ProtocolVersion::btc_xmr_v1(),
            test_swap_id(),
            test_message_id(),
            peer,
            1,
            vec![],
        );
        let inbound = InboundMessage::new(peer, envelope, "2024-01-01T00:00:00Z".to_string());
        assert_eq!(inbound.peer, peer);
        assert_eq!(inbound.received_at, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_envelope_serialization() {
        let swap_id = test_swap_id();
        let message_id = test_message_id();
        let sender = test_participant_id();
        let envelope = ProtocolEnvelope::new(
            ProtocolVersion::btc_xmr_v1(),
            swap_id,
            message_id,
            sender,
            42,
            vec![10, 20, 30],
        );
        let serialized = serde_json::to_string(&envelope).unwrap();
        let deserialized: ProtocolEnvelope = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.swap_id, swap_id);
        assert_eq!(deserialized.message_id, message_id);
        assert_eq!(deserialized.sequence, 42);
    }
}
