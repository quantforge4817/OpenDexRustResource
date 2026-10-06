/// Monero wallet client abstraction layer.
///
/// Connects OpenDex to a running `monero-wallet-rpc` service to perform
/// on-chain XMR operations: balance queries, transfers, payment monitoring,
/// and address management. This bridges the internal escrow system with
/// actual Monero blockchain transactions.
///
/// Uses raw JSON-RPC over HTTP via reqwest, avoiding native dependencies.
/// The wallet RPC must be running externally (e.g., `monero-wallet-rpc`
/// binary) and reachable at the configured address.
use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Monero amount scale: 1 XMR = 10^12 atomic units (piconero).
pub const XMR_SCALE: u64 = 1_000_000_000_000;

// ---------------------------------------------------------------------------
// JSON-RPC types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest<T: Serialize> {
    jsonrpc: String,
    id: String,
    method: String,
    params: T,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    id: String,
    result: T,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

// ---------------------------------------------------------------------------
// Monero wallet RPC response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GetBalanceResponse {
    balance: u64,
    unlocked_balance: u64,
}

#[derive(Debug, Deserialize)]
struct GetAddressResponse {
    address: String,
}

#[derive(Debug, Deserialize)]
struct CreateSubaddressResponse {
    address: String,
    index: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransferDestination {
    address: String,
    amount: u64,
}

#[derive(Debug, Deserialize)]
struct TransferResponse {
    tx_hash: String,
    fee: u64,
    #[serde(default)]
    tx_metadata: Option<String>,
    #[serde(default)]
    tx_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PaymentEntry {
    amount: u64,
    lock_time: u64,
    payment_id: String,
    tx_hash: String,
}

#[derive(Debug, Deserialize)]
struct GetPaymentsResponse {
    payments: Vec<PaymentEntry>,
}

#[derive(Debug, Deserialize)]
struct BulkPaymentEntry {
    amount: u64,
    payment_id: String,
    tx_hash: String,
}

#[derive(Debug, Deserialize)]
struct GetBulkPaymentsResponse {
    payments: Vec<BulkPaymentEntry>,
}

#[derive(Debug, Deserialize)]
struct CheckTxKeyResponse {
    in_pool: bool,
    confirmations: u64,
    unlocked: bool,
    #[serde(default)]
    confirmed: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GetHeightResponse {
    height: u64,
}

#[derive(Debug, Deserialize)]
struct StoreResponse {
    #[serde(default)]
    _dummy: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Config and state
// ---------------------------------------------------------------------------

/// Configuration for connecting to a Monero wallet RPC service.
#[derive(Debug, Clone)]
pub struct MoneroWalletConfig {
    /// RPC endpoint, e.g., "http://127.0.0.1:18082"
    pub rpc_url: String,
    /// Username for RPC authentication (if --rpc-login is set)
    pub username: Option<String>,
    /// Password for RPC authentication
    pub password: Option<String>,
}

impl Default for MoneroWalletConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:18082".to_string(),
            username: None,
            password: None,
        }
    }
}

/// Represents the connection state of the Monero wallet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletState {
    /// Wallet RPC is not configured or not connected
    Disconnected,
    /// Connected and ready to perform operations
    Connected,
    /// Connection failed with an error message
    Error(String),
}

/// A thin wrapper around the Monero wallet RPC providing
/// OpenDex-specific operations via raw JSON-RPC over HTTP.
pub struct MoneroWalletClient {
    http: reqwest::Client,
    config: MoneroWalletConfig,
    state: WalletState,
}

impl MoneroWalletClient {
    /// Create a disconnected wallet client with the given configuration.
    pub fn new(config: MoneroWalletConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
            state: WalletState::Disconnected,
        }
    }

    // -----------------------------------------------------------------------
    // Internal: JSON-RPC call helper
    // -----------------------------------------------------------------------

    /// Execute a JSON-RPC call with empty params.
    async fn call_no_params<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
    ) -> Result<T> {
        self.call::<(), T>(method, ()).await
    }

    /// Execute a JSON-RPC call with typed params.
    async fn call<P: Serialize, T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: P,
    ) -> Result<T> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "1".to_string(),
            method: method.to_string(),
            params,
        };

        let mut builder = self
            .http
            .post(&self.config.rpc_url)
            .json(&request);

        // Attach basic auth if configured
        if let (Some(user), Some(pass)) = (&self.config.username, &self.config.password) {
            builder = builder.basic_auth(user, Some(pass));
        }

        let response = builder
            .send()
            .await
            .context("Failed to send RPC request")?
            .json::<JsonRpcResponse<T>>()
            .await
            .context("Failed to parse RPC response")?;

        if let Some(err) = response.error {
            Err(anyhow!("RPC error {}: {}", err.code, err.message))
        } else {
            Ok(response.result)
        }
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Connect to the Monero wallet RPC service by testing with get_balance.
    pub async fn connect(&mut self) -> Result<()> {
        tracing::info!(
            url = %self.config.rpc_url,
            "Connecting to Monero wallet RPC"
        );

        match self.get_balance(0).await {
            Ok(_) => {
                self.state = WalletState::Connected;
                tracing::info!("Successfully connected to Monero wallet RPC");
                Ok(())
            }
            Err(e) => {
                self.state = WalletState::Error(format!("Connection test failed: {}", e));
                Err(anyhow!(
                    "Failed to connect to Monero wallet RPC: {}",
                    e
                ))
            }
        }
    }

    /// Check if the wallet is currently connected.
    pub fn is_connected(&self) -> bool {
        matches!(self.state, WalletState::Connected)
    }

    /// Get the current wallet state.
    pub fn state(&self) -> &WalletState {
        &self.state
    }

    /// Get the primary wallet address (account 0, address index 0).
    pub async fn get_address(&self) -> Result<String> {
        let resp: GetAddressResponse = self.call_no_params("get_address").await?;
        Ok(resp.address)
    }

    /// Create a new subaddress for receiving payments.
    /// Returns the new subaddress string.
    pub async fn create_subaddress(
        &self,
        account_index: u32,
        label: Option<String>,
    ) -> Result<String> {
        #[derive(Serialize)]
        struct Params {
            account_index: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            label: Option<String>,
        }

        let resp: CreateSubaddressResponse = self
            .call(
                "create_subaddress",
                Params {
                    account_index,
                    label,
                },
            )
            .await?;
        Ok(resp.address)
    }

    /// Get the wallet balance in atomic units (piconero).
    ///
    /// Returns (available_balance, unlocked_balance) in atomic units.
    pub async fn get_balance(&self, account_index: u32) -> Result<(u64, u64)> {
        #[derive(Serialize)]
        struct Params {
            account_index: u32,
        }

        let resp: GetBalanceResponse =
            self.call("get_balance", Params { account_index }).await?;
        Ok((resp.balance, resp.unlocked_balance))
    }

    /// Get the wallet balance formatted as XMR (float).
    pub async fn get_balance_xmr(&self, account_index: u32) -> Result<(f64, f64)> {
        let (available, unlocked) = self.get_balance(account_index).await?;
        Ok((
            available as f64 / XMR_SCALE as f64,
            unlocked as f64 / XMR_SCALE as f64,
        ))
    }

    /// Transfer XMR to a destination address.
    ///
    /// # Arguments
    /// * `destination` - Monero address string
    /// * `amount_piconero` - Amount in atomic units (piconero)
    /// * `payment_id` - Optional 64-hex-char payment ID for tracking
    ///
    /// Returns the transaction hash as a hex string.
    pub async fn transfer(
        &self,
        destination: &str,
        amount_piconero: u64,
        payment_id: Option<String>,
    ) -> Result<String> {
        #[derive(Serialize)]
        struct Params {
            destinations: Vec<TransferDestination>,
            mixin: u64,
            #[serde(skip_serializing_if = "Option::is_none")]
            payment_id: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            get_tx_key: Option<bool>,
        }

        tracing::info!(
            destination = destination,
            amount_piconero = amount_piconero,
            payment_id = ?payment_id,
            "Initiating Monero transfer"
        );

        let resp: TransferResponse = self
            .call(
                "transfer",
                Params {
                    destinations: vec![TransferDestination {
                        address: destination.to_string(),
                        amount: amount_piconero,
                    }],
                    mixin: 0,
                    payment_id,
                    get_tx_key: Some(true),
                },
            )
            .await
            .context("Monero transfer failed")?;

        tracing::info!(
            tx_hash = %resp.tx_hash,
            fee = resp.fee,
            "Monero transfer submitted"
        );

        Ok(resp.tx_hash)
    }

    /// Check for incoming payments with a specific payment ID.
    ///
    /// Returns a list of payment amounts (in atomic units) that match
    /// the given payment ID.
    pub async fn get_payments(&self, payment_id: &str) -> Result<Vec<u64>> {
        #[derive(Serialize)]
        struct Params {
            payment_ids: Vec<String>,
        }

        let resp: GetPaymentsResponse = self
            .call(
                "get_payments",
                Params {
                    payment_ids: vec![payment_id.to_string()],
                },
            )
            .await
            .context("Failed to query payments")?;

        Ok(resp.payments.into_iter().map(|p| p.amount).collect())
    }

    /// Check for bulk payments using multiple payment IDs.
    ///
    /// Returns a vector of (payment_id, amount) pairs.
    pub async fn get_bulk_payments(
        &self,
        payment_ids: Vec<String>,
        _min_block_height: Option<u64>,
    ) -> Result<Vec<(String, u64)>> {
        #[derive(Serialize)]
        struct Params {
            payment_ids: Vec<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            min_block_height: Option<u64>,
        }

        let resp: GetBulkPaymentsResponse = self
            .call(
                "get_bulk_payments",
                Params {
                    payment_ids,
                    min_block_height: _min_block_height,
                },
            )
            .await
            .context("Failed to query bulk payments")?;

        Ok(resp
            .payments
            .into_iter()
            .map(|t| (t.payment_id, t.amount))
            .collect())
    }

    /// Verify a transaction has been confirmed on-chain using check_tx_key.
    pub async fn check_transaction(&self, tx_hash: &str, address: &str) -> Result<bool> {
        #[derive(Serialize)]
        struct Params {
            tx_id: String,
            address: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            tx_key: Option<String>,
        }

        let resp: CheckTxKeyResponse = self
            .call(
                "check_tx_key",
                Params {
                    tx_id: tx_hash.to_string(),
                    address: address.to_string(),
                    tx_key: None,
                },
            )
            .await
            .context("Failed to check transaction")?;

        Ok(resp.confirmed.unwrap_or(resp.unlocked && resp.confirmations > 0))
    }

    /// Get the current blockchain height the wallet is synced to.
    pub async fn get_height(&self) -> Result<u64> {
        let resp: GetHeightResponse = self.call_no_params("get_height").await?;
        Ok(resp.height)
    }

    /// Save the wallet state to disk.
    pub async fn store(&self) -> Result<bool> {
        let _resp: StoreResponse = self.call_no_params("store").await?;
        tracing::info!("Wallet stored");
        Ok(true)
    }

    /// Convert OpenDex internal amount (scaled by AMOUNT_SCALE) to
    /// Monero atomic units (piconero).
    ///
    /// OpenDex uses AMOUNT_SCALE = 100_000_000 (8 decimal places).
    /// Monero uses 12 decimal places (piconero).
    /// This converts from OpenDex units to piconero.
    pub fn opendex_amount_to_piconero(opendex_amount: u64) -> u64 {
        // OpenDex amount is in units of 1/100_000_000 XMR
        // Convert to piconero (1/1_000_000_000_000 XMR)
        // Multiply by 10_000 to go from 8-decimal to 12-decimal
        opendex_amount.saturating_mul(10_000)
    }

    /// Convert Monero atomic units (piconero) to OpenDex internal amount.
    pub fn piconero_to_opendex_amount(piconero: u64) -> u64 {
        piconero / 10_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_conversion_roundtrip() {
        let original = 1_000_000_000u64; // 10 XMR in OpenDex units
        let piconero = MoneroWalletClient::opendex_amount_to_piconero(original);
        let back = MoneroWalletClient::piconero_to_opendex_amount(piconero);
        assert_eq!(original, back);
    }

    #[test]
    fn test_amount_conversion_one_xmr() {
        // 1 XMR = 100_000_000 in OpenDex units
        let one_xmr_opendex = protocol::message::AMOUNT_SCALE;
        let piconero = MoneroWalletClient::opendex_amount_to_piconero(one_xmr_opendex);
        // Should be 1_000_000_000_000 piconero
        assert_eq!(piconero, XMR_SCALE);
    }

    #[test]
    fn test_wallet_client_default_config() {
        let config = MoneroWalletConfig::default();
        assert_eq!(config.rpc_url, "http://127.0.0.1:18082");
        assert!(config.username.is_none());
        assert!(config.password.is_none());
    }

    #[test]
    fn test_wallet_state_initial() {
        let client = MoneroWalletClient::new(MoneroWalletConfig::default());
        assert!(!client.is_connected());
        assert!(matches!(client.state(), WalletState::Disconnected));
    }
}
