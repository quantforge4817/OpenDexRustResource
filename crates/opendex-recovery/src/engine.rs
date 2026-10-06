//! Deterministic recovery engine.

use opendex_errors::OpenDexError;
use opendex_protocol_types::{ChainObservation, SwapState};

use crate::decision::RecoveryDecision;

/// Evaluates swap state + chain state and returns a recovery decision.
pub trait RecoveryEngine: Send + Sync {
    fn evaluate(
        &self,
        state: &SwapState,
        observations: &[ChainObservation],
    ) -> Result<RecoveryDecision, OpenDexError>;
}

/// Default deterministic recovery engine.
///
/// Same inputs always produce the same decision.
pub struct DeterministicRecoveryEngine;

impl DeterministicRecoveryEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeterministicRecoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RecoveryEngine for DeterministicRecoveryEngine {
    fn evaluate(
        &self,
        state: &SwapState,
        observations: &[ChainObservation],
    ) -> Result<RecoveryDecision, OpenDexError> {
        // Check for refund transactions
        for obs in observations {
            match obs {
                ChainObservation::BtcRefundSeen { .. } => {
                    return Ok(RecoveryDecision::Refund {
                        chain: opendex_types::Asset::Btc,
                    });
                }
                ChainObservation::XmrRefundSeen { .. } => {
                    return Ok(RecoveryDecision::Refund {
                        chain: opendex_types::Asset::Xmr,
                    });
                }
                ChainObservation::BtcRedemptionSeen { .. } => {
                    return Ok(RecoveryDecision::Redeem {
                        chain: opendex_types::Asset::Btc,
                    });
                }
                ChainObservation::XmrRedemptionSeen { .. } => {
                    return Ok(RecoveryDecision::Redeem {
                        chain: opendex_types::Asset::Xmr,
                    });
                }
                _ => {}
            }
        }

        // Check funding confirmations
        let btc_funded = observations.iter().any(|obs| {
            matches!(
                obs,
                ChainObservation::BtcFundingConfirmed { confirmations: c, .. } if *c >= 6
            )
        });
        let xmr_funded = observations.iter().any(|obs| {
            matches!(
                obs,
                ChainObservation::XmrFundingConfirmed { confirmations: c, .. } if *c >= 10
            )
        });

        match state {
            SwapState::Funding => {
                if btc_funded && xmr_funded {
                    Ok(RecoveryDecision::Wait)
                } else {
                    Ok(RecoveryDecision::Wait)
                }
            }
            SwapState::Funded => {
                if btc_funded && xmr_funded {
                    Ok(RecoveryDecision::Redeem {
                        chain: opendex_types::Asset::Btc,
                    })
                } else {
                    Ok(RecoveryDecision::Wait)
                }
            }
            SwapState::Redeeming => Ok(RecoveryDecision::Wait),
            SwapState::Recovering => Ok(RecoveryDecision::Wait),
            SwapState::Completed | SwapState::Refunded | SwapState::Failed => {
                Ok(RecoveryDecision::Abort)
            }
            _ => Ok(RecoveryDecision::Wait),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendex_types::{Piconero, Satoshis, SwapId};

    fn test_swap_id() -> SwapId {
        SwapId(uuid::Uuid::new_v4())
    }

    #[test]
    fn test_deterministic_wait_on_created() {
        let engine = DeterministicRecoveryEngine::new();
        let decision = engine
            .evaluate(&SwapState::Created, &[])
            .unwrap();
        assert_eq!(decision, RecoveryDecision::Wait);
    }

    #[test]
    fn test_deterministic_wait_on_funding() {
        let engine = DeterministicRecoveryEngine::new();
        let decision = engine
            .evaluate(&SwapState::Funding, &[])
            .unwrap();
        assert_eq!(decision, RecoveryDecision::Wait);
    }

    #[test]
    fn test_deterministic_redeem_on_funded() {
        let engine = DeterministicRecoveryEngine::new();
        let swap_id = test_swap_id();
        let observations = vec![
            ChainObservation::BtcFundingConfirmed {
                swap_id,
                txid: "abc123".to_string(),
                confirmations: 6,
            },
            ChainObservation::XmrFundingConfirmed {
                swap_id,
                txid: "def456".to_string(),
                confirmations: 10,
            },
        ];
        let decision = engine.evaluate(&SwapState::Funded, &observations).unwrap();
        assert_eq!(
            decision,
            RecoveryDecision::Redeem {
                chain: opendex_types::Asset::Btc
            }
        );
    }

    #[test]
    fn test_deterministic_refund_on_btc_refund_seen() {
        let engine = DeterministicRecoveryEngine::new();
        let swap_id = test_swap_id();
        let observations = vec![ChainObservation::BtcRefundSeen {
            swap_id,
            txid: "refund123".to_string(),
        }];
        let decision = engine.evaluate(&SwapState::Funded, &observations).unwrap();
        assert_eq!(
            decision,
            RecoveryDecision::Refund {
                chain: opendex_types::Asset::Btc
            }
        );
    }

    #[test]
    fn test_deterministic_refund_on_xmr_refund_seen() {
        let engine = DeterministicRecoveryEngine::new();
        let swap_id = test_swap_id();
        let observations = vec![ChainObservation::XmrRefundSeen {
            swap_id,
            txid: "refund456".to_string(),
        }];
        let decision = engine.evaluate(&SwapState::Funded, &observations).unwrap();
        assert_eq!(
            decision,
            RecoveryDecision::Refund {
                chain: opendex_types::Asset::Xmr
            }
        );
    }

    #[test]
    fn test_deterministic_redeem_on_btc_redemption_seen() {
        let engine = DeterministicRecoveryEngine::new();
        let swap_id = test_swap_id();
        let observations = vec![ChainObservation::BtcRedemptionSeen {
            swap_id,
            txid: "redeem123".to_string(),
        }];
        let decision = engine.evaluate(&SwapState::Funded, &observations).unwrap();
        assert_eq!(
            decision,
            RecoveryDecision::Redeem {
                chain: opendex_types::Asset::Btc
            }
        );
    }

    #[test]
    fn test_deterministic_abort_on_completed() {
        let engine = DeterministicRecoveryEngine::new();
        let decision = engine
            .evaluate(&SwapState::Completed, &[])
            .unwrap();
        assert_eq!(decision, RecoveryDecision::Abort);
    }

    #[test]
    fn test_deterministic_same_inputs_same_output() {
        let engine = DeterministicRecoveryEngine::new();
        let swap_id = test_swap_id();
        let observations = vec![
            ChainObservation::BtcFundingSeen {
                swap_id,
                txid: "abc123".to_string(),
                amount: Satoshis(100_000),
                confirmations: 1,
            },
            ChainObservation::XmrFundingSeen {
                swap_id,
                txid: "def456".to_string(),
                amount: Piconero(2_000_000_000_000),
                confirmations: 1,
            },
        ];

        let d1 = engine.evaluate(&SwapState::Funding, &observations).unwrap();
        let d2 = engine.evaluate(&SwapState::Funding, &observations).unwrap();
        assert_eq!(d1, d2);
    }
}
