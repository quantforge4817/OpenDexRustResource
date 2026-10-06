//! Protocol registry and version negotiation.
//!
//! This crate owns the protocol registry, version negotiation, and
//! capability discovery. It does not own order matching, pricing, or UI.

use std::collections::HashMap;

use opendex_types::{ProtocolVersion, TradingPair};
use opendex_errors::ProtocolError;

// ─── Protocol Registry ──────────────────────────────────────────────────────

/// In-memory protocol registry that tracks which protocol versions
/// are registered for each trading pair.
///
/// Since `SettlementProtocol` has associated types, it cannot be used
/// as a trait object directly. The registry stores version information,
/// and the application resolves the concrete protocol type.
pub struct Registry {
    registered: HashMap<(String, String, String), ProtocolVersion>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            registered: HashMap::new(),
        }
    }

    /// Register a protocol version for a trading pair.
    pub fn register(
        &mut self,
        pair: TradingPair,
        version: ProtocolVersion,
    ) {
        let base = format!("{}", pair.base);
        let quote = format!("{}", pair.quote);
        let key = (base, quote, version.id.clone());
        self.registered.insert(key, version);
    }

    /// Check if a protocol version is registered for a trading pair.
    pub fn is_registered(
        &self,
        pair: &TradingPair,
        version: &ProtocolVersion,
    ) -> bool {
        let base = format!("{}", pair.base);
        let quote = format!("{}", pair.quote);
        let key = (base, quote, version.id.clone());
        self.registered.contains_key(&key)
    }

    /// Resolve the registered version for a trading pair.
    pub fn resolve(
        &self,
        pair: &TradingPair,
        version: &ProtocolVersion,
    ) -> Result<ProtocolVersion, ProtocolError> {
        let base = format!("{}", pair.base);
        let quote = format!("{}", pair.quote);
        let key = (base, quote, version.id.clone());
        self.registered
            .get(&key)
            .cloned()
            .ok_or_else(|| ProtocolError::UnsupportedVersion(version.id.clone()))
    }
}

// ─── Protocol Negotiation ───────────────────────────────────────────────────

/// Offer sent during protocol negotiation.
#[derive(Debug, Clone)]
pub struct ProtocolOffer {
    pub swap_id: opendex_types::SwapId,
    pub supported: Vec<ProtocolVersion>,
}

/// Response selecting a common protocol version.
#[derive(Debug, Clone)]
pub struct ProtocolSelection {
    pub swap_id: opendex_types::SwapId,
    pub selected: ProtocolVersion,
}

/// Find the best common protocol version between two participants.
pub fn negotiate(
    maker_supported: &[ProtocolVersion],
    taker_supported: &[ProtocolVersion],
) -> Result<ProtocolVersion, ProtocolError> {
    // Select the highest version both support
    maker_supported
        .iter()
        .rev()
        .find(|v| taker_supported.contains(v))
        .cloned()
        .ok_or(ProtocolError::NoCommonProtocol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiate_finds_common_version() {
        let maker = vec![
            ProtocolVersion::btc_xmr_v1(),
        ];
        let taker = vec![
            ProtocolVersion::btc_xmr_v1(),
            ProtocolVersion::new("OPENDEX-BTC-XMR-2", 2, 0),
        ];

        let selected = negotiate(&maker, &taker).unwrap();
        assert_eq!(selected, ProtocolVersion::btc_xmr_v1());
    }

    #[test]
    fn negotiate_fails_with_no_common() {
        let maker = vec![ProtocolVersion::btc_xmr_v1()];
        let taker = vec![ProtocolVersion::new("OTHER-PROTOCOL", 1, 0)];

        assert_eq!(negotiate(&maker, &taker).unwrap_err(), ProtocolError::NoCommonProtocol);
    }
}
