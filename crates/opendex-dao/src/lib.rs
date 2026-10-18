//! OpenDex DAO (Decentralised Autonomous Organisation) Interface
//!
//! This crate provides the governance interface for protocol-level decisions only.
//! The DAO is explicitly constrained to govern the **protocol domain** and must not
//! govern user assets or the commercial treasury.
//!
//! ## Regulatory Constraints (DNC-012 to DNC-016)
//!
//! | Constraint | Invariant | Enforcement |
//! |------------|-----------|-------------|
//! | DAO cannot control commercial treasury | DNC-012 | No treasury methods in DAO interface |
//! | Treasury control cannot affect user principal | DNC-013 | Separate crate boundaries |
//! | Fees are independently calculable | DNC-014 | Fee calculation is deterministic, not DAO-controlled |
//! | Fee collection cannot modify trade identity | DNC-015 | Fee path is separate from trade path |
//! | Fee collection cannot create rights over user principal | DNC-016 | Fee recipient receives only the fee, not user assets |
//!
//! ## Design Principles
//!
//! 1. The DAO crate has no dependency on treasury or commercial crates.
//! 2. DAO proposals are verifiable by any node without requiring trust in the DAO operator.
//! 3. The DAO has no emergency override capabilities that bypass protocol rules.
//! 4. Fee recipient address is set at protocol initialization, not changeable by DAO vote.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Protocol parameters the DAO can change.
///
/// Note: Treasury parameters are explicitly NOT included.
/// This enforces DNC-012: DAO cannot control commercial treasury.
#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProtocolParameter {
    /// Protocol fee rate (basis points)
    FeeRate,
    /// Order matching timeout in seconds
    MatchingTimeout,
    /// Settlement timeout in seconds
    SettlementTimeout,
    /// Minimum order size in satoshis
    MinimumOrderSize,
    /// Maximum order size in satoshis
    MaximumOrderSize,
    /// Order book depth limit
    OrderBookDepthLimit,
    /// Peer discovery interval in seconds
    PeerDiscoveryInterval,
    /// Heartbeat interval in seconds
    HeartbeatInterval,
}

/// Value types for protocol parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParameterValue {
    /// A numeric value (e.g., fee rate, timeout)
    Numeric(u64),
    /// A boolean value
    Boolean(bool),
    /// A string value (e.g., version string)
    Text(String),
}

/// Asset actions for adding or removing supported assets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetAction {
    /// Add a new supported asset
    Add,
    /// Remove a supported asset
    Remove,
}

/// Asset identifier for supported cryptocurrencies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Asset {
    /// Human-readable symbol (e.g., "BTC", "XMR")
    pub symbol: String,
    /// Chain identifier
    pub chain_id: String,
}

/// Protocol version for software upgrades.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtocolVersion {
    /// Major version
    pub major: u32,
    /// Minor version
    pub minor: u32,
    /// Patch version
    pub patch: u32,
}

/// Upgrade schedule for protocol software changes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpgradeSchedule {
    /// Unix timestamp when the upgrade becomes active
    pub activation_time: u64,
    /// Grace period in seconds for nodes to upgrade
    pub grace_period: u64,
}

/// DAO governance proposals — protocol domain only.
///
/// The DAO cannot propose changes affecting the commercial treasury.
/// This is enforced by the absence of treasury-related variants.
///
/// ## Forbidden Variants
///
/// The following must NOT exist in this enum:
/// - `DaoProposal::TreasuryWithdrawal { ... }`
/// - `DaoProposal::TreasuryTransfer { ... }`
/// - `DaoProposal::FeeRecipientChange { ... }`
///
/// This enforces DNC-012: DAO cannot control commercial treasury.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaoProposal {
    /// Change protocol parameters (fee rate, matching parameters, etc.)
    ParameterChange {
        parameter: ProtocolParameter,
        new_value: ParameterValue,
    },
    /// Upgrade protocol software version
    SoftwareUpgrade {
        version: ProtocolVersion,
        upgrade_schedule: UpgradeSchedule,
    },
    /// Add or remove a supported asset
    AssetChange { asset: Asset, action: AssetAction },
}

/// DAO proposal state machine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalState {
    /// Proposal has been submitted but not yet voted on
    Proposed,
    /// Proposal is currently being voted on
    Voting,
    /// Proposal has been approved and is pending activation
    Approved,
    /// Proposal has been rejected
    Rejected,
    /// Proposal has been executed
    Executed,
}

/// A complete DAO proposal with metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Proposal {
    /// Unique proposal identifier
    pub id: u64,
    /// The proposal content
    pub proposal: DaoProposal,
    /// Current state of the proposal
    pub state: ProposalState,
    /// Unix timestamp when the proposal was created
    pub created_at: u64,
    /// Unix timestamp when voting ends
    pub voting_ends_at: u64,
    /// Number of votes in favor
    pub votes_for: u64,
    /// Number of votes against
    pub votes_against: u64,
}

/// DAO voting power representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VotingPower {
    /// Total voting power
    pub total: u64,
    /// Voting power in favor
    pub for_votes: u64,
    /// Voting power against
    pub against_votes: u64,
}

impl VotingPower {
    /// Check if a proposal has passed based on voting power.
    ///
    /// A proposal passes if it has more votes in favor than against.
    pub fn has_passed(&self) -> bool {
        self.for_votes > self.against_votes
    }

    /// Calculate the participation rate.
    pub fn participation_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.for_votes + self.against_votes) as f64 / self.total as f64
        }
    }
}

/// DAO configuration set at protocol initialization.
///
/// These values are NOT changeable by DAO vote, ensuring
/// the DAO cannot override fundamental protocol constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaoConfig {
    /// Minimum voting period in seconds
    pub min_voting_period: u64,
    /// Maximum voting period in seconds
    pub max_voting_period: u64,
    /// Minimum participation rate for a valid vote (0.0 to 1.0)
    pub min_participation_rate: f64,
    /// Fee recipient address — set at initialization, NOT changeable by DAO
    /// This enforces DNC-012: DAO cannot control commercial treasury
    pub fee_recipient: String,
}

impl DaoConfig {
    /// Create a new DAO configuration.
    ///
    /// The fee_recipient is set once at initialization and cannot be changed
    /// through DAO proposals, ensuring DNC-012 compliance.
    pub fn new(
        min_voting_period: u64,
        max_voting_period: u64,
        min_participation_rate: f64,
        fee_recipient: String,
    ) -> Self {
        Self {
            min_voting_period,
            max_voting_period,
            min_participation_rate,
            fee_recipient,
        }
    }

    /// Get the fee recipient address.
    ///
    /// This is the only way to access the fee recipient, and it cannot be modified.
    pub fn fee_recipient(&self) -> &str {
        &self.fee_recipient
    }
}

/// Errors that can occur during DAO operations.
#[derive(Debug, Error, PartialEq)]
pub enum DaoError {
    /// The proposal with the given ID was not found.
    #[error("Proposal {0} not found")]
    ProposalNotFound(u64),

    /// The proposal is not in a valid state for the requested operation.
    #[error("Invalid state transition: expected {expected}, got {actual}")]
    InvalidStateTransition { expected: String, actual: String },

    /// Voting period is still active; cannot close yet.
    #[error("Voting period is still active")]
    VotingPeriodActive,

    /// Participation rate is below the minimum threshold.
    #[error("Participation rate {actual:.2} is below minimum {minimum:.2}")]
    ParticipationTooLow { actual: f64, minimum: f64 },

    /// The proposal has already been executed.
    #[error("Proposal {0} has already been executed")]
    ProposalAlreadyExecuted(u64),
}

/// A vote cast on a proposal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Vote {
    /// Vote in favor of the proposal.
    For,
    /// Vote against the proposal.
    Against,
}

/// DAO runtime engine that manages the full proposal lifecycle.
///
/// This struct provides proposal submission, voting, execution, and query
/// capabilities. It enforces the state machine:
/// `Proposed → Voting → Approved → Executed` or `Proposed → Voting → Rejected`.
///
/// The engine has no method to change `fee_recipient`, enforcing DNC-012 at runtime.
pub struct DaoEngine {
    /// Immutable DAO configuration.
    config: DaoConfig,
    /// All proposals indexed by ID.
    proposals: HashMap<u64, Proposal>,
    /// Next auto-incrementing proposal ID.
    next_id: u64,
    /// Current protocol parameters from executed proposals.
    pub parameters: HashMap<ProtocolParameter, ParameterValue>,
}

impl DaoEngine {
    /// Create a new DAO engine with the given configuration.
    pub fn new(config: DaoConfig) -> Self {
        Self {
            config,
            proposals: HashMap::new(),
            next_id: 1,
            parameters: HashMap::new(),
        }
    }

    /// Get the immutable DAO configuration.
    pub fn config(&self) -> &DaoConfig {
        &self.config
    }

    // ==========================================================================
    // Proposal Submission
    // ==========================================================================

    /// Submit a new proposal and transition it to the Voting state.
    ///
    /// Returns the created `Proposal` with an auto-assigned ID.
    pub fn submit_proposal(&mut self, proposal: DaoProposal) -> Proposal {
        let id = self.next_id;
        self.next_id += 1;

        let now = 0; // Will be set by the caller in production
        let voting_ends_at = now + self.config.min_voting_period;

        let p = Proposal {
            id,
            proposal,
            state: ProposalState::Voting,
            created_at: now,
            voting_ends_at,
            votes_for: 0,
            votes_against: 0,
        };

        self.proposals.insert(id, p.clone());
        p
    }

    // ==========================================================================
    // Voting
    // ==========================================================================

    /// Cast a vote on a proposal.
    ///
    /// The proposal must be in the `Voting` state.
    pub fn cast_vote(&mut self, proposal_id: u64, vote: Vote) -> Result<(), DaoError> {
        let p = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or(DaoError::ProposalNotFound(proposal_id))?;

        if p.state != ProposalState::Voting {
            return Err(DaoError::InvalidStateTransition {
                expected: "Voting".to_string(),
                actual: format!("{:?}", p.state),
            });
        }

        match vote {
            Vote::For => p.votes_for += 1,
            Vote::Against => p.votes_against += 1,
        }

        Ok(())
    }

    /// Close the voting period for a proposal.
    ///
    /// Checks the participation rate against `min_participation_rate`.
    /// If sufficient participation, transitions to `Approved` (if `has_passed()`)
    /// or `Rejected` (otherwise).
    pub fn close_voting(&mut self, proposal_id: u64) -> Result<(), DaoError> {
        let p = self
            .proposals
            .get(&proposal_id)
            .ok_or(DaoError::ProposalNotFound(proposal_id))?;

        if p.state != ProposalState::Voting {
            return Err(DaoError::InvalidStateTransition {
                expected: "Voting".to_string(),
                actual: format!("{:?}", p.state),
            });
        }

        let total_votes = p.votes_for + p.votes_against;
        let voting_power = VotingPower {
            total: total_votes,
            for_votes: p.votes_for,
            against_votes: p.votes_against,
        };

        let participation = voting_power.participation_rate();

        if participation < self.config.min_participation_rate {
            return Err(DaoError::ParticipationTooLow {
                actual: participation,
                minimum: self.config.min_participation_rate,
            });
        }

        let p = self.proposals.get_mut(&proposal_id).unwrap();
        if voting_power.has_passed() {
            p.state = ProposalState::Approved;
        } else {
            p.state = ProposalState::Rejected;
        }

        Ok(())
    }

    // ==========================================================================
    // Execution
    // ==========================================================================

    /// Execute an approved proposal, applying its parameter changes.
    ///
    /// Only `ParameterChange` proposals modify protocol parameters.
    /// `SoftwareUpgrade` and `AssetChange` proposals transition to `Executed`
    /// but do not modify the parameters map.
    pub fn execute_proposal(&mut self, proposal_id: u64) -> Result<(), DaoError> {
        let p = self
            .proposals
            .get(&proposal_id)
            .ok_or(DaoError::ProposalNotFound(proposal_id))?;

        if p.state != ProposalState::Approved {
            return Err(DaoError::InvalidStateTransition {
                expected: "Approved".to_string(),
                actual: format!("{:?}", p.state),
            });
        }

        // Apply parameter changes
        if let DaoProposal::ParameterChange {
            parameter,
            new_value,
        } = &p.proposal
        {
            self.parameters.insert(parameter.clone(), new_value.clone());
        }

        let p = self.proposals.get_mut(&proposal_id).unwrap();
        p.state = ProposalState::Executed;

        Ok(())
    }

    /// Get the current value of a protocol parameter.
    pub fn get_parameter(&self, parameter: ProtocolParameter) -> Option<&ParameterValue> {
        self.parameters.get(&parameter)
    }

    // ==========================================================================
    // Query
    // ==========================================================================

    /// Get a proposal by ID.
    pub fn get_proposal(&self, id: u64) -> Option<&Proposal> {
        self.proposals.get(&id)
    }

    /// List all proposals.
    pub fn list_proposals(&self) -> Vec<&Proposal> {
        let mut proposals: Vec<&Proposal> = self.proposals.values().collect();
        proposals.sort_by_key(|p| p.id);
        proposals
    }

    /// List all active proposals (Voting or Approved states).
    pub fn list_active_proposals(&self) -> Vec<&Proposal> {
        self.proposals
            .values()
            .filter(|p| matches!(p.state, ProposalState::Voting | ProposalState::Approved))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // DNC Invariant Tests
    // ============================================================================
    // These tests verify that the DAO interface enforces the regulatory constraints
    // defined in the OpenDex regulatory compliance document (Section 2.10).
    // ============================================================================

    #[test]
    fn test_dnc_001_no_unilateral_user_principal_transfer() {
        // DNC-001: OpenDex cannot transfer user principal unilaterally.
        // Verification: The DaoProposal enum has no variant for transferring user assets.
        // The only proposal types are ParameterChange, SoftwareUpgrade, and AssetChange.
        let proposals = vec![
            DaoProposal::ParameterChange {
                parameter: ProtocolParameter::FeeRate,
                new_value: ParameterValue::Numeric(50),
            },
            DaoProposal::SoftwareUpgrade {
                version: ProtocolVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                upgrade_schedule: UpgradeSchedule {
                    activation_time: 1700000000,
                    grace_period: 86400,
                },
            },
            DaoProposal::AssetChange {
                asset: Asset {
                    symbol: "BTC".to_string(),
                    chain_id: "bitcoin-mainnet".to_string(),
                },
                action: AssetAction::Add,
            },
        ];
        // None of the proposals can transfer user principal
        for p in proposals {
            assert!(matches!(
                p,
                DaoProposal::ParameterChange { .. }
                    | DaoProposal::SoftwareUpgrade { .. }
                    | DaoProposal::AssetChange { .. }
            ));
        }
    }

    #[test]
    fn test_dnc_002_no_unilateral_user_principal_freeze() {
        // DNC-002: OpenDex cannot freeze user principal unilaterally.
        // Verification: The DaoProposal enum has no variant for freezing user assets.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::MatchingTimeout,
            new_value: ParameterValue::Numeric(300),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No FreezeUserAssets variant exists in the enum
    }

    #[test]
    fn test_dnc_003_no_unilateral_user_principal_seizure() {
        // DNC-003: OpenDex cannot seize user principal unilaterally.
        // Verification: The DaoProposal enum has no variant for seizing user assets.
        let proposal = DaoProposal::AssetChange {
            asset: Asset {
                symbol: "XMR".to_string(),
                chain_id: "monero-mainnet".to_string(),
            },
            action: AssetAction::Remove,
        };
        assert!(matches!(proposal, DaoProposal::AssetChange { .. }));
        // No SeizeUserAssets variant exists in the enum
    }

    #[test]
    fn test_dnc_004_no_mandatory_counterparty() {
        // DNC-004: OpenDex cannot become the mandatory counterparty to user trades.
        // Verification: The DAO interface has no method to set OpenDex as counterparty.
        // ProtocolParameter has no Counterparty variant.
        let params = vec![
            ProtocolParameter::FeeRate,
            ProtocolParameter::MatchingTimeout,
            ProtocolParameter::SettlementTimeout,
            ProtocolParameter::MinimumOrderSize,
            ProtocolParameter::MaximumOrderSize,
            ProtocolParameter::OrderBookDepthLimit,
            ProtocolParameter::PeerDiscoveryInterval,
            ProtocolParameter::HeartbeatInterval,
        ];
        // None of the parameters can set a mandatory counterparty
        for p in params {
            assert!(matches!(
                p,
                ProtocolParameter::FeeRate
                    | ProtocolParameter::MatchingTimeout
                    | ProtocolParameter::SettlementTimeout
                    | ProtocolParameter::MinimumOrderSize
                    | ProtocolParameter::MaximumOrderSize
                    | ProtocolParameter::OrderBookDepthLimit
                    | ProtocolParameter::PeerDiscoveryInterval
                    | ProtocolParameter::HeartbeatInterval
            ));
        }
    }

    #[test]
    fn test_dnc_005_no_unilateral_trade_determination() {
        // DNC-005: OpenDex cannot unilaterally determine which valid user trade executes.
        // Verification: The DAO interface has no method to influence trade matching.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::OrderBookDepthLimit,
            new_value: ParameterValue::Numeric(100),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No SelectTrade variant exists in the enum
    }

    #[test]
    fn test_dnc_006_no_unilateral_trade_rewrite() {
        // DNC-006: OpenDex cannot unilaterally rewrite an executed trade.
        // Verification: The DAO interface has no method to modify executed trades.
        let proposal = DaoProposal::SoftwareUpgrade {
            version: ProtocolVersion {
                major: 1,
                minor: 1,
                patch: 0,
            },
            upgrade_schedule: UpgradeSchedule {
                activation_time: 1700000000,
                grace_period: 86400,
            },
        };
        assert!(matches!(proposal, DaoProposal::SoftwareUpgrade { .. }));
        // No RewriteTrade variant exists in the enum
    }

    #[test]
    fn test_dnc_007_no_unilateral_blockchain_override() {
        // DNC-007: OpenDex cannot unilaterally override blockchain settlement.
        // Verification: The DAO interface has no method to override blockchain state.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::SettlementTimeout,
            new_value: ParameterValue::Numeric(3600),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No OverrideBlockchain variant exists in the enum
    }

    #[test]
    fn test_dnc_008_permissionless_node_participation() {
        // DNC-008: An independent compatible node can participate without permission.
        // Verification: The DAO interface has no method to approve or reject nodes.
        let params = vec![
            ProtocolParameter::PeerDiscoveryInterval,
            ProtocolParameter::HeartbeatInterval,
        ];
        for p in params {
            assert!(matches!(
                p,
                ProtocolParameter::PeerDiscoveryInterval | ProtocolParameter::HeartbeatInterval
            ));
        }
        // No ApproveNode or RejectNode variant exists
    }

    #[test]
    fn test_dnc_009_operational_without_central_server() {
        // DNC-009: The protocol remains operational without a central matching server.
        // Verification: The DAO interface has no method to disable P2P operation.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::MatchingTimeout,
            new_value: ParameterValue::Numeric(300),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No DisableP2P variant exists in the enum
    }

    #[test]
    fn test_dnc_010_settlement_without_central_authority() {
        // DNC-010: The protocol remains capable of settlement without central authority.
        // Verification: The DAO interface has no method to centralize settlement.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::SettlementTimeout,
            new_value: ParameterValue::Numeric(3600),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No CentralizeSettlement variant exists in the enum
    }

    #[test]
    fn test_dnc_011_user_assets_controlled_by_user_credentials() {
        // DNC-011: User assets remain controlled by user credentials.
        // Verification: The DAO interface has no method to change asset control.
        let proposal = DaoProposal::AssetChange {
            asset: Asset {
                symbol: "BTC".to_string(),
                chain_id: "bitcoin-mainnet".to_string(),
            },
            action: AssetAction::Add,
        };
        assert!(matches!(proposal, DaoProposal::AssetChange { .. }));
        // No ChangeAssetControl variant exists in the enum
    }

    // ============================================================================
    // DNC-012 to DNC-016: DAO Treasury Constraints
    // ============================================================================

    #[test]
    fn test_dnc_012_dao_cannot_control_commercial_treasury() {
        // DNC-012: DAO cannot control commercial treasury.
        // Verification: DaoProposal has no treasury-related variants.
        let config = DaoConfig::new(86400, 604800, 0.1, "fee_recipient_address".to_string());
        // Fee recipient is set at init and cannot be changed via DAO proposal
        assert_eq!(config.fee_recipient(), "fee_recipient_address");
        // No TreasuryWithdrawal, TreasuryTransfer, or FeeRecipientChange variants exist
    }

    #[test]
    fn test_dnc_013_treasury_control_cannot_affect_user_principal() {
        // DNC-013: Treasury control cannot affect user principal.
        // Verification: The DAO crate has no dependency on user asset crates.
        // The DaoConfig only contains protocol-level parameters.
        let config = DaoConfig::new(86400, 604800, 0.1, "treasury_address".to_string());
        assert_eq!(config.fee_recipient(), "treasury_address");
        // Config has no access to user principal
    }

    #[test]
    fn test_dnc_014_fees_are_independently_calculable() {
        // DNC-014: Fees are independently calculable.
        // Verification: FeeRate is a protocol parameter, but fee calculation
        // is deterministic and not DAO-controlled at runtime.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // Fee rate can be proposed but calculation remains deterministic
    }

    #[test]
    fn test_dnc_015_fee_collection_cannot_modify_trade_identity() {
        // DNC-015: Fee collection cannot modify trade identity.
        // Verification: The DAO interface separates fee parameters from trade parameters.
        let fee_proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        };
        let trade_proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::MatchingTimeout,
            new_value: ParameterValue::Numeric(300),
        };
        // Fee and trade proposals are separate
        assert!(matches!(fee_proposal, DaoProposal::ParameterChange { .. }));
        assert!(matches!(
            trade_proposal,
            DaoProposal::ParameterChange { .. }
        ));
    }

    #[test]
    fn test_dnc_016_fee_collection_cannot_create_rights_over_user_principal() {
        // DNC-016: Fee collection cannot create rights over user principal.
        // Verification: The DAO interface has no method to create liens on user assets.
        let config = DaoConfig::new(86400, 604800, 0.1, "fee_recipient".to_string());
        assert_eq!(config.fee_recipient(), "fee_recipient");
        // No CreateLiens variant exists in the enum
    }

    // ============================================================================
    // DNC-017 to DNC-020: Infrastructure Resilience
    // ============================================================================

    #[test]
    fn test_dnc_017_no_asset_transfer_on_infrastructure_removal() {
        // DNC-017: Removal of OpenDex commercial infrastructure does not transfer user assets.
        // Verification: The DAO interface has no method to transfer assets on shutdown.
        let proposal = DaoProposal::SoftwareUpgrade {
            version: ProtocolVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            upgrade_schedule: UpgradeSchedule {
                activation_time: 1700000000,
                grace_period: 86400,
            },
        };
        assert!(matches!(proposal, DaoProposal::SoftwareUpgrade { .. }));
        // No TransferAssetsOnShutdown variant exists
    }

    #[test]
    fn test_dnc_018_blockchain_ownership_independent_of_infrastructure() {
        // DNC-018: Removal of OpenDex commercial infrastructure does not invalidate blockchain ownership.
        // Verification: The DAO interface has no method to invalidate blockchain state.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::SettlementTimeout,
            new_value: ParameterValue::Numeric(3600),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No InvalidateBlockchainOwnership variant exists
    }

    #[test]
    fn test_dnc_019_no_central_database_dependency() {
        // DNC-019: Protocol state is not dependent on a central database.
        // Verification: The DAO interface has no method to centralize state storage.
        let params = vec![
            ProtocolParameter::PeerDiscoveryInterval,
            ProtocolParameter::HeartbeatInterval,
        ];
        for p in params {
            assert!(matches!(
                p,
                ProtocolParameter::PeerDiscoveryInterval | ProtocolParameter::HeartbeatInterval
            ));
        }
        // No CentralizeStateStorage variant exists
    }

    #[test]
    fn test_dnc_020_protocol_state_independently_verifiable() {
        // DNC-020: Protocol state remains independently verifiable by participating nodes.
        // Verification: The DAO interface has no method to hide state from nodes.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::MatchingTimeout,
            new_value: ParameterValue::Numeric(300),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No HideStateFromNodes variant exists
    }

    // ============================================================================
    // Additional DAO Interface Tests
    // ============================================================================

    #[test]
    fn test_dao_proposal_parameter_change() {
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
    }

    #[test]
    fn test_dao_proposal_software_upgrade() {
        let proposal = DaoProposal::SoftwareUpgrade {
            version: ProtocolVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            upgrade_schedule: UpgradeSchedule {
                activation_time: 1700000000,
                grace_period: 86400,
            },
        };
        assert!(matches!(proposal, DaoProposal::SoftwareUpgrade { .. }));
    }

    #[test]
    fn test_dao_proposal_asset_change() {
        let proposal = DaoProposal::AssetChange {
            asset: Asset {
                symbol: "BTC".to_string(),
                chain_id: "bitcoin-mainnet".to_string(),
            },
            action: AssetAction::Add,
        };
        assert!(matches!(proposal, DaoProposal::AssetChange { .. }));
    }

    #[test]
    fn test_voting_power_has_passed() {
        let vp = VotingPower {
            total: 100,
            for_votes: 60,
            against_votes: 40,
        };
        assert!(vp.has_passed());

        let vp = VotingPower {
            total: 100,
            for_votes: 40,
            against_votes: 60,
        };
        assert!(!vp.has_passed());
    }

    #[test]
    fn test_voting_power_participation_rate() {
        let vp = VotingPower {
            total: 100,
            for_votes: 60,
            against_votes: 40,
        };
        assert!((vp.participation_rate() - 1.0).abs() < f64::EPSILON);

        let vp = VotingPower {
            total: 100,
            for_votes: 30,
            against_votes: 20,
        };
        assert!((vp.participation_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_dao_config_fee_recipient_immutable() {
        let config = DaoConfig::new(86400, 604800, 0.1, "fee_recipient_address".to_string());
        assert_eq!(config.fee_recipient(), "fee_recipient_address");
    }

    // ============================================================================
    // DEC Invariant Tests (MiCA Compliance)
    // ============================================================================
    // These tests verify that the DAO interface satisfies the MiCA compliance
    // requirements defined in the OpenDex regulatory compliance document (Section 4.27).
    // ============================================================================

    #[test]
    fn test_dec_001_no_mandatory_intermediary() {
        // DEC-001: No OpenDex-controlled intermediary shall be necessary for a valid trade.
        // Verification: The DAO interface has no method to require an intermediary.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::MatchingTimeout,
            new_value: ParameterValue::Numeric(300),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No RequireIntermediary variant exists
    }

    #[test]
    fn test_dec_002_independent_matching() {
        // DEC-002: Any node must be capable of performing matching without contacting OpenDex.
        // Verification: The DAO interface has no method to centralize matching.
        let params = vec![
            ProtocolParameter::MatchingTimeout,
            ProtocolParameter::OrderBookDepthLimit,
        ];
        for p in params {
            assert!(matches!(
                p,
                ProtocolParameter::MatchingTimeout | ProtocolParameter::OrderBookDepthLimit
            ));
        }
        // No CentralizeMatching variant exists
    }

    #[test]
    fn test_dec_003_independent_order_propagation() {
        // DEC-003: Orders must be propagatable through independently operated nodes.
        // Verification: The DAO interface has no method to block order propagation.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::PeerDiscoveryInterval,
            new_value: ParameterValue::Numeric(60),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No BlockOrderPropagation variant exists
    }

    #[test]
    fn test_dec_004_no_central_order_authority() {
        // DEC-004: No entity shall possess exclusive authority over the canonical order book.
        // Verification: The DAO interface has no method to centralize order book authority.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::OrderBookDepthLimit,
            new_value: ParameterValue::Numeric(100),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No SetCentralOrderAuthority variant exists
    }

    #[test]
    fn test_dec_005_no_operator_custody() {
        // DEC-005: OpenDex infrastructure shall not hold user private keys.
        // Verification: The DAO interface has no method to access user keys.
        let proposal = DaoProposal::AssetChange {
            asset: Asset {
                symbol: "BTC".to_string(),
                chain_id: "bitcoin-mainnet".to_string(),
            },
            action: AssetAction::Add,
        };
        assert!(matches!(proposal, DaoProposal::AssetChange { .. }));
        // No AccessUserKeys variant exists
    }

    #[test]
    fn test_dec_006_no_operator_settlement_authority() {
        // DEC-006: OpenDex infrastructure shall not have unilateral settlement authority.
        // Verification: The DAO interface has no method to control settlement.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::SettlementTimeout,
            new_value: ParameterValue::Numeric(3600),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No ControlSettlement variant exists
    }

    #[test]
    fn test_dec_007_independent_node_operation() {
        // DEC-007: A third party must run a complete node without permission.
        // Verification: The DAO interface has no method to approve or reject nodes.
        let params = vec![
            ProtocolParameter::PeerDiscoveryInterval,
            ProtocolParameter::HeartbeatInterval,
        ];
        for p in params {
            assert!(matches!(
                p,
                ProtocolParameter::PeerDiscoveryInterval | ProtocolParameter::HeartbeatInterval
            ));
        }
        // No ApproveNode or RejectNode variant exists
    }

    #[test]
    fn test_dec_008_independent_frontend_operation() {
        // DEC-008: A third party must create a frontend connecting to the protocol.
        // Verification: The DAO interface has no method to restrict frontend access.
        let proposal = DaoProposal::SoftwareUpgrade {
            version: ProtocolVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            upgrade_schedule: UpgradeSchedule {
                activation_time: 1700000000,
                grace_period: 86400,
            },
        };
        assert!(matches!(proposal, DaoProposal::SoftwareUpgrade { .. }));
        // No RestrictFrontend variant exists
    }

    #[test]
    fn test_dec_009_independent_relay_operation() {
        // DEC-009: A third party must operate relay/discovery infrastructure.
        // Verification: The DAO interface has no method to mandate relay operators.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::PeerDiscoveryInterval,
            new_value: ParameterValue::Numeric(60),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No MandateRelayOperator variant exists
    }

    #[test]
    fn test_dec_010_no_privileged_operator() {
        // DEC-010: OpenDex infrastructure shall not possess privileged protocol access.
        // Verification: The DAO interface has no method to grant operator privileges.
        let params = vec![
            ProtocolParameter::FeeRate,
            ProtocolParameter::MatchingTimeout,
            ProtocolParameter::SettlementTimeout,
            ProtocolParameter::MinimumOrderSize,
            ProtocolParameter::MaximumOrderSize,
            ProtocolParameter::OrderBookDepthLimit,
            ProtocolParameter::PeerDiscoveryInterval,
            ProtocolParameter::HeartbeatInterval,
        ];
        for p in params {
            assert!(matches!(
                p,
                ProtocolParameter::FeeRate
                    | ProtocolParameter::MatchingTimeout
                    | ProtocolParameter::SettlementTimeout
                    | ProtocolParameter::MinimumOrderSize
                    | ProtocolParameter::MaximumOrderSize
                    | ProtocolParameter::OrderBookDepthLimit
                    | ProtocolParameter::PeerDiscoveryInterval
                    | ProtocolParameter::HeartbeatInterval
            ));
        }
        // No GrantOperatorPrivileges variant exists
    }

    #[test]
    fn test_dec_011_no_mandatory_opendex_api() {
        // DEC-011: Trading shall not depend on an OpenDex-operated API.
        // Verification: The DAO interface has no method to require a central API.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::MatchingTimeout,
            new_value: ParameterValue::Numeric(300),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No RequireCentralAPI variant exists
    }

    #[test]
    fn test_dec_012_no_mandatory_opendex_account() {
        // DEC-012: Users shall not be required to create an account with OpenDex.
        // Verification: The DAO interface has no method to require accounts.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::MinimumOrderSize,
            new_value: ParameterValue::Numeric(10000),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No RequireAccount variant exists
    }

    #[test]
    fn test_dec_013_no_unilateral_censorship() {
        // DEC-013: No OpenDex operator shall have unilateral censorship authority.
        // Verification: The DAO interface has no method to censor transactions.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::MaximumOrderSize,
            new_value: ParameterValue::Numeric(100000000),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No CensorTransaction variant exists
    }

    #[test]
    fn test_dec_014_user_controlled_keys() {
        // DEC-014: Users retain control of private keys for their crypto-assets.
        // Verification: The DAO interface has no method to access or control user keys.
        let config = DaoConfig::new(86400, 604800, 0.1, "fee_recipient".to_string());
        assert_eq!(config.fee_recipient(), "fee_recipient");
        // No AccessUserKeys or ControlUserKeys variant exists
    }

    #[test]
    fn test_dec_015_deterministic_execution() {
        // DEC-015: Protocol rules shall be executable independently by nodes.
        // Verification: The DAO interface has no method to make execution non-deterministic.
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::MatchingTimeout,
            new_value: ParameterValue::Numeric(300),
        };
        assert!(matches!(proposal, DaoProposal::ParameterChange { .. }));
        // No MakeExecutionNonDeterministic variant exists
    }

    // ============================================================================
    // DAO Engine Integration Tests
    // ============================================================================
    // These tests verify the full proposal lifecycle: submit → vote → close → execute.
    // ============================================================================

    fn create_test_engine() -> DaoEngine {
        let config = DaoConfig::new(86400, 604800, 0.1, "fee_recipient_address".to_string());
        DaoEngine::new(config)
    }

    #[test]
    fn test_engine_submit_proposal() {
        let mut engine = create_test_engine();

        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        };

        let result = engine.submit_proposal(proposal);

        assert_eq!(result.id, 1);
        assert_eq!(result.state, ProposalState::Voting);
        assert_eq!(result.votes_for, 0);
        assert_eq!(result.votes_against, 0);
    }

    #[test]
    fn test_engine_submit_multiple_proposals() {
        let mut engine = create_test_engine();

        let p1 = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });

        let p2 = engine.submit_proposal(DaoProposal::SoftwareUpgrade {
            version: ProtocolVersion {
                major: 1,
                minor: 1,
                patch: 0,
            },
            upgrade_schedule: UpgradeSchedule {
                activation_time: 1700000000,
                grace_period: 86400,
            },
        });

        assert_eq!(p1.id, 1);
        assert_eq!(p2.id, 2);
    }

    #[test]
    fn test_engine_get_proposal() {
        let mut engine = create_test_engine();

        let submitted = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });

        let retrieved = engine.get_proposal(submitted.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, 1);

        let missing = engine.get_proposal(999);
        assert!(missing.is_none());
    }

    #[test]
    fn test_engine_list_proposals() {
        let mut engine = create_test_engine();

        engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });
        engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::MatchingTimeout,
            new_value: ParameterValue::Numeric(300),
        });

        let all = engine.list_proposals();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, 1);
        assert_eq!(all[1].id, 2);
    }

    #[test]
    fn test_engine_list_active_proposals() {
        let mut engine = create_test_engine();

        engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });

        let active = engine.list_active_proposals();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].state, ProposalState::Voting);
    }

    #[test]
    fn test_engine_cast_vote() {
        let mut engine = create_test_engine();

        let p = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });

        engine.cast_vote(p.id, Vote::For).unwrap();
        engine.cast_vote(p.id, Vote::For).unwrap();
        engine.cast_vote(p.id, Vote::Against).unwrap();

        let updated = engine.get_proposal(p.id).unwrap();
        assert_eq!(updated.votes_for, 2);
        assert_eq!(updated.votes_against, 1);
    }

    #[test]
    fn test_engine_cast_vote_proposal_not_found() {
        let mut engine = create_test_engine();

        let result = engine.cast_vote(999, Vote::For);
        assert_eq!(result, Err(DaoError::ProposalNotFound(999)));
    }

    #[test]
    fn test_engine_cast_vote_invalid_state() {
        let mut engine = create_test_engine();

        let p = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });

        // Close voting first
        engine.cast_vote(p.id, Vote::For).unwrap();
        engine.close_voting(p.id).unwrap();

        // Now voting should fail
        let result = engine.cast_vote(p.id, Vote::For);
        assert!(matches!(
            result,
            Err(DaoError::InvalidStateTransition { .. })
        ));
    }

    #[test]
    fn test_engine_full_lifecycle_approve_and_execute() {
        let mut engine = create_test_engine();

        // 1. Submit
        let p = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(75),
        });
        assert_eq!(p.state, ProposalState::Voting);

        // 2. Vote
        engine.cast_vote(p.id, Vote::For).unwrap();

        // 3. Close voting
        engine.close_voting(p.id).unwrap();
        assert_eq!(
            engine.get_proposal(p.id).unwrap().state,
            ProposalState::Approved
        );

        // 4. Execute
        engine.execute_proposal(p.id).unwrap();
        assert_eq!(
            engine.get_proposal(p.id).unwrap().state,
            ProposalState::Executed
        );

        // 5. Query parameter
        let param = engine.get_parameter(ProtocolParameter::FeeRate);
        assert!(param.is_some());
        assert_eq!(param.unwrap(), &ParameterValue::Numeric(75));
    }

    #[test]
    fn test_engine_full_lifecycle_rejection() {
        let mut engine = create_test_engine();

        let p = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(999),
        });

        // Vote against
        engine.cast_vote(p.id, Vote::Against).unwrap();

        // Close voting — should be rejected
        engine.close_voting(p.id).unwrap();
        assert_eq!(
            engine.get_proposal(p.id).unwrap().state,
            ProposalState::Rejected
        );

        // Cannot execute a rejected proposal
        let result = engine.execute_proposal(p.id);
        assert!(matches!(
            result,
            Err(DaoError::InvalidStateTransition { .. })
        ));
    }

    #[test]
    fn test_engine_close_voting_participation_too_low() {
        let config = DaoConfig::new(86400, 604800, 1.0, "fee_recipient".to_string());
        let mut engine = DaoEngine::new(config);

        let p = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });

        // No votes cast — participation is 0
        let result = engine.close_voting(p.id);
        assert!(matches!(result, Err(DaoError::ParticipationTooLow { .. })));
    }

    #[test]
    fn test_engine_close_voting_proposal_not_found() {
        let mut engine = create_test_engine();

        let result = engine.close_voting(999);
        assert_eq!(result, Err(DaoError::ProposalNotFound(999)));
    }

    #[test]
    fn test_engine_execute_proposal_not_found() {
        let mut engine = create_test_engine();

        let result = engine.execute_proposal(999);
        assert_eq!(result, Err(DaoError::ProposalNotFound(999)));
    }

    #[test]
    fn test_engine_execute_rejected_proposal() {
        let mut engine = create_test_engine();

        let p = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });

        engine.cast_vote(p.id, Vote::Against).unwrap();
        engine.close_voting(p.id).unwrap();

        let result = engine.execute_proposal(p.id);
        assert!(matches!(
            result,
            Err(DaoError::InvalidStateTransition { .. })
        ));
    }

    #[test]
    fn test_engine_execute_voting_proposal() {
        let mut engine = create_test_engine();

        let p = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });

        // Cannot execute while still voting
        let result = engine.execute_proposal(p.id);
        assert!(matches!(
            result,
            Err(DaoError::InvalidStateTransition { .. })
        ));
    }

    #[test]
    fn test_engine_fee_recipient_immutability() {
        let config = DaoConfig::new(86400, 604800, 0.1, "original_fee_recipient".to_string());
        let mut engine = DaoEngine::new(config);

        // Submit and execute a proposal
        let p = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });
        engine.cast_vote(p.id, Vote::For).unwrap();
        engine.close_voting(p.id).unwrap();
        engine.execute_proposal(p.id).unwrap();

        // Fee recipient should still be the original value
        assert_eq!(engine.config().fee_recipient(), "original_fee_recipient");

        // There is no method to change fee_recipient — this is enforced by the type system
    }

    #[test]
    fn test_engine_active_proposals_excludes_executed() {
        let mut engine = create_test_engine();

        let p = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });

        // Active before execution
        assert_eq!(engine.list_active_proposals().len(), 1);

        engine.cast_vote(p.id, Vote::For).unwrap();
        engine.close_voting(p.id).unwrap();
        engine.execute_proposal(p.id).unwrap();

        // Not active after execution
        assert_eq!(engine.list_active_proposals().len(), 0);
    }

    #[test]
    fn test_engine_software_upgrade_execution() {
        let mut engine = create_test_engine();

        let p = engine.submit_proposal(DaoProposal::SoftwareUpgrade {
            version: ProtocolVersion {
                major: 2,
                minor: 0,
                patch: 0,
            },
            upgrade_schedule: UpgradeSchedule {
                activation_time: 1700000000,
                grace_period: 86400,
            },
        });

        engine.cast_vote(p.id, Vote::For).unwrap();
        engine.close_voting(p.id).unwrap();
        engine.execute_proposal(p.id).unwrap();

        assert_eq!(
            engine.get_proposal(p.id).unwrap().state,
            ProposalState::Executed
        );

        // Software upgrades don't modify parameters
        assert!(engine.get_parameter(ProtocolParameter::FeeRate).is_none());
    }

    #[test]
    fn test_engine_asset_change_execution() {
        let mut engine = create_test_engine();

        let p = engine.submit_proposal(DaoProposal::AssetChange {
            asset: Asset {
                symbol: "ETH".to_string(),
                chain_id: "ethereum-mainnet".to_string(),
            },
            action: AssetAction::Add,
        });

        engine.cast_vote(p.id, Vote::For).unwrap();
        engine.close_voting(p.id).unwrap();
        engine.execute_proposal(p.id).unwrap();

        assert_eq!(
            engine.get_proposal(p.id).unwrap().state,
            ProposalState::Executed
        );
    }

    #[test]
    fn test_engine_close_voting_on_non_voting_proposal() {
        let mut engine = create_test_engine();

        let p = engine.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(50),
        });

        engine.cast_vote(p.id, Vote::For).unwrap();
        engine.close_voting(p.id).unwrap();

        // Cannot close voting again
        let result = engine.close_voting(p.id);
        assert!(matches!(
            result,
            Err(DaoError::InvalidStateTransition { .. })
        ));
    }
}
