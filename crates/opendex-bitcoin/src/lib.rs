//! Bitcoin chain interface with real Electrum RPC and mock implementation.
//!
//! This crate defines the `BitcoinChain` trait used by the swap engine.
//! `ElectrumBitcoinChain` provides real Bitcoin blockchain access via Electrum RPC,
//! while `MockBitcoinChain` allows testing without a real Bitcoin node.
//!
//! # Real Electrum Integration
//! The `electrum-client` crate provides the `Client` type with the `ElectrumApi` trait.
//! To use it, construct a client and call the trait methods:
//!
//! ```ignore
//! use electrum_client::{Client, ElectrumApi};
//! let client = Client::new("ssl://electrum.blockstream.info:50002")?;
//! let (header, height) = client.block_headers_subscribe()?;
//! ```

pub mod wallet;

use std::str::FromStr;

use opendex_errors::ChainError;
use opendex_types::Satoshis;

use bitcoin::{Txid, consensus::Decodable};
use electrum_client::ElectrumApi;

// ─── Transaction ID Utilities ────────────────────────────────────────────────

/// Parse a hex-encoded Bitcoin transaction ID string into a `bitcoin::Txid`.
///
/// # Arguments
/// * `txid_hex` - A hex-encoded transaction ID (64 hex characters, without `0x` prefix)
///
/// # Returns
/// The parsed `Txid` on success, or a `ChainError` if the hex is invalid.
pub fn parse_txid(txid_hex: &str) -> Result<Txid, ChainError> {
    Txid::from_str(txid_hex)
        .map_err(|e| ChainError::SerializationError(format!("Invalid txid hex: {}", e)))
}

/// Encode a `bitcoin::Txid` as a hex string.
///
/// # Arguments
/// * `txid` - The transaction ID to encode
///
/// # Returns
/// A 64-character lowercase hex string without `0x` prefix.
pub fn txid_to_hex(txid: &Txid) -> String {
    let bytes: &[u8] = txid.as_ref();
    hex::encode(bytes)
}

// ─── Real Bitcoin Chain via Electrum RPC ─────────────────────────────────────

/// Real Bitcoin chain client using Electrum RPC.
///
/// Connects to an Electrum server to query the Bitcoin blockchain for
/// transaction lookups, block height, confirmations, and broadcasting.
///
/// # Note
/// The `electrum-client` crate provides synchronous methods via the `ElectrumApi` trait.
/// This struct stores the client; callers should use `ElectrumApi` methods directly
/// for full functionality.
pub struct ElectrumBitcoinChain {
    url: String,
}

impl ElectrumBitcoinChain {
    /// Create a new Electrum Bitcoin chain client configuration.
    ///
    /// # Arguments
    /// * `server_url` - The Electrum server URL (e.g., "ssl://electrum.blockstream.info:50002")
    pub fn new(server_url: &str) -> Result<Self, ChainError> {
        // Validate the URL by attempting to create a client
        electrum_client::Client::new(server_url)
            .map_err(|e| ChainError::ConnectionFailed(e.to_string()))?;
        Ok(Self {
            url: server_url.to_string(),
        })
    }

    /// Create an `electrum_client::Client` for direct API access.
    pub fn client(&self) -> Result<electrum_client::Client, ChainError> {
        electrum_client::Client::new(&self.url)
            .map_err(|e| ChainError::ConnectionFailed(e.to_string()))
    }
}

impl BitcoinChain for ElectrumBitcoinChain {
    fn transaction(&self, txid: &str) -> Result<Option<BitcoinTransaction>, ChainError> {
        let txid_parsed = parse_txid(txid)?;
        let client = self.client()?;
        let raw_bytes = client.transaction_get_raw(&txid_parsed).map_err(|e| {
            ChainError::RpcError(format!("Electrum transaction_get_raw failed: {}", e))
        })?;

        // Parse the raw transaction to extract amount and other details
        let mut cursor = std::io::Cursor::new(&raw_bytes[..]);
        let tx = bitcoin::Transaction::consensus_decode(&mut cursor).map_err(|e| {
            ChainError::SerializationError(format!("Failed to decode transaction: {}", e))
        })?;

        // Calculate total output amount
        let total_amount: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();

        Ok(Some(BitcoinTransaction {
            txid: txid_to_hex(&txid_parsed),
            amount: Satoshis(total_amount),
            confirmations: 0, // Requires block height comparison
            block_height: None,
        }))
    }

    fn tip(&self) -> Result<u64, ChainError> {
        let client = self.client()?;
        let notification = client.block_headers_subscribe().map_err(|e| {
            ChainError::RpcError(format!("Electrum block_headers_subscribe failed: {}", e))
        })?;
        Ok(notification.height as u64)
    }

    fn broadcast(&self, tx_hex: &str) -> Result<String, ChainError> {
        let client = self.client()?;
        let raw_bytes = hex::decode(tx_hex).map_err(|e| {
            ChainError::SerializationError(format!("Invalid tx hex for broadcast: {}", e))
        })?;
        let txid = client.transaction_broadcast_raw(&raw_bytes).map_err(|e| {
            ChainError::RpcError(format!("Electrum transaction_broadcast_raw failed: {}", e))
        })?;
        Ok(txid_to_hex(&txid))
    }

    fn confirmations(&self, txid: &str) -> Result<u32, ChainError> {
        let txid_parsed = parse_txid(txid)?;
        let client = self.client()?;

        // Get the current tip height
        let tip_height = client
            .block_headers_subscribe()
            .map_err(|e| {
                ChainError::RpcError(format!("Electrum block_headers_subscribe failed: {}", e))
            })?
            .height;

        // Get the transaction to extract the locking script (output)
        let tx = client
            .transaction_get(&txid_parsed)
            .map_err(|e| ChainError::RpcError(format!("Electrum transaction_get failed: {}", e)))?;

        // Use the first output's script to query history
        if tx.output.is_empty() {
            return Ok(0);
        }
        let script = &tx.output[0].script_pubkey;

        // Query script history to find the block height
        let history = client.script_get_history(script).map_err(|e| {
            ChainError::RpcError(format!("Electrum script_get_history failed: {}", e))
        })?;

        // Find the confirmation for this txid
        let mut block_height: i32 = 0;
        for entry in history {
            if entry.tx_hash == txid_parsed {
                block_height = entry.height;
                break;
            }
        }

        // If height is 0, the transaction is unconfirmed (mempool)
        if block_height == 0 {
            return Ok(0);
        }

        // Compute confirmations as tip_height - block_height
        let block_height_usize = block_height as usize;
        let confirmations = if tip_height > block_height_usize {
            (tip_height - block_height_usize) as u32
        } else {
            0
        };
        Ok(confirmations)
    }
}

// ─── Bitcoin Chain Trait ────────────────────────────────────────────────────

/// Interface to the Bitcoin blockchain.
///
/// Implementations can use Bitcoin Core RPC, Electrum, Esplora, or any
/// other backend without changing the swap engine.
pub trait BitcoinChain: Send + Sync {
    /// Look up a transaction by its ID.
    fn transaction(&self, txid: &str) -> Result<Option<BitcoinTransaction>, ChainError>;

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
    pub fn register_transaction(&self, txid: String, amount: Satoshis, confirmations: u32) {
        let tx = BitcoinTransaction {
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
        chain.register_transaction("abc123".to_string(), Satoshis(100_000_000), 6);

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
