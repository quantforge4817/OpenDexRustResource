//! Generic settlement protocol abstractions.
//!
//! This crate defines the `SettlementProtocol` trait — the most important
//! interface in the entire OpenDex project. Everything above it is
//! marketplace-neutral; everything below it is settlement-specific.
//!
//! The marketplace should not know whether settlement uses atomic swap,
//! HTLC, adaptor signature, multisig, or any future protocol. It only
//! sees `SettlementProtocol`.

use serde::{Deserialize, Serialize};

use opendex_errors::ProtocolError;
use opendex_types::{
    Asset, BitcoinNetwork, MoneroNetwork, ParticipantId, Piconero, ProtocolVersion, Satoshis,
    SwapId, TradeId,
};

// ─── Swap Context ───────────────────────────────────────────────────────────

/// Immutable context for a swap, established before protocol initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapContext {
    pub swap_id: SwapId,
    pub trade_id: TradeId,

    pub btc_amount: Satoshis,
    pub xmr_amount: Piconero,

    pub bitcoin_network: BitcoinNetwork,
    pub monero_network: MoneroNetwork,

    pub maker: ParticipantId,
    pub taker: ParticipantId,

    pub btc_destination: String,
    pub xmr_destination: String,

    pub protocol_version: ProtocolVersion,
}

// ─── Protocol Actions ───────────────────────────────────────────────────────

/// Actions the protocol may request after processing a message or observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolAction {
    /// Send a message to the counterparty.
    SendMessage(ProtocolMessage),

    /// Build and broadcast a transaction.
    BuildTransaction(TransactionIntent),

    /// Wait for a specific condition.
    Wait(WaitCondition),

    /// The swap is complete.
    Complete,

    /// Begin recovery procedure.
    Recover(RecoveryAction),

    /// The swap has failed.
    Fail(ProtocolFailure),
}

/// A message to be sent to the counterparty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub swap_id: SwapId,
    pub payload: Vec<u8>,
    /// Unique message identifier for replay protection.
    pub message_id: opendex_types::MessageId,
    /// The sender of this message.
    pub sender: opendex_types::ParticipantId,
    /// Monotonically increasing sequence number for ordering.
    pub sequence: u64,
}

impl ProtocolMessage {
    pub fn new(
        swap_id: SwapId,
        payload: Vec<u8>,
        message_id: opendex_types::MessageId,
        sender: opendex_types::ParticipantId,
        sequence: u64,
    ) -> Self {
        Self {
            swap_id,
            payload,
            message_id,
            sender,
            sequence,
        }
    }
}

/// Intent to build and broadcast a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionIntent {
    pub swap_id: SwapId,
    pub tx_type: TransactionType,
    pub chain: Asset,
    /// Transaction ID after broadcast; empty string if not yet broadcast.
    pub txid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    BtcFunding,
    XmrFunding,
    BtcRedemption,
    XmrRedemption,
    BtcRefund,
    XmrRefund,
}

/// Condition the protocol is waiting for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaitCondition {
    BtcFunding(Satoshis),
    XmrFunding(Piconero),
    BtcRedemption,
    XmrRedemption,
    Timeout(u64),
}

/// Recovery action requested by the protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAction {
    RefundBtc,
    RefundXmr,
    RedeemBtc,
    RedeemXmr,
    Abort,
}

/// Reason for protocol failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolFailure {
    pub reason: String,
}

// ─── Chain Observations ─────────────────────────────────────────────────────

/// An observation from a blockchain watcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainObservation {
    BtcFundingSeen {
        swap_id: SwapId,
        txid: String,
        amount: Satoshis,
        confirmations: u32,
    },
    BtcFundingConfirmed {
        swap_id: SwapId,
        txid: String,
        confirmations: u32,
    },
    BtcRedemptionSeen {
        swap_id: SwapId,
        txid: String,
    },
    XmrFundingSeen {
        swap_id: SwapId,
        txid: String,
        amount: Piconero,
        confirmations: u32,
    },
    XmrFundingConfirmed {
        swap_id: SwapId,
        txid: String,
        confirmations: u32,
    },
    XmrRedemptionSeen {
        swap_id: SwapId,
        txid: String,
    },
    BtcRefundSeen {
        swap_id: SwapId,
        txid: String,
    },
    XmrRefundSeen {
        swap_id: SwapId,
        txid: String,
    },
    Reorg {
        chain: Asset,
        height: u64,
    },
}

// ─── Settlement Protocol Trait ──────────────────────────────────────────────

/// The central settlement interface.
///
/// The marketplace calls this trait without knowing whether the underlying
/// implementation uses atomic swap, HTLC, adaptor signatures, or any future
/// protocol.
pub trait SettlementProtocol {
    type State: Serialize + for<'de> Deserialize<'de> + Send + Sync;
    type Message: Serialize + for<'de> Deserialize<'de> + Send + Sync;
    type Error: From<ProtocolError> + std::error::Error + Send + Sync;

    /// Initialize the protocol for a new swap.
    fn initialize(&self, context: SwapContext) -> Result<Self::State, Self::Error>;

    /// Process a message from the counterparty.
    fn process_message(
        &self,
        state: &mut Self::State,
        message: Self::Message,
    ) -> Result<Vec<ProtocolAction>, Self::Error>;

    /// Process a blockchain observation.
    fn observe_chain(
        &self,
        state: &mut Self::State,
        observation: ChainObservation,
    ) -> Result<Vec<ProtocolAction>, Self::Error>;

    /// Attempt recovery from an interrupted swap.
    fn recover(&self, state: &Self::State) -> Result<Vec<ProtocolAction>, Self::Error>;
}

// ─── Swap State ─────────────────────────────────────────────────────────────

// ─── Swap Record ────────────────────────────────────────────────────────────

/// Generic swap record managed by the swap engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Swap {
    pub id: SwapId,
    pub trade_id: TradeId,

    pub maker: ParticipantId,
    pub taker: ParticipantId,

    pub sell_asset: Asset,
    pub buy_asset: Asset,

    pub btc_amount: Satoshis,
    pub xmr_amount: Piconero,

    pub protocol: ProtocolVersion,

    pub state: SwapState,
}

impl Swap {
    pub fn new(
        id: SwapId,
        trade_id: TradeId,
        maker: ParticipantId,
        taker: ParticipantId,
        btc_amount: Satoshis,
        xmr_amount: Piconero,
        protocol: ProtocolVersion,
    ) -> Self {
        Self {
            id,
            trade_id,
            maker,
            taker,
            sell_asset: Asset::Btc,
            buy_asset: Asset::Xmr,
            btc_amount,
            xmr_amount,
            protocol,
            state: SwapState::Created,
        }
    }
}

// ─── Swap State ─────────────────────────────────────────────────────────────

/// Generic swap state machine states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapState {
    Created,
    Negotiating,
    ProtocolSetup,
    Funding,
    Funded,
    Redeeming,
    Completed,
    Recovering,
    Refunded,
    Failed,
    WaitingForPeer,
}

impl SwapState {
    /// Validate a state transition.
    pub fn can_transition_to(&self, next: &SwapState) -> bool {
        use SwapState::*;
        matches!(
            (self, next),
            // Normal flow
            (Created, Negotiating)
            | (Negotiating, ProtocolSetup)
            | (ProtocolSetup, Funding)
            | (Funding, Funded)
            | (Funded, Redeeming)
            | (Redeeming, Completed)
            // Recovery paths
            | (Created, Recovering)
            | (Negotiating, Recovering)
            | (ProtocolSetup, Recovering)
            | (Funding, Recovering)
            | (Funded, Recovering)
            | (Redeeming, Recovering)
            | (Recovering, Refunded)
            | (Recovering, Failed)
            // Failure paths
            | (Created, Failed)
            | (Negotiating, Failed)
            | (ProtocolSetup, Failed)
            | (Funding, Failed)
            | (Funded, Failed)
            | (Redeeming, Failed)
            // Peer disconnection - can go to WaitingForPeer from active states
            | (Funding, WaitingForPeer)
            | (Funded, WaitingForPeer)
            | (Redeeming, WaitingForPeer)
            | (Negotiating, WaitingForPeer)
            | (ProtocolSetup, WaitingForPeer)
            // Peer reconnection - resume from WaitingForPeer
            | (WaitingForPeer, Funding)
            | (WaitingForPeer, Funded)
            | (WaitingForPeer, Redeeming)
            | (WaitingForPeer, Negotiating)
            | (WaitingForPeer, ProtocolSetup)
            | (WaitingForPeer, Failed)
        )
    }
}
