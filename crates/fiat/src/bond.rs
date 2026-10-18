//! Security bond system for fiat trades.
//!
//! Bonds incentivize good behavior by locking funds from both parties.
//! If a participant misbehaves, their bond can be partially or fully slashed.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use opendex_types::{Asset, ParticipantId, TradeId};

/// Unique identifier for a security bond.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BondId(pub Uuid);

/// State of a security bond.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum BondState {
    /// Bond is required but not yet locked.
    Required,
    /// Bond funds are locked.
    Locked,
    /// Bond released after successful trade.
    Released,
    /// Bond partially slashed due to minor violation.
    PartiallySlashed,
    /// Bond fully slashed due to serious violation.
    Slashed,
    /// Bond refunded after dispute resolution.
    Refunded,
}

impl std::fmt::Display for BondState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondState::Required => write!(f, "Required"),
            BondState::Locked => write!(f, "Locked"),
            BondState::Released => write!(f, "Released"),
            BondState::PartiallySlashed => write!(f, "Partially Slashed"),
            BondState::Slashed => write!(f, "Slashed"),
            BondState::Refunded => write!(f, "Refunded"),
        }
    }
}

/// Bond requirement for a trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondRequirement {
    /// Minimum bond amount in basis points of trade value.
    pub min_bps: u32,
    /// Maximum bond amount in basis points of trade value.
    pub max_bps: u32,
}

/// A security bond locked by a participant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityBond {
    /// Unique bond identifier.
    pub id: BondId,
    /// Trade this bond is for.
    pub trade_id: TradeId,
    /// Participant who locked the bond.
    pub participant: ParticipantId,
    /// Asset the bond is denominated in.
    pub asset: Asset,
    /// Bond amount (in satoshis, piconeros, or fiat minor units).
    pub amount: u64,
    /// Current state of the bond.
    pub state: BondState,
    /// Reason for any slash (if applicable).
    pub slash_reason: Option<String>,
    /// Rule violated (if slashed).
    pub rule_violated: Option<String>,
    /// Timestamp of last state change (Unix seconds).
    pub updated_at: u64,
}

impl SecurityBond {
    /// Create a new bond in the Required state.
    pub fn new(
        trade_id: TradeId,
        participant: ParticipantId,
        asset: Asset,
        amount: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            id: BondId(Uuid::new_v4()),
            trade_id,
            participant,
            asset,
            amount,
            state: BondState::Required,
            slash_reason: None,
            rule_violated: None,
            updated_at: timestamp,
        }
    }

    /// Lock the bond.
    pub fn lock(&mut self, timestamp: u64) -> anyhow::Result<()> {
        if !self.state.can_transition_to(&BondState::Locked) {
            return Err(anyhow::anyhow!("cannot lock bond in state: {}", self.state));
        }
        self.state = BondState::Locked;
        self.updated_at = timestamp;
        Ok(())
    }

    /// Release the bond after successful trade.
    pub fn release(&mut self, timestamp: u64) -> anyhow::Result<()> {
        if !self.state.can_transition_to(&BondState::Released) {
            return Err(anyhow::anyhow!(
                "cannot release bond in state: {}",
                self.state
            ));
        }
        self.state = BondState::Released;
        self.updated_at = timestamp;
        Ok(())
    }

    /// Slash the bond due to a violation.
    pub fn slash(
        &mut self,
        reason: impl Into<String>,
        rule: impl Into<String>,
        timestamp: u64,
    ) -> anyhow::Result<()> {
        if !self.state.can_transition_to(&BondState::Slashed) {
            return Err(anyhow::anyhow!(
                "cannot slash bond in state: {}",
                self.state
            ));
        }
        self.state = BondState::Slashed;
        self.slash_reason = Some(reason.into());
        self.rule_violated = Some(rule.into());
        self.updated_at = timestamp;
        Ok(())
    }
}

impl BondState {
    /// Validate a state transition.
    pub fn can_transition_to(&self, next: &BondState) -> bool {
        use BondState::*;
        matches!(
            (self, next),
            (Required, Locked)
                | (Required, Released)
                | (Locked, Released)
                | (Locked, PartiallySlashed)
                | (Locked, Slashed)
                | (PartiallySlashed, Released)
                | (PartiallySlashed, Slashed)
                | (Slashed, Refunded)
                | (Released, Refunded)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bond() -> SecurityBond {
        SecurityBond::new(
            TradeId(Uuid::new_v4()),
            ParticipantId(Uuid::new_v4()),
            Asset::Btc,
            100_000,
            1,
        )
    }

    #[test]
    fn bond_lifecycle() {
        let mut bond = make_bond();
        assert_eq!(bond.state, BondState::Required);

        bond.lock(2).unwrap();
        assert_eq!(bond.state, BondState::Locked);

        bond.release(3).unwrap();
        assert_eq!(bond.state, BondState::Released);
    }

    #[test]
    fn bond_slash() {
        let mut bond = make_bond();
        bond.lock(2).unwrap();
        bond.slash("Payment not received", "Rule 1.1", 3).unwrap();
        assert_eq!(bond.state, BondState::Slashed);
        assert_eq!(bond.slash_reason.as_deref(), Some("Payment not received"));
    }

    #[test]
    fn bond_invalid_transition() {
        let mut bond = make_bond();
        // Cannot slash without locking first
        assert!(bond.slash("reason", "rule", 2).is_err());
    }

    #[test]
    fn bond_release_from_required() {
        // Early cancellation allows releasing bonds before they are locked
        let mut bond = make_bond();
        assert_eq!(bond.state, BondState::Required);
        bond.release(2).unwrap();
        assert_eq!(bond.state, BondState::Released);
    }
}
