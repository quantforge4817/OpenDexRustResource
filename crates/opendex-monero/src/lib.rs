//! Monero chain interface and mock implementation.
//!
//! This crate defines the Monero chain trait used by the swap engine.
//! The mock implementation allows testing without a real Monero node.

use opendex_errors::ChainError;
use opendex_types::Piconero;

// ─── Monero Chain Trait ─────────────────────────────────────────────────────

/// Interface to the Monero blockchain.
pub trait MoneroChain: Send + Sync {
    /// Look up a transaction by its hash.
    fn transaction(
        &self,
        txid: &str,
    ) -> Result<Option<MoneroTransaction>, ChainError>;

    /// Get the current block height.
    fn height(&self) -> Result<u64, ChainError>;

    /// Broadcast a raw transaction.
    fn broadcast(&self, tx_hex: &str) -> Result<String, ChainError>;

    /// Get confirmation count for a transaction.
    fn confirmations(&self, txid: &str) -> Result<u32, ChainError>;
}

/// Simplified Monero transaction representation.
#[derive(Debug, Clone)]
pub struct MoneroTransaction {
    pub txid: String,
    pub amount: Piconero,
    pub confirmations: u32,
    pub block_height: Option<u64>,
}

// ─── Mock Monero Chain ──────────────────────────────────────────────────────

/// Mock Monero chain for testing.
#[derive(Debug, Default)]
pub struct MockMoneroChain {
    transactions: std::sync::Mutex<std::collections::HashMap<String, MoneroTransaction>>,
    broadcasted: std::sync::Mutex<Vec<String>>,
    current_height: std::sync::Mutex<u64>,
}

impl MockMoneroChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-register a transaction for testing.
    pub fn register_transaction(
        &self,
        txid: String,
        amount: Piconero,
        confirmations: u32,
    ) {
        let tx = MoneroTransaction {
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

impl MoneroChain for MockMoneroChain {
    fn transaction(&self, txid: &str) -> Result<Option<MoneroTransaction>, ChainError> {
        Ok(self.transactions.lock().unwrap().get(txid).cloned())
    }

    fn height(&self) -> Result<u64, ChainError> {
        Ok(*self.current_height.lock().unwrap())
    }

    fn broadcast(&self, tx_hex: &str) -> Result<String, ChainError> {
        let txid = format!("mock_xmr_{}", &tx_hex[..8]);
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
        let chain = MockMoneroChain::new();
        chain.register_transaction(
            "def456".to_string(),
            Piconero(100_000_000_000_000),
            20,
        );

        let tx = chain.transaction("def456").unwrap().unwrap();
        assert_eq!(tx.amount, Piconero(100_000_000_000_000));
        assert_eq!(tx.confirmations, 20);
    }

    #[test]
    fn mock_chain_broadcast_records_tx() {
        let chain = MockMoneroChain::new();
        let txid = chain.broadcast("0100000002...").unwrap();
        assert!(!txid.is_empty());
        assert_eq!(chain.get_broadcasted().len(), 1);
    }
}
