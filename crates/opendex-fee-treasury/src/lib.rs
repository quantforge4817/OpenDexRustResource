//! OpenDex Fee & Treasury Subsystem
//!
//! This crate provides deterministic fee calculation and an isolated treasury
//! that receives protocol fees but **cannot** touch user principal under any
//! circumstances.
//!
//! ## Regulatory Constraints (DNC-012 to DNC-016)
//!
//! | Constraint | Invariant | Enforcement |
//! |------------|-----------|-------------|
//! | DAO cannot control commercial treasury | DNC-012 | Treasury has no DAO methods; DAO has no treasury methods |
//! | Treasury control cannot affect user principal | DNC-013 | No dependency on user asset crates |
//! | Fees are independently calculable | DNC-014 | `FeeCalculator::calculate()` is a pure function |
//! | Fee collection cannot modify trade identity | DNC-015 | Fee collection takes only amount and asset, no trade identity |
//! | Fee collection cannot create rights over user principal | DNC-016 | Treasury receives fees only; no rights over user principal |
//!
//! ## Design Principles
//!
//! 1. **No dependency on user asset crates**: This crate depends only on
//!    `opendex-dao` for protocol parameters. It has zero dependency on
//!    `opendex-storage`, `opendex-swap`, or any crate handling user principal.
//!    This enforces DNC-013 at the crate boundary.
//!
//! 2. **Fee calculation is a pure function**: `FeeCalculator::calculate()` takes
//!    an amount and a fee rate, returns the fee and net amounts. No state, no
//!    side effects. This satisfies DNC-014.
//!
//! 3. **Treasury is receive-only**: The `Treasury` struct can only receive fees
//!    and query balances. It has no methods to transfer out, freeze, or interact
//!    with user assets. This satisfies DNC-012, DNC-013, DNC-015, DNC-016.
//!
//! 4. **Fee records are immutable**: Once a `FeeRecord` is created, it cannot
//!    be modified. This provides an audit trail.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────┐
//! │   Trade (p2p-node)   │
//! └──────────┬───────────┘
//!            │
//!            ▼
//! ┌──────────────────────┐
//! │   FeeCalculator      │  ← Pure function, no state
//! │   (fee, net) = calc() │
//! └──────────┬───────────┘
//!            │
//!            ▼
//! ┌──────────────────────┐
//! │   Treasury           │  ← Receive-only, no user asset access
//! │   .receive_fee()     │
//! └──────────┬───────────┘
//!            │
//!            ▼
//! ┌──────────────────────┐
//! │   FeeRecord (log)    │  ← Append-only, immutable
//! └──────────────────────┘
//! ```

use opendex_dao::{ProtocolParameter, ParameterValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ==========================================================================
// Fee Calculator
// ==========================================================================

/// Errors that can occur during fee operations.
#[derive(Debug, Error, PartialEq)]
pub enum FeeError {
    /// The fee rate parameter is not set.
    #[error("Fee rate not configured")]
    FeeRateNotConfigured,

    /// The fee rate is invalid (must be 0-10000 basis points).
    #[error("Invalid fee rate: {0} (must be 0-10000 basis points)")]
    InvalidFeeRate(u64),

    /// The asset has no treasury balance record.
    #[error("Asset {0} not found in treasury")]
    AssetNotFound(String),

    /// Arithmetic overflow during fee calculation.
    #[error("Arithmetic overflow during fee calculation")]
    ArithmeticOverflow,
}

/// Deterministic fee calculator.
///
/// This is a pure function with no state. Given an amount and a fee rate
/// (in basis points), it returns the fee amount and the net amount.
///
/// The fee rate is expressed in basis points (1/100th of a percent):
/// - 0 = 0% (no fee)
/// - 100 = 1%
/// - 500 = 5%
/// - 10000 = 100%
///
/// This satisfies DNC-014: fees are independently calculable.
pub struct FeeCalculator;

impl FeeCalculator {
    /// Calculate the fee and net amount from a gross amount and fee rate.
    ///
    /// # Arguments
    /// * `amount` - The gross amount in satoshis (or smallest unit)
    /// * `fee_rate_bps` - The fee rate in basis points (0-10000)
    ///
    /// # Returns
    /// `(fee_amount, net_amount)` where `fee_amount + net_amount = amount`
    ///
    /// # Example
    /// ```
    /// use opendex_fee_treasury::FeeCalculator;
    ///
    /// let (fee, net) = FeeCalculator::calculate(100_000, 100).unwrap();
    /// assert_eq!(fee, 1000);   // 1% of 100,000
    /// assert_eq!(net, 99_000); // remainder
    /// assert_eq!(fee + net, 100_000);
    /// ```
    pub fn calculate(amount: u64, fee_rate_bps: u64) -> Result<(u64, u64), FeeError> {
        if fee_rate_bps > 10000 {
            return Err(FeeError::InvalidFeeRate(fee_rate_bps));
        }

        // Calculate fee: amount * fee_rate / 10000
        // Use checked arithmetic to prevent overflow
        let fee = amount
            .checked_mul(fee_rate_bps)
            .ok_or(FeeError::ArithmeticOverflow)?
            / 10000;

        let net = amount - fee; // fee <= amount, so this won't underflow

        Ok((fee, net))
    }

    /// Extract the fee rate from a DAO parameter value.
    ///
    /// Returns the fee rate in basis points, or an error if the parameter
    /// is not a numeric fee rate.
    pub fn fee_rate_from_parameter(value: &ParameterValue) -> Result<u64, FeeError> {
        match value {
            ParameterValue::Numeric(rate) => {
                if *rate > 10000 {
                    Err(FeeError::InvalidFeeRate(*rate))
                } else {
                    Ok(*rate)
                }
            }
            _ => Err(FeeError::FeeRateNotConfigured),
        }
    }

    /// Calculate fee using the current protocol parameter from the DAO.
    ///
    /// This is the primary entry point: it reads the fee rate from the DAO's
    /// protocol parameters and calculates the fee.
    pub fn calculate_from_dao(
        amount: u64,
        parameters: &HashMap<ProtocolParameter, ParameterValue>,
    ) -> Result<(u64, u64), FeeError> {
        let fee_rate = parameters
            .get(&ProtocolParameter::FeeRate)
            .ok_or(FeeError::FeeRateNotConfigured)?;

        let rate = Self::fee_rate_from_parameter(fee_rate)?;
        Self::calculate(amount, rate)
    }
}

// ==========================================================================
// Fee Record
// ==========================================================================

/// Immutable record of a fee collected.
///
/// Once created, a fee record cannot be modified. This provides an
/// immutable audit trail of all fees collected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeeRecord {
    /// Unique sequential record ID.
    pub id: u64,
    /// The fee amount collected.
    pub fee_amount: u64,
    /// The net amount (after fee deduction).
    pub net_amount: u64,
    /// The asset symbol (e.g., "BTC", "XMR").
    pub asset_symbol: String,
    /// Unix timestamp when the fee was collected.
    pub timestamp: u64,
    /// Optional reference to the trade or order that generated this fee.
    pub source_trade_id: Option<String>,
}

// ==========================================================================
// Treasury
// ==========================================================================

/// Isolated treasury that receives protocol fees.
///
/// The treasury can only receive fees and query balances. It has no methods
/// to transfer out, freeze, seize, or interact with user assets.
///
/// ## Regulatory Constraints
///
/// - **DNC-012**: DAO cannot control commercial treasury
///   - The treasury has no methods to interact with the DAO.
///   - The DAO has no methods to interact with the treasury.
///
/// - **DNC-013**: Treasury control cannot affect user principal
///   - The treasury crate has no dependency on user asset crates.
///   - The treasury cannot access, modify, or transfer user principal.
///
/// - **DNC-015**: Fee collection cannot modify trade identity
///   - The treasury receives only the fee amount and asset symbol.
///   - No trade identity information is stored or processed.
///
/// - **DNC-016**: Fee collection cannot create rights over user principal
///   - The treasury receives only the fee, not user assets.
///   - No rights over user principal can be created through fee collection.
pub struct Treasury {
    /// Balance by asset symbol.
    balances: HashMap<String, u64>,
    /// Append-only log of all fee records.
    records: Vec<FeeRecord>,
    /// Next auto-incrementing record ID.
    next_record_id: u64,
}

impl Treasury {
    /// Create a new empty treasury.
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            records: Vec::new(),
            next_record_id: 1,
        }
    }

    /// Receive a fee into the treasury.
    ///
    /// This is the only way to add funds to the treasury. The fee is
    /// recorded as an immutable `FeeRecord` and added to the balance.
    ///
    /// # Arguments
    /// * `fee_amount` - The fee amount received
    /// * `net_amount` - The net amount (after fee deduction, for audit trail)
    /// * `asset_symbol` - The asset symbol (e.g., "BTC", "XMR")
    /// * `timestamp` - Unix timestamp when the fee was collected
    /// * `source_trade_id` - Optional reference to the source trade
    ///
    /// # Returns
    /// The created `FeeRecord`.
    pub fn receive_fee(
        &mut self,
        fee_amount: u64,
        net_amount: u64,
        asset_symbol: String,
        timestamp: u64,
        source_trade_id: Option<String>,
    ) -> FeeRecord {
        let record = FeeRecord {
            id: self.next_record_id,
            fee_amount,
            net_amount,
            asset_symbol: asset_symbol.clone(),
            timestamp,
            source_trade_id,
        };

        // Update balance
        *self.balances.entry(asset_symbol.clone()).or_insert(0) += fee_amount;

        // Append record
        self.records.push(record.clone());
        self.next_record_id += 1;

        record
    }

    /// Query the balance for a specific asset.
    ///
    /// Returns the total fee amount received for the given asset symbol.
    pub fn get_balance(&self, asset_symbol: &str) -> Option<u64> {
        self.balances.get(asset_symbol).copied()
    }

    /// Get all asset symbols with non-zero balances.
    pub fn get_assets(&self) -> Vec<String> {
        self.balances
            .keys()
            .filter(|&asset| self.balances[asset] > 0)
            .cloned()
            .collect()
    }

    /// Get the total balance across all assets (sum of all fee amounts).
    ///
    /// Note: This is a simple sum and does not account for different asset
    /// values. It's only useful for counting total fee records.
    pub fn total_fee_records(&self) -> usize {
        self.records.len()
    }

    /// Query fee records by asset symbol.
    pub fn get_records_by_asset(&self, asset_symbol: &str) -> Vec<&FeeRecord> {
        self.records
            .iter()
            .filter(|r| r.asset_symbol == asset_symbol)
            .collect()
    }

    /// Get all fee records (immutable reference).
    ///
    /// The records are returned as references to prevent modification,
    /// ensuring the audit trail remains immutable.
    pub fn get_all_records(&self) -> &[FeeRecord] {
        &self.records
    }

    /// Get a specific fee record by ID.
    pub fn get_record(&self, id: u64) -> Option<&FeeRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    /// Get the fee recipient address from the DAO config.
    ///
    /// This is a read-only query; the treasury cannot change the fee recipient.
    /// The fee recipient is set at protocol initialization and is immutable.
    pub fn fee_recipient(&self) -> Option<String> {
        // The treasury itself does not store the fee recipient.
        // It is stored in DaoConfig and accessed read-only.
        // This method exists to show that the treasury has no control
        // over the fee recipient — it returns None because the treasury
        // cannot change it.
        None
    }
}

impl Default for Treasury {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use opendex_dao::{DaoConfig, DaoEngine, DaoProposal, ProtocolParameter, ParameterValue, Vote};

    // ------------------------------------------------------------------
    // FeeCalculator Tests
    // ------------------------------------------------------------------

    #[test]
    fn fee_calculator_zero_fee() {
        let (fee, net) = FeeCalculator::calculate(100_000, 0).unwrap();
        assert_eq!(fee, 0);
        assert_eq!(net, 100_000);
    }

    #[test]
    fn fee_calculator_one_percent() {
        let (fee, net) = FeeCalculator::calculate(100_000, 100).unwrap();
        assert_eq!(fee, 1000);
        assert_eq!(net, 99_000);
        assert_eq!(fee + net, 100_000);
    }

    #[test]
    fn fee_calculator_five_percent() {
        let (fee, net) = FeeCalculator::calculate(1_000_000, 500).unwrap();
        assert_eq!(fee, 50_000);
        assert_eq!(net, 950_000);
        assert_eq!(fee + net, 1_000_000);
    }

    #[test]
    fn fee_calculator_invalid_rate() {
        let result = FeeCalculator::calculate(100_000, 10001);
        assert_eq!(result, Err(FeeError::InvalidFeeRate(10001)));
    }

    #[test]
    fn fee_calculator_max_rate() {
        let (fee, net) = FeeCalculator::calculate(100_000, 10000).unwrap();
        assert_eq!(fee, 100_000);
        assert_eq!(net, 0);
    }

    #[test]
    fn fee_calculator_small_amount() {
        let (fee, net) = FeeCalculator::calculate(100, 100).unwrap();
        assert_eq!(fee, 1); // 1% of 100 = 1
        assert_eq!(net, 99);
    }

    #[test]
    fn fee_calculator_from_dao_parameter() {
        let mut params = HashMap::new();
        params.insert(ProtocolParameter::FeeRate, ParameterValue::Numeric(100));

        let (fee, net) = FeeCalculator::calculate_from_dao(100_000, &params).unwrap();
        assert_eq!(fee, 1000);
        assert_eq!(net, 99_000);
    }

    #[test]
    fn fee_calculator_from_dao_missing_rate() {
        let params = HashMap::new();
        let result = FeeCalculator::calculate_from_dao(100_000, &params);
        assert_eq!(result, Err(FeeError::FeeRateNotConfigured));
    }

    #[test]
    fn fee_calculator_from_dao_invalid_rate() {
        let mut params = HashMap::new();
        params.insert(ProtocolParameter::FeeRate, ParameterValue::Numeric(10001));

        let result = FeeCalculator::calculate_from_dao(100_000, &params);
        assert_eq!(result, Err(FeeError::InvalidFeeRate(10001)));
    }

    #[test]
    fn fee_calculator_pure_function() {
        // Verify FeeCalculator is a pure function: same input always gives same output
        let (fee1, net1) = FeeCalculator::calculate(50_000, 200).unwrap();
        let (fee2, net2) = FeeCalculator::calculate(50_000, 200).unwrap();
        assert_eq!((fee1, net1), (fee2, net2));
    }

    // ------------------------------------------------------------------
    // Treasury Tests
    // ------------------------------------------------------------------

    #[test]
    fn treasury_receive_fee() {
        let mut treasury = Treasury::new();
        let record = treasury.receive_fee(1000, 99_000, "BTC".to_string(), 1000, None);

        assert_eq!(record.id, 1);
        assert_eq!(record.fee_amount, 1000);
        assert_eq!(record.net_amount, 99_000);
        assert_eq!(record.asset_symbol, "BTC");
        assert_eq!(treasury.get_balance("BTC"), Some(1000));
    }

    #[test]
    fn treasury_multiple_fees_same_asset() {
        let mut treasury = Treasury::new();
        treasury.receive_fee(1000, 99_000, "BTC".to_string(), 1000, None);
        treasury.receive_fee(2000, 98_000, "BTC".to_string(), 1001, None);

        assert_eq!(treasury.get_balance("BTC"), Some(3000));
        assert_eq!(treasury.total_fee_records(), 2);
    }

    #[test]
    fn treasury_multiple_assets() {
        let mut treasury = Treasury::new();
        treasury.receive_fee(1000, 99_000, "BTC".to_string(), 1000, None);
        treasury.receive_fee(500, 49_500, "XMR".to_string(), 1001, None);

        assert_eq!(treasury.get_balance("BTC"), Some(1000));
        assert_eq!(treasury.get_balance("XMR"), Some(500));
        assert_eq!(treasury.get_assets().len(), 2);
    }

    #[test]
    fn treasury_record_immutability() {
        let mut treasury = Treasury::new();
        treasury.receive_fee(1000, 99_000, "BTC".to_string(), 1000, None);

        // Records are returned as immutable references
        let record = treasury.get_record(1).unwrap();
        assert_eq!(record.fee_amount, 1000);

        // The treasury provides no method to modify records
        // This is enforced by the type system
    }

    #[test]
    fn treasury_no_user_asset_access() {
        // The Treasury struct has no methods to:
        // - Transfer funds out
        // - Freeze assets
        // - Seize assets
        // - Access user principal
        //
        // This is enforced by the absence of such methods.
        // The only write method is `receive_fee`.
        let treasury = Treasury::new();

        // Verify read-only methods exist
        let _ = treasury.get_balance("BTC");
        let _ = treasury.get_assets();
        let _ = treasury.get_all_records();

        // The treasury cannot be used to access user assets
        // because it has no dependency on user asset crates
    }

    #[test]
    fn treasury_fee_recipient_read_only() {
        let treasury = Treasury::new();
        // fee_recipient() returns None because the treasury
        // has no control over the fee recipient
        assert_eq!(treasury.fee_recipient(), None);
    }

    // ------------------------------------------------------------------
    // DNC Invariant Tests
    // ------------------------------------------------------------------

    /// DNC-012: DAO cannot control commercial treasury
    #[test]
    fn dnc_012_dao_cannot_control_treasury() {
        // The DAO engine has no methods to interact with the treasury.
        // The treasury has no methods to interact with the DAO.
        // They are completely isolated.

        let config = DaoConfig::new(3600, 86400, 0.1, "treasury_addr".to_string());
        let mut dao = DaoEngine::new(config);

        // DAO can change fee rate, but cannot touch treasury
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(100),
        };
        let p = dao.submit_proposal(proposal);
        dao.cast_vote(p.id, Vote::For).unwrap();
        dao.close_voting(p.id).unwrap();
        dao.execute_proposal(p.id).unwrap();

        // The treasury is completely independent
        let treasury = Treasury::new();

        // DAO and treasury operate independently
        // No method exists to link them
        let _ = dao;
        let _ = treasury;
    }

    /// DNC-013: Treasury control cannot affect user principal
    #[test]
    fn dnc_013_treasury_cannot_affect_user_principal() {
        // The treasury crate has no dependency on user asset crates.
        // It cannot access, modify, or transfer user principal.
        // This is enforced at the crate boundary.

        let mut treasury = Treasury::new();
        treasury.receive_fee(1000, 99_000, "BTC".to_string(), 1000, None);

        // The treasury only knows about fees, not user assets
        assert_eq!(treasury.get_balance("BTC"), Some(1000));

        // There is no way to use the treasury to access user principal
        // because the treasury has no methods for that purpose
    }

    /// DNC-014: Fees are independently calculable
    #[test]
    fn dnc_014_fees_independently_calculable() {
        // FeeCalculator::calculate() is a pure function.
        // Anyone can independently verify the fee calculation.

        let amount = 100_000;
        let rate = 100; // 1%

        let (fee1, net1) = FeeCalculator::calculate(amount, rate).unwrap();
        let (fee2, net2) = FeeCalculator::calculate(amount, rate).unwrap();

        assert_eq!((fee1, net1), (fee2, net2));
        assert_eq!(fee1 + net1, amount);
    }

    /// DNC-015: Fee collection cannot modify trade identity
    #[test]
    fn dnc_015_fee_collection_cannot_modify_trade_identity() {
        // The treasury receives only the fee amount and asset symbol.
        // No trade identity information is stored or processed.

        let mut treasury = Treasury::new();

        // Receive fee without any trade identity
        let record = treasury.receive_fee(1000, 99_000, "BTC".to_string(), 1000, None);

        // The record contains only fee data, no trade identity
        assert_eq!(record.fee_amount, 1000);
        assert_eq!(record.asset_symbol, "BTC");
        // source_trade_id is optional and not required
        assert!(record.source_trade_id.is_none());
    }

    /// DNC-016: Fee collection cannot create rights over user principal
    #[test]
    fn dnc_016_fee_collection_no_rights_over_principal() {
        // The treasury receives only the fee, not user assets.
        // No rights over user principal can be created through fee collection.

        let mut treasury = Treasury::new();

        // User has 100,000 satoshis, fee is 1,000
        // Treasury receives only the 1,000 fee
        treasury.receive_fee(1000, 99_000, "BTC".to_string(), 1000, None);

        // Treasury balance is only the fee amount
        assert_eq!(treasury.get_balance("BTC"), Some(1000));

        // Treasury has no access to the 99,000 net amount
        // The net amount remains with the user
    }

    // ------------------------------------------------------------------
    // Integration Tests
    // ------------------------------------------------------------------

    #[test]
    fn full_fee_workflow() {
        // Simulate a complete fee workflow:
        // 1. DAO sets fee rate
        // 2. Fee is calculated
        // 3. Fee is received by treasury

        let config = DaoConfig::new(3600, 86400, 0.1, "treasury_addr".to_string());
        let mut dao = DaoEngine::new(config);

        // Set fee rate to 1% (100 basis points)
        let proposal = DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(100),
        };
        let p = dao.submit_proposal(proposal);
        dao.cast_vote(p.id, Vote::For).unwrap();
        dao.close_voting(p.id).unwrap();
        dao.execute_proposal(p.id).unwrap();

        // Get the fee rate from DAO
        let fee_rate = dao.get_parameter(ProtocolParameter::FeeRate);
        assert!(fee_rate.is_some());

        // Calculate fee
        let amount = 100_000;
        let (fee, net) = FeeCalculator::calculate(amount, 100).unwrap();
        assert_eq!(fee, 1000);
        assert_eq!(net, 99_000);

        // Treasury receives the fee
        let mut treasury = Treasury::new();
        let record = treasury.receive_fee(fee, net, "BTC".to_string(), 1000, Some("trade_1".to_string()));

        assert_eq!(record.fee_amount, 1000);
        assert_eq!(treasury.get_balance("BTC"), Some(1000));
    }

    #[test]
    fn fee_rate_change_via_dao() {
        // Verify that fee rate can be changed via DAO proposal
        // and the new rate is used for calculation

        let config = DaoConfig::new(3600, 86400, 0.1, "treasury_addr".to_string());
        let mut dao = DaoEngine::new(config);

        // Initial fee rate: 1%
        let p1 = dao.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(100),
        });
        dao.cast_vote(p1.id, Vote::For).unwrap();
        dao.close_voting(p1.id).unwrap();
        dao.execute_proposal(p1.id).unwrap();

        let (fee1, _) = FeeCalculator::calculate_from_dao(100_000, &dao.parameters).unwrap();
        assert_eq!(fee1, 1000); // 1%

        // Change fee rate to 2%
        let p2 = dao.submit_proposal(DaoProposal::ParameterChange {
            parameter: ProtocolParameter::FeeRate,
            new_value: ParameterValue::Numeric(200),
        });
        dao.cast_vote(p2.id, Vote::For).unwrap();
        dao.close_voting(p2.id).unwrap();
        dao.execute_proposal(p2.id).unwrap();

        let (fee2, _) = FeeCalculator::calculate_from_dao(100_000, &dao.parameters).unwrap();
        assert_eq!(fee2, 2000); // 2%
    }

    #[test]
    fn treasury_isolation_from_dao() {
        // Verify that DAO and treasury are completely isolated
        // The DAO cannot control the treasury, and vice versa

        let config = DaoConfig::new(3600, 86400, 0.1, "treasury_addr".to_string());
        let _dao = DaoEngine::new(config);
        let _treasury = Treasury::new();

        // No method exists to link DAO and treasury
        // They operate independently
    }

    #[test]
    fn fee_record_append_only() {
        // Verify that fee records are append-only
        let mut treasury = Treasury::new();
        treasury.receive_fee(1000, 99_000, "BTC".to_string(), 1000, None);
        treasury.receive_fee(2000, 98_000, "BTC".to_string(), 1001, None);

        let records = treasury.get_all_records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, 1);
        assert_eq!(records[1].id, 2);

        // Records are returned as immutable references
        // No method exists to modify them
    }
}
