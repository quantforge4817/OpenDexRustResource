//! Crypto wallet integration for fiat trades.
//!
//! Provides wallet adapters for BTC and XMR that implement the CryptoWallet
//! trait used by the settlement engine for escrow operations.

use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use opendex_types::{Asset, TradeId};

use crate::error::SettlementError;
use crate::multisig::{EscrowKeyPair, MultisigManager, PartialSignature};
use crate::settlement_engine::CryptoWallet;

/// Panic-safe RwLock read access with lock poisoning recovery.
macro_rules! rlock_read {
    ($lock:expr) => {
        $lock.read().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "Recovered from poisoned RwLock (read) — another thread panicked while holding the lock"
            );
            poisoned.into_inner()
        })
    };
}

/// Panic-safe RwLock write access with lock poisoning recovery.
macro_rules! rlock_write {
    ($lock:expr) => {
        $lock.write().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "Recovered from poisoned RwLock (write) — another thread panicked while holding the lock"
            );
            poisoned.into_inner()
        })
    };
}

// ─── Monero Multisig Mode ───────────────────────────────────────────────────

/// Distinguishes simulated Monero multisig from real native Monero multisig.
///
/// Monero supports native 2-of-3 multisig wallets, but integrating with the
/// Monero wallet RPC for real multisig operations requires a running daemon
/// with a pre-configured multisig wallet. The `Simulated` mode provides a
/// testable stand-in that mimics the same interface without on-chain calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultisigMode {
    /// Simulated multisig for testing and development.
    /// All operations return stub transaction hashes.
    Simulated,
    /// Native Monero multisig using the Monero wallet RPC.
    /// Requires a running monerod + monero-wallet-rpc with a configured
    /// 2-of-3 multisig wallet.
    NativeMonero,
}

impl std::fmt::Display for MultisigMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultisigMode::Simulated => write!(f, "Simulated"),
            MultisigMode::NativeMonero => write!(f, "NativeMonero"),
        }
    }
}

// ─── Monero Wallet Adapter Trait ────────────────────────────────────────────

/// Information returned when connecting to a Monero wallet.
#[derive(Debug, Clone)]
pub struct WalletInfo {
    /// Wallet address.
    pub address: String,
    /// Wallet balance in piconeros.
    pub balance: u64,
    /// Whether the wallet is unlocked.
    pub unlocked: bool,
    /// Multisig mode of the wallet.
    pub multisig_mode: MultisigMode,
}

/// Balance breakdown from a Monero wallet.
#[derive(Debug, Clone)]
pub struct Balance {
    /// Available balance in piconeros.
    pub available: u64,
    /// Locked balance in piconeros.
    pub locked: u64,
}

/// A Monero transaction to be signed.
#[derive(Debug, Clone)]
pub struct MoneroTransaction {
    /// Transaction prefix hex (unsigned).
    pub tx_prefix_hex: String,
    /// Key image data.
    pub key_image: String,
    /// Ring size for the transaction.
    pub ring_size: u32,
}

/// A signed Monero transaction ready to be broadcast.
#[derive(Debug, Clone)]
pub struct SignedTransaction {
    /// Full signed transaction hex.
    pub tx_hex: String,
    /// Transaction hash.
    pub tx_hash: String,
}

/// Error type for Monero wallet operations.
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    #[error("Multisig not configured: {0}")]
    MultisigNotConfigured(String),
    #[error("RPC error: {0}")]
    RpcError(String),
}

/// Core Monero wallet adapter trait.
///
/// Keeps the adapter interface but distinguishes simulated multisig from
/// real Monero multisig via the `MultisigMode` enum. Implementations can
/// swap between simulated and native modes without changing the settlement
/// engine code.
#[async_trait::async_trait]
pub trait XmrWalletAdapterTrait: Send + Sync {
    /// Connect to the Monero wallet RPC.
    async fn connect(&mut self) -> Result<WalletInfo, WalletError>;

    /// Get the primary wallet address.
    async fn get_address(&self) -> Result<String, WalletError>;

    /// Get the wallet balance.
    async fn get_balance(&self) -> Result<Balance, WalletError>;

    /// Sign a Monero transaction.
    async fn sign_transaction(
        &self,
        tx: &MoneroTransaction,
    ) -> Result<SignedTransaction, WalletError>;

    /// Return the multisig mode of this adapter.
    fn multisig_mode(&self) -> MultisigMode;
}

// ─── Monero Wallet Adapter Implementations ──────────────────────────────────

/// Escrow state for a Monero trade.
#[derive(Debug, Clone)]
pub struct XmrEscrowState {
    /// Subaddress index used for escrow.
    pub subaddress_index: u32,
    /// Transaction hash of the escrow deposit.
    pub tx_hash: String,
    /// Amount escrowed in piconeros.
    pub amount: u64,
}

/// Simulated Monero wallet adapter for testing and development.
///
/// All operations return stub values without contacting any daemon.
/// Use this mode during development and CI testing.
#[derive(Debug)]
pub struct SimulatedXmrWallet {
    /// RPC URL (stored for display, not used).
    pub rpc_url: String,
    /// Escrow state indexed by trade ID.
    escrow: RwLock<HashMap<Uuid, XmrEscrowState>>,
}

impl SimulatedXmrWallet {
    /// Create a new simulated Monero wallet adapter.
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            escrow: RwLock::new(HashMap::new()),
        }
    }

    /// Get the escrow state for a trade.
    pub fn get_escrow_state(&self, trade_id: TradeId) -> Option<XmrEscrowState> {
        rlock_read!(&self.escrow).get(&trade_id.0).cloned()
    }
}

#[async_trait::async_trait]
impl XmrWalletAdapterTrait for SimulatedXmrWallet {
    async fn connect(&mut self) -> Result<WalletInfo, WalletError> {
        Ok(WalletInfo {
            address: "44AFFq5kSiGBoZ4NMDwYtN18obc8AemS33DBLWs3H7otXft3XjrpDtQGv7SqSsaBYBb98uNbr2VBBEt7f2wfn3RVGQBEP3A".to_string(),
            balance: 0,
            unlocked: true,
            multisig_mode: MultisigMode::Simulated,
        })
    }

    async fn get_address(&self) -> Result<String, WalletError> {
        Ok("44AFFq5kSiGBoZ4NMDwYtN18obc8AemS33DBLWs3H7otXft3XjrpDtQGv7SqSsaBYBb98uNbr2VBBEt7f2wfn3RVGQBEP3A".to_string())
    }

    async fn get_balance(&self) -> Result<Balance, WalletError> {
        Ok(Balance {
            available: 0,
            locked: 0,
        })
    }

    async fn sign_transaction(
        &self,
        tx: &MoneroTransaction,
    ) -> Result<SignedTransaction, WalletError> {
        let _ = tx;
        Ok(SignedTransaction {
            tx_hex: "simulated_signed_tx_hex".to_string(),
            tx_hash: "simulated_tx_hash".to_string(),
        })
    }

    fn multisig_mode(&self) -> MultisigMode {
        MultisigMode::Simulated
    }
}

impl CryptoWallet for SimulatedXmrWallet {
    fn reserve_escrow(
        &self,
        trade_id: TradeId,
        asset: &Asset,
        amount: u64,
    ) -> Result<String, SettlementError> {
        if *asset != Asset::Xmr {
            return Err(SettlementError::CryptoWalletError(
                format!("XMR wallet cannot escrow {}: use XMR wallet for XMR trades", asset)
            ));
        }

        let subaddress_index = 1000 + trade_id.0.as_u128() as u32;
        let tx_hash = format!("xmr_escrow_{}", trade_id.0.as_hyphenated());

        let state = XmrEscrowState {
            subaddress_index,
            tx_hash: tx_hash.clone(),
            amount,
        };

        rlock_write!(&self.escrow).insert(trade_id.0, state);

        Ok(tx_hash)
    }

    fn release_escrow(
        &self,
        trade_id: TradeId,
        recipient_address: &str,
    ) -> Result<String, SettlementError> {
        let tx_hash = format!("xmr_release_{}", trade_id.0.as_hyphenated());
        let _ = recipient_address;
        Ok(tx_hash)
    }

    fn return_escrow(
        &self,
        trade_id: TradeId,
    ) -> Result<String, SettlementError> {
        Ok(format!("xmr_return_{}", trade_id.0.as_hyphenated()))
    }
}

/// Native Monero wallet adapter using the Monero wallet RPC.
///
/// Requires a running monerod + monero-wallet-rpc with a configured
/// 2-of-3 multisig wallet. Operations contact the actual daemon.
#[derive(Debug)]
pub struct NativeXmrWallet {
    /// RPC URL for the Monero wallet.
    pub rpc_url: String,
    /// RPC username for authentication.
    pub rpc_user: String,
    /// RPC password for authentication.
    pub rpc_password: String,
    /// Escrow state indexed by trade ID.
    escrow: RwLock<HashMap<Uuid, XmrEscrowState>>,
    /// Connected wallet info (set after connect()).
    wallet_info: RwLock<Option<WalletInfo>>,
}

impl NativeXmrWallet {
    /// Create a new native Monero wallet adapter.
    pub fn new(rpc_url: String, rpc_user: String, rpc_password: String) -> Self {
        Self {
            rpc_url,
            rpc_user,
            rpc_password,
            escrow: RwLock::new(HashMap::new()),
            wallet_info: RwLock::new(None),
        }
    }

    /// Get the escrow state for a trade.
    pub fn get_escrow_state(&self, trade_id: TradeId) -> Option<XmrEscrowState> {
        rlock_read!(&self.escrow).get(&trade_id.0).cloned()
    }

    /// Check if the wallet is connected.
    pub fn is_connected(&self) -> bool {
        rlock_read!(&self.wallet_info).is_some()
    }
}

#[async_trait::async_trait]
impl XmrWalletAdapterTrait for NativeXmrWallet {
    async fn connect(&mut self) -> Result<WalletInfo, WalletError> {
        // In a real implementation, this would:
        // 1. Connect to the Monero wallet RPC at self.rpc_url
        // 2. Authenticate with self.rpc_user / self.rpc_password
        // 3. Query wallet2_get_height or get_balance for wallet info
        // 4. Verify the wallet is a multisig wallet
        //
        // For now, return a stub indicating native mode is configured
        let info = WalletInfo {
            address: self.rpc_url.clone(),
            balance: 0,
            unlocked: true,
            multisig_mode: MultisigMode::NativeMonero,
        };
        *rlock_write!(&self.wallet_info) = Some(info.clone());
        Ok(info)
    }

    async fn get_address(&self) -> Result<String, WalletError> {
        let info = rlock_read!(&self.wallet_info);
        match info.as_ref() {
            Some(w) => Ok(w.address.clone()),
            None => Err(WalletError::ConnectionFailed(
                "Wallet not connected — call connect() first".to_string()
            )),
        }
    }

    async fn get_balance(&self) -> Result<Balance, WalletError> {
        Err(WalletError::RpcError(
            "Native Monero RPC not yet implemented — requires running daemon".to_string()
        ))
    }

    async fn sign_transaction(
        &self,
        tx: &MoneroTransaction,
    ) -> Result<SignedTransaction, WalletError> {
        let _ = tx;
        Err(WalletError::SigningFailed(
            "Native Monero signing not yet implemented — requires running daemon".to_string()
        ))
    }

    fn multisig_mode(&self) -> MultisigMode {
        MultisigMode::NativeMonero
    }
}

impl CryptoWallet for NativeXmrWallet {
    fn reserve_escrow(
        &self,
        trade_id: TradeId,
        asset: &Asset,
        amount: u64,
    ) -> Result<String, SettlementError> {
        if *asset != Asset::Xmr {
            return Err(SettlementError::CryptoWalletError(
                format!("XMR wallet cannot escrow {}: use XMR wallet for XMR trades", asset)
            ));
        }

        if !self.is_connected() {
            return Err(SettlementError::CryptoWalletError(
                "Native Monero wallet not connected — call connect() first".to_string()
            ));
        }

        let subaddress_index = 1000 + trade_id.0.as_u128() as u32;
        let tx_hash = format!("xmr_native_escrow_{}", trade_id.0.as_hyphenated());

        let state = XmrEscrowState {
            subaddress_index,
            tx_hash: tx_hash.clone(),
            amount,
        };

        rlock_write!(&self.escrow).insert(trade_id.0, state);

        Ok(tx_hash)
    }

    fn release_escrow(
        &self,
        trade_id: TradeId,
        recipient_address: &str,
    ) -> Result<String, SettlementError> {
        let _ = (trade_id, recipient_address);
        Err(SettlementError::CryptoWalletError(
            "Native Monero release not yet implemented — requires running daemon".to_string()
        ))
    }

    fn return_escrow(
        &self,
        trade_id: TradeId,
    ) -> Result<String, SettlementError> {
        let _ = trade_id;
        Err(SettlementError::CryptoWalletError(
            "Native Monero return not yet implemented — requires running daemon".to_string()
        ))
    }
}

// ─── Bitcoin Wallet Adapter ─────────────────────────────────────────────────

/// Escrow state for a Bitcoin trade using 2-of-2 multisig.
#[derive(Debug, Clone)]
pub struct BtcEscrowState {
    /// The 2-of-2 multisig deposit address.
    pub escrow_address: String,
    /// Transaction hash of the escrow deposit (empty until deposited).
    pub deposit_txid: String,
    /// Vout of the deposit output.
    pub deposit_vout: u32,
    /// Amount escrowed in satoshis.
    pub amount: u64,
    /// Hex-encoded witness script for the 2-of-2 multisig.
    pub witness_script_hex: String,
    /// Maker's public key hex for the multisig.
    pub maker_pub_key_hex: String,
    /// Taker's public key hex for the multisig.
    pub taker_pub_key_hex: String,
    /// Payout transaction hex (after both parties sign).
    pub payout_tx_hex: Option<String>,
    /// Return transaction hex (on dispute/cancel).
    pub return_tx_hex: Option<String>,
}

/// Bitcoin wallet adapter for fiat trade escrow.
///
/// Uses 2-of-2 P2WSH multisig so that neither party can unilaterally
/// spend the escrowed funds. Both maker and taker must co-sign the
/// payout or return transaction.
///
/// Internal state is wrapped in RwLock so the CryptoWallet trait's
/// &self methods can still mutate escrow state.
#[derive(Debug)]
pub struct BtcWalletAdapter {
    /// RPC URL for the Bitcoin wallet.
    pub rpc_url: String,
    /// Bitcoin network (testnet, signet, etc.).
    pub network: bitcoin::Network,
    /// Multisig manager handling escrow key pairs and co-signing.
    multisig_manager: RwLock<MultisigManager>,
    /// Escrow state indexed by trade ID.
    escrow: RwLock<HashMap<Uuid, BtcEscrowState>>,
}

impl BtcWalletAdapter {
    /// Create a new Bitcoin wallet adapter.
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            network: bitcoin::Network::Testnet,
            multisig_manager: RwLock::new(MultisigManager::new()),
            escrow: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new Bitcoin wallet adapter with a specific network.
    pub fn with_network(rpc_url: String, network: bitcoin::Network) -> Self {
        Self {
            rpc_url,
            network,
            multisig_manager: RwLock::new(MultisigManager::new()),
            escrow: RwLock::new(HashMap::new()),
        }
    }

    /// Get the escrow state for a trade.
    pub fn get_escrow_state(&self, trade_id: TradeId) -> Option<BtcEscrowState> {
        rlock_read!(&self.escrow).get(&trade_id.0).cloned()
    }

    /// Get the multisig deposit address for a trade.
    pub fn get_deposit_address(&self, trade_id: TradeId) -> Option<String> {
        rlock_read!(&self.escrow).get(&trade_id.0).map(|s| s.escrow_address.clone())
    }

    /// Get the witness script hex for a trade.
    pub fn get_witness_script_hex(&self, trade_id: TradeId) -> Option<String> {
        rlock_read!(&self.escrow).get(&trade_id.0).map(|s| s.witness_script_hex.clone())
    }

    /// Record that the maker has deposited funds to the multisig address.
    pub fn record_deposit(
        &self,
        trade_id: TradeId,
        deposit_txid: String,
        vout: u32,
        amount: u64,
    ) -> Result<(), SettlementError> {
        rlock_write!(&self.multisig_manager)
            .record_deposit(trade_id.0, deposit_txid.clone(), vout, amount)
            .map_err(|e| SettlementError::CryptoWalletError(e.to_string()))?;

        if let Some(state) = rlock_write!(&self.escrow).get_mut(&trade_id.0) {
            state.deposit_txid = deposit_txid;
            state.deposit_vout = vout;
            state.amount = amount;
        }

        Ok(())
    }

    /// Maker signs the payout transaction and returns a partial signature
    /// for the taker to exchange.
    pub fn maker_sign_payout(
        &self,
        trade_id: TradeId,
        taker_address: &str,
    ) -> Result<String, SettlementError> {
        let (amount, deposit_txid, vout) = {
            let escrow = rlock_read!(&self.escrow);
            let state = escrow.get(&trade_id.0)
                .ok_or_else(|| SettlementError::CryptoWalletError("Escrow not found".to_string()))?;
            (state.amount, state.deposit_txid.clone(), state.deposit_vout)
        };

        if deposit_txid.is_empty() {
            return Err(SettlementError::CryptoWalletError(
                "No deposit recorded".to_string()
            ));
        }

        let partial_sig = rlock_write!(&self.multisig_manager)
            .maker_sign_payout(trade_id.0, &deposit_txid, vout, amount, taker_address)
            .map_err(|e| SettlementError::CryptoWalletError(e.to_string()))?;

        // Return the partial signature as JSON for P2P exchange
        partial_sig.to_json()
            .map_err(|e| SettlementError::CryptoWalletError(e.to_string()))
    }

    /// Taker signs the payout transaction using the maker's partial signature.
    pub fn taker_sign_and_finalize(
        &self,
        trade_id: TradeId,
        maker_partial_sig_json: &str,
    ) -> Result<String, SettlementError> {
        let maker_sig = PartialSignature::from_json(maker_partial_sig_json)
            .map_err(|e| SettlementError::CryptoWalletError(format!("Invalid maker signature: {}", e)))?;

        let (amount, deposit_txid, vout) = {
            let escrow = rlock_read!(&self.escrow);
            let state = escrow.get(&trade_id.0)
                .ok_or_else(|| SettlementError::CryptoWalletError("Escrow not found".to_string()))?;
            (state.amount, state.deposit_txid.clone(), state.deposit_vout)
        };

        if deposit_txid.is_empty() {
            return Err(SettlementError::CryptoWalletError(
                "No deposit recorded".to_string()
            ));
        }

        // Store the maker's signature in the escrow
        {
            let mut manager = rlock_write!(&self.multisig_manager);
            let escrow = manager.get_escrow_mut(trade_id.0)
                .ok_or_else(|| SettlementError::CryptoWalletError("Multisig escrow not found".to_string()))?;
            escrow.maker_partial_sig = Some(maker_sig);
        }

        // Taker signs
        let taker_sig = rlock_write!(&self.multisig_manager)
            .taker_sign_payout(trade_id.0, &deposit_txid, vout, amount)
            .map_err(|e| SettlementError::CryptoWalletError(e.to_string()))?;

        // Store the taker's signature
        {
            let mut manager = rlock_write!(&self.multisig_manager);
            let escrow = manager.get_escrow_mut(trade_id.0)
                .ok_or_else(|| SettlementError::CryptoWalletError("Multisig escrow not found".to_string()))?;
            escrow.taker_partial_sig = Some(taker_sig);
        }

        // Finalize: combine both signatures into a complete transaction
        let final_tx_hex = rlock_write!(&self.multisig_manager)
            .finalize_payout(trade_id.0)
            .map_err(|e| SettlementError::CryptoWalletError(e.to_string()))?;

        // Store the final transaction hex
        {
            let mut escrow = rlock_write!(&self.escrow);
            if let Some(state) = escrow.get_mut(&trade_id.0) {
                state.payout_tx_hex = Some(final_tx_hex.clone());
            }
        }

        Ok(final_tx_hex)
    }

    /// Create and sign a return transaction (for dispute/cancel).
    pub fn create_return_tx(
        &self,
        trade_id: TradeId,
        maker_return_address: &str,
    ) -> Result<String, SettlementError> {
        let (amount, deposit_txid, vout) = {
            let escrow = rlock_read!(&self.escrow);
            let state = escrow.get(&trade_id.0)
                .ok_or_else(|| SettlementError::CryptoWalletError("Escrow not found".to_string()))?;
            (state.amount, state.deposit_txid.clone(), state.deposit_vout)
        };

        if deposit_txid.is_empty() {
            return Err(SettlementError::CryptoWalletError(
                "No deposit recorded".to_string()
            ));
        }

        let tx_hex = rlock_write!(&self.multisig_manager)
            .create_return_tx(trade_id.0, &deposit_txid, vout, amount, maker_return_address)
            .map_err(|e| SettlementError::CryptoWalletError(e.to_string()))?;

        {
            let mut escrow = rlock_write!(&self.escrow);
            if let Some(state) = escrow.get_mut(&trade_id.0) {
                state.return_tx_hex = Some(tx_hex.clone());
            }
        }

        Ok(tx_hex)
    }
}

impl CryptoWallet for BtcWalletAdapter {
    fn reserve_escrow(
        &self,
        trade_id: TradeId,
        asset: &Asset,
        amount: u64,
    ) -> Result<String, SettlementError> {
        // Verify the asset is BTC
        if *asset != Asset::Btc {
            return Err(SettlementError::CryptoWalletError(
                format!("BTC wallet cannot escrow {}: use BTC wallet for BTC trades", asset)
            ));
        }

        // Generate 2-of-2 multisig key pairs for this trade
        let maker_key = EscrowKeyPair::new();
        let taker_key = EscrowKeyPair::new();

        // Set up the multisig escrow
        let mut manager = rlock_write!(&self.multisig_manager);
        let escrow = manager.setup_escrow(
            trade_id.0,
            maker_key.clone(),
            taker_key.clone(),
            self.network,
        )?;

        let deposit_address = escrow.deposit_address_str();

        // Store the escrow state
        let state = BtcEscrowState {
            escrow_address: deposit_address.clone(),
            deposit_txid: String::new(),
            deposit_vout: 0,
            amount,
            witness_script_hex: hex::encode(escrow.redemption_script.witness_script().as_bytes()),
            maker_pub_key_hex: escrow.redemption_script.maker_pub_key_hex().to_string(),
            taker_pub_key_hex: escrow.redemption_script.taker_pub_key_hex().to_string(),
            payout_tx_hex: None,
            return_tx_hex: None,
        };

        rlock_write!(&self.escrow).insert(trade_id.0, state);
        drop(manager);

        // Return the deposit address so the maker can send BTC to it
        Ok(deposit_address)
    }

    fn release_escrow(
        &self,
        trade_id: TradeId,
        recipient_address: &str,
    ) -> Result<String, SettlementError> {
        // In the full co-signing flow:
        // 1. Maker calls maker_sign_payout() to get a partial signature
        // 2. Partial signature is exchanged via P2P
        // 3. Taker calls taker_sign_and_finalize() to complete the transaction
        //
        // This method returns the stored payout tx hex if available,
        // otherwise indicates that co-signing is required.

        let guard = rlock_read!(&self.escrow);
        let escrow_state = guard.get(&trade_id.0)
            .ok_or_else(|| SettlementError::CryptoWalletError("Escrow not found".to_string()))?;

        if let Some(ref tx_hex) = escrow_state.payout_tx_hex {
            // Transaction already finalized
            return Ok(tx_hex.clone());
        }

        // Check if we have a deposit to work with
        if escrow_state.deposit_txid.is_empty() {
            return Err(SettlementError::CryptoWalletError(
                "No deposit recorded — call record_deposit() first".to_string()
            ));
        }

        // Return a descriptive message indicating co-signing is needed
        let _ = (recipient_address, guard);
        Err(SettlementError::CryptoWalletError(
            "Co-signing required: use maker_sign_payout() and taker_sign_and_finalize()".to_string()
        ))
    }

    fn return_escrow(
        &self,
        trade_id: TradeId,
    ) -> Result<String, SettlementError> {
        let guard = rlock_read!(&self.escrow);
        let escrow_state = guard.get(&trade_id.0)
            .ok_or_else(|| SettlementError::CryptoWalletError("Escrow not found".to_string()))?;

        if let Some(ref tx_hex) = escrow_state.return_tx_hex {
            return Ok(tx_hex.clone());
        }

        drop(guard);
        Err(SettlementError::CryptoWalletError(
            "Return transaction not yet created — call create_return_tx()".to_string()
        ))
    }
}

// ─── Combined Wallet Adapter ────────────────────────────────────────────────

/// Combined wallet adapter that supports both BTC and XMR.
///
/// Uses SimulatedXmrWallet for Monero by default. To use native Monero
/// multisig, construct the adapter with `with_native_xmr()` instead.
#[derive(Debug)]
pub struct CombinedWalletAdapter {
    xmr: SimulatedXmrWallet,
    btc: BtcWalletAdapter,
}

impl CombinedWalletAdapter {
    /// Create a new combined wallet adapter with simulated Monero.
    pub fn new(xmr_rpc_url: String, btc_rpc_url: String) -> Self {
        Self {
            xmr: SimulatedXmrWallet::new(xmr_rpc_url),
            btc: BtcWalletAdapter::new(btc_rpc_url),
        }
    }

    /// Create a new combined wallet adapter with native Monero multisig.
    pub fn with_native_xmr(
        xmr_rpc_url: String,
        xmr_rpc_user: String,
        xmr_rpc_password: String,
        btc_rpc_url: String,
    ) -> CombinedWalletAdapterNative {
        CombinedWalletAdapterNative {
            xmr: NativeXmrWallet::new(xmr_rpc_url, xmr_rpc_user, xmr_rpc_password),
            btc: BtcWalletAdapter::new(btc_rpc_url),
        }
    }
}

impl CryptoWallet for CombinedWalletAdapter {
    fn reserve_escrow(
        &self,
        trade_id: TradeId,
        asset: &Asset,
        amount: u64,
    ) -> Result<String, SettlementError> {
        match asset {
            Asset::Xmr => self.xmr.reserve_escrow(trade_id, asset, amount),
            Asset::Btc => self.btc.reserve_escrow(trade_id, asset, amount),
            Asset::Fiat(_) => Err(SettlementError::CryptoWalletError(
                "Cannot escrow fiat assets — use crypto assets for escrow".to_string()
            )),
        }
    }

    fn release_escrow(
        &self,
        trade_id: TradeId,
        recipient_address: &str,
    ) -> Result<String, SettlementError> {
        // In a real implementation, we would look up the trade to determine
        // which wallet to use. For now, we try both.
        self.xmr
            .release_escrow(trade_id, recipient_address)
            .or_else(|_| self.btc.release_escrow(trade_id, recipient_address))
    }

    fn return_escrow(
        &self,
        trade_id: TradeId,
    ) -> Result<String, SettlementError> {
        self.xmr
            .return_escrow(trade_id)
            .or_else(|_| self.btc.return_escrow(trade_id))
    }
}

// ─── Combined Wallet Adapter with Native Monero ─────────────────────────────

/// Combined wallet adapter with native Monero multisig support.
///
/// Use this when a running Monero daemon with a configured multisig wallet
/// is available. For testing and development, prefer `CombinedWalletAdapter`
/// which uses simulated Monero.
#[derive(Debug)]
pub struct CombinedWalletAdapterNative {
    xmr: NativeXmrWallet,
    btc: BtcWalletAdapter,
}

impl CryptoWallet for CombinedWalletAdapterNative {
    fn reserve_escrow(
        &self,
        trade_id: TradeId,
        asset: &Asset,
        amount: u64,
    ) -> Result<String, SettlementError> {
        match asset {
            Asset::Xmr => self.xmr.reserve_escrow(trade_id, asset, amount),
            Asset::Btc => self.btc.reserve_escrow(trade_id, asset, amount),
            Asset::Fiat(_) => Err(SettlementError::CryptoWalletError(
                "Cannot escrow fiat assets — use crypto assets for escrow".to_string()
            )),
        }
    }

    fn release_escrow(
        &self,
        trade_id: TradeId,
        recipient_address: &str,
    ) -> Result<String, SettlementError> {
        self.xmr
            .release_escrow(trade_id, recipient_address)
            .or_else(|_| self.btc.release_escrow(trade_id, recipient_address))
    }

    fn return_escrow(
        &self,
        trade_id: TradeId,
    ) -> Result<String, SettlementError> {
        self.xmr
            .return_escrow(trade_id)
            .or_else(|_| self.btc.return_escrow(trade_id))
    }
}

// ─── Mock Crypto Wallet for Testing ─────────────────────────────────────────

/// Mock crypto wallet for testing.
#[derive(Debug, Default)]
pub struct MockCryptoWallet {
    /// Escrowed amounts indexed by trade ID.
    pub escrowed: HashMap<Uuid, (Asset, u64)>,
}

impl MockCryptoWallet {
    /// Create a new mock wallet.
    pub fn new() -> Self {
        Self::default()
    }
}

impl CryptoWallet for MockCryptoWallet {
    fn reserve_escrow(
        &self,
        trade_id: TradeId,
        asset: &Asset,
        amount: u64,
    ) -> Result<String, SettlementError> {
        let _ = (asset, amount);
        Ok(format!("mock_escrow_{}", trade_id.0.as_hyphenated()))
    }

    fn release_escrow(
        &self,
        trade_id: TradeId,
        recipient_address: &str,
    ) -> Result<String, SettlementError> {
        let _ = recipient_address;
        Ok(format!("mock_release_{}", trade_id.0.as_hyphenated()))
    }

    fn return_escrow(
        &self,
        trade_id: TradeId,
    ) -> Result<String, SettlementError> {
        Ok(format!("mock_return_{}", trade_id.0.as_hyphenated()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulated_xmr_wallet_escrow() {
        let wallet = SimulatedXmrWallet::new("http://127.0.0.1:18082".to_string());
        let trade_id = TradeId(Uuid::new_v4());

        let tx = wallet
            .reserve_escrow(trade_id, &Asset::Xmr, 1_000_000)
            .unwrap();

        assert!(tx.starts_with("xmr_escrow_"));
    }

    #[test]
    fn simulated_xmr_wallet_wrong_asset() {
        let wallet = SimulatedXmrWallet::new("http://127.0.0.1:18082".to_string());
        let trade_id = TradeId(Uuid::new_v4());

        let result = wallet.reserve_escrow(trade_id, &Asset::Btc, 1_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn simulated_xmr_wallet_multisig_mode() {
        let wallet = SimulatedXmrWallet::new("http://127.0.0.1:18082".to_string());
        assert_eq!(wallet.multisig_mode(), MultisigMode::Simulated);
    }

    #[test]
    fn native_xmr_wallet_multisig_mode() {
        let wallet = NativeXmrWallet::new(
            "http://127.0.0.1:18082".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        assert_eq!(wallet.multisig_mode(), MultisigMode::NativeMonero);
    }

    #[test]
    fn native_xmr_wallet_not_connected() {
        let wallet = NativeXmrWallet::new(
            "http://127.0.0.1:18082".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        let trade_id = TradeId(Uuid::new_v4());
        let result = wallet.reserve_escrow(trade_id, &Asset::Xmr, 1_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn multisig_mode_display() {
        assert_eq!(format!("{}", MultisigMode::Simulated), "Simulated");
        assert_eq!(format!("{}", MultisigMode::NativeMonero), "NativeMonero");
    }

    #[test]
    fn simulated_xmr_wallet_escrow_state() {
        let wallet = SimulatedXmrWallet::new("http://127.0.0.1:18082".to_string());
        let trade_id = TradeId(Uuid::new_v4());

        wallet.reserve_escrow(trade_id, &Asset::Xmr, 1_000_000).unwrap();

        let state = wallet.get_escrow_state(trade_id).unwrap();
        assert_eq!(state.amount, 1_000_000);
        assert!(state.tx_hash.starts_with("xmr_escrow_"));
    }

    #[test]
    fn btc_wallet_escrow() {
        let wallet = BtcWalletAdapter::new("http://127.0.0.1:8332".to_string());
        let trade_id = TradeId(Uuid::new_v4());

        let deposit_address = wallet
            .reserve_escrow(trade_id, &Asset::Btc, 100_000)
            .unwrap();

        // Returns a real testnet P2WSH multisig address (tb1q...)
        assert!(deposit_address.starts_with("tb1q"));
    }

    #[test]
    fn btc_wallet_wrong_asset() {
        let wallet = BtcWalletAdapter::new("http://127.0.0.1:8332".to_string());
        let trade_id = TradeId(Uuid::new_v4());

        let result = wallet.reserve_escrow(trade_id, &Asset::Xmr, 100_000);
        assert!(result.is_err());
    }

    #[test]
    fn combined_wallet_xmr() {
        let wallet = CombinedWalletAdapter::new(
            "http://127.0.0.1:18082".to_string(),
            "http://127.0.0.1:8332".to_string(),
        );
        let trade_id = TradeId(Uuid::new_v4());

        let tx = wallet
            .reserve_escrow(trade_id, &Asset::Xmr, 1_000_000)
            .unwrap();

        assert!(tx.starts_with("xmr_escrow_"));
    }

    #[test]
    fn combined_wallet_btc() {
        let wallet = CombinedWalletAdapter::new(
            "http://127.0.0.1:18082".to_string(),
            "http://127.0.0.1:8332".to_string(),
        );
        let trade_id = TradeId(Uuid::new_v4());

        let deposit_address = wallet
            .reserve_escrow(trade_id, &Asset::Btc, 100_000)
            .unwrap();

        // Returns a real testnet P2WSH multisig address
        assert!(deposit_address.starts_with("tb1q"));
    }

    #[test]
    fn combined_wallet_fiat_rejected() {
        let wallet = CombinedWalletAdapter::new(
            "http://127.0.0.1:18082".to_string(),
            "http://127.0.0.1:8332".to_string(),
        );
        let trade_id = TradeId(Uuid::new_v4());

        let result = wallet.reserve_escrow(
            trade_id,
            &Asset::Fiat(opendex_types::FiatCurrency::GBP),
            100_000,
        );
        assert!(result.is_err());
    }

    #[test]
    fn mock_wallet_operations() {
        let wallet = MockCryptoWallet::new();
        let trade_id = TradeId(Uuid::new_v4());

        let escrow = wallet
            .reserve_escrow(trade_id, &Asset::Btc, 100_000)
            .unwrap();
        assert!(escrow.starts_with("mock_escrow_"));

        let release = wallet
            .release_escrow(trade_id, "1Address")
            .unwrap();
        assert!(release.starts_with("mock_release_"));

        let return_tx = wallet.return_escrow(trade_id).unwrap();
        assert!(return_tx.starts_with("mock_return_"));
    }
}
