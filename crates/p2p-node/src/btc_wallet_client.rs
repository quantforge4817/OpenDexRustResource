#![allow(dead_code)]
/// Bitcoin wallet client abstraction layer.
///
/// Connects OpenDex to a running Bitcoin Core (`bitcoind`) wallet RPC service
/// to perform on-chain BTC operations: balance queries, transfers, payment
/// monitoring, and address management. This bridges the internal escrow system
/// with actual Bitcoin blockchain transactions.
///
/// Uses raw JSON-RPC over HTTP via reqwest, avoiding native dependencies.
/// The wallet RPC must be running externally (e.g., `bitcoind` with wallet
/// support enabled) and reachable at the configured address.
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

/// Bitcoin amount scale: 1 BTC = 10^8 satoshis.
#[allow(dead_code)]
pub const BTC_SCALE: u64 = 100_000_000;

// ---------------------------------------------------------------------------
// JSON-RPC types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest<T: Serialize> {
    jsonrpc: String,
    id: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<T>,
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
// Bitcoin Core RPC response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GetBalanceResponse {
    /// Balance in BTC
    balance: f64,
}

#[derive(Debug, Deserialize)]
struct GetNewAddressResponse {
    address: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SendToAddressResponse {
    txid: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GetReceivedByAddressResponse {
    /// Amount received in BTC
    amount: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GetTransactionDetail {
    address: String,
    category: String,
    amount: f64,
    #[serde(default)]
    fee: Option<f64>,
    #[serde(default)]
    confirmations: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GetTransactionInfo {
    #[serde(default)]
    confirmations: i64,
    #[serde(default)]
    blockhash: Option<String>,
    #[serde(default)]
    blockheight: Option<u64>,
    #[serde(default)]
    fee: f64,
    details: Vec<GetTransactionDetail>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GetTransactionResponse {
    #[serde(default)]
    confirmations: i64,
    #[serde(default)]
    #[allow(dead_code)]
    blockhash: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    blockheight: Option<u64>,
    #[serde(default)]
    #[allow(dead_code)]
    fee: f64,
    #[allow(dead_code)]
    details: Vec<GetTransactionDetail>,
    #[serde(default)]
    #[allow(dead_code)]
    hex: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GetWalletInfoResponse {
    #[serde(default)]
    #[allow(dead_code)]
    walletname: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    balance: f64,
    #[serde(default)]
    #[allow(dead_code)]
    unconfirmed_balance: f64,
    #[serde(default)]
    #[allow(dead_code)]
    immature_balance: f64,
}

#[derive(Debug, Deserialize)]
pub struct ListUnspentEntry {
    #[allow(dead_code)]
    txid: String,
    #[allow(dead_code)]
    vout: u32,
    #[allow(dead_code)]
    address: String,
    #[allow(dead_code)]
    label: String,
    #[allow(dead_code)]
    amount: f64,
    #[allow(dead_code)]
    confirmations: u32,
    #[allow(dead_code)]
    spendable: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ListUnspentResponse {
    entries: Vec<ListUnspentEntry>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GetBlockCountResponse {
    height: u64,
}

// ---------------------------------------------------------------------------
// Config and state
// ---------------------------------------------------------------------------

/// Configuration for connecting to a Bitcoin Core wallet RPC service.
#[derive(Debug, Clone)]
pub struct BitcoinWalletConfig {
    /// RPC endpoint, e.g., "http://127.0.0.1:8332"
    pub rpc_url: String,
    /// Username for RPC authentication
    pub username: Option<String>,
    /// Password for RPC authentication
    pub password: Option<String>,
    /// Optional wallet name for multi-wallet setups
    pub wallet_name: Option<String>,
}

impl Default for BitcoinWalletConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8332".to_string(),
            username: None,
            password: None,
            wallet_name: None,
        }
    }
}

/// Represents the connection state of the Bitcoin wallet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitcoinWalletState {
    /// Wallet RPC is not configured or not connected
    Disconnected,
    /// Connected and ready to perform operations
    Connected,
    /// Connection failed with an error message
    Error(String),
}

/// A thin wrapper around the Bitcoin Core wallet RPC providing
/// OpenDex-specific operations via raw JSON-RPC over HTTP.
#[allow(dead_code)]
pub struct BitcoinWalletClient {
    http: reqwest::Client,
    config: BitcoinWalletConfig,
    state: BitcoinWalletState,
}

impl BitcoinWalletClient {
    /// Create a disconnected wallet client with the given configuration.
    pub fn new(config: BitcoinWalletConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
            state: BitcoinWalletState::Disconnected,
        }
    }

    // -----------------------------------------------------------------------
    // Internal: JSON-RPC call helper
    // -----------------------------------------------------------------------

    /// Build the RPC URL, optionally including the wallet name path.
    fn rpc_url(&self) -> String {
        if let Some(ref wallet) = self.config.wallet_name {
            format!("{}/wallet/{}", self.config.rpc_url, wallet)
        } else {
            self.config.rpc_url.clone()
        }
    }

    /// Execute a JSON-RPC call with empty params.
    async fn call_no_params<T: serde::de::DeserializeOwned>(&self, method: &str) -> Result<T> {
        self.call::<(), T>(method, None).await
    }

    /// Execute a JSON-RPC call with typed params.
    async fn call<P: Serialize, T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Option<P>,
    ) -> Result<T> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "1".to_string(),
            method: method.to_string(),
            params,
        };

        let url = self.rpc_url();
        let mut builder = self.http.post(&url).json(&request);

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

    /// Connect to the Bitcoin wallet RPC service by testing with getblockcount.
    /// Uses getblockcount instead of getbalance because it works without a loaded wallet.
    pub async fn connect(&mut self) -> Result<()> {
        tracing::info!(
            url = %self.config.rpc_url,
            "Connecting to Bitcoin wallet RPC"
        );

        match self.get_block_count().await {
            Ok(height) => {
                self.state = BitcoinWalletState::Connected;
                tracing::info!(
                    block_height = height,
                    "Successfully connected to Bitcoin wallet RPC"
                );
                Ok(())
            }
            Err(e) => {
                self.state = BitcoinWalletState::Error(format!("Connection test failed: {}", e));
                Err(anyhow!("Failed to connect to Bitcoin wallet RPC: {}", e))
            }
        }
    }

    /// Check if the wallet is currently connected.
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        matches!(self.state, BitcoinWalletState::Connected)
    }

    /// Get the current wallet state.
    #[allow(dead_code)]
    pub fn state(&self) -> &BitcoinWalletState {
        &self.state
    }

    /// Get the wallet balance in BTC.
    pub async fn get_balance(&self) -> Result<f64> {
        let resp: GetBalanceResponse = self.call_no_params("getbalance").await?;
        Ok(resp.balance)
    }

    /// Get the wallet balance in satoshis.
    pub async fn get_balance_satoshi(&self) -> Result<u64> {
        let balance = self.get_balance().await?;
        Ok((balance * BTC_SCALE as f64) as u64)
    }

    /// Get wallet info including balance details.
    #[allow(dead_code)]
    pub async fn get_wallet_info(&self) -> Result<GetWalletInfoResponse> {
        let resp: GetWalletInfoResponse = self.call_no_params("getwalletinfo").await?;
        Ok(resp)
    }

    /// Get the current blockchain block count.
    /// Bitcoin's getblockcount returns a plain integer, not an object.
    pub async fn get_block_count(&self) -> Result<u64> {
        let count: u64 = self.call_no_params("getblockcount").await?;
        Ok(count)
    }

    /// Generate a new Bitcoin address for receiving payments.
    ///
    /// # Arguments
    /// * `label` - Optional label for the address
    ///
    /// Returns the new Bitcoin address string.
    pub async fn get_new_address(&self, label: Option<String>) -> Result<String> {
        #[derive(Serialize)]
        struct Params {
            #[serde(skip_serializing_if = "Option::is_none")]
            label: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            address_type: Option<String>,
        }

        let resp: GetNewAddressResponse = self
            .call(
                "getnewaddress",
                Some(Params {
                    label,
                    address_type: Some("bech34".to_string()),
                }),
            )
            .await?;
        Ok(resp.address)
    }

    /// Send BTC to a destination address.
    ///
    /// # Arguments
    /// * `destination` - Bitcoin address string
    /// * `amount_btc` - Amount in BTC (float)
    /// * `comment` - Optional comment for the transaction
    ///
    /// Returns the transaction ID as a hex string.
    pub async fn send_to_address(
        &self,
        destination: &str,
        amount_btc: f64,
        comment: Option<String>,
    ) -> Result<String> {
        #[derive(Serialize)]
        struct Params {
            address: String,
            amount: f64,
            #[serde(skip_serializing_if = "Option::is_none")]
            comment: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            comment_to: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            subtract_fee_from_amount: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            replaceable: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            conf_target: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            estimate_mode: Option<String>,
        }

        tracing::info!(
            destination = destination,
            amount_btc = amount_btc,
            "Initiating Bitcoin transfer"
        );

        let resp: SendToAddressResponse = self
            .call(
                "sendtoaddress",
                Some(Params {
                    address: destination.to_string(),
                    amount: amount_btc,
                    comment,
                    comment_to: None,
                    subtract_fee_from_amount: Some(false),
                    replaceable: Some(true),
                    conf_target: Some(6),
                    estimate_mode: Some("economical".to_string()),
                }),
            )
            .await
            .context("Bitcoin transfer failed")?;

        tracing::info!(
            txid = %resp.txid,
            "Bitcoin transfer submitted"
        );

        Ok(resp.txid)
    }

    /// Send BTC to a destination address using satoshi amount.
    ///
    /// # Arguments
    /// * `destination` - Bitcoin address string
    /// * `amount_satoshi` - Amount in satoshis
    ///
    /// Returns the transaction ID as a hex string.
    pub async fn send_to_address_satoshi(
        &self,
        destination: &str,
        amount_satoshi: u64,
    ) -> Result<String> {
        let amount_btc = amount_satoshi as f64 / BTC_SCALE as f64;
        self.send_to_address(destination, amount_btc, None).await
    }

    /// Get the total amount received by an address.
    ///
    /// # Arguments
    /// * `address` - Bitcoin address to check
    /// * `min_confirmations` - Minimum number of confirmations
    ///
    /// Returns the amount received in BTC.
    pub async fn get_received_by_address(
        &self,
        address: &str,
        min_confirmations: u32,
    ) -> Result<f64> {
        #[derive(Serialize)]
        struct Params {
            address: String,
            minconf: u32,
        }

        let resp: GetReceivedByAddressResponse = self
            .call(
                "getreceivedbyaddress",
                Some(Params {
                    address: address.to_string(),
                    minconf: min_confirmations,
                }),
            )
            .await?;
        Ok(resp.amount)
    }

    /// Get the total amount received by an address in satoshis.
    pub async fn get_received_by_address_satoshi(
        &self,
        address: &str,
        min_confirmations: u32,
    ) -> Result<u64> {
        let amount = self
            .get_received_by_address(address, min_confirmations)
            .await?;
        Ok((amount * BTC_SCALE as f64) as u64)
    }

    /// Get details about a transaction by TXID.
    ///
    /// Returns confirmations, block height, fee, and transaction details.
    #[allow(dead_code)]
    pub async fn get_transaction(&self, txid: &str) -> Result<GetTransactionResponse> {
        #[derive(Serialize)]
        struct Params {
            txid: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            include_watchonly: Option<bool>,
        }

        let resp: GetTransactionResponse = self
            .call(
                "gettransaction",
                Some(Params {
                    txid: txid.to_string(),
                    include_watchonly: Some(true),
                }),
            )
            .await?;
        Ok(resp)
    }

    /// Check if a transaction is confirmed on-chain.
    ///
    /// # Arguments
    /// * `txid` - Transaction ID to check
    /// * `min_confirmations` - Minimum confirmations required
    ///
    /// Returns true if the transaction has at least the required confirmations.
    pub async fn is_transaction_confirmed(
        &self,
        txid: &str,
        min_confirmations: u32,
    ) -> Result<bool> {
        let tx = self.get_transaction(txid).await?;
        Ok(tx.confirmations >= min_confirmations as i64)
    }

    /// List unspent outputs (UTXOs) available for spending.
    ///
    /// # Arguments
    /// * `min_confirmations` - Minimum confirmations for UTXOs
    ///
    /// Returns a list of UTXO entries.
    #[allow(dead_code)]
    pub async fn list_unspent(&self, min_confirmations: u32) -> Result<Vec<ListUnspentEntry>> {
        #[derive(Serialize)]
        struct Params {
            minconf: u32,
        }

        let resp: ListUnspentResponse = self
            .call(
                "listunspent",
                Some(Params {
                    minconf: min_confirmations,
                }),
            )
            .await?;
        Ok(resp.entries)
    }

    /// Convert OpenDex internal amount (scaled by AMOUNT_SCALE) to satoshis.
    ///
    /// OpenDex uses AMOUNT_SCALE = 100_000_000 (8 decimal places).
    /// Bitcoin also uses 8 decimal places (satoshis).
    /// So the conversion is 1:1.
    pub fn opendex_amount_to_satoshi(opendex_amount: u64) -> u64 {
        // Both use 8 decimal places, so the values are identical
        opendex_amount
    }

    /// Convert satoshis to OpenDex internal amount.
    pub fn satoshi_to_opendex_amount(satoshi: u64) -> u64 {
        // Both use 8 decimal places, so the values are identical
        satoshi
    }

    /// Convert BTC float to OpenDex internal amount.
    pub fn btc_to_opendex_amount(btc: f64) -> u64 {
        (btc * BTC_SCALE as f64) as u64
    }

    /// Convert OpenDex internal amount to BTC float.
    pub fn opendex_amount_to_btc(opendex_amount: u64) -> f64 {
        opendex_amount as f64 / BTC_SCALE as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_conversion_roundtrip() {
        let original = 1_000_000_000u64; // 10 BTC in OpenDex units
        let satoshi = BitcoinWalletClient::opendex_amount_to_satoshi(original);
        let back = BitcoinWalletClient::satoshi_to_opendex_amount(satoshi);
        assert_eq!(original, back);
    }

    #[test]
    fn test_btc_float_conversion() {
        // 1 BTC
        let one_btc = 1.0_f64;
        let opendex = BitcoinWalletClient::btc_to_opendex_amount(one_btc);
        assert_eq!(opendex, protocol::message::AMOUNT_SCALE);

        let back = BitcoinWalletClient::opendex_amount_to_btc(opendex);
        assert!((back - one_btc).abs() < 0.00000001);
    }

    #[test]
    fn test_wallet_client_default_config() {
        let config = BitcoinWalletConfig::default();
        assert_eq!(config.rpc_url, "http://127.0.0.1:8332");
        assert!(config.username.is_none());
        assert!(config.password.is_none());
        assert!(config.wallet_name.is_none());
    }

    #[test]
    fn test_wallet_state_initial() {
        let client = BitcoinWalletClient::new(BitcoinWalletConfig::default());
        assert!(!client.is_connected());
        assert!(matches!(client.state(), BitcoinWalletState::Disconnected));
    }

    #[test]
    fn test_wallet_name_in_rpc_url() {
        let config = BitcoinWalletConfig {
            rpc_url: "http://127.0.0.1:8332".to_string(),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            wallet_name: Some("mywallet".to_string()),
        };
        let client = BitcoinWalletClient::new(config);
        assert_eq!(client.rpc_url(), "http://127.0.0.1:8332/wallet/mywallet");
    }

    #[test]
    fn test_wallet_name_none_in_rpc_url() {
        let config = BitcoinWalletConfig::default();
        let client = BitcoinWalletClient::new(config);
        assert_eq!(client.rpc_url(), "http://127.0.0.1:8332");
    }
}
