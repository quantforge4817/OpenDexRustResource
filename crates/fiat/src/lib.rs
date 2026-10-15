//! Fiat settlement subsystem for OpenDex P2P trading.
//!
//! This crate implements the fiat side of the OpenDex protocol, enabling
//! trades between fiat currencies (GBP, EUR, USD) and cryptoassets (BTC, XMR).
//!
//! Unlike BTC/XMR atomic swaps, fiat settlement cannot be cryptographically
//! atomic. Instead, it uses payment adapters, verification, security bonds,
//! and dispute resolution to manage the inherent trust requirements.
//!
//! # Architecture
//!
//! ```text
//! FIAT TRADE
//!     │
//!     ▼
//! Fiat Settlement Engine
//!     │
//!  ┌──┴──┐
//!  │     │
//!  ▼     ▼
//! Payment Adapter    Crypto Wallet
//!  │     │
//!  ▼     ▼
//! Bank/PSP     BTC/XMR
//!  │
//!  ▼
//! Payment Verification
//!  │
//!  ▼
//! Settlement Decision
//!  │
//!  ▼
//! Crypto Release
//! ```
//!
//! # Key Concepts
//!
//! - **PaymentAdapter**: Trait for plugging in bank APIs (SEPA, UK Faster Payments, etc.)
//! - **FiatTrade**: A trade between fiat and crypto with a full state machine
//! - **SecurityBond**: Locked funds from both parties to incentivize good behavior
//! - **Dispute**: Resolution mechanism when payment verification fails
//! - **MockPaymentProvider**: Test implementation for development and testing
//! - **SettlementEngine**: Orchestrates the complete fiat trade lifecycle
//! - **P2P Bridge**: Converts between fiat and P2P protocol types
//! - **CryptoWallet**: Trait for BTC/XMR escrow operations

pub mod payment;
pub mod adapter;
pub mod verification;
pub mod trade;
pub mod bond;
pub mod dispute;
pub mod error;
pub mod settlement_engine;
pub mod p2p_bridge;
pub mod real_payment_provider;
pub mod crypto_wallet;
pub mod multisig;

#[cfg(test)]
mod lifecycle_tests;

#[cfg(kani)]
mod bond_kani;

#[cfg(kani)]
mod trade_kani;

// Re-export core types for convenience
pub use payment::{
    FiatAmount, PaymentId, PaymentReference, PaymentInstructions,
    PaymentRecipient, PaymentRequest, PaymentStatus, PaymentOperation,
};
pub use adapter::{PaymentAdapter, MockPaymentProvider};
pub use verification::{VerifiedPayment, PaymentVerificationError};
pub use trade::{
    FiatTrade, FiatTradeState, SettlementMethod, FeeSchedule,
    SettlementDeadlines, FiatOffer,
};
pub use bond::{SecurityBond, BondState, BondRequirement};
pub use dispute::{
    Dispute, DisputeId, DisputeReason, DisputeState, Evidence, EvidenceType,
};
pub use error::{FiatError, SettlementError};
pub use settlement_engine::{SettlementEngine, SettlementEvent, CryptoWallet};
pub use p2p_bridge::{
    fiat_offer_to_p2p, p2p_to_fiat_offer, fiat_trade_to_p2p,
    p2p_to_fiat_trade_state, fiat_trade_state_to_p2p,
    fiat_currency_to_p2p, p2p_to_fiat_currency,
    settlement_method_to_p2p, p2p_to_settlement_method,
    crypto_amount_from_fiat, fiat_amount_from_crypto,
};
pub use real_payment_provider::{
    SepaProvider, SepaProviderConfig, SepaPayment,
    UkFasterPayments, UkFasterPaymentsConfig, UkFasterPayment,
    PaymentDetectionService,
};
pub use crypto_wallet::{
    // Multisig mode
    MultisigMode,
    // Monero wallet adapter trait and types
    XmrWalletAdapterTrait, WalletInfo, Balance, MoneroTransaction, SignedTransaction, WalletError,
    // Monero wallet implementations
    SimulatedXmrWallet, NativeXmrWallet, XmrEscrowState,
    // Bitcoin wallet
    BtcWalletAdapter, BtcEscrowState,
    // Combined wallets
    CombinedWalletAdapter, CombinedWalletAdapterNative,
    // Mock
    MockCryptoWallet,
};
pub use multisig::{
    EscrowKeyPair, RedemptionScript, PartialSignature, MultisigEscrowState,
    MultisigEscrow, MultisigManager,
};
