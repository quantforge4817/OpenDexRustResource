//! Payment adapter trait and mock implementation.
//!
//! The `PaymentAdapter` trait abstracts away the specifics of different
//! payment providers (SEPA, UK Faster Payments, etc.). The protocol crate
//! owns the interface; this crate owns implementations.

use serde::{Deserialize, Serialize};

use crate::error::FiatError;
use crate::payment::{PaymentId, PaymentRequest, PaymentStatus};
use crate::verification::VerifiedPayment;

/// Core trait for plugging in payment providers.
///
/// Implementations can connect to bank APIs, payment service providers,
/// or cash deposit verification systems.
pub trait PaymentAdapter: Send + Sync {
    /// The payment handle returned by this adapter.
    type Payment: Serialize + for<'de> Deserialize<'de> + Send + Sync;
    /// Error type for this adapter.
    type Error: std::error::Error + Send + Sync;

    /// Create a new payment request with the provider.
    fn create_payment(&self, request: PaymentRequest) -> Result<Self::Payment, Self::Error>;

    /// Check the current status of a payment.
    fn check_status(&self, payment: &Self::Payment) -> Result<PaymentStatus, Self::Error>;

    /// Verify a payment and return detailed verification results.
    fn verify(&self, payment: &Self::Payment) -> Result<VerifiedPayment, Self::Error>;

    /// Check payment status by payment ID (for automatic detection).
    fn check_payment_status(
        &self,
        payment_id: &uuid::Uuid,
    ) -> Result<Option<PaymentStatus>, Self::Error> {
        // Default implementation: not supported
        let _ = payment_id;
        Ok(None)
    }
}

// ─── Mock Payment Provider ──────────────────────────────────────────────────

/// Internal state of a mock payment for testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockPayment {
    pub id: PaymentId,
    pub request: PaymentRequest,
    pub status: PaymentStatus,
    /// Simulated verification data.
    pub verified: bool,
}

/// A mock payment provider for testing the fiat settlement lifecycle.
///
/// This simulates a bank transfer provider with configurable behavior:
/// - Payments can be created, detected, confirmed, or failed
/// - Verification succeeds if the payment is marked as verified
/// - All operations are in-memory for testing
#[derive(Debug, Default)]
pub struct MockPaymentProvider {
    /// Simulated payments stored by ID.
    payments: std::collections::HashMap<uuid::Uuid, MockPayment>,
    /// Auto-confirm payments on creation (for simple tests).
    pub auto_confirm: bool,
}

impl MockPaymentProvider {
    /// Create a new mock provider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a provider that auto-confirms payments.
    pub fn auto_confirm() -> Self {
        Self {
            payments: Default::default(),
            auto_confirm: true,
        }
    }

    /// Get the number of stored payments.
    pub fn payment_count(&self) -> usize {
        self.payments.len()
    }
}

impl PaymentAdapter for MockPaymentProvider {
    type Payment = MockPayment;
    type Error = FiatError;

    fn create_payment(&self, request: PaymentRequest) -> Result<MockPayment, FiatError> {
        let id = PaymentId(uuid::Uuid::new_v4());
        let status = if self.auto_confirm {
            PaymentStatus::Confirmed
        } else {
            PaymentStatus::Created
        };
        let payment = MockPayment {
            id,
            request,
            status,
            verified: self.auto_confirm,
        };
        // In a real implementation, this would be stored in a shared state.
        // For testing, we return the payment directly.
        Ok(payment)
    }

    fn check_status(&self, payment: &MockPayment) -> Result<PaymentStatus, FiatError> {
        Ok(payment.status.clone())
    }

    fn verify(&self, payment: &MockPayment) -> Result<VerifiedPayment, FiatError> {
        if !payment.verified {
            return Err(FiatError::SettlementError(
                "payment not yet verified".to_string(),
            ));
        }
        Ok(VerifiedPayment {
            payment_id: payment.id,
            currency: payment.request.amount.currency.clone(),
            amount: payment.request.amount.clone(),
            sender_reference: payment.request.reference.clone(),
            recipient_reference: payment.request.reference.clone(),
            verified_at: chrono::Utc::now(),
            status: PaymentStatus::Confirmed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendex_types::{FiatCurrency, ParticipantId, TradeId};
    use uuid::Uuid;

    fn make_request() -> PaymentRequest {
        PaymentRequest {
            trade_id: TradeId(Uuid::new_v4()),
            amount: crate::payment::FiatAmount::new(10000, FiatCurrency::GBP),
            reference: crate::payment::PaymentReference::generate(),
            payer_id: ParticipantId(Uuid::new_v4()),
            payee_id: ParticipantId(Uuid::new_v4()),
        }
    }

    #[test]
    fn mock_provider_creates_payment() {
        let provider = MockPaymentProvider::new();
        let payment = provider.create_payment(make_request()).unwrap();
        assert_eq!(payment.status, PaymentStatus::Created);
    }

    #[test]
    fn mock_provider_auto_confirm() {
        let provider = MockPaymentProvider::auto_confirm();
        let payment = provider.create_payment(make_request()).unwrap();
        assert_eq!(payment.status, PaymentStatus::Confirmed);
        assert!(payment.verified);
    }

    #[test]
    fn mock_provider_verify_unverified_fails() {
        let provider = MockPaymentProvider::new();
        let payment = provider.create_payment(make_request()).unwrap();
        assert!(provider.verify(&payment).is_err());
    }

    #[test]
    fn mock_provider_verify_confirmed_succeeds() {
        let provider = MockPaymentProvider::auto_confirm();
        let payment = provider.create_payment(make_request()).unwrap();
        let verified = provider.verify(&payment).unwrap();
        assert_eq!(verified.status, PaymentStatus::Confirmed);
    }
}
