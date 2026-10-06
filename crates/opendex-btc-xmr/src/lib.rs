//! BTC/XMR atomic swap protocol adapter.
//!
//! This is the most security-sensitive OpenDex crate. It serves as the
//! compatibility boundary around the Eigenwallet BTC/XMR implementation.
//!
//! Responsibility:
//! - OpenDex types → Eigenwallet protocol → OpenDex protocol interface
//!
//! The marketplace must never import this crate directly. It goes through
//! the SettlementProtocol trait.

use serde::{Deserialize, Serialize};

use opendex_errors::ProtocolError;
use opendex_protocol_types::{
    ChainObservation, ProtocolAction, SettlementProtocol,
    SwapContext, SwapState,
};

// ─── BTC/XMR Swap State ─────────────────────────────────────────────────────

/// Protocol-specific state for a BTC/XMR swap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcXmrSwapState {
    pub swap_id: opendex_types::SwapId,
    pub state: SwapState,
    pub context: SwapContext,

    // Protocol-specific fields (populated during cryptographic setup)
    pub btc_funding_txid: Option<String>,
    pub xmr_funding_txid: Option<String>,
    pub btc_funding_confirmed: bool,
    pub xmr_funding_confirmed: bool,
    pub btc_redeemed: bool,
    pub xmr_redeemed: bool,
}

// ─── BTC/XMR Protocol Message ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BtcXmrMessage {
    /// Protocol initialization from maker.
    Init {
        swap_id: opendex_types::SwapId,
        protocol_version: opendex_types::ProtocolVersion,
    },
    /// Protocol response from taker.
    Response {
        swap_id: opendex_types::SwapId,
    },
    /// Cryptographic commitment/proof exchange.
    Proof {
        swap_id: opendex_types::SwapId,
        data: Vec<u8>,
    },
    /// Funding notification.
    Funded {
        swap_id: opendex_types::SwapId,
        chain: opendex_types::Asset,
        txid: String,
    },
}

// ─── BTC/XMR Adapter (Skeleton) ─────────────────────────────────────────────

/// Skeleton adapter for the Eigenwallet BTC/XMR protocol.
///
/// This is a placeholder implementation that demonstrates the interface.
/// The real implementation would integrate with the Eigenwallet library.
pub struct BtcXmrAdapter;

impl BtcXmrAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl SettlementProtocol for BtcXmrAdapter {
    type State = BtcXmrSwapState;
    type Message = BtcXmrMessage;
    type Error = ProtocolError;

    fn initialize(&self, context: SwapContext) -> Result<Self::State, Self::Error> {
        Ok(BtcXmrSwapState {
            swap_id: context.swap_id,
            state: SwapState::Created,
            context,
            btc_funding_txid: None,
            xmr_funding_txid: None,
            btc_funding_confirmed: false,
            xmr_funding_confirmed: false,
            btc_redeemed: false,
            xmr_redeemed: false,
        })
    }

    fn process_message(
        &self,
        state: &mut Self::State,
        message: Self::Message,
    ) -> Result<Vec<ProtocolAction>, Self::Error> {
        match message {
            BtcXmrMessage::Init { .. } => {
                if state.state == SwapState::Created {
                    state.state = SwapState::Negotiating;
                }
                Ok(vec![ProtocolAction::Wait(
                    opendex_protocol_types::WaitCondition::Timeout(600),
                )])
            }
            BtcXmrMessage::Response { .. } => {
                if state.state == SwapState::Negotiating {
                    state.state = SwapState::ProtocolSetup;
                }
                Ok(vec![])
            }
            BtcXmrMessage::Proof { .. } => {
                if state.state == SwapState::ProtocolSetup {
                    state.state = SwapState::Funding;
                }
                Ok(vec![ProtocolAction::Wait(
                    opendex_protocol_types::WaitCondition::BtcFunding(
                        opendex_types::Satoshis(0),
                    ),
                )])
            }
            BtcXmrMessage::Funded { chain, txid, .. } => {
                match chain {
                    opendex_types::Asset::Btc => {
                        state.btc_funding_txid = Some(txid);
                    }
                    opendex_types::Asset::Xmr => {
                        state.xmr_funding_txid = Some(txid);
                    }
                    _ => {}
                }
                Ok(vec![])
            }
        }
    }

    fn observe_chain(
        &self,
        state: &mut Self::State,
        observation: ChainObservation,
    ) -> Result<Vec<ProtocolAction>, Self::Error> {
        match observation {
            ChainObservation::BtcFundingConfirmed { .. } => {
                state.btc_funding_confirmed = true;
                if state.xmr_funding_confirmed {
                    state.state = SwapState::Funded;
                    return Ok(vec![ProtocolAction::BuildTransaction(
                        opendex_protocol_types::TransactionIntent {
                            swap_id: state.swap_id,
                            tx_type: opendex_protocol_types::TransactionType::XmrFunding,
                            chain: opendex_types::Asset::Xmr,
                        },
                    )]);
                }
            }
            ChainObservation::XmrFundingConfirmed { .. } => {
                state.xmr_funding_confirmed = true;
                if state.btc_funding_confirmed {
                    state.state = SwapState::Funded;
                    return Ok(vec![ProtocolAction::BuildTransaction(
                        opendex_protocol_types::TransactionIntent {
                            swap_id: state.swap_id,
                            tx_type: opendex_protocol_types::TransactionType::BtcRedemption,
                            chain: opendex_types::Asset::Btc,
                        },
                    )]);
                }
            }
            ChainObservation::BtcRedemptionSeen { .. } => {
                state.btc_redeemed = true;
                state.state = SwapState::Redeeming;
            }
            ChainObservation::XmrRedemptionSeen { .. } => {
                state.xmr_redeemed = true;
                if state.btc_redeemed {
                    state.state = SwapState::Completed;
                    return Ok(vec![ProtocolAction::Complete]);
                }
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn recover(&self, state: &Self::State) -> Result<Vec<ProtocolAction>, Self::Error> {
        use opendex_protocol_types::RecoveryAction;
        match &state.state {
            SwapState::Funding | SwapState::Negotiating | SwapState::ProtocolSetup => {
                Ok(vec![ProtocolAction::Recover(RecoveryAction::Abort)])
            }
            SwapState::Funded | SwapState::Redeeming => {
                Ok(vec![ProtocolAction::Recover(RecoveryAction::RefundBtc)])
            }
            _ => Ok(vec![]),
        }
    }
}
