/// P2P trading protocol for fiat-to-crypto trades.
///
/// Extends the OpenDex protocol with support for peer-to-peer trading where
/// one party pays with fiat (cash, bank transfer, etc.) and the other
/// delivers XMR on-chain. Similar to Bisq's P2P trading model.
///
/// Key concepts:
/// - **PaymentMethod**: The fiat payment method used (bank transfer, cash, etc.)
/// - **P2POffer**: An offer to buy or sell XMR for a specific fiat currency
///   using a specific payment method
/// - **P2PTrade**: A trade created when an offer is taken, with XMR escrowed
///   on-chain via the Monero wallet
///
/// Trade lifecycle:
/// 1. Maker posts a P2POffer (e.g., "Sell 0.1 XMR for 60 EUR via BankTransfer")
/// 2. Taker takes the offer, creating a P2PTrade
/// 3. Maker's XMR is escrowed on-chain (sent to a subaddress)
/// 4. Taker sends fiat payment off-platform
/// 5. Taker marks payment as sent
/// 6. Maker confirms receipt of fiat
/// 7. XMR is released from escrow to taker's address
/// 8. Trade is settled

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::message::Side;

/// Crypto asset being traded in P2P fiat trades.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CryptoAsset {
    XMR,
    BTC,
}

impl std::fmt::Display for CryptoAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoAsset::XMR => write!(f, "XMR"),
            CryptoAsset::BTC => write!(f, "BTC"),
        }
    }
}

/// Supported fiat payment methods for P2P trading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PaymentMethod {
    /// Bank wire/SEPA transfer
    BankTransfer,
    /// Face-to-face cash exchange
    Cash,
    /// Revolut mobile payment
    Revolut,
    /// Wise (formerly TransferWise)
    Wise,
    /// PayPal
    PayPal,
    /// Zelle (US bank transfer)
    Zelle,
    /// Cash sent by mail
    CashByMail,
    /// Western Union money transfer
    WesternUnion,
    /// Custom payment method with a description
    Custom(String),
}

impl std::fmt::Display for PaymentMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentMethod::BankTransfer => write!(f, "Bank Transfer"),
            PaymentMethod::Cash => write!(f, "Cash"),
            PaymentMethod::Revolut => write!(f, "Revolut"),
            PaymentMethod::Wise => write!(f, "Wise"),
            PaymentMethod::PayPal => write!(f, "PayPal"),
            PaymentMethod::Zelle => write!(f, "Zelle"),
            PaymentMethod::CashByMail => write!(f, "Cash by Mail"),
            PaymentMethod::WesternUnion => write!(f, "Western Union"),
            PaymentMethod::Custom(desc) => write!(f, "{}", desc),
        }
    }
}

/// ISO 4217 fiat currency codes commonly used in P2P trading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FiatCurrency {
    USD,
    EUR,
    GBP,
    CAD,
    AUD,
    CHF,
    JPY,
    CNY,
    INR,
    BRL,
    RUB,
    KRW,
    TRY,
    Other(String),
}

impl std::fmt::Display for FiatCurrency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FiatCurrency::USD => write!(f, "USD"),
            FiatCurrency::EUR => write!(f, "EUR"),
            FiatCurrency::GBP => write!(f, "GBP"),
            FiatCurrency::CAD => write!(f, "CAD"),
            FiatCurrency::AUD => write!(f, "AUD"),
            FiatCurrency::CHF => write!(f, "CHF"),
            FiatCurrency::JPY => write!(f, "JPY"),
            FiatCurrency::CNY => write!(f, "CNY"),
            FiatCurrency::INR => write!(f, "INR"),
            FiatCurrency::BRL => write!(f, "BRL"),
            FiatCurrency::RUB => write!(f, "RUB"),
            FiatCurrency::KRW => write!(f, "KRW"),
            FiatCurrency::TRY => write!(f, "TRY"),
            FiatCurrency::Other(code) => write!(f, "{}", code),
        }
    }
}

/// Scale for fiat amounts: 2 decimal places (cents/pence/etc.).
/// For JPY and similar currencies without subunits, treat as whole units.
pub const FIAT_SCALE: u64 = 100;

/// State machine for P2P trades.
///
/// Lifecycle:
/// - OfferTaken: Taker accepted the offer
/// - EscrowFunded: Maker's crypto is locked on-chain
/// - FiatPaymentPending: Waiting for taker to send fiat
/// - FiatPaymentSent: Taker claims to have sent fiat
/// - FiatPaymentConfirmed: Maker confirms receipt
/// - CryptoReleased: Crypto sent from escrow to taker
/// - Settled: Trade complete
///
/// Exceptional states:
/// - Cancelled: Trade cancelled before completion
/// - Disputed: Dispute raised by either party
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum P2PTradeState {
    OfferTaken,
    EscrowFunded,
    FiatPaymentPending,
    FiatPaymentSent,
    FiatPaymentConfirmed,
    CryptoReleased,
    Settled,

    Cancelled,
    Disputed,
    Expired,
}

impl P2PTradeState {
    /// Check if a state transition is valid.
    pub fn can_transition_to(&self, next: &P2PTradeState) -> bool {
        use P2PTradeState::*;

        matches!(
            (self, next),
            (OfferTaken, EscrowFunded)
                | (OfferTaken, Cancelled)
                | (OfferTaken, Disputed)
                | (EscrowFunded, FiatPaymentPending)
                | (EscrowFunded, Cancelled)
                | (EscrowFunded, Disputed)
                | (FiatPaymentPending, FiatPaymentSent)
                | (FiatPaymentPending, Cancelled)
                | (FiatPaymentPending, Expired)
                | (FiatPaymentPending, Disputed)
                | (FiatPaymentSent, FiatPaymentConfirmed)
                | (FiatPaymentSent, Disputed)
                | (FiatPaymentConfirmed, CryptoReleased)
                | (FiatPaymentConfirmed, Disputed)
                | (CryptoReleased, Settled)
                | (CryptoReleased, Disputed)
                | (Disputed, Settled)
                | (Disputed, Cancelled)
        )
    }
}

/// A P2P offer to buy or sell crypto (XMR or BTC) for fiat currency.
///
/// Posted by a maker and broadcast to the network. Other peers can take
/// the offer to initiate a trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2POffer {
    /// Unique offer identifier
    pub id: Uuid,
    /// Trader who posted the offer
    pub maker: String,
    /// Crypto asset being traded
    pub crypto_asset: CryptoAsset,
    /// Buy or sell crypto
    pub side: Side,
    /// Crypto amount available at this price (scaled by AMOUNT_SCALE)
    pub amount: u64,
    /// Price in fiat per unit of crypto (scaled by FIAT_SCALE)
    /// e.g., 600000 = 6000.00 EUR/XMR or 50000000 = 500000.00 EUR/BTC
    pub price: u64,
    /// Fiat currency
    pub currency: FiatCurrency,
    /// Payment method required
    pub payment_method: PaymentMethod,
    /// Payment instructions for the counterparty
    /// e.g., bank account details, Revolut handle, etc.
    pub payment_instructions: String,
    /// Maximum time in seconds for the counterparty to complete payment
    pub payment_timeout_secs: u64,
    /// Minimum trade amount in crypto (for partial fills)
    pub min_amount: u64,
    /// Maximum trade amount in crypto (for partial fills)
    pub max_amount: u64,
    /// Offer expiration timestamp (Unix seconds)
    pub expires_at: u64,
    /// Created timestamp
    pub created_at: u64,
}

/// A P2P trade between a maker and taker.
///
/// Created when a taker accepts a P2POffer. Crypto is escrowed on-chain
/// and fiat is paid off-platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PTrade {
    /// Unique trade identifier
    pub id: Uuid,
    /// Reference to the original offer
    pub offer_id: Uuid,
    /// Maker (offer creator)
    pub maker: String,
    /// Taker (offer acceptor)
    pub taker: String,
    /// Crypto asset being traded
    pub crypto_asset: CryptoAsset,
    /// Buy or sell crypto (from taker's perspective)
    pub side: Side,
    /// Crypto amount being traded (scaled by AMOUNT_SCALE)
    pub amount: u64,
    /// Price in fiat per unit of crypto (scaled by FIAT_SCALE)
    pub price: u64,
    /// Total fiat amount (scaled by FIAT_SCALE)
    pub fiat_amount: u64,
    /// Fiat currency
    pub currency: FiatCurrency,
    /// Payment method used
    pub payment_method: PaymentMethod,
    /// Current trade state
    pub state: P2PTradeState,
    /// Taker's address for receiving crypto
    pub taker_crypto_address: String,
    /// Escrow address where crypto is locked
    pub escrow_address: String,
    /// Transaction hash of the escrow deposit
    pub escrow_tx_hash: Option<String>,
    /// Payment ID for tracking the escrow transaction
    pub payment_id: Option<String>,
    /// Fiat payment proof (receipt, transaction ID, etc.)
    pub fiat_payment_proof: Option<String>,
    /// Transaction hash of the crypto release to taker
    pub release_tx_hash: Option<String>,
    /// Created timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
}

impl P2PTrade {
    /// Advance the trade to a new state.
    pub fn transition(&mut self, next: P2PTradeState, timestamp: u64) -> anyhow::Result<()> {
        if self.state == next {
            return Ok(());
        }

        if !self.state.can_transition_to(&next) {
            return Err(anyhow::anyhow!(
                "invalid P2P trade state transition: {:?} -> {:?}",
                self.state,
                next
            ));
        }

        self.state = next;
        self.updated_at = timestamp;
        Ok(())
    }
}

/// Notification of a P2P trade state change, broadcast to the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PTradeStateChanged {
    pub trade_id: Uuid,
    pub state: P2PTradeState,
    pub timestamp: u64,
}

/// Cancellation of a P2P offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2POfferCancelled {
    pub offer_id: Uuid,
    pub maker: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trade() -> P2PTrade {
        P2PTrade {
            id: Uuid::new_v4(),
            offer_id: Uuid::new_v4(),
            maker: "maker".to_string(),
            taker: "taker".to_string(),
            crypto_asset: CryptoAsset::XMR,
            side: Side::Buy,
            amount: 100_000_000,
            price: 600000,
            fiat_amount: 60000000,
            currency: FiatCurrency::EUR,
            payment_method: PaymentMethod::BankTransfer,
            state: P2PTradeState::OfferTaken,
            taker_crypto_address: "4...".to_string(),
            escrow_address: "4...".to_string(),
            escrow_tx_hash: None,
            payment_id: None,
            fiat_payment_proof: None,
            release_tx_hash: None,
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn valid_p2p_trade_transition() {
        let mut trade = make_trade();
        assert!(trade.transition(P2PTradeState::EscrowFunded, 2).is_ok());
        assert_eq!(trade.state, P2PTradeState::EscrowFunded);
    }

    #[test]
    fn invalid_p2p_trade_transition() {
        let mut trade = make_trade();
        assert!(trade.transition(P2PTradeState::Settled, 2).is_err());
        assert_eq!(trade.state, P2PTradeState::OfferTaken);
    }

    #[test]
    fn complete_p2p_trade_lifecycle() {
        let mut trade = make_trade();
        let states = [
            P2PTradeState::EscrowFunded,
            P2PTradeState::FiatPaymentPending,
            P2PTradeState::FiatPaymentSent,
            P2PTradeState::FiatPaymentConfirmed,
            P2PTradeState::CryptoReleased,
            P2PTradeState::Settled,
        ];
        for (i, state) in states.into_iter().enumerate() {
            trade.transition(state, (i + 2) as u64).unwrap();
        }
        assert_eq!(trade.state, P2PTradeState::Settled);
    }

    #[test]
    fn payment_method_display() {
        assert_eq!(PaymentMethod::BankTransfer.to_string(), "Bank Transfer");
        assert_eq!(PaymentMethod::Cash.to_string(), "Cash");
        assert_eq!(PaymentMethod::Custom("Crypto ATM".to_string()).to_string(), "Crypto ATM");
    }

    #[test]
    fn fiat_currency_display() {
        assert_eq!(FiatCurrency::USD.to_string(), "USD");
        assert_eq!(FiatCurrency::EUR.to_string(), "EUR");
        assert_eq!(FiatCurrency::Other("XYZ".to_string()).to_string(), "XYZ");
    }

    #[test]
    fn crypto_asset_display() {
        assert_eq!(CryptoAsset::XMR.to_string(), "XMR");
        assert_eq!(CryptoAsset::BTC.to_string(), "BTC");
    }
}
