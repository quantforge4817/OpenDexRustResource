//! Core domain types for OpenDex.
//!
//! This crate provides the foundational types used across all OpenDex crates:
//! identifiers, assets, amounts, prices, and protocol versions.
//!
//! Following the specification, monetary values use integer-based types
//! (Satoshis, Piconero) rather than floating point.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Identifiers ────────────────────────────────────────────────────────────

/// Unique identifier for an atomic swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SwapId(pub Uuid);

/// Unique identifier for a marketplace trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradeId(pub Uuid);

/// Unique identifier for an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub Uuid);

/// Unique identifier for a participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParticipantId(pub Uuid);

/// Unique identifier for a protocol message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

// ─── Assets ─────────────────────────────────────────────────────────────────

/// Asset types traded on OpenDex.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Asset {
    Btc,
    Xmr,
    Fiat(FiatCurrency),
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Asset::Btc => write!(f, "BTC"),
            Asset::Xmr => write!(f, "XMR"),
            Asset::Fiat(currency) => write!(f, "{currency}"),
        }
    }
}

/// Fiat currencies supported for P2P trading.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FiatCurrency {
    GBP,
    EUR,
    USD,
}

impl std::fmt::Display for FiatCurrency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FiatCurrency::GBP => write!(f, "GBP"),
            FiatCurrency::EUR => write!(f, "EUR"),
            FiatCurrency::USD => write!(f, "USD"),
        }
    }
}

// ─── Money Types ────────────────────────────────────────────────────────────

/// Bitcoin amount in satoshis (integer-based, never f64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Satoshis(pub u64);

impl Satoshis {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn checked_add(self, other: Satoshis) -> Option<Satoshis> {
        self.0.checked_add(other.0).map(Satoshis)
    }

    pub fn checked_sub(self, other: Satoshis) -> Option<Satoshis> {
        self.0.checked_sub(other.0).map(Satoshis)
    }

    /// Convert satoshis to BTC as a string with 8 decimal places.
    pub fn to_btc_string(&self) -> String {
        let whole = self.0 / 100_000_000;
        let frac = self.0 % 100_000_000;
        format!("{whole}.{frac:08}")
    }
}

impl std::fmt::Display for Satoshis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} BTC", self.to_btc_string())
    }
}

/// Monero amount in piconeros (integer-based, never f64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Piconero(pub u64);

impl Piconero {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn checked_add(self, other: Piconero) -> Option<Piconero> {
        self.0.checked_add(other.0).map(Piconero)
    }

    pub fn checked_sub(self, other: Piconero) -> Option<Piconero> {
        self.0.checked_sub(other.0).map(Piconero)
    }

    /// Convert piconeros to XMR as a string with 12 decimal places.
    pub fn to_xmr_string(&self) -> String {
        let whole = self.0 / 1_000_000_000_000;
        let frac = self.0 % 1_000_000_000_000;
        format!("{whole}.{frac:012}")
    }
}

impl std::fmt::Display for Piconero {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} XMR", self.to_xmr_string())
    }
}

/// Generic amount tied to a specific asset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Amount {
    pub asset: Asset,
    pub value: u64,
}

// ─── Price ──────────────────────────────────────────────────────────────────

/// Fixed-point price representation (numerator/denominator, never f64).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Price {
    pub base: Asset,
    pub quote: Asset,
    pub numerator: u128,
    pub denominator: u128,
}

impl Price {
    pub fn new(base: Asset, quote: Asset, numerator: u128, denominator: u128) -> Self {
        Self { base, quote, numerator, denominator }
    }
}

// ─── Trading Pair ───────────────────────────────────────────────────────────

/// A trading pair (base/quote).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradingPair {
    pub base: Asset,
    pub quote: Asset,
}

impl std::fmt::Display for TradingPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)
    }
}

// ─── Protocol Version ───────────────────────────────────────────────────────

/// Identifies a specific settlement protocol version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub id: String,
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub fn new(id: impl Into<String>, major: u16, minor: u16) -> Self {
        Self { id: id.into(), major, minor }
    }

    /// The canonical BTC-XMR protocol identifier.
    pub fn btc_xmr_v1() -> Self {
        Self::new("OPENDEX-BTC-XMR-1", 1, 0)
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} v{}.{}", self.id, self.major, self.minor)
    }
}

// ─── Side ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "Buy"),
            Side::Sell => write!(f, "Sell"),
        }
    }
}

// ─── Blockchain Networks ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BitcoinNetwork {
    Mainnet,
    Testnet,
    Signet,
    Regtest,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoneroNetwork {
    Mainnet,
    Testnet,
    Stagenet,
    Local,
}

// ─── Fee Policy ─────────────────────────────────────────────────────────────

/// Fee configuration in basis points (100 bps = 1%).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeePolicy {
    pub maker_fee_bps: u32,
    pub taker_fee_bps: u32,
}

impl FeePolicy {
    /// Default BTC/XMR fees: maker 0.10%, taker 0.30%.
    pub fn btc_xmr_default() -> Self {
        Self {
            maker_fee_bps: 10,
            taker_fee_bps: 30,
        }
    }
}
