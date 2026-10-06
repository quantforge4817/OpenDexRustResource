//! Mock transport for testing without a network.
//!
//! Uses in-memory tokio channels to connect two sides of a swap.
//! Each side has its own `MockTransport` half, connected via shared channels.

use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

use opendex_types::ParticipantId;

use crate::envelope::{InboundMessage, ProtocolEnvelope};
use crate::transport::Transport;

use async_trait::async_trait;

/// Error type for mock transport.
#[derive(Debug)]
pub enum MockTransportError {
    /// The receive channel was closed (peer disconnected).
    ChannelClosed,
    /// Send failed because the channel was closed.
    SendFailed,
}

impl std::fmt::Display for MockTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockTransportError::ChannelClosed => write!(f, "channel closed"),
            MockTransportError::SendFailed => write!(f, "send failed"),
        }
    }
}

impl std::error::Error for MockTransportError {}

/// A pair of connected mock transports for testing.
///
/// Creates two `MockTransport` instances that can send and receive
/// messages to each other via in-memory channels.
pub struct MockTransportPair {
    /// Transport for participant A.
    pub transport_a: MockTransport,
    /// Transport for participant B.
    pub transport_b: MockTransport,
}

impl MockTransportPair {
    /// Create a new pair of connected mock transports.
    pub fn new(participant_a: ParticipantId, participant_b: ParticipantId) -> Self {
        let (tx_a, rx_a) = mpsc::channel::<(ParticipantId, ProtocolEnvelope)>(64);
        let (tx_b, rx_b) = mpsc::channel::<(ParticipantId, ProtocolEnvelope)>(64);

        let transport_a = MockTransport {
            participant_id: participant_a,
            sender: Arc::new(tx_a),
            receiver: Arc::new(Mutex::new(rx_b)),
        };

        let transport_b = MockTransport {
            participant_id: participant_b,
            sender: Arc::new(tx_b),
            receiver: Arc::new(Mutex::new(rx_a)),
        };

        Self { transport_a, transport_b }
    }
}

/// In-memory mock transport for testing.
///
/// Connected to another `MockTransport` via tokio channels.
pub struct MockTransport {
    participant_id: ParticipantId,
    sender: Arc<mpsc::Sender<(ParticipantId, ProtocolEnvelope)>>,
    receiver: Arc<Mutex<mpsc::Receiver<(ParticipantId, ProtocolEnvelope)>>>,
}

impl MockTransport {
    pub fn participant_id(&self) -> ParticipantId {
        self.participant_id
    }
}

#[async_trait]
impl Transport for MockTransport {
    type Error = MockTransportError;

    async fn send(&self, _peer: ParticipantId, envelope: ProtocolEnvelope) -> Result<(), Self::Error> {
        self.sender
            .send((self.participant_id, envelope))
            .await
            .map_err(|_| MockTransportError::SendFailed)
    }

    async fn receive(&self) -> Result<InboundMessage<ProtocolEnvelope>, Self::Error> {
        let mut rx = self.receiver.lock().await;
        match rx.recv().await {
            Some((peer, envelope)) => {
                let now = chrono::Utc::now().to_rfc3339();
                Ok(InboundMessage::new(peer, envelope, now))
            }
            None => Err(MockTransportError::ChannelClosed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use opendex_types::{SwapId, MessageId, ProtocolVersion};

    fn test_participant_id() -> ParticipantId {
        ParticipantId(Uuid::new_v4())
    }

    fn test_swap_id() -> SwapId {
        SwapId(Uuid::new_v4())
    }

    fn test_message_id() -> MessageId {
        MessageId(Uuid::new_v4())
    }

    fn test_envelope() -> ProtocolEnvelope {
        ProtocolEnvelope::new(
            ProtocolVersion::btc_xmr_v1(),
            test_swap_id(),
            test_message_id(),
            test_participant_id(),
            1,
            vec![1, 2, 3],
        )
    }

    #[tokio::test]
    async fn test_mock_transport_send_receive() {
        let participant_a = test_participant_id();
        let participant_b = test_participant_id();
        let pair = MockTransportPair::new(participant_a, participant_b);

        let envelope = test_envelope();
        let envelope_clone = envelope.clone();

        // Send from A to B
        pair.transport_a
            .send(participant_b, envelope)
            .await
            .unwrap();

        // Receive at B
        let inbound = pair.transport_b.receive().await.unwrap();
        assert_eq!(inbound.peer, participant_a);
        assert_eq!(inbound.message.swap_id, envelope_clone.swap_id);
        assert_eq!(inbound.message.message_id, envelope_clone.message_id);
    }

    #[tokio::test]
    async fn test_mock_transport_bidirectional() {
        let participant_a = test_participant_id();
        let participant_b = test_participant_id();
        let pair = MockTransportPair::new(participant_a, participant_b);

        let envelope_a = test_envelope();
        let envelope_b = test_envelope();

        // Send from A to B
        pair.transport_a
            .send(participant_b, envelope_a.clone())
            .await
            .unwrap();

        // Send from B to A
        pair.transport_b
            .send(participant_a, envelope_b.clone())
            .await
            .unwrap();

        // Receive at B (from A)
        let inbound_b = pair.transport_b.receive().await.unwrap();
        assert_eq!(inbound_b.peer, participant_a);

        // Receive at A (from B)
        let inbound_a = pair.transport_a.receive().await.unwrap();
        assert_eq!(inbound_a.peer, participant_b);
    }

    #[tokio::test]
    async fn test_mock_transport_multiple_messages() {
        let participant_a = test_participant_id();
        let participant_b = test_participant_id();
        let pair = MockTransportPair::new(participant_a, participant_b);

        for i in 0..5 {
            let envelope = ProtocolEnvelope::new(
                ProtocolVersion::btc_xmr_v1(),
                test_swap_id(),
                test_message_id(),
                participant_a,
                i,
                vec![i as u8],
            );
            pair.transport_a
                .send(participant_b, envelope)
                .await
                .unwrap();
        }

        for i in 0..5 {
            let inbound = pair.transport_b.receive().await.unwrap();
            assert_eq!(inbound.message.sequence, i as u64);
        }
    }
}

