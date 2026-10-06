//! Bitcoin chain interface and mock implementation.
//!
//! This crate defines the Bitcoin chain trait used by the swap engine.
//! The mock implementation allows testing without a real Bitcoin node.

use opendex_errors::ChainError;
use opendex_types::Satoshis;

// ─── Bitcoin Chain Trait ────────────────────────────────────────────────────

/// Interface to the Bitcoin blockchain.
///
/// Implementations can use Bitcoin Core RPC, Electrum, Esplora, or any
/// other backend without changing the swap engine.
pub trait BitcoinChain: Send + Sync {
    /// Look up a transaction by its ID.
    fn transaction(
        &self,
        txid: &str,
    ) -> Result<Option<BitcoinTransaction>, ChainError>;

    /// Get the current block tip height.
    fn tip(&self) -> Result<u64, ChainError>;

    /// Broadcast a raw transaction.
    fn broadcast(&self, tx_hex: &str) -> Result<String, ChainError>;

    /// Get confirmation count for a transaction.
    fn confirmations(&self, txid: &str) -> Result<u32, ChainError>;
}

/// Simplified Bitcoin transaction representation.
#[derive(Debug, Clone)]
pub struct BitcoinTransaction {
    pub txid: String,
    pub amount: Satoshis,
    pub confirmations: u32,
    pub block_height: Option<u64>,
}

// ─── Mock Bitcoin Chain ─────────────────────────────────────────────────────

/// Mock Bitcoin chain for testing.
///
/// Simulates a blockchain that can be controlled by tests: transactions
/// can be pre-registered, confirmations can be set, and broadcasts are
/// recorded.
#[derive(Debug, Default)]
pub struct MockBitcoinChain {
    transactions: std::sync::Mutex<std::collections::HashMap<String, BitcoinTransaction>>,
    broadcasted: std::sync::Mutex<Vec<String>>,
    current_height: std::sync::Mutex<u64>,
}

impl MockBitcoinChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-register a transaction for testing.
    pub fn register_transaction(
        &self,
        txid: String,
        amount: Satoshis,
        confirmations: u32,
    ) {
        let tx = BitcoinTransaction {
            txid: txid.clone(),
            amount,
            confirmations,
            block_height: if confirmations > 0 {
                Some(self.current_height.lock().unwrap().saturating_sub(confirmations as u64))
            } else {
                None
            },
        };
        self.transactions.lock().unwrap().insert(txid, tx);
    }

    /// Set the current block height.
    pub fn set_height(&self, height: u64) {
        *self.current_height.lock().unwrap() = height;
    }

    /// Get all broadcasted transaction hexes.
    pub fn get_broadcasted(&self) -> Vec<String> {
        self.broadcasted.lock().unwrap().clone()
    }
}

impl BitcoinChain for MockBitcoinChain {
    fn transaction(&self, txid: &str) -> Result<Option<BitcoinTransaction>, ChainError> {
        Ok(self.transactions.lock().unwrap().get(txid).cloned())
    }

    fn tip(&self) -> Result<u64, ChainError> {
        Ok(*self.current_height.lock().unwrap())
    }

    fn broadcast(&self, tx_hex: &str) -> Result<String, ChainError> {
        let txid = format!("mock_btc_{}", &tx_hex[..8]);
        self.broadcasted.lock().unwrap().push(tx_hex.to_string());
        Ok(txid)
    }

    fn confirmations(&self, txid: &str) -> Result<u32, ChainError> {
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .get(txid)
            .map(|tx| tx.confirmations)
            .unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_chain_returns_registered_transaction() {
        let chain = MockBitcoinChain::new();
        chain.register_transaction(
            "abc123".to_string(),
            Satoshis(100_000_000),
            6,
        );

        let tx = chain.transaction("abc123").unwrap().unwrap();
        assert_eq!(tx.amount, Satoshis(100_000_000));
        assert_eq!(tx.confirmations, 6);
    }

    #[test]
    fn mock_chain_returns_none_for_unknown_transaction() {
        let chain = MockBitcoinChain::new();
        assert!(chain.transaction("unknown").unwrap().is_none());
    }

    #[test]
    fn mock_chain_broadcast_records_tx() {
        let chain = MockBitcoinChain::new();
        let txid = chain.broadcast("0200000001...").unwrap();
        assert!(!txid.is_empty());
        assert_eq!(chain.get_broadcasted().len(), 1);
    }
}
