//! Generic swap lifecycle management.
//!
//! This crate owns the swap state machine and coordinates between the
//! protocol adapter, chain watchers, and wallet. It does not know
//! cryptographic details — those belong in the protocol adapter.

use serde::{Deserialize, Serialize};

use opendex_types::{
    BitcoinNetwork, MoneroNetwork, ParticipantId, ProtocolVersion,
    Satoshis, Piconero, SwapId, TradeId,
};
use opendex_errors::{OpenDexError, ProtocolError, StorageError};
use opendex_protocol_types::{
    ChainObservation, ProtocolAction, SettlementProtocol, Swap, SwapContext, SwapState,
};
use opendex_events::{EventId, SwapEvent};
use opendex_storage::{EventRepository, SwapRepository, TransactionIntentRepository};
use opendex_recovery::{RecoveryEngine, RecoveryDecision};
use opendex_transport::{MessageStore, ProtocolEnvelope};
use opendex_types::MessageId;

// Re-export Swap for convenience
pub use opendex_protocol_types::Swap as ProtocolSwap;

// ─── Swap Engine ────────────────────────────────────────────────────────────

/// Coordinates a swap using a settlement protocol with persistence and transport.
pub struct SwapEngine<
    P: SettlementProtocol,
    SR: SwapRepository,
    ER: EventRepository,
    TR: TransactionIntentRepository,
    RE: RecoveryEngine,
> {
    protocol: P,
    swap_repo: SR,
    event_repo: ER,
    tx_intent_repo: TR,
    recovery_engine: RE,
    /// Tracks seen message IDs for replay protection.
    message_store: MessageStore,
    /// Next sequence number for outgoing messages.
    next_sequence: u64,
}

impl<
    P: SettlementProtocol,
    SR: SwapRepository,
    ER: EventRepository,
    TR: TransactionIntentRepository,
    RE: RecoveryEngine,
> SwapEngine<P, SR, ER, TR, RE>
{
    pub fn new(
        protocol: P,
        swap_repo: SR,
        event_repo: ER,
        tx_intent_repo: TR,
        recovery_engine: RE,
    ) -> Self {
        Self {
            protocol,
            swap_repo,
            event_repo,
            tx_intent_repo,
            recovery_engine,
            message_store: MessageStore::new(),
            next_sequence: 0,
        }
    }

    /// Create a new swap, persist it, and record the event.
    pub fn create_swap(
        &self,
        swap: &Swap,
        btc_destination: String,
        xmr_destination: String,
    ) -> Result<P::State, OpenDexError> {
        // Persist the swap
        self.swap_repo.create(swap).map_err(|e| OpenDexError::Storage(e))?;

        // Record SwapCreated event
        let event = SwapEvent::SwapCreated {
            swap_id: swap.id,
            trade_id: swap.trade_id,
            btc_amount: swap.btc_amount,
            xmr_amount: swap.xmr_amount,
        };
        self.event_repo
            .append(event)
            .map_err(|e| OpenDexError::Storage(e))?;

        // Initialize protocol
        let context = SwapContext {
            swap_id: swap.id,
            trade_id: swap.trade_id,
            btc_amount: swap.btc_amount,
            xmr_amount: swap.xmr_amount,
            bitcoin_network: BitcoinNetwork::Regtest,
            monero_network: MoneroNetwork::Local,
            maker: swap.maker,
            taker: swap.taker,
            btc_destination,
            xmr_destination,
            protocol_version: swap.protocol.clone(),
        };

        self.protocol.initialize(context).map_err(|e| {
            OpenDexError::Protocol(ProtocolError::Upstream(e.to_string()))
        })
    }

    /// Transition swap state, persisting the change and recording an event.
    pub fn transition(
        &self,
        swap_id: SwapId,
        next_state: SwapState,
    ) -> Result<SwapState, OpenDexError> {
        // Load current state
        let swap = self
            .swap_repo
            .get(swap_id)
            .map_err(|e| OpenDexError::Storage(e))?
            .ok_or_else(|| {
                OpenDexError::Storage(StorageError::NotFound(format!(
                    "swap {} not found",
                    swap_id.0
                )))
            })?;

        // Validate transition
        let new_state = transition_state(&swap.state, next_state)?;

        // Persist state change
        self.swap_repo
            .update_state(swap_id, new_state.clone())
            .map_err(|e| OpenDexError::Storage(e))?;

        // Record event
        let event = SwapEvent::SwapStateChanged {
            swap_id,
            from_state: swap.state,
            to_state: new_state.clone(),
        };
        self.event_repo
            .append(event)
            .map_err(|e| OpenDexError::Storage(e))?;

        Ok(new_state)
    }

    /// Process a blockchain observation.
    pub fn observe(
        &self,
        state: &mut P::State,
        observation: ChainObservation,
    ) -> Result<Vec<ProtocolAction>, OpenDexError> {
        self.protocol.observe_chain(state, observation).map_err(|e| {
            OpenDexError::Protocol(ProtocolError::Upstream(e.to_string()))
        })
    }

    /// Record a transaction intent before broadcast.
    pub fn record_tx_intent(
        &self,
        intent: &opendex_protocol_types::TransactionIntent,
    ) -> Result<(), OpenDexError> {
        self.tx_intent_repo
            .create(intent)
            .map_err(|e| OpenDexError::Storage(e))
    }

    /// Reconcile incomplete swaps on startup.
    pub fn reconcile(&self) -> Result<Vec<Swap>, OpenDexError> {
        let incomplete = self
            .swap_repo
            .list_incomplete()
            .map_err(|e| OpenDexError::Storage(e))?;

        for swap in &incomplete {
            // Load events for this swap
            let events = self
                .event_repo
                .load_for_swap(swap.id)
                .map_err(|e| OpenDexError::Storage(e))?;

            // Load transaction intents
            let intents = self
                .tx_intent_repo
                .find(swap.id)
                .map_err(|e| OpenDexError::Storage(e))?;

            // Check for unrecorded broadcasts
            for intent in &intents {
                // In a real implementation, we'd check the blockchain here
                // to see if the transaction was actually broadcast
                let _ = intent;
            }

            // Check for relevant events
            for event in &events {
                let _ = event;
            }
        }

        Ok(incomplete)
    }

    /// Recover an interrupted swap using the recovery engine.
    pub fn recover(
        &self,
        swap_id: SwapId,
        observations: &[ChainObservation],
    ) -> Result<RecoveryDecision, OpenDexError> {
        let swap = self
            .swap_repo
            .get(swap_id)
            .map_err(|e| OpenDexError::Storage(e))?
            .ok_or_else(|| {
                OpenDexError::Storage(StorageError::NotFound(format!(
                    "swap {} not found",
                    swap_id.0
                )))
            })?;

        // Transition to recovering state
        if swap.state != SwapState::Recovering {
            self.transition(swap_id, SwapState::Recovering)?;
        }

        // Evaluate recovery decision
        self.recovery_engine
            .evaluate(&swap.state, observations)
    }

    /// Reconstruct swap state from the event log.
    pub fn reconstruct_from_events(
        &self,
        swap_id: SwapId,
    ) -> Result<Option<SwapState>, OpenDexError> {
        let events = self
            .event_repo
            .load_for_swap(swap_id)
            .map_err(|e| OpenDexError::Storage(e))?;

        let mut state = SwapState::Created;
        for record in &events {
            if let SwapEvent::SwapStateChanged { to_state, .. } = &record.event {
                state = to_state.clone();
            }
        }

        Ok(if events.is_empty() {
            None
        } else {
            Some(state)
        })
    }

    /// Complete a swap, recording the final event.
    pub fn complete_swap(&self, swap_id: SwapId) -> Result<(), OpenDexError> {
        self.transition(swap_id, SwapState::Completed)?;
        let event = SwapEvent::SwapCompleted { swap_id };
        self.event_repo
            .append(event)
            .map_err(|e| OpenDexError::Storage(e))?;
        Ok(())
    }

    /// Fail a swap with a reason, recording the event.
    pub fn fail_swap(
        &self,
        swap_id: SwapId,
        reason: String,
    ) -> Result<(), OpenDexError> {
        self.transition(swap_id, SwapState::Failed)?;
        let event = SwapEvent::SwapFailed { swap_id, reason };
        self.event_repo
            .append(event)
            .map_err(|e| OpenDexError::Storage(e))?;
        Ok(())
    }

    /// Check if a message has already been seen (replay protection).
    ///
    /// Returns `true` if the message is a duplicate and should be rejected.
    pub fn replay_protection(&self, swap_id: SwapId, message_id: MessageId) -> bool {
        self.message_store.is_seen(swap_id, message_id)
    }

    /// Mark a message as seen and update sequence tracking.
    pub fn mark_message_seen(&mut self, swap_id: SwapId, message_id: MessageId, sequence: u64) {
        self.message_store.mark_seen(swap_id, message_id, sequence);
    }

    /// Get the next sequence number for an outgoing message.
    pub fn next_sequence(&mut self) -> u64 {
        let seq = self.next_sequence;
        self.next_sequence = seq.saturating_add(1);
        seq
    }

    /// Build a protocol envelope from a protocol message.
    pub fn build_envelope(
        &mut self,
        message: &opendex_protocol_types::ProtocolMessage,
        sender: ParticipantId,
        protocol_version: ProtocolVersion,
    ) -> ProtocolEnvelope {
        let payload = serde_json::to_vec(message).unwrap_or_default();
        ProtocolEnvelope::new(
            protocol_version,
            message.swap_id,
            message.message_id,
            sender,
            message.sequence,
            payload,
        )
    }

    /// Get the last seen sequence number for a swap (for reconnection).
    pub fn last_sequence(&self, swap_id: SwapId) -> Option<u64> {
        self.message_store.last_sequence(swap_id)
    }

    /// Get the message store reference for inspection.
    pub fn message_store(&self) -> &MessageStore {
        &self.message_store
    }
}

// ─── State Transition Validation ────────────────────────────────────────────

/// Validate and apply a state transition.
pub fn transition_state(current: &SwapState, next: SwapState) -> Result<SwapState, ProtocolError> {
    if current.can_transition_to(&next) {
        Ok(next)
    } else {
        Err(ProtocolError::StateMachine(format!(
            "cannot transition from {:?} to {:?}",
            current, next
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendex_btc_xmr::BtcXmrAdapter;
    use opendex_storage::{
        InMemoryEventRepository, InMemorySwapRepository,
        InMemoryTransactionIntentRepository,
    };
    use opendex_recovery::DeterministicRecoveryEngine;
    use opendex_protocol_types::SwapState;

    type TestEngine = SwapEngine<
        BtcXmrAdapter,
        InMemorySwapRepository,
        InMemoryEventRepository,
        InMemoryTransactionIntentRepository,
        DeterministicRecoveryEngine,
    >;

    fn create_test_engine() -> (TestEngine, Swap) {
        let protocol = BtcXmrAdapter::new();
        let swap_repo = InMemorySwapRepository::new();
        let event_repo = InMemoryEventRepository::new();
        let tx_repo = InMemoryTransactionIntentRepository::new();
        let recovery = DeterministicRecoveryEngine::new();

        let engine = TestEngine::new(protocol, swap_repo, event_repo, tx_repo, recovery);

        let swap = Swap::new(
            SwapId(uuid::Uuid::new_v4()),
            TradeId(uuid::Uuid::new_v4()),
            ParticipantId(uuid::Uuid::new_v4()),
            ParticipantId(uuid::Uuid::new_v4()),
            Satoshis(100_000_000),
            Piconero(100_000_000_000_000),
            ProtocolVersion::btc_xmr_v1(),
        );

        (engine, swap)
    }

    #[test]
    fn valid_state_transitions() {
        use SwapState::*;
        assert!(Created.can_transition_to(&Negotiating));
        assert!(Negotiating.can_transition_to(&ProtocolSetup));
        assert!(ProtocolSetup.can_transition_to(&Funding));
        assert!(Funding.can_transition_to(&Funded));
        assert!(Funded.can_transition_to(&Redeeming));
        assert!(Redeeming.can_transition_to(&Completed));
    }

    #[test]
    fn invalid_state_transitions() {
        use SwapState::*;
        assert!(!Created.can_transition_to(&Completed));
        assert!(!Funded.can_transition_to(&Created));
        assert!(!Completed.can_transition_to(&Funding));
    }

    #[test]
    fn transition_state_valid() {
        let result = transition_state(&SwapState::Created, SwapState::Negotiating);
        assert!(result.is_ok());
    }

    #[test]
    fn transition_state_invalid() {
        let result = transition_state(&SwapState::Created, SwapState::Completed);
        assert!(result.is_err());
    }

    #[test]
    fn swap_creation() {
        let swap = Swap::new(
            SwapId(uuid::Uuid::new_v4()),
            TradeId(uuid::Uuid::new_v4()),
            ParticipantId(uuid::Uuid::new_v4()),
            ParticipantId(uuid::Uuid::new_v4()),
            Satoshis(100_000_000),
            Piconero(100_000_000_000_000),
            ProtocolVersion::btc_xmr_v1(),
        );

        assert_eq!(swap.state, SwapState::Created);
        assert_eq!(swap.btc_amount, Satoshis(100_000_000));
    }

    #[test]
    fn test_create_swap_persists() {
        let (engine, swap) = create_test_engine();
        let result = engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_transition_persists_state() {
        let (engine, swap) = create_test_engine();
        engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();

        let new_state = engine.transition(swap.id, SwapState::Negotiating).unwrap();
        assert_eq!(new_state, SwapState::Negotiating);
    }

    #[test]
    fn test_transition_records_event() {
        let (engine, swap) = create_test_engine();
        engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();
        engine.transition(swap.id, SwapState::Negotiating).unwrap();

        let reconstructed = engine.reconstruct_from_events(swap.id).unwrap();
        assert_eq!(reconstructed, Some(SwapState::Negotiating));
    }

    #[test]
    fn test_complete_swap() {
        let (engine, swap) = create_test_engine();
        engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();
        engine.transition(swap.id, SwapState::Negotiating).unwrap();
        engine.transition(swap.id, SwapState::ProtocolSetup).unwrap();
        engine.transition(swap.id, SwapState::Funding).unwrap();
        engine.transition(swap.id, SwapState::Funded).unwrap();
        engine.transition(swap.id, SwapState::Redeeming).unwrap();
        engine.complete_swap(swap.id).unwrap();

        let reconstructed = engine.reconstruct_from_events(swap.id).unwrap();
        assert_eq!(reconstructed, Some(SwapState::Completed));
    }

    #[test]
    fn test_recover_returns_decision() {
        let (engine, swap) = create_test_engine();
        engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();
        engine.transition(swap.id, SwapState::Negotiating).unwrap();
        engine.transition(swap.id, SwapState::ProtocolSetup).unwrap();
        engine.transition(swap.id, SwapState::Funding).unwrap();

        let decision = engine.recover(swap.id, &[]).unwrap();
        assert_eq!(decision, RecoveryDecision::Wait);
    }

    #[test]
    fn test_reconcile_returns_incomplete_swaps() {
        let (engine, swap) = create_test_engine();
        engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();
        engine.transition(swap.id, SwapState::Negotiating).unwrap();
        engine.transition(swap.id, SwapState::ProtocolSetup).unwrap();
        engine.transition(swap.id, SwapState::Funding).unwrap();

        let incomplete = engine.reconcile().unwrap();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].id, swap.id);
    }

    #[test]
    fn test_waiting_for_peer_transitions() {
        use SwapState::*;
        // Can go to WaitingForPeer from active states
        assert!(Funding.can_transition_to(&WaitingForPeer));
        assert!(Funded.can_transition_to(&WaitingForPeer));
        assert!(Redeeming.can_transition_to(&WaitingForPeer));
        assert!(Negotiating.can_transition_to(&WaitingForPeer));
        assert!(ProtocolSetup.can_transition_to(&WaitingForPeer));
        // Can resume from WaitingForPeer
        assert!(WaitingForPeer.can_transition_to(&Funding));
        assert!(WaitingForPeer.can_transition_to(&Funded));
        assert!(WaitingForPeer.can_transition_to(&Redeeming));
        assert!(WaitingForPeer.can_transition_to(&Negotiating));
        assert!(WaitingForPeer.can_transition_to(&ProtocolSetup));
        assert!(WaitingForPeer.can_transition_to(&Failed));
    }

    #[test]
    fn test_replay_protection() {
        let (mut engine, swap) = create_test_engine();
        engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();

        let message_id = opendex_types::MessageId(uuid::Uuid::new_v4());

        // First time: not seen
        assert!(!engine.replay_protection(swap.id, message_id));

        // Mark as seen
        engine.mark_message_seen(swap.id, message_id, 1);

        // Second time: detected as replay
        assert!(engine.replay_protection(swap.id, message_id));
    }

    #[test]
    fn test_sequence_tracking() {
        let (mut engine, swap) = create_test_engine();
        engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();

        let message_id = opendex_types::MessageId(uuid::Uuid::new_v4());
        engine.mark_message_seen(swap.id, message_id, 5);

        assert_eq!(engine.last_sequence(swap.id), Some(5));
    }

    #[test]
    fn test_next_sequence_increments() {
        let (mut engine, swap) = create_test_engine();
        engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();

        let seq1 = engine.next_sequence();
        let seq2 = engine.next_sequence();
        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
    }

    #[test]
    fn test_build_envelope() {
        let (mut engine, swap) = create_test_engine();
        engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();

        let message_id = opendex_types::MessageId(uuid::Uuid::new_v4());
        let sender = swap.maker;
        let message = opendex_protocol_types::ProtocolMessage::new(
            swap.id,
            vec![1, 2, 3],
            message_id,
            sender,
            1,
        );

        let envelope = engine.build_envelope(&message, sender, ProtocolVersion::btc_xmr_v1());
        assert_eq!(envelope.swap_id, swap.id);
        assert_eq!(envelope.message_id, message_id);
        assert_eq!(envelope.sender, sender);
        assert_eq!(envelope.sequence, 1);
    }

    #[test]
    fn test_message_store_isolation() {
        let (engine, swap) = create_test_engine();
        engine.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();

        let store = engine.message_store();
        let message_id = opendex_types::MessageId(uuid::Uuid::new_v4());
        // Different swap IDs should be isolated
        let other_swap_id = SwapId(uuid::Uuid::new_v4());
        assert!(!store.is_seen(other_swap_id, message_id));
    }

    #[tokio::test]
    async fn test_p2p_swap_over_mock_transport() {
        use opendex_transport::{MockTransportPair, Transport};
        use opendex_types::MessageId;

        let participant_a = ParticipantId(uuid::Uuid::new_v4());
        let participant_b = ParticipantId(uuid::Uuid::new_v4());
        let pair = MockTransportPair::new(participant_a, participant_b);

        let swap = Swap::new(
            SwapId(uuid::Uuid::new_v4()),
            TradeId(uuid::Uuid::new_v4()),
            participant_a,
            participant_b,
            Satoshis(100_000_000),
            Piconero(100_000_000_000_000),
            ProtocolVersion::btc_xmr_v1(),
        );

        // Engine A creates the swap
        let protocol_a = BtcXmrAdapter::new();
        let swap_repo_a = InMemorySwapRepository::new();
        let event_repo_a = InMemoryEventRepository::new();
        let tx_repo_a = InMemoryTransactionIntentRepository::new();
        let recovery_a = DeterministicRecoveryEngine::new();
        let mut engine_a = TestEngine::new(protocol_a, swap_repo_a, event_repo_a, tx_repo_a, recovery_a);

        engine_a.create_swap(&swap, "btc_dest".to_string(), "xmr_dest".to_string()).unwrap();

        // Engine A sends a message to Engine B
        let message_id = MessageId(uuid::Uuid::new_v4());
        let message = opendex_protocol_types::ProtocolMessage::new(
            swap.id,
            vec![1, 2, 3],
            message_id,
            participant_a,
            0,
        );
        let envelope = engine_a.build_envelope(&message, participant_a, ProtocolVersion::btc_xmr_v1());

        // Send via transport
        pair.transport_a
            .send(participant_b, envelope)
            .await
            .unwrap();

        // Engine B receives the message
        let inbound = pair.transport_b.receive().await.unwrap();
        assert_eq!(inbound.peer, participant_a);
        assert_eq!(inbound.message.swap_id, swap.id);
        assert_eq!(inbound.message.message_id, message_id);

        // Verify replay protection: mark as seen
        engine_a.mark_message_seen(swap.id, message_id, 0);
        assert!(engine_a.replay_protection(swap.id, message_id));
    }

    #[tokio::test]
    async fn test_mock_transport_channel_closed() {
        use opendex_transport::{MockTransportError, MockTransportPair, Transport};

        let participant_a = ParticipantId(uuid::Uuid::new_v4());
        let participant_b = ParticipantId(uuid::Uuid::new_v4());
        let pair = MockTransportPair::new(participant_a, participant_b);

        // Drop the sender side
        drop(pair.transport_a);

        // Receiving from the other side should eventually fail
        let result = pair.transport_b.receive().await;
        assert!(matches!(result, Err(MockTransportError::ChannelClosed)));
    }
}
