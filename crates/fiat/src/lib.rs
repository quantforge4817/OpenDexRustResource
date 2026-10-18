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

#![allow(dead_code, unused_variables, unused_imports)]
#![allow(warnings)]

pub mod adapter;
pub mod bond;
pub mod crypto_wallet;
pub mod dispute;
pub mod error;
pub mod multisig;
pub mod p2p_bridge;
pub mod payment;
pub mod real_payment_provider;
pub mod settlement_engine;
pub mod trade;
pub mod verification;

#[cfg(test)]
mod lifecycle_tests;

#[cfg(kani)]
mod bond_kani;

#[cfg(kani)]
mod trade_kani;

// Re-export core types for convenience
pub use adapter::{MockPaymentProvider, PaymentAdapter};
pub use bond::{BondRequirement, BondState, SecurityBond};
pub use crypto_wallet::{
    Balance,
    BtcEscrowState,
    // Bitcoin wallet
    BtcWalletAdapter,
    // Combined wallets
    CombinedWalletAdapter,
    CombinedWalletAdapterNative,
    // Mock
    MockCryptoWallet,
    MoneroTransaction,
    // Multisig mode
    MultisigMode,
    NativeXmrWallet,
    SignedTransaction,
    // Monero wallet implementations
    SimulatedXmrWallet,
    WalletError,
    WalletInfo,
    XmrEscrowState,
    // Monero wallet adapter trait and types
    XmrWalletAdapterTrait,
};
pub use dispute::{Dispute, DisputeId, DisputeReason, DisputeState, Evidence, EvidenceType};
pub use error::{FiatError, SettlementError};
pub use multisig::{
    EscrowKeyPair, MultisigEscrow, MultisigEscrowState, MultisigManager, PartialSignature,
    RedemptionScript,
};
pub use p2p_bridge::{
    crypto_amount_from_fiat, fiat_amount_from_crypto, fiat_currency_to_p2p, fiat_offer_to_p2p,
    fiat_trade_state_to_p2p, fiat_trade_to_p2p, p2p_to_fiat_currency, p2p_to_fiat_offer,
    p2p_to_fiat_trade_state, p2p_to_settlement_method, settlement_method_to_p2p,
};
pub use payment::{
    FiatAmount, PaymentId, PaymentInstructions, PaymentOperation, PaymentRecipient,
    PaymentReference, PaymentRequest, PaymentStatus,
};
pub use real_payment_provider::{
    PaymentDetectionService, SepaPayment, SepaProvider, SepaProviderConfig, UkFasterPayment,
    UkFasterPayments, UkFasterPaymentsConfig,
};
pub use settlement_engine::{CryptoWallet, SettlementEngine, SettlementEvent};
pub use trade::{
    FeeSchedule, FiatOffer, FiatTrade, FiatTradeState, SettlementDeadlines, SettlementMethod,
};
pub use verification::{PaymentVerificationError, VerifiedPayment};
