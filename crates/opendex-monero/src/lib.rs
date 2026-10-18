//! Monero chain interface with real RPC and mock implementation.
//!
//! This crate defines the `MoneroChain` trait used by the swap engine.
//! `DaemonMoneroChain` provides real Monero blockchain access via JSON-RPC,
//! while `MockMoneroChain` allows testing without a real Monero node.
//!
//! # Real Monero Integration
//! The `monero-rpc` crate provides `RpcClient` with async methods.
//! To use it:
//!
//! ```ignore
//! let rpc = monero_rpc::RpcClient::new("http://127.0.0.1:18081".to_string())?;
//! let daemon = rpc.daemon();
//! let height = daemon.get_height().await?;
//! ```

use opendex_errors::ChainError;
use opendex_types::Piconero;

// ─── Real Monero Chain via Daemon RPC ────────────────────────────────────────

/// Real Monero chain client using the Monero daemon JSON-RPC API.
///
/// Connects to a running `monerod` instance to query the Monero blockchain.
/// The `monero-rpc` crate provides async methods via `DaemonJsonRpcClient`.
pub struct DaemonMoneroChain {
    url: String,
}

impl DaemonMoneroChain {
    /// Create a new Monero daemon chain client configuration.
    ///
    /// # Arguments
    /// * `url` - The Monero daemon URL (e.g., "http://127.0.0.1:18081")
    pub fn new(url: &str) -> Result<Self, ChainError> {
        // Validate URL
        monero_rpc::RpcClientBuilder::new()
            .build(url.to_string())
            .map_err(|e| ChainError::ConnectionFailed(e.to_string()))?;
        Ok(Self {
            url: url.to_string(),
        })
    }

    /// Create a `monero_rpc::DaemonJsonRpcClient` for direct API access.
    pub fn daemon_client(&self) -> Result<monero_rpc::DaemonJsonRpcClient, ChainError> {
        let rpc = monero_rpc::RpcClientBuilder::new()
            .build(self.url.clone())
            .map_err(|e| ChainError::ConnectionFailed(e.to_string()))?;
        Ok(rpc.daemon())
    }
}

impl MoneroChain for DaemonMoneroChain {
    fn transaction(&self, txid: &str) -> Result<Option<MoneroTransaction>, ChainError> {
        // Parse the txid as a Monero hash
        let txid_bytes = hex::decode(txid)
            .map_err(|e| ChainError::SerializationError(format!("Invalid txid hex: {}", e)))?;

        if txid_bytes.len() != 32 {
            return Err(ChainError::SerializationError(
                "Monero txid must be 32 bytes".to_string(),
            ));
        }

        let crypto_hash = monero_rpc::monero::cryptonote::hash::Hash::new(&txid_bytes);

        // Create daemon RPC client and look up the transaction
        let rpc = monero_rpc::RpcClientBuilder::new()
            .build(self.url.clone())
            .map_err(|e| ChainError::ConnectionFailed(e.to_string()))?;
        let daemon = rpc.daemon_rpc();

        let txid_clone = txid.to_string();
        let result = tokio::runtime::Handle::current()
            .block_on(async { daemon.get_transactions(vec![crypto_hash], None, None).await })
            .map_err(|e| ChainError::RpcError(format!("get_transactions failed: {}", e)))?;

        // Extract transaction data from the response
        if let Some(ref txs) = result.txs {
            if let Some(tx) = txs.first() {
                let block_height = tx.block_height;

                // Compute confirmations from current height - block height
                let current_height = self.height()?;
                let confirmations = if let Some(bh) = block_height {
                    if current_height > bh {
                        (current_height - bh) as u32
                    } else {
                        0
                    }
                } else {
                    0
                };

                // Amount is not directly available from daemon RPC;
                // it requires parsing the transaction JSON or using wallet RPC
                // For now, return 0 as a placeholder
                let amount = Piconero(0);

                return Ok(Some(MoneroTransaction {
                    txid: txid_clone,
                    amount,
                    confirmations,
                    block_height,
                }));
            }
        }

        Ok(None)
    }

    fn height(&self) -> Result<u64, ChainError> {
        let rpc = monero_rpc::RpcClientBuilder::new()
            .build(self.url.clone())
            .map_err(|e| ChainError::ConnectionFailed(e.to_string()))?;
        let daemon = rpc.daemon();

        // get_block_count returns the number of blocks (height + 1)
        let count = tokio::runtime::Handle::current()
            .block_on(async { daemon.get_block_count().await })
            .map_err(|e| ChainError::RpcError(format!("get_block_count failed: {}", e)))?;

        // Block count is height + 1, so subtract 1
        let height = count.get();
        if height == 0 {
            return Ok(0);
        }
        Ok(height - 1)
    }

    fn broadcast(&self, _tx_hex: &str) -> Result<String, ChainError> {
        // relay_tx is available on the wallet client, not daemon
        // For daemon-only access, we use submit_block or relay through wallet
        // Since we don't have a wallet, use the daemon's relay_tx if available
        // The monero-rpc crate's DaemonJsonRpcClient doesn't have relay_tx
        // We need to use the wallet client or a different approach
        // For now, return an error indicating wallet is needed
        Err(ChainError::RpcError(
            "Transaction relay requires a Monero wallet client. Use submit_block for raw blocks."
                .to_string(),
        ))
    }

    fn confirmations(&self, txid: &str) -> Result<u32, ChainError> {
        // Look up the transaction to get its confirmations
        if let Some(tx) = self.transaction(txid)? {
            Ok(tx.confirmations)
        } else {
            Ok(0)
        }
    }
}

// ─── Monero Chain Trait ─────────────────────────────────────────────────────

/// Interface to the Monero blockchain.
pub trait MoneroChain: Send + Sync {
    /// Look up a transaction by its hash.
    fn transaction(&self, txid: &str) -> Result<Option<MoneroTransaction>, ChainError>;

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
    pub fn register_transaction(&self, txid: String, amount: Piconero, confirmations: u32) {
        let tx = MoneroTransaction {
            txid: txid.clone(),
            amount,
            confirmations,
            block_height: if confirmations > 0 {
                Some(
                    self.current_height
                        .lock()
                        .unwrap()
                        .saturating_sub(confirmations as u64),
                )
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
        chain.register_transaction("def456".to_string(), Piconero(100_000_000_000_000), 20);

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
