//! Fiat trade types and state machine.
//!
//! Defines the core trade structures for fiat-to-crypto trades, including
//! settlement methods, trade states, fee schedules, and deadlines.

use serde::{Deserialize, Serialize};

use opendex_types::{Asset, FiatCurrency, ParticipantId, TradeId};

use crate::payment::{FiatAmount, PaymentId, PaymentReference, PaymentRecipient};

// ─── Settlement Method ──────────────────────────────────────────────────────

/// How a fiat trade is settled.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SettlementMethod {
    /// Bank wire/SEPA transfer.
    BankTransfer {
        currency: FiatCurrency,
    },
    /// Cash deposited at a designated location.
    CashDeposit {
        currency: FiatCurrency,
    },
    /// In-person cash exchange.
    CashInPerson {
        currency: FiatCurrency,
    },
}

impl SettlementMethod {
    /// Get the currency for this settlement method.
    pub fn currency(&self) -> FiatCurrency {
        match self {
            SettlementMethod::BankTransfer { currency } => currency.clone(),
            SettlementMethod::CashDeposit { currency } => currency.clone(),
            SettlementMethod::CashInPerson { currency } => currency.clone(),
        }
    }
}

impl std::fmt::Display for SettlementMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettlementMethod::BankTransfer { currency } => {
                write!(f, "Bank Transfer ({})", currency)
            }
            SettlementMethod::CashDeposit { currency } => {
                write!(f, "Cash Deposit ({})", currency)
            }
            SettlementMethod::CashInPerson { currency } => {
                write!(f, "In-Person Cash ({})", currency)
            }
        }
    }
}

// ─── Fee Schedule ───────────────────────────────────────────────────────────

/// Fee configuration in basis points (100 bps = 1%).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeSchedule {
    /// Maker fee in basis points.
    pub maker_bps: u32,
    /// Taker fee in basis points.
    pub taker_bps: u32,
}

impl FeeSchedule {
    /// Default bank transfer fees: maker 0.25%, taker 0.50%.
    pub fn bank_transfer_default() -> Self {
        Self {
            maker_bps: 25,
            taker_bps: 50,
        }
    }

    /// Default cash fees: maker 0.50%, taker 1.00%.
    pub fn cash_default() -> Self {
        Self {
            maker_bps: 50,
            taker_bps: 100,
        }
    }
}

// ─── Settlement Deadlines ───────────────────────────────────────────────────

/// Timeouts for a fiat trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementDeadlines {
    /// Deadline for the taker to submit payment (Unix timestamp seconds).
    pub payment_deadline: u64,
    /// Deadline for payment confirmation (Unix timestamp seconds).
    pub confirmation_deadline: u64,
    /// Deadline for opening disputes (Unix timestamp seconds).
    pub dispute_deadline: u64,
}

// ─── Fiat Trade State ───────────────────────────────────────────────────────

/// State machine for fiat trades.
///
/// Automatic (adapter-based) lifecycle:
/// Created → Accepted → CryptoReserved → PaymentRequested → PaymentPending
///   → Detected → Confirming → Confirmed → CryptoRelease → Complete
///
/// Manual (button-based) lifecycle:
/// Created → Accepted → CryptoReserved → PaymentRequested → PaymentSubmitted
///   → MakerConfirmed → CryptoRelease → Complete
///
/// Exceptional paths:
/// - Any active state → Disputed
/// - PaymentPending → Failed (timeout)
/// - PaymentSubmitted → Failed (timeout)
/// - Complete → ReversalDetected → Dispute
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum FiatTradeState {
    /// Trade created, awaiting acceptance.
    Created,
    /// Both parties accepted the trade.
    Accepted,
    /// Crypto reserved in escrow.
    CryptoReserved,
    /// Payment instructions (bank details) sent to taker.
    PaymentRequested,
    /// Taker has submitted payment and is waiting for maker confirmation.
    /// In manual mode, taker clicks "I've paid" to reach this state.
    PaymentSubmitted,
    /// Waiting for taker to submit payment (adapter-based monitoring).
    PaymentPending,
    /// Payment detected by the adapter.
    Detected,
    /// Payment under confirmation.
    Confirming,
    /// Maker has confirmed receiving the payment (manual mode).
    /// In manual mode, maker clicks "I confirm payment" to reach this state.
    MakerConfirmed,
    /// Payment confirmed and cleared.
    Confirmed,
    /// Crypto being released to taker.
    CryptoRelease,
    /// Trade completed successfully.
    Complete,
    /// Payment reversal detected after completion.
    ReversalDetected,
    /// Trade is under dispute.
    Disputed,
    /// Trade failed (timeout, error, etc.).
    Failed,
    /// Trade cancelled by participant.
    Cancelled,
}

impl FiatTradeState {
    /// Validate a state transition.
    pub fn can_transition_to(&self, next: &FiatTradeState) -> bool {
        use FiatTradeState::*;
        matches!(
            (self, next),
            // Normal flow — both paths share the early states
            (Created, Accepted)
            | (Accepted, CryptoReserved)
            | (CryptoReserved, PaymentRequested)
            // Manual (button-based) flow: taker confirms payment, then maker confirms
            | (PaymentRequested, PaymentSubmitted)
            | (PaymentSubmitted, MakerConfirmed)
            | (MakerConfirmed, CryptoRelease)
            // Automatic (adapter-based) flow: payment pending, detected, confirmed
            | (PaymentRequested, PaymentPending)
            | (PaymentPending, Detected)
            | (Detected, Confirming)
            | (Confirming, Confirmed)
            | (Confirmed, CryptoRelease)
            | (CryptoRelease, Complete)
            // Timeout/failure paths
            | (PaymentPending, Failed)
            | (PaymentSubmitted, Failed)
            | (Confirming, Failed)
            | (CryptoReserved, Failed)
            | (PaymentRequested, Failed)
            | (MakerConfirmed, Failed)
            | (CryptoRelease, Failed)
            // Cancellation paths — both parties can cancel from any active state
            | (Created, Cancelled)
            | (Accepted, Cancelled)
            | (CryptoReserved, Cancelled)
            | (PaymentRequested, Cancelled)
            | (PaymentSubmitted, Cancelled)
            | (PaymentPending, Cancelled)
            | (Detected, Cancelled)
            | (Confirming, Cancelled)
            | (MakerConfirmed, Cancelled)
            | (Confirmed, Cancelled)
            // Dispute paths — can be opened from most active states
            | (Created, Disputed)
            | (Accepted, Disputed)
            | (CryptoReserved, Disputed)
            | (PaymentRequested, Disputed)
            | (PaymentSubmitted, Disputed)
            | (PaymentPending, Disputed)
            | (Detected, Disputed)
            | (Confirming, Disputed)
            | (MakerConfirmed, Disputed)
            | (Confirmed, Disputed)
            | (CryptoRelease, Disputed)
            | (Complete, ReversalDetected)
            | (ReversalDetected, Disputed)
            // Dispute resolution
            | (Disputed, Complete)
            | (Disputed, Failed)
            | (Disputed, Cancelled)
        )
    }
}

impl std::fmt::Display for FiatTradeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FiatTradeState::Created => write!(f, "Created"),
            FiatTradeState::Accepted => write!(f, "Accepted"),
            FiatTradeState::CryptoReserved => write!(f, "Crypto Reserved"),
            FiatTradeState::PaymentRequested => write!(f, "Payment Requested"),
            FiatTradeState::PaymentSubmitted => write!(f, "Payment Submitted"),
            FiatTradeState::PaymentPending => write!(f, "Payment Pending"),
            FiatTradeState::Detected => write!(f, "Detected"),
            FiatTradeState::Confirming => write!(f, "Confirming"),
            FiatTradeState::MakerConfirmed => write!(f, "Maker Confirmed"),
            FiatTradeState::Confirmed => write!(f, "Confirmed"),
            FiatTradeState::CryptoRelease => write!(f, "Crypto Release"),
            FiatTradeState::Complete => write!(f, "Complete"),
            FiatTradeState::ReversalDetected => write!(f, "Reversal Detected"),
            FiatTradeState::Disputed => write!(f, "Disputed"),
            FiatTradeState::Failed => write!(f, "Failed"),
            FiatTradeState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

// ─── Fiat Offer ─────────────────────────────────────────────────────────────

/// A P2P offer to buy or sell crypto for fiat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiatOffer {
    /// Unique trade identifier.
    pub trade_id: TradeId,
    /// Maker participant ID (offer creator).
    pub maker: ParticipantId,
    /// Fiat currency.
    pub currency: FiatCurrency,
    /// Crypto asset being traded.
    pub crypto_asset: Asset,
    /// Fiat amount.
    pub amount: FiatAmount,
    /// Price of crypto in fiat.
    pub price: opendex_types::Price,
    /// Settlement method.
    pub settlement_method: SettlementMethod,
    /// Time window for settlement (seconds).
    pub settlement_window: u64,
    /// Bond requirement in basis points.
    pub bond_requirement: u32,
}

impl FiatOffer {
    /// Calculate the crypto amount from the fiat amount and price.
    /// Price is numerator/denominator (e.g., fiat per unit of crypto).
    pub fn calculate_crypto_amount(&self) -> u64 {
        if self.price.denominator == 0 {
            return 0;
        }
        // crypto_amount = fiat_amount * denominator / numerator
        let fiat_minor = self.amount.minor_units as u128;
        let crypto = fiat_minor * self.price.denominator / self.price.numerator;
        crypto as u64
    }
}

// ─── Fiat Trade ─────────────────────────────────────────────────────────────

/// A fiat-to-crypto trade with full state tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiatTrade {
    /// Unique trade identifier.
    pub id: TradeId,
    /// Maker (offer creator, sells crypto, receives fiat).
    pub maker: ParticipantId,
    /// Taker (offer acceptor, buys crypto, sends fiat).
    pub taker: ParticipantId,
    /// Crypto asset being traded.
    pub crypto_asset: Asset,
    /// Fiat currency.
    pub fiat_currency: FiatCurrency,
    /// Crypto amount (in satoshis or piconeros).
    pub crypto_amount: u64,
    /// Fiat amount (in minor units).
    pub fiat_amount: FiatAmount,
    /// Settlement method.
    pub settlement_method: SettlementMethod,
    /// Fee schedule.
    pub fee_schedule: FeeSchedule,
    /// Current trade state.
    pub state: FiatTradeState,
    /// Maker's bank details for receiving fiat payment.
    pub maker_bank_details: Option<PaymentRecipient>,
    /// Taker's crypto wallet address for receiving crypto.
    pub taker_crypto_address: String,
    /// Payment ID (assigned when payment is created).
    pub payment_id: Option<PaymentId>,
    /// Payment reference for reconciliation.
    pub payment_reference: Option<PaymentReference>,
    /// Deadlines for the trade.
    pub deadlines: SettlementDeadlines,
    /// Created timestamp (Unix seconds).
    pub created_at: u64,
    /// Last updated timestamp (Unix seconds).
    pub updated_at: u64,
}

impl FiatTrade {
    /// Advance the trade to a new state.
    pub fn transition(&mut self, next: FiatTradeState, timestamp: u64) -> anyhow::Result<()> {
        if self.state == next {
            return Err(anyhow::anyhow!(
                "trade already in state: {}",
                self.state
            ));
        }

        if !self.state.can_transition_to(&next) {
            return Err(anyhow::anyhow!(
                "invalid fiat trade state transition: {} -> {}",
                self.state,
                next
            ));
        }

        self.state = next;
        self.updated_at = timestamp;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_trade() -> FiatTrade {
        FiatTrade {
            id: TradeId(Uuid::new_v4()),
            maker: ParticipantId(Uuid::new_v4()),
            taker: ParticipantId(Uuid::new_v4()),
            crypto_asset: Asset::Btc,
            fiat_currency: FiatCurrency::GBP,
            crypto_amount: 1_000_000,
            fiat_amount: FiatAmount::new(7500000, FiatCurrency::GBP),
            settlement_method: SettlementMethod::BankTransfer {
                currency: FiatCurrency::GBP,
            },
            fee_schedule: FeeSchedule::bank_transfer_default(),
            state: FiatTradeState::Created,
            maker_bank_details: None,
            taker_crypto_address: "bc1qtest123".to_string(),
            payment_id: None,
            payment_reference: None,
            deadlines: SettlementDeadlines {
                payment_deadline: 1000000,
                confirmation_deadline: 2000000,
                dispute_deadline: 3000000,
            },
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn valid_fiat_trade_transition() {
        let mut trade = make_trade();
        assert!(trade.transition(FiatTradeState::Accepted, 2).is_ok());
        assert_eq!(trade.state, FiatTradeState::Accepted);
    }

    #[test]
    fn invalid_fiat_trade_transition() {
        let mut trade = make_trade();
        assert!(trade.transition(FiatTradeState::Complete, 2).is_err());
        assert_eq!(trade.state, FiatTradeState::Created);
    }

    #[test]
    fn complete_fiat_trade_lifecycle() {
        let mut trade = make_trade();
        let states = [
            FiatTradeState::Accepted,
            FiatTradeState::CryptoReserved,
            FiatTradeState::PaymentRequested,
            FiatTradeState::PaymentPending,
            FiatTradeState::Detected,
            FiatTradeState::Confirming,
            FiatTradeState::Confirmed,
            FiatTradeState::CryptoRelease,
            FiatTradeState::Complete,
        ];
        for (i, state) in states.into_iter().enumerate() {
            trade.transition(state, (i + 2) as u64).unwrap();
        }
        assert_eq!(trade.state, FiatTradeState::Complete);
    }

    #[test]
    fn manual_fiat_trade_lifecycle() {
        // Manual flow: taker confirms payment, maker confirms receipt, crypto released
        let mut trade = make_trade();
        let states = [
            FiatTradeState::Accepted,
            FiatTradeState::CryptoReserved,
            FiatTradeState::PaymentRequested,
            FiatTradeState::PaymentSubmitted,  // Taker clicks "I've paid"
            FiatTradeState::MakerConfirmed,    // Maker clicks "I confirm payment"
            FiatTradeState::CryptoRelease,
            FiatTradeState::Complete,
        ];
        for (i, state) in states.into_iter().enumerate() {
            trade.transition(state, (i + 2) as u64).unwrap();
        }
        assert_eq!(trade.state, FiatTradeState::Complete);
    }

    #[test]
    fn manual_flow_invalid_skip_maker_confirm() {
        // Taker cannot skip from PaymentSubmitted directly to CryptoRelease
        let mut trade = make_trade();
        trade.transition(FiatTradeState::Accepted, 2).unwrap();
        trade.transition(FiatTradeState::CryptoReserved, 3).unwrap();
        trade.transition(FiatTradeState::PaymentRequested, 4).unwrap();
        trade.transition(FiatTradeState::PaymentSubmitted, 5).unwrap();
        // Should fail: maker must confirm first
        assert!(trade.transition(FiatTradeState::CryptoRelease, 6).is_err());
    }

    #[test]
    fn settlement_method_display() {
        let method = SettlementMethod::BankTransfer {
            currency: FiatCurrency::GBP,
        };
        assert_eq!(method.to_string(), "Bank Transfer (GBP)");
    }

    #[test]
    fn fee_schedule_defaults() {
        let fees = FeeSchedule::bank_transfer_default();
        assert_eq!(fees.maker_bps, 25);
        assert_eq!(fees.taker_bps, 50);
    }
}
