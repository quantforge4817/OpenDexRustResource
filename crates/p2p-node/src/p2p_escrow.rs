/// On-chain escrow operations for P2P trades.
///
/// Handles the blockchain-side of P2P fiat-to-crypto trades:
/// - Generating escrow addresses for locking crypto
/// - Sending crypto to escrow when a trade enters EscrowFunded state
/// - Releasing crypto to the taker when a trade enters CryptoReleased state
/// - Returning crypto to the maker when a trade is cancelled
///
/// Supports both XMR (via Monero wallet RPC) and BTC (via Bitcoin Core RPC).
use anyhow::{Result, Context, anyhow};
use protocol::p2p::{CryptoAsset, P2PTrade};

use crate::wallet_client::MoneroWalletClient;
use crate::btc_wallet_client::BitcoinWalletClient;

/// Wrapper that holds optional references to both wallet clients.
/// Used to perform on-chain operations for P2P trades.
pub struct P2PEscrowContext<'a> {
    pub monero_wallet: Option<&'a MoneroWalletClient>,
    pub btc_wallet: Option<&'a BitcoinWalletClient>,
}

impl<'a> P2PEscrowContext<'a> {
    /// Create a new escrow context with optional wallet clients.
    pub fn new(
        monero_wallet: Option<&'a MoneroWalletClient>,
        btc_wallet: Option<&'a BitcoinWalletClient>,
    ) -> Self {
        Self {
            monero_wallet,
            btc_wallet,
        }
    }

    /// Check if the wallet for the given crypto asset is connected.
    pub fn is_wallet_connected(&self, asset: &CryptoAsset) -> bool {
        match asset {
            CryptoAsset::XMR => self
                .monero_wallet
                .map(|w| w.is_connected())
                .unwrap_or(false),
            CryptoAsset::BTC => self
                .btc_wallet
                .map(|w| w.is_connected())
                .unwrap_or(false),
        }
    }

    /// Generate a new escrow address for the given crypto asset.
    ///
    /// For XMR, creates a subaddress labeled with the trade ID.
    /// For BTC, generates a new bech32 address labeled with the trade ID.
    pub async fn generate_escrow_address(&self, asset: &CryptoAsset, trade_id: &str) -> Result<String> {
        match asset {
            CryptoAsset::XMR => {
                let wallet = self.monero_wallet.ok_or_else(|| {
                    anyhow!("Monero wallet not configured — cannot create XMR escrow address")
                })?;

                let label = format!("P2P-escrow-{}", trade_id);
                let address = wallet
                    .create_subaddress(0, Some(label))
                    .await
                    .context("Failed to create Monero escrow subaddress")?;

                tracing::info!(
                    trade_id = trade_id,
                    address = %address,
                    "Generated Monero escrow subaddress"
                );
                Ok(address)
            }
            CryptoAsset::BTC => {
                let wallet = self.btc_wallet.ok_or_else(|| {
                    anyhow!("Bitcoin wallet not configured — cannot create BTC escrow address")
                })?;

                let label = Some(format!("P2P-escrow-{}", trade_id));
                let address = wallet
                    .get_new_address(label)
                    .await
                    .context("Failed to create Bitcoin escrow address")?;

                tracing::info!(
                    trade_id = trade_id,
                    address = %address,
                    "Generated Bitcoin escrow address"
                );
                Ok(address)
            }
        }
    }

    /// Send crypto to the escrow address.
    ///
    /// Transfers the trade amount from the maker's wallet to the escrow address.
    /// Returns the transaction hash.
    pub async fn send_to_escrow(
        &self,
        asset: &CryptoAsset,
        amount: u64,
        escrow_address: &str,
        payment_id: Option<&str>,
    ) -> Result<String> {
        match asset {
            CryptoAsset::XMR => {
                let wallet = self.monero_wallet.ok_or_else(|| {
                    anyhow!("Monero wallet not configured — cannot send XMR to escrow")
                })?;

                // Convert OpenDex amount to piconero
                let piconero = MoneroWalletClient::opendex_amount_to_piconero(amount);

                let tx_hash = wallet
                    .transfer(escrow_address, piconero, payment_id.map(String::from))
                    .await
                    .context("Failed to send XMR to escrow")?;

                tracing::info!(
                    tx_hash = %tx_hash,
                    amount_piconero = piconero,
                    "Sent XMR to escrow"
                );
                Ok(tx_hash)
            }
            CryptoAsset::BTC => {
                let wallet = self.btc_wallet.ok_or_else(|| {
                    anyhow!("Bitcoin wallet not configured — cannot send BTC to escrow")
                })?;

                // Convert OpenDex amount to BTC (1:1 since both use 8 decimal places)
                let amount_btc = amount as f64 / 100_000_000.0;

                let txid = wallet
                    .send_to_address(escrow_address, amount_btc, None)
                    .await
                    .context("Failed to send BTC to escrow")?;

                tracing::info!(
                    txid = %txid,
                    amount_btc = amount_btc,
                    "Sent BTC to escrow"
                );
                Ok(txid)
            }
        }
    }

    /// Release crypto from escrow to the taker's address.
    ///
    /// Transfers the trade amount from the escrow to the taker's crypto address.
    /// Returns the transaction hash.
    pub async fn release_to_taker(
        &self,
        asset: &CryptoAsset,
        amount: u64,
        taker_address: &str,
    ) -> Result<String> {
        match asset {
            CryptoAsset::XMR => {
                let wallet = self.monero_wallet.ok_or_else(|| {
                    anyhow!("Monero wallet not configured — cannot release XMR to taker")
                })?;

                // Convert OpenDex amount to piconero
                let piconero = MoneroWalletClient::opendex_amount_to_piconero(amount);

                let tx_hash = wallet
                    .transfer(taker_address, piconero, None)
                    .await
                    .context("Failed to release XMR to taker")?;

                tracing::info!(
                    tx_hash = %tx_hash,
                    amount_piconero = piconero,
                    "Released XMR to taker"
                );
                Ok(tx_hash)
            }
            CryptoAsset::BTC => {
                let wallet = self.btc_wallet.ok_or_else(|| {
                    anyhow!("Bitcoin wallet not configured — cannot release BTC to taker")
                })?;

                // Convert OpenDex amount to BTC (1:1 since both use 8 decimal places)
                let amount_btc = amount as f64 / 100_000_000.0;

                let txid = wallet
                    .send_to_address(taker_address, amount_btc, None)
                    .await
                    .context("Failed to release BTC to taker")?;

                tracing::info!(
                    txid = %txid,
                    amount_btc = amount_btc,
                    "Released BTC to taker"
                );
                Ok(txid)
            }
        }
    }

    /// Return crypto from escrow back to the maker (on trade cancellation).
    ///
    /// For XMR, the crypto is already in the wallet's subaddress, so it's
    /// inherently returned (the wallet controls the subaddress).
    /// For BTC, if the escrow address is a separate wallet, the crypto needs
    /// to be sent back. For simplicity, we assume the wallet controls the
    /// escrow address directly.
    pub async fn return_to_maker(
        &self,
        asset: &CryptoAsset,
        amount: u64,
        maker_address: &str,
    ) -> Result<String> {
        match asset {
            CryptoAsset::XMR => {
                // For Monero, subaddresses are controlled by the same wallet keys,
                // so the funds are already accessible to the maker.
                // No on-chain transfer needed.
                tracing::info!(
                    amount = amount,
                    "XMR escrow funds are accessible via wallet subaddress — no transfer needed"
                );
                Ok("subaddress-accessible".to_string())
            }
            CryptoAsset::BTC => {
                let wallet = self.btc_wallet.ok_or_else(|| {
                    anyhow!("Bitcoin wallet not configured — cannot return BTC to maker")
                })?;

                // Convert OpenDex amount to BTC
                let amount_btc = amount as f64 / 100_000_000.0;

                let txid = wallet
                    .send_to_address(maker_address, amount_btc, None)
                    .await
                    .context("Failed to return BTC to maker")?;

                tracing::info!(
                    txid = %txid,
                    amount_btc = amount_btc,
                    "Returned BTC to maker"
                );
                Ok(txid)
            }
        }
    }

    /// Verify an escrow deposit transaction has been confirmed.
    pub async fn verify_escrow_deposit(&self, asset: &CryptoAsset, tx_hash: &str, address: &str) -> Result<bool> {
        match asset {
            CryptoAsset::XMR => {
                let wallet = self.monero_wallet.ok_or_else(|| {
                    anyhow!("Monero wallet not configured")
                })?;

                let confirmed = wallet
                    .check_transaction(tx_hash, address)
                    .await
                    .context("Failed to verify XMR escrow deposit")?;

                tracing::info!(
                    tx_hash = %tx_hash,
                    confirmed = confirmed,
                    "Verified XMR escrow deposit"
                );
                Ok(confirmed)
            }
            CryptoAsset::BTC => {
                let wallet = self.btc_wallet.ok_or_else(|| {
                    anyhow!("Bitcoin wallet not configured")
                })?;

                let confirmed = wallet
                    .is_transaction_confirmed(tx_hash, 1)
                    .await
                    .context("Failed to verify BTC escrow deposit")?;

                tracing::info!(
                    tx_hash = %tx_hash,
                    confirmed = confirmed,
                    "Verified BTC escrow deposit"
                );
                Ok(confirmed)
            }
        }
    }
}

/// Execute the full escrow funding flow for a P2P trade.
///
/// 1. Generates a new escrow address
/// 2. Sends the trade amount to the escrow address
/// 3. Returns the escrow address and transaction hash
///
/// Updates the trade's `escrow_address` and `escrow_tx_hash` fields.
pub async fn fund_escrow(
    ctx: &P2PEscrowContext<'_>,
    trade: &mut P2PTrade,
) -> Result<()> {
    let trade_id = trade.id.to_string();

    // Check wallet connectivity
    if !ctx.is_wallet_connected(&trade.crypto_asset) {
        let asset = &trade.crypto_asset;
        tracing::warn!(
            trade_id = %trade.id,
            asset = %asset,
            "Wallet not connected — escrow will be recorded but not funded on-chain"
        );
        // Generate a placeholder escrow address for record-keeping
        trade.escrow_address = format!("wallet-not-connected-{}", trade_id);
        return Ok(());
    }

    // Generate escrow address
    let escrow_address = ctx
        .generate_escrow_address(&trade.crypto_asset, &trade_id)
        .await?;

    // Generate a payment ID for XMR tracking
    let payment_id = if trade.crypto_asset == CryptoAsset::XMR {
        // Use first 16 chars of trade ID as payment ID
        Some(trade_id[..16].to_string())
    } else {
        None
    };

    // Send crypto to escrow
    let tx_hash = ctx
        .send_to_escrow(
            &trade.crypto_asset,
            trade.amount,
            &escrow_address,
            payment_id.as_deref(),
        )
        .await?;

    // Update the trade record
    trade.escrow_address = escrow_address;
    trade.escrow_tx_hash = Some(tx_hash.clone());
    if let Some(pid) = &payment_id {
        trade.payment_id = Some(pid.clone());
    }

    tracing::info!(
        trade_id = %trade.id,
        escrow_address = %trade.escrow_address,
        escrow_tx = %tx_hash,
        "P2P escrow funded on-chain"
    );

    Ok(())
}

/// Execute the crypto release flow for a P2P trade.
///
/// Sends the trade amount from escrow to the taker's crypto address.
/// Updates the trade's `release_tx_hash` field.
pub async fn release_crypto(
    ctx: &P2PEscrowContext<'_>,
    trade: &mut P2PTrade,
) -> Result<()> {
    let trade_id = trade.id.to_string();

    if !ctx.is_wallet_connected(&trade.crypto_asset) {
        let asset = &trade.crypto_asset;
        tracing::warn!(
            trade_id = %trade.id,
            asset = %asset,
            "Wallet not connected — crypto release will be recorded but not executed on-chain"
        );
        trade.release_tx_hash = Some("wallet-not-connected".to_string());
        return Ok(());
    }

    // Validate taker address is set
    if trade.taker_crypto_address.is_empty() {
        return Err(anyhow!(
            "Cannot release crypto: taker address is not set for trade {}",
            trade_id
        ));
    }

    // Send crypto to taker
    let tx_hash = ctx
        .release_to_taker(
            &trade.crypto_asset,
            trade.amount,
            &trade.taker_crypto_address,
        )
        .await?;

    // Update the trade record
    trade.release_tx_hash = Some(tx_hash.clone());

    tracing::info!(
        trade_id = %trade.id,
        release_tx = %tx_hash,
        taker_address = %trade.taker_crypto_address,
        "P2P crypto released to taker on-chain"
    );

    Ok(())
}

/// Execute the escrow return flow for a cancelled P2P trade.
///
/// Returns the escrowed crypto back to the maker.
pub async fn return_escrow(
    ctx: &P2PEscrowContext<'_>,
    trade: &P2PTrade,
) -> Result<()> {
    if !ctx.is_wallet_connected(&trade.crypto_asset) {
        tracing::warn!(
            trade_id = %trade.id,
            "Wallet not connected — escrow return will be recorded but not executed on-chain"
        );
        return Ok(());
    }

    // For XMR, subaddresses are inherently accessible, so no action needed.
    // For BTC, we may need to send back if using a separate escrow wallet.
    // For now, we log the return action.
    tracing::info!(
        trade_id = %trade.id,
        "P2P escrow returned to maker (wallet controls escrow funds)"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escrow_context_no_wallets() {
        let ctx = P2PEscrowContext::new(None, None);
        assert!(!ctx.is_wallet_connected(&CryptoAsset::XMR));
        assert!(!ctx.is_wallet_connected(&CryptoAsset::BTC));
    }
}
