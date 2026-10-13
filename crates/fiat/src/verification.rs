//! Payment verification types.
//!
//! A fiat payment requires verification before crypto can be released.
//! This module defines the verified payment structure and verification errors.

use serde::{Deserialize, Serialize};
use chrono::DateTime;
use chrono::Utc;

use crate::payment::{PaymentId, FiatAmount, PaymentReference, PaymentStatus};
use opendex_types::FiatCurrency;

/// Result of a successful payment verification.
///
/// The system must verify currency, amount, recipient, payment reference,
/// and status before releasing crypto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedPayment {
    /// Unique payment identifier.
    pub payment_id: PaymentId,
    /// Currency of the payment.
    pub currency: FiatCurrency,
    /// Verified amount.
    pub amount: FiatAmount,
    /// Sender's payment reference.
    pub sender_reference: PaymentReference,
    /// Recipient's payment reference.
    pub recipient_reference: PaymentReference,
    /// When verification was performed.
    pub verified_at: DateTime<Utc>,
    /// Final verified status.
    pub status: PaymentStatus,
}

/// Errors that can occur during payment verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentVerificationError {
    /// Payment amount does not match expected.
    AmountMismatch { expected: u64, actual: u64 },
    /// Payment currency does not match expected.
    CurrencyMismatch { expected: FiatCurrency, actual: FiatCurrency },
    /// Payment reference does not match the trade reference.
    ReferenceMismatch,
    /// Payment status is not confirmed.
    NotConfirmed(PaymentStatus),
    /// Payment has been reversed.
    Reversed,
    /// Verification data is missing.
    MissingData(String),
}

impl std::fmt::Display for PaymentVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentVerificationError::AmountMismatch { expected, actual } => {
                write!(f, "amount mismatch: expected {}, actual {}", expected, actual)
            }
            PaymentVerificationError::CurrencyMismatch { expected, actual } => {
                write!(f, "currency mismatch: expected {}, actual {}", expected, actual)
            }
            PaymentVerificationError::ReferenceMismatch => {
                write!(f, "payment reference mismatch")
            }
            PaymentVerificationError::NotConfirmed(status) => {
                write!(f, "payment not confirmed, status: {}", status)
            }
            PaymentVerificationError::Reversed => {
                write!(f, "payment has been reversed")
            }
            PaymentVerificationError::MissingData(field) => {
                write!(f, "missing verification data: {}", field)
            }
        }
    }
}

impl std::error::Error for PaymentVerificationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn verified_payment_creation() {
        let verified = VerifiedPayment {
            payment_id: PaymentId(Uuid::new_v4()),
            currency: FiatCurrency::GBP,
            amount: FiatAmount::new(10000, FiatCurrency::GBP),
            sender_reference: PaymentReference::generate(),
            recipient_reference: PaymentReference::generate(),
            verified_at: Utc::now(),
            status: PaymentStatus::Confirmed,
        };
        assert_eq!(verified.status, PaymentStatus::Confirmed);
        assert_eq!(verified.currency, FiatCurrency::GBP);
    }

    #[test]
    fn verification_error_display() {
        let err = PaymentVerificationError::AmountMismatch {
            expected: 10000,
            actual: 5000,
        };
        assert!(err.to_string().contains("amount mismatch"));
    }
}
