//! Real payment provider implementations.
//!
//! Provides skeleton implementations for real-world payment providers:
//! - SEPA Credit Transfer (European bank transfers)
//! - UK Faster Payments (UK bank transfers)
//!
//! These are skeleton implementations that define the API structure
//! for integrating with real payment service providers.

use serde::{Deserialize, Serialize};

use crate::adapter::PaymentAdapter;
use crate::error::FiatError;
use crate::payment::{PaymentId, PaymentRequest, PaymentStatus};
use crate::verification::VerifiedPayment;

// ─── SEPA Credit Transfer ───────────────────────────────────────────────────

/// SEPA payment handle for tracking transfers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SepaPayment {
    /// Unique payment identifier.
    pub id: PaymentId,
    /// End-to-End transaction ID (e2e).
    pub end_to_end_id: String,
    /// SEPA transaction ID.
    pub sepa_transaction_id: String,
    /// Current status.
    pub status: PaymentStatus,
    /// IBAN of the payer.
    pub payer_iban: String,
    /// IBAN of the payee.
    pub payee_iban: String,
}

/// Configuration for SEPA payment provider.
#[derive(Debug, Clone)]
pub struct SepaProviderConfig {
    /// API endpoint for the payment service provider.
    pub api_endpoint: String,
    /// API key for authentication.
    pub api_key: String,
    /// Merchant IBAN for receiving payments.
    pub merchant_iban: String,
    /// Merchant BIC/SWIFT code.
    pub merchant_bic: String,
}

/// SEPA Credit Transfer payment adapter.
///
/// This is a skeleton implementation that defines the API structure
/// for integrating with a real SEPA payment service provider.
#[derive(Debug)]
pub struct SepaProvider {
    config: SepaProviderConfig,
    /// Stored payments for status checking (in production, this would be a database).
    payments: std::collections::HashMap<uuid::Uuid, SepaPayment>,
}

impl SepaProvider {
    /// Create a new SEPA provider.
    pub fn new(config: SepaProviderConfig) -> Self {
        Self {
            config,
            payments: std::collections::HashMap::new(),
        }
    }

    /// Generate a unique end-to-end transaction ID.
    fn generate_end_to_end_id() -> String {
        format!("E2E-{}", uuid::Uuid::new_v4().hyphenated().to_string()[..8].to_uppercase())
    }

    /// Generate a unique SEPA transaction ID.
    fn generate_sepa_transaction_id() -> String {
        format!("SEPA-{}", uuid::Uuid::new_v4().hyphenated().to_string()[..8].to_uppercase())
    }
}

impl PaymentAdapter for SepaProvider {
    type Payment = SepaPayment;
    type Error = FiatError;

    fn create_payment(&self, request: PaymentRequest) -> Result<SepaPayment, FiatError> {
        let id = PaymentId(uuid::Uuid::new_v4());

        let payment = SepaPayment {
            id,
            end_to_end_id: Self::generate_end_to_end_id(),
            sepa_transaction_id: Self::generate_sepa_transaction_id(),
            status: PaymentStatus::Created,
            payer_iban: "".to_string(), // Filled when payment is received
            payee_iban: self.config.merchant_iban.clone(),
        };

        // In a real implementation, this would:
        // 1. Call the PSP's API to register the expected payment
        // 2. Return the payment handle with tracking IDs
        // 3. Set up webhooks for payment notifications

        Ok(payment)
    }

    fn check_status(&self, payment: &SepaPayment) -> Result<PaymentStatus, FiatError> {
        // In a real implementation, this would:
        // 1. Query the PSP's API for the current payment status
        // 2. Map the PSP status to our PaymentStatus enum
        // 3. Return the current status

        Ok(payment.status.clone())
    }

    fn verify(&self, payment: &SepaPayment) -> Result<VerifiedPayment, FiatError> {
        // In a real implementation, this would:
        // 1. Verify the payment was received from the correct payer
        // 2. Confirm the amount matches the expected amount
        // 3. Check for any holds or reversals
        // 4. Return verification details

        if payment.status != PaymentStatus::Confirmed {
            return Err(FiatError::SettlementError(
                "SEPA payment not yet confirmed".to_string(),
            ));
        }

        Ok(VerifiedPayment {
            payment_id: payment.id,
            currency: opendex_types::FiatCurrency::EUR,
            amount: crate::payment::FiatAmount::new(0, opendex_types::FiatCurrency::EUR),
            sender_reference: crate::payment::PaymentReference(payment.end_to_end_id.clone()),
            recipient_reference: crate::payment::PaymentReference(payment.sepa_transaction_id.clone()),
            verified_at: chrono::Utc::now(),
            status: PaymentStatus::Confirmed,
        })
    }
}

// ─── UK Faster Payments ─────────────────────────────────────────────────────

/// UK Faster Payments handle for tracking transfers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UkFasterPayment {
    /// Unique payment identifier.
    pub id: PaymentId,
    /// Sort code of the payer.
    pub payer_sort_code: String,
    /// Account number of the payer.
    pub payer_account: String,
    /// Sort code of the payee.
    pub payee_sort_code: String,
    /// Account number of the payee.
    pub payee_account: String,
    /// Current status.
    pub status: PaymentStatus,
    /// Transaction reference.
    pub reference: String,
}

/// Configuration for UK Faster Payments provider.
#[derive(Debug, Clone)]
pub struct UkFasterPaymentsConfig {
    /// API endpoint for the payment service provider.
    pub api_endpoint: String,
    /// API key for authentication.
    pub api_key: String,
    /// Merchant sort code (e.g., "12-34-56").
    pub merchant_sort_code: String,
    /// Merchant account number.
    pub merchant_account: String,
    /// Merchant account name.
    pub merchant_name: String,
}

/// UK Faster Payments adapter.
///
/// This is a skeleton implementation that defines the API structure
/// for integrating with a real UK Faster Payments service provider.
#[derive(Debug)]
pub struct UkFasterPayments {
    config: UkFasterPaymentsConfig,
    /// Stored payments for status checking.
    payments: std::collections::HashMap<uuid::Uuid, UkFasterPayment>,
}

impl UkFasterPayments {
    /// Create a new UK Faster Payments provider.
    pub fn new(config: UkFasterPaymentsConfig) -> Self {
        Self {
            config,
            payments: std::collections::HashMap::new(),
        }
    }
}

impl PaymentAdapter for UkFasterPayments {
    type Payment = UkFasterPayment;
    type Error = FiatError;

    fn create_payment(&self, request: PaymentRequest) -> Result<UkFasterPayment, FiatError> {
        let id = PaymentId(uuid::Uuid::new_v4());

        let payment = UkFasterPayment {
            id,
            payer_sort_code: "".to_string(), // Filled when payment is received
            payer_account: "".to_string(),
            payee_sort_code: self.config.merchant_sort_code.clone(),
            payee_account: self.config.merchant_account.clone(),
            status: PaymentStatus::Created,
            reference: request.reference.0.clone(),
        };

        // In a real implementation, this would:
        // 1. Register the expected payment with the PSP
        // 2. Set up real-time payment notifications
        // 3. Return the payment handle

        Ok(payment)
    }

    fn check_status(&self, payment: &UkFasterPayment) -> Result<PaymentStatus, FiatError> {
        // In a real implementation, this would query the PSP's API
        Ok(payment.status.clone())
    }

    fn verify(&self, payment: &UkFasterPayment) -> Result<VerifiedPayment, FiatError> {
        if payment.status != PaymentStatus::Confirmed {
            return Err(FiatError::SettlementError(
                "UK Faster Payment not yet confirmed".to_string(),
            ));
        }

        Ok(VerifiedPayment {
            payment_id: payment.id,
            currency: opendex_types::FiatCurrency::GBP,
            amount: crate::payment::FiatAmount::new(0, opendex_types::FiatCurrency::GBP),
            sender_reference: crate::payment::PaymentReference(payment.reference.clone()),
            recipient_reference: crate::payment::PaymentReference(payment.reference.clone()),
            verified_at: chrono::Utc::now(),
            status: PaymentStatus::Confirmed,
        })
    }
}

// ─── Payment Detection Service ──────────────────────────────────────────────

/// Service for detecting incoming payments.
///
/// In a real implementation, this would:
/// - Poll the payment provider's API for new payments
/// - Process webhook notifications from the PSP
/// - Match incoming payments to expected trades
/// - Update payment status accordingly
pub struct PaymentDetectionService<PA>
where
    PA: PaymentAdapter,
{
    adapter: PA,
}

impl<PA> PaymentDetectionService<PA>
where
    PA: PaymentAdapter,
{
    /// Create a new payment detection service.
    pub fn new(adapter: PA) -> Self {
        Self { adapter }
    }

    /// Check for new payments (polling-based).
    ///
    /// In a real implementation, this would poll the PSP's API
    /// for any new or updated payments.
    pub fn poll_for_payments(&self) -> Result<Vec<PaymentStatus>, PA::Error> {
        // Placeholder — real implementation would poll the API
        Ok(Vec::new())
    }

    /// Handle a webhook notification from the PSP.
    ///
    /// In a real implementation, this would:
    /// 1. Verify the webhook signature
    /// 2. Parse the payment notification
    /// 3. Match it to an expected trade
    /// 4. Update the payment status
    pub fn handle_webhook(&self, payload: &[u8]) -> Result<(), PA::Error> {
        let _ = payload;
        // Placeholder — real implementation would process the webhook
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payment::PaymentReference;
    use opendex_types::{FiatCurrency, ParticipantId, TradeId};
    use uuid::Uuid;

    fn make_request() -> PaymentRequest {
        PaymentRequest {
            trade_id: TradeId(Uuid::new_v4()),
            amount: crate::payment::FiatAmount::new(10000, FiatCurrency::EUR),
            reference: PaymentReference::generate(),
            payer_id: ParticipantId(Uuid::new_v4()),
            payee_id: ParticipantId(Uuid::new_v4()),
        }
    }

    #[test]
    fn sepa_provider_creates_payment() {
        let config = SepaProviderConfig {
            api_endpoint: "https://api.example.com".to_string(),
            api_key: "test-key".to_string(),
            merchant_iban: "DE89370400440532013000".to_string(),
            merchant_bic: "COBADEFFXXX".to_string(),
        };
        let provider = SepaProvider::new(config);
        let payment = provider.create_payment(make_request()).unwrap();

        assert_eq!(payment.status, PaymentStatus::Created);
        assert!(!payment.end_to_end_id.is_empty());
        assert!(!payment.sepa_transaction_id.is_empty());
    }

    #[test]
    fn uk_faster_payments_creates_payment() {
        let config = UkFasterPaymentsConfig {
            api_endpoint: "https://api.example.com".to_string(),
            api_key: "test-key".to_string(),
            merchant_sort_code: "12-34-56".to_string(),
            merchant_account: "12345678".to_string(),
            merchant_name: "Test Merchant".to_string(),
        };
        let provider = UkFasterPayments::new(config);
        let payment = provider.create_payment(make_request()).unwrap();

        assert_eq!(payment.status, PaymentStatus::Created);
        assert_eq!(payment.payee_sort_code, "12-34-56");
    }

    #[test]
    fn sepa_provider_verify_unconfirmed_fails() {
        let config = SepaProviderConfig {
            api_endpoint: "https://api.example.com".to_string(),
            api_key: "test-key".to_string(),
            merchant_iban: "DE89370400440532013000".to_string(),
            merchant_bic: "COBADEFFXXX".to_string(),
        };
        let provider = SepaProvider::new(config);
        let payment = provider.create_payment(make_request()).unwrap();

        assert!(provider.verify(&payment).is_err());
    }

    #[test]
    fn payment_detection_service() {
        let config = SepaProviderConfig {
            api_endpoint: "https://api.example.com".to_string(),
            api_key: "test-key".to_string(),
            merchant_iban: "DE89370400440532013000".to_string(),
            merchant_bic: "COBADEFFXXX".to_string(),
        };
        let provider = SepaProvider::new(config);
        let service = PaymentDetectionService::new(provider);

        let payments = service.poll_for_payments().unwrap();
        assert!(payments.is_empty());
    }
}
