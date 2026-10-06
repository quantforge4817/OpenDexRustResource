//! Recovery decisions for interrupted swaps.

use serde::{Deserialize, Serialize};

use opendex_types::Asset;

/// Decision made by the recovery engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryDecision {
    /// No action needed; continue waiting.
    Wait,

    /// Redeem assets on the specified chain.
    Redeem { chain: Asset },

    /// Refund assets on the specified chain.
    Refund { chain: Asset },

    /// Punish the counterparty (take collateral).
    Punish,

    /// Abort the swap entirely.
    Abort,
}

impl std::fmt::Display for RecoveryDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryDecision::Wait => write!(f, "Wait"),
            RecoveryDecision::Redeem { chain } => write!(f, "Redeem {}", chain),
            RecoveryDecision::Refund { chain } => write!(f, "Refund {}", chain),
            RecoveryDecision::Punish => write!(f, "Punish"),
            RecoveryDecision::Abort => write!(f, "Abort"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_decision_display() {
        assert_eq!(format!("{}", RecoveryDecision::Wait), "Wait");
        assert_eq!(
            format!("{}", RecoveryDecision::Redeem { chain: Asset::Btc }),
            "Redeem BTC"
        );
        assert_eq!(
            format!("{}", RecoveryDecision::Refund { chain: Asset::Xmr }),
            "Refund XMR"
        );
        assert_eq!(format!("{}", RecoveryDecision::Punish), "Punish");
        assert_eq!(format!("{}", RecoveryDecision::Abort), "Abort");
    }

    #[test]
    fn test_recovery_decision_equality() {
        let d1 = RecoveryDecision::Redeem { chain: Asset::Btc };
        let d2 = RecoveryDecision::Redeem { chain: Asset::Btc };
        let d3 = RecoveryDecision::Redeem { chain: Asset::Xmr };
        assert_eq!(d1, d2);
        assert_ne!(d1, d3);
    }

    #[test]
    fn test_recovery_decision_serialization() {
        let decision = RecoveryDecision::Refund { chain: Asset::Btc };
        let serialized = serde_json::to_string(&decision).unwrap();
        let deserialized: RecoveryDecision = serde_json::from_str(&serialized).unwrap();
        assert_eq!(decision, deserialized);
    }
}
