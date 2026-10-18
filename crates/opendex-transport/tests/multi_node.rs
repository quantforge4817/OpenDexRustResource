//! Multi-node adversarial tests using MockTransportPair.

use opendex_transport::mock::MockTransportPair;
use opendex_types::ParticipantId;
use uuid::Uuid;

#[test]
fn test_mock_transport_pair_connects() {
    let participant_a = ParticipantId(Uuid::new_v4());
    let participant_b = ParticipantId(Uuid::new_v4());

    let pair = MockTransportPair::new(participant_a, participant_b);

    assert_eq!(pair.transport_a.participant_id(), participant_a);
    assert_eq!(pair.transport_b.participant_id(), participant_b);
}

#[test]
fn test_message_replay_detection() {
    use opendex_transport::message_store::MessageStore;

    let mut store = MessageStore::new();
    let swap_id = opendex_types::SwapId(Uuid::new_v4());
    let msg_id = opendex_types::MessageId(Uuid::new_v4());

    assert!(!store.is_seen(swap_id, msg_id));
    store.mark_seen(swap_id, msg_id, 1);
    assert!(store.is_seen(swap_id, msg_id));
}

#[test]
fn test_sequence_tracking() {
    use opendex_transport::message_store::MessageStore;

    let mut store = MessageStore::new();
    let swap_id = opendex_types::SwapId(Uuid::new_v4());

    let msg1 = opendex_types::MessageId(Uuid::new_v4());
    store.mark_seen(swap_id, msg1, 1);
    assert_eq!(store.last_sequence(swap_id), Some(1));

    let msg2 = opendex_types::MessageId(Uuid::new_v4());
    store.mark_seen(swap_id, msg2, 2);
    assert_eq!(store.last_sequence(swap_id), Some(2));

    assert_eq!(store.last_sequence(opendex_types::SwapId(Uuid::new_v4())), None);
}

#[test]
fn test_malicious_state_rejected() {
    use opendex_protocol_types::SwapState;

    // Invalid state transitions should be rejected
    assert!(!SwapState::Created.can_transition_to(&SwapState::Completed));
    assert!(!SwapState::Funded.can_transition_to(&SwapState::Created));
    assert!(!SwapState::Completed.can_transition_to(&SwapState::Funding));
}
