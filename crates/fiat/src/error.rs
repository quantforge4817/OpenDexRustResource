//! Error types for the fiat settlement subsystem.

use serde::{Deserialize, Serialize};

/// Errors specific to fiat operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FiatError {
    /// Payment not found by the adapter.
    PaymentNotFound,
    /// Payment amount does not match the expected amount.
    AmountMismatch { expected: u64, actual: u64 },
    /// Payment currency does not match the expected currency.
    CurrencyMismatch,
    /// Payment reference does not match the trade reference.
    ReferenceMismatch,
    /// Payment has been reversed after confirmation.
    PaymentReversed,
    /// Payment adapter returned an unknown status.
    UnknownPaymentStatus,
    /// Payment deadline has expired.
    PaymentTimeout,
    /// Confirmation deadline has expired.
    ConfirmationTimeout,
    /// Bond is insufficient for the trade.
    InsufficientBond,
    /// Bond operation failed.
    BondError(String),
    /// Dispute is already open for this trade.
    DisputeAlreadyOpen,
    /// Invalid state transition.
    InvalidStateTransition { from: String, to: String },
    /// General settlement error.
    SettlementError(String),
}

impl std::fmt::Display for FiatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FiatError::PaymentNotFound => write!(f, "payment not found"),
            FiatError::AmountMismatch { expected, actual } => {
                write!(
                    f,
                    "payment amount mismatch: expected {}, actual {}",
                    expected, actual
                )
            }
            FiatError::CurrencyMismatch => write!(f, "payment currency mismatch"),
            FiatError::ReferenceMismatch => write!(f, "payment reference mismatch"),
            FiatError::PaymentReversed => write!(f, "payment reversed after confirmation"),
            FiatError::UnknownPaymentStatus => write!(f, "unknown payment status from adapter"),
            FiatError::PaymentTimeout => write!(f, "payment deadline expired"),
            FiatError::ConfirmationTimeout => write!(f, "confirmation deadline expired"),
            FiatError::InsufficientBond => write!(f, "bond insufficient for trade"),
            FiatError::BondError(msg) => write!(f, "bond error: {}", msg),
            FiatError::DisputeAlreadyOpen => write!(f, "dispute already open for this trade"),
            FiatError::InvalidStateTransition { from, to } => {
                write!(f, "invalid state transition: {} -> {}", from, to)
            }
            FiatError::SettlementError(msg) => write!(f, "settlement error: {}", msg),
        }
    }
}

impl std::error::Error for FiatError {}

/// Errors from the settlement engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementError {
    /// Underlying fiat error.
    Fiat(FiatError),
    /// Crypto wallet operation failed.
    CryptoWalletError(String),
    /// Payment adapter error.
    AdapterError(String),
    /// Verification failed.
    VerificationFailed(String),
    /// Multisig escrow operation failed.
    MultisigError(String),
    /// Caller is not authorized to perform this operation.
    Unauthorized,
    /// Rate limit exceeded.
    RateLimited,
}

impl std::fmt::Display for SettlementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettlementError::Fiat(err) => write!(f, "fiat error: {}", err),
            SettlementError::CryptoWalletError(msg) => write!(f, "crypto wallet error: {}", msg),
            SettlementError::AdapterError(msg) => write!(f, "payment adapter error: {}", msg),
            SettlementError::VerificationFailed(msg) => {
                write!(f, "verification failed: {}", msg)
            }
            SettlementError::MultisigError(msg) => write!(f, "multisig error: {}", msg),
            SettlementError::Unauthorized => write!(f, "unauthorized: caller not permitted"),
            SettlementError::RateLimited => {
                write!(f, "rate limit exceeded, please try again later")
            }
        }
    }
}

impl std::error::Error for SettlementError {}

impl From<FiatError> for SettlementError {
    fn from(err: FiatError) -> Self {
        SettlementError::Fiat(err)
    }
}

impl From<anyhow::Error> for SettlementError {
    fn from(err: anyhow::Error) -> Self {
        SettlementError::Fiat(FiatError::SettlementError(err.to_string()))
    }
}
