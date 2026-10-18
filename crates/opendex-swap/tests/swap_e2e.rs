//! End-to-end integration tests for the BTC→XMR atomic swap protocol.

use opendex_btc_xmr::BtcXmrAdapter;
use opendex_protocol_types::{Swap, SwapState};
use opendex_recovery::DeterministicRecoveryEngine;
use opendex_storage::{
    InMemoryEventRepository, InMemorySwapRepository, InMemoryTransactionIntentRepository,
};
use opendex_swap::SwapEngine;
use opendex_types::{Piconero, ProtocolVersion, Satoshis, SwapId, TradeId, ParticipantId};
use uuid::Uuid;

type TestEngine = SwapEngine<
    BtcXmrAdapter,
    InMemorySwapRepository,
    InMemoryEventRepository,
    InMemoryTransactionIntentRepository,
    DeterministicRecoveryEngine,
    opendex_bitcoin::MockBitcoinChain,
    opendex_monero::MockMoneroChain,
>;

fn create_engine() -> TestEngine {
    let protocol = BtcXmrAdapter::new();
    let swap_repo = InMemorySwapRepository::new();
    let event_repo = InMemoryEventRepository::new();
    let tx_repo = InMemoryTransactionIntentRepository::new();
    let recovery = DeterministicRecoveryEngine::new();
    let bitcoin_chain = opendex_bitcoin::MockBitcoinChain::new();
    let monero_chain = opendex_monero::MockMoneroChain::new();

    TestEngine::new(
        protocol, swap_repo, event_repo, tx_repo, recovery, bitcoin_chain, monero_chain,
    )
}

fn create_swap() -> Swap {
    Swap::new(
        SwapId(Uuid::new_v4()),
        TradeId(Uuid::new_v4()),
        ParticipantId(Uuid::new_v4()),
        ParticipantId(Uuid::new_v4()),
        Satoshis(100_000_000),
        Piconero(100_000_000_000_000),
        ProtocolVersion::btc_xmr_v1(),
    )
}

#[test]
fn test_swap_creation() {
    let engine = create_engine();
    let swap = create_swap();

    let state = engine
        .create_swap(&swap, "bcrt1qtest".to_string(), "44AFFqB".to_string())
        .expect("swap creation should succeed");

    assert_eq!(state.state, SwapState::Created);
}

#[test]
fn test_state_transitions() {
    let engine = create_engine();
    let swap = create_swap();

    engine
        .create_swap(&swap, "bcrt1qtest".to_string(), "44AFFqB".to_string())
        .unwrap();

    engine.transition(swap.id, SwapState::Negotiating).unwrap();
    engine.transition(swap.id, SwapState::ProtocolSetup).unwrap();
    engine.transition(swap.id, SwapState::Funding).unwrap();
    engine.transition(swap.id, SwapState::Funded).unwrap();
    engine.transition(swap.id, SwapState::Redeeming).unwrap();
}

#[test]
fn test_complete_swap() {
    let engine = create_engine();
    let swap = create_swap();

    engine.create_swap(&swap, "bcrt1qtest".to_string(), "44AFFqB".to_string()).unwrap();
    engine.transition(swap.id, SwapState::Negotiating).unwrap();
    engine.transition(swap.id, SwapState::ProtocolSetup).unwrap();
    engine.transition(swap.id, SwapState::Funding).unwrap();
    engine.transition(swap.id, SwapState::Funded).unwrap();
    engine.transition(swap.id, SwapState::Redeeming).unwrap();
    engine.complete_swap(swap.id).expect("should complete swap");
}

#[test]
fn test_fail_swap() {
    let engine = create_engine();
    let swap = create_swap();

    engine.create_swap(&swap, "bcrt1qtest".to_string(), "44AFFqB".to_string()).unwrap();
    engine.fail_swap(swap.id, "timeout".to_string()).expect("should fail swap");
}

#[test]
fn test_timeout_refund_scenario() {
    let engine = create_engine();
    let swap = create_swap();

    engine.create_swap(&swap, "bcrt1qtest".to_string(), "44AFFqB".to_string()).unwrap();
    engine.transition(swap.id, SwapState::Negotiating).unwrap();
    engine.transition(swap.id, SwapState::ProtocolSetup).unwrap();
    engine.transition(swap.id, SwapState::Funding).unwrap();

    engine.fail_swap(swap.id, "timeout exceeded".to_string()).unwrap();
}

#[test]
fn test_reconciliation_runs() {
    let engine = create_engine();
    let swap = create_swap();

    engine.create_swap(&swap, "bcrt1qtest".to_string(), "44AFFqB".to_string()).unwrap();
    let result = engine.reconcile();
    assert!(result.is_ok(), "reconciliation should succeed");
}

#[test]
fn test_replay_protection() {
    let mut engine = create_engine();
    let swap = create_swap();

    let msg_id = opendex_types::MessageId(Uuid::new_v4());

    assert!(!engine.replay_protection(swap.id, msg_id));
    engine.mark_message_seen(swap.id, msg_id, 1);
    assert!(engine.replay_protection(swap.id, msg_id));
}

#[test]
fn test_state_machine_valid_transitions() {
    use opendex_swap::transition_state;

    assert!(transition_state(&SwapState::Created, SwapState::Negotiating).is_ok());
    assert!(transition_state(&SwapState::Negotiating, SwapState::ProtocolSetup).is_ok());
    assert!(transition_state(&SwapState::ProtocolSetup, SwapState::Funding).is_ok());
    assert!(transition_state(&SwapState::Funding, SwapState::Funded).is_ok());
    assert!(transition_state(&SwapState::Funded, SwapState::Redeeming).is_ok());
    assert!(transition_state(&SwapState::Redeeming, SwapState::Completed).is_ok());
}

#[test]
fn test_state_machine_invalid_transitions() {
    use opendex_swap::transition_state;

    assert!(transition_state(&SwapState::Created, SwapState::Completed).is_err());
    assert!(transition_state(&SwapState::Funded, SwapState::Created).is_err());
    assert!(transition_state(&SwapState::Completed, SwapState::Funding).is_err());
}

#[test]
fn test_swap_amounts_preserved() {
    let engine = create_engine();
    let swap = Swap::new(
        SwapId(Uuid::new_v4()),
        TradeId(Uuid::new_v4()),
        ParticipantId(Uuid::new_v4()),
        ParticipantId(Uuid::new_v4()),
        Satoshis(50_000_000),
        Piconero(50_000_000_000_000),
        ProtocolVersion::btc_xmr_v1(),
    );

    let state = engine
        .create_swap(&swap, "bcrt1qtest".to_string(), "44AFFqB".to_string())
        .unwrap();

    assert_eq!(state.context.btc_amount, Satoshis(50_000_000));
    assert_eq!(state.context.xmr_amount, Piconero(50_000_000_000_000));
}
