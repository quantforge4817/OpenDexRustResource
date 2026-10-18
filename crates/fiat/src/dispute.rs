//! Dispute resolution system for fiat trades.
//!
//! When payment verification fails or a participant disputes a trade,
//! this module provides the structures for evidence collection and resolution.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use opendex_types::{ParticipantId, TradeId};

/// Unique identifier for a dispute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DisputeId(pub Uuid);

/// Reasons a dispute can be opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeReason {
    /// Taker claims payment was not received by maker.
    PaymentNotReceived,
    /// Payment amount does not match the trade amount.
    IncorrectAmount,
    /// Payment was reversed after confirmation.
    PaymentReversed,
    /// Wrong payment reference was used.
    WrongPaymentReference,
    /// Counterparty did not show up (in-person trades).
    CounterpartyNoShow,
    /// Payment provider failed.
    PaymentProviderFailure,
    /// Suspected fraudulent activity.
    SuspectedFraud,
    /// Other reason with a description.
    Other(String),
}

impl std::fmt::Display for DisputeReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisputeReason::PaymentNotReceived => write!(f, "Payment not received"),
            DisputeReason::IncorrectAmount => write!(f, "Incorrect amount"),
            DisputeReason::PaymentReversed => write!(f, "Payment reversed"),
            DisputeReason::WrongPaymentReference => write!(f, "Wrong payment reference"),
            DisputeReason::CounterpartyNoShow => write!(f, "Counterparty no-show"),
            DisputeReason::PaymentProviderFailure => write!(f, "Payment provider failure"),
            DisputeReason::SuspectedFraud => write!(f, "Suspected fraud"),
            DisputeReason::Other(desc) => write!(f, "Other: {}", desc),
        }
    }
}

/// State of a dispute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeState {
    /// Dispute just opened.
    Open,
    /// Collecting evidence from both parties.
    EvidenceCollection,
    /// Dispute under review by arbitrator.
    UnderReview,
    /// Dispute resolved.
    Resolved,
    /// Resolution appealed.
    Appealed,
    /// Dispute closed.
    Closed,
}

impl std::fmt::Display for DisputeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisputeState::Open => write!(f, "Open"),
            DisputeState::EvidenceCollection => write!(f, "Evidence Collection"),
            DisputeState::UnderReview => write!(f, "Under Review"),
            DisputeState::Resolved => write!(f, "Resolved"),
            DisputeState::Appealed => write!(f, "Appealed"),
            DisputeState::Closed => write!(f, "Closed"),
        }
    }
}

/// Type of evidence submitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    /// Bank statement or transaction receipt.
    BankStatement,
    /// Screenshot of payment confirmation.
    Screenshot,
    /// Communication between parties.
    ChatLog,
    /// Payment provider confirmation.
    ProviderConfirmation,
    /// Other evidence.
    Other(String),
}

/// Evidence submitted during a dispute.
///
/// Evidence is stored as references with content hashes, not as inline data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Unique evidence identifier.
    pub id: Uuid,
    /// Type of evidence.
    pub kind: EvidenceType,
    /// Hash of the evidence content for integrity.
    pub content_hash: String,
    /// Reference to where the evidence is stored.
    pub storage_reference: String,
    /// Participant who submitted this evidence.
    pub submitted_by: ParticipantId,
    /// Timestamp of submission (Unix seconds).
    pub timestamp: u64,
}

/// A dispute for a fiat trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispute {
    /// Unique dispute identifier.
    pub id: DisputeId,
    /// Trade this dispute is for.
    pub trade_id: TradeId,
    /// Participant who opened the dispute.
    pub opened_by: ParticipantId,
    /// Reason for the dispute.
    pub reason: DisputeReason,
    /// Evidence collected so far.
    pub evidence: Vec<Evidence>,
    /// Current state of the dispute.
    pub state: DisputeState,
    /// Resolution decision (if resolved).
    pub resolution: Option<String>,
    /// Created timestamp (Unix seconds).
    pub created_at: u64,
    /// Last updated timestamp (Unix seconds).
    pub updated_at: u64,
}

impl Dispute {
    /// Create a new dispute.
    pub fn new(
        trade_id: TradeId,
        opened_by: ParticipantId,
        reason: DisputeReason,
        timestamp: u64,
    ) -> Self {
        Self {
            id: DisputeId(Uuid::new_v4()),
            trade_id,
            opened_by,
            reason,
            evidence: Vec::new(),
            state: DisputeState::Open,
            resolution: None,
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    /// Add evidence to the dispute.
    pub fn add_evidence(&mut self, evidence: Evidence, timestamp: u64) {
        self.evidence.push(evidence);
        self.updated_at = timestamp;
    }

    /// Advance the dispute state.
    pub fn transition(&mut self, next: DisputeState, timestamp: u64) -> anyhow::Result<()> {
        if !self.state.can_transition_to(&next) {
            return Err(anyhow::anyhow!(
                "invalid dispute state transition: {} -> {}",
                self.state,
                next
            ));
        }
        self.state = next;
        self.updated_at = timestamp;
        Ok(())
    }

    /// Resolve the dispute with a decision.
    pub fn resolve(&mut self, resolution: impl Into<String>, timestamp: u64) -> anyhow::Result<()> {
        self.transition(DisputeState::Resolved, timestamp)?;
        self.resolution = Some(resolution.into());
        Ok(())
    }
}

impl DisputeState {
    /// Validate a state transition.
    pub fn can_transition_to(&self, next: &DisputeState) -> bool {
        use DisputeState::*;
        matches!(
            (self, next),
            (Open, EvidenceCollection)
                | (EvidenceCollection, UnderReview)
                | (UnderReview, Resolved)
                | (Resolved, Appealed)
                | (Appealed, UnderReview)
                | (Resolved, Closed)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dispute() -> Dispute {
        Dispute::new(
            TradeId(Uuid::new_v4()),
            ParticipantId(Uuid::new_v4()),
            DisputeReason::PaymentNotReceived,
            1,
        )
    }

    #[test]
    fn dispute_lifecycle() {
        let mut dispute = make_dispute();
        assert_eq!(dispute.state, DisputeState::Open);

        dispute
            .transition(DisputeState::EvidenceCollection, 2)
            .unwrap();
        assert_eq!(dispute.state, DisputeState::EvidenceCollection);

        dispute.transition(DisputeState::UnderReview, 3).unwrap();
        assert_eq!(dispute.state, DisputeState::UnderReview);

        dispute
            .resolve("Payment confirmed, trade completed", 4)
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Resolved);
    }

    #[test]
    fn add_evidence() {
        let mut dispute = make_dispute();
        let evidence = Evidence {
            id: Uuid::new_v4(),
            kind: EvidenceType::BankStatement,
            content_hash: "abc123".to_string(),
            storage_reference: "s3://evidence/abc123".to_string(),
            submitted_by: ParticipantId(Uuid::new_v4()),
            timestamp: 2,
        };
        dispute.add_evidence(evidence, 2);
        assert_eq!(dispute.evidence.len(), 1);
    }

    #[test]
    fn dispute_reason_display() {
        let reason = DisputeReason::PaymentNotReceived;
        assert_eq!(reason.to_string(), "Payment not received");
    }
}
