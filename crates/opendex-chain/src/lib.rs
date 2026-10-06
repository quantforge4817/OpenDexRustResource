//! Generic chain abstraction for OpenDex.
//!
//! Provides a common `Chain` trait that both `BitcoinChain` and `MoneroChain`
//! implement, enabling generic watcher and reconciliation code.

use opendex_errors::ChainError;

/// Generic blockchain chain abstraction.
///
/// Provides a common interface for chain-agnostic operations like
/// getting the current tip and looking up transactions.
pub trait Chain: Send + Sync {
    /// The transaction type for this chain.
    type Tx: Send + Sync + std::fmt::Debug + Clone;
    /// The transaction ID type for this chain.
    type TxId: Clone + std::fmt::Debug + PartialEq + Eq;
    /// The block height type for this chain.
    type BlockHeight: Clone + std::fmt::Debug + PartialEq + Eq;

    /// Get the current chain tip (latest block height).
    fn tip(&self) -> Result<Self::BlockHeight, ChainError>;

    /// Look up a transaction by its ID.
    fn get_transaction(&self, txid: &Self::TxId) -> Result<Option<Self::Tx>, ChainError>;

    /// Broadcast a raw transaction to the network.
    fn broadcast(&self, raw_tx: &str) -> Result<Self::TxId, ChainError>;

    /// Get the number of confirmations for a transaction.
    fn confirmations(&self, txid: &Self::TxId) -> Result<u32, ChainError>;
}

// ─── BitcoinChain implements Chain ──────────────────────────────────────────

impl Chain for opendex_bitcoin::MockBitcoinChain {
    type Tx = opendex_bitcoin::BitcoinTransaction;
    type TxId = String;
    type BlockHeight = u64;

    fn tip(&self) -> Result<Self::BlockHeight, ChainError> {
        <Self as opendex_bitcoin::BitcoinChain>::tip(self)
    }

    fn get_transaction(&self, txid: &Self::TxId) -> Result<Option<Self::Tx>, ChainError> {
        <Self as opendex_bitcoin::BitcoinChain>::transaction(self, txid)
    }

    fn broadcast(&self, raw_tx: &str) -> Result<Self::TxId, ChainError> {
        <Self as opendex_bitcoin::BitcoinChain>::broadcast(self, raw_tx)
    }

    fn confirmations(&self, txid: &Self::TxId) -> Result<u32, ChainError> {
        <Self as opendex_bitcoin::BitcoinChain>::confirmations(self, txid)
    }
}

// ─── MoneroChain implements Chain ───────────────────────────────────────────

impl Chain for opendex_monero::MockMoneroChain {
    type Tx = opendex_monero::MoneroTransaction;
    type TxId = String;
    type BlockHeight = u64;

    fn tip(&self) -> Result<Self::BlockHeight, ChainError> {
        <Self as opendex_monero::MoneroChain>::height(self)
    }

    fn get_transaction(&self, txid: &Self::TxId) -> Result<Option<Self::Tx>, ChainError> {
        <Self as opendex_monero::MoneroChain>::transaction(self, txid)
    }

    fn broadcast(&self, raw_tx: &str) -> Result<Self::TxId, ChainError> {
        <Self as opendex_monero::MoneroChain>::broadcast(self, raw_tx)
    }

    fn confirmations(&self, txid: &Self::TxId) -> Result<u32, ChainError> {
        <Self as opendex_monero::MoneroChain>::confirmations(self, txid)
    }
}

// ─── Re-exports ─────────────────────────────────────────────────────────────

pub use opendex_bitcoin::{BitcoinChain, BitcoinTransaction, MockBitcoinChain};
pub use opendex_monero::{MoneroChain, MoneroTransaction, MockMoneroChain};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcoin_chain_trait_object() {
        let chain = MockBitcoinChain::new();
        chain.set_height(100);
        chain.register_transaction("abc123".to_string(), opendex_types::Satoshis(100_000), 3);

        // Test via Chain trait
        let tip = Chain::tip(&chain).unwrap();
        assert_eq!(tip, 100);

        let tx = Chain::get_transaction(&chain, &"abc123".to_string()).unwrap().unwrap();
        assert_eq!(tx.confirmations, 3);
    }

    #[test]
    fn test_monero_chain_trait_object() {
        let chain = MockMoneroChain::new();
        chain.set_height(200);
        chain.register_transaction("def456".to_string(), opendex_types::Piconero(2_000_000_000_000), 5);

        // Test via Chain trait
        let tip = Chain::tip(&chain).unwrap();
        assert_eq!(tip, 200);

        let tx = Chain::get_transaction(&chain, &"def456".to_string()).unwrap().unwrap();
        assert_eq!(tx.confirmations, 5);
    }

    #[test]
    fn test_bitcoin_broadcast_via_chain_trait() {
        let chain = MockBitcoinChain::new();
        let txid = Chain::broadcast(&chain, "0200000001...").unwrap();
        assert!(!txid.is_empty());
    }

    #[test]
    fn test_monero_broadcast_via_chain_trait() {
        let chain = MockMoneroChain::new();
        let txid = Chain::broadcast(&chain, "0100000002...").unwrap();
        assert!(!txid.is_empty());
    }
}

