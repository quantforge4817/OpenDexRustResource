//! Core payment types for fiat settlement.
//!
//! Defines the data structures used to represent fiat payments, including
//! amounts, payment instructions, payment requests, and payment status.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use opendex_types::FiatCurrency;

/// Unique identifier for a payment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaymentId(pub Uuid);

impl std::fmt::Display for PaymentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Fiat amount represented in minor units (cents, pence, etc.).
///
/// For currencies without subunits (e.g., JPY), this represents whole units.
/// Scale factor: 100 for most currencies (2 decimal places).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FiatAmount {
    /// Amount in minor units (e.g., cents for USD, pence for GBP).
    pub minor_units: u64,
    /// Currency of the amount.
    pub currency: FiatCurrency,
}

impl FiatAmount {
    /// Create a new fiat amount from minor units.
    pub fn new(minor_units: u64, currency: FiatCurrency) -> Self {
        Self {
            minor_units,
            currency,
        }
    }

    /// Create a zero amount.
    pub fn zero(currency: FiatCurrency) -> Self {
        Self {
            minor_units: 0,
            currency,
        }
    }

    /// Convert minor units to a human-readable string with decimal places.
    pub fn to_string_2dp(&self) -> String {
        let whole = self.minor_units / 100;
        let frac = self.minor_units % 100;
        format!("{}.{}", whole, frac)
    }

    /// Add two fiat amounts (must be the same currency).
    pub fn checked_add(&self, other: &FiatAmount) -> Option<FiatAmount> {
        if self.currency != other.currency {
            return None;
        }
        self.minor_units
            .checked_add(other.minor_units)
            .map(|minor_units| FiatAmount {
                minor_units,
                currency: self.currency.clone(),
            })
    }

    /// Subtract another fiat amount (must be the same currency).
    pub fn checked_sub(&self, other: &FiatAmount) -> Option<FiatAmount> {
        if self.currency != other.currency {
            return None;
        }
        self.minor_units
            .checked_sub(other.minor_units)
            .map(|minor_units| FiatAmount {
                minor_units,
                currency: self.currency.clone(),
            })
    }
}

impl std::fmt::Display for FiatAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.to_string_2dp(), self.currency)
    }
}

/// Unique payment reference for trade reconciliation.
///
/// Format: OPENDEX-XXXXXX (6 hex characters)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaymentReference(pub String);

impl PaymentReference {
    /// Generate a new unique payment reference.
    pub fn generate() -> Self {
        let uuid = Uuid::new_v4();
        let short = uuid.to_string()[..6].to_uppercase();
        Self(format!("OPENDEX-{}", short))
    }
}

impl std::fmt::Display for PaymentReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Recipient information for a fiat payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRecipient {
    /// Recipient name or account holder.
    pub name: String,
    /// Bank account number or equivalent identifier.
    pub account_number: String,
    /// Sort code, routing number, or equivalent.
    pub routing_code: String,
    /// Bank name.
    pub bank_name: String,
    /// IBAN (for SEPA transfers).
    pub iban: Option<String>,
    /// SWIFT/BIC code (for international transfers).
    pub swift_code: Option<String>,
}

/// Instructions for making a fiat payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInstructions {
    /// Trade this payment is for.
    pub trade_id: opendex_types::TradeId,
    /// Unique payment identifier.
    pub payment_id: PaymentId,
    /// Currency to pay in.
    pub currency: FiatCurrency,
    /// Amount to pay (in minor units).
    pub amount: FiatAmount,
    /// Payment reference to include with the transfer.
    pub reference: PaymentReference,
    /// Recipient bank details.
    pub recipient: PaymentRecipient,
    /// Deadline for payment (Unix timestamp seconds).
    pub deadline: u64,
}

/// Request to create a payment via an adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    /// Trade this payment is for.
    pub trade_id: opendex_types::TradeId,
    /// Amount to pay.
    pub amount: FiatAmount,
    /// Payment reference for reconciliation.
    pub reference: PaymentReference,
    /// Payer information.
    pub payer_id: opendex_types::ParticipantId,
    /// Payee information.
    pub payee_id: opendex_types::ParticipantId,
}

/// Status of a fiat payment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// Payment record created, awaiting submission.
    Created,
    /// Payment submitted, awaiting detection.
    Pending,
    /// Payment detected by the adapter, awaiting confirmation.
    Detected,
    /// Payment confirmed and cleared.
    Confirmed,
    /// Payment failed.
    Failed,
    /// Payment was reversed after confirmation.
    Reversed,
    /// Adapter returned an unknown status — must not be auto-resolved.
    Unknown,
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentStatus::Created => write!(f, "Created"),
            PaymentStatus::Pending => write!(f, "Pending"),
            PaymentStatus::Detected => write!(f, "Detected"),
            PaymentStatus::Confirmed => write!(f, "Confirmed"),
            PaymentStatus::Failed => write!(f, "Failed"),
            PaymentStatus::Reversed => write!(f, "Reversed"),
            PaymentStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Idempotency key for payment operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentOperation {
    /// Unique idempotency key to prevent duplicate operations.
    pub idempotency_key: String,
    /// Trade this operation belongs to.
    pub trade_id: opendex_types::TradeId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiat_amount_display() {
        let amount = FiatAmount::new(10050, FiatCurrency::GBP);
        assert_eq!(amount.to_string(), "100.50 GBP");
    }

    #[test]
    fn fiat_amount_addition() {
        let a = FiatAmount::new(1000, FiatCurrency::USD);
        let b = FiatAmount::new(2500, FiatCurrency::USD);
        let c = a.checked_add(&b).unwrap();
        assert_eq!(c.minor_units, 3500);
        assert_eq!(c.currency, FiatCurrency::USD);
    }

    #[test]
    fn fiat_amount_currency_mismatch() {
        let a = FiatAmount::new(1000, FiatCurrency::USD);
        let b = FiatAmount::new(2500, FiatCurrency::EUR);
        assert!(a.checked_add(&b).is_none());
    }

    #[test]
    fn payment_reference_format() {
        let ref1 = PaymentReference::generate();
        let _ref2 = PaymentReference::generate();
        assert!(ref1.0.starts_with("OPENDEX-"));
        // Format is OPENDEX-XXXXXX (6 chars from UUID)
        assert_eq!(ref1.0.len(), 14);
    }

    #[test]
    fn payment_status_display() {
        assert_eq!(PaymentStatus::Confirmed.to_string(), "Confirmed");
        assert_eq!(PaymentStatus::Unknown.to_string(), "Unknown");
    }
}
