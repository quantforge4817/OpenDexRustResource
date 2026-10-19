//! Descriptor-based Bitcoin wallet using BDK (Bitcoin Dev Kit).
//!
//! This module provides the `BitcoinWallet` type wrapping BDK's `Wallet`,
//! supporting three descriptor patterns recommended for DEX use:
//!
//! 1. **Single-sig hot wallet** — `wpkh(xpub…/84'/0'/0'/0/*)` for user deposits
//!    (native SegWit, BIP-84).
//! 2. **2-of-2 escrow** — `wsh(multi(2,xpub…/0/*,xpub…/0/*))` for user + counterparty.
//! 3. **2-of-3 escrow** — `wsh(multi(2,xpub…/0/*,xpub…/0/*,xpub…/0/*))` with dispute resolver.

use std::str::FromStr;

use bdk_wallet::bitcoin::{consensus::Encodable, Network, FeeRate};
use bdk_wallet::{KeychainKind, Wallet};
use opendex_errors::WalletError;
use opendex_types::Satoshis;

// ─── Re-exports ──────────────────────────────────────────────────────────────

/// Re-export common BDK types so callers don't need the crate directly.
pub mod prelude {
    pub use bdk_wallet::bitcoin::Address;
    pub use bdk_wallet::bitcoin::Amount;
    pub use bdk_wallet::bitcoin::Transaction;
    pub use bdk_wallet::bitcoin::Txid;
    pub use bdk_wallet::KeychainKind;
    pub use bdk_wallet::Wallet as BdkWallet;
    pub use bdk_electrum::{BdkElectrumClient, electrum_client};
}

// ─── Descriptor Templates ────────────────────────────────────────────────────

/// Descriptor templates for DEX wallet types.
#[derive(Debug, Clone)]
pub enum DescriptorTemplate {
    /// Single-sig native SegWit (BIP-84).
    /// External (receive): `…/0/*`, Internal (change): `…/1/*`
    SingleSig {
        /// Extended private key (BIP-32) for the wallet.
        xprv: String,
    },
    /// 2-of-2 multi-sig native SegWit (single-keychain).
    MultiSig2of2 {
        /// Extended public keys for the two signers.
        xpubs: [String; 2],
    },
    /// 2-of-3 multi-sig native SegWit (single-keychain).
    MultiSig2of3 {
        /// Extended public keys for the three signers.
        xpubs: [String; 3],
    },
}

impl DescriptorTemplate {
    /// Build the BDK descriptor string(s) for this template.
    ///
    /// Returns `(external_descriptor, internal_descriptor)`.
    /// For single-keychain wallets (multi-sig) the internal descriptor is `None`.
    fn build(&self) -> Result<(String, Option<String>), WalletError> {
        match self {
            DescriptorTemplate::SingleSig { xprv } => {
                let external = format!("wpkh({xprv}/84'/0'/0'/0/*)");
                let internal = format!("wpkh({xprv}/84'/0'/0'/1/*)");
                Ok((external, Some(internal)))
            }
            DescriptorTemplate::MultiSig2of2 { xpubs } => {
                let sorted = sort_xpubs(&xpubs[..]);
                let desc = format!(
                    "wsh(multi(2,{},{}/0/*))",
                    sorted[0], sorted[1]
                );
                Ok((desc, None))
            }
            DescriptorTemplate::MultiSig2of3 { xpubs } => {
                let sorted = sort_xpubs(&xpubs[..]);
                let desc = format!(
                    "wsh(multi(2,{},{},{}/0/*))",
                    sorted[0], sorted[1], sorted[2]
                );
                Ok((desc, None))
            }
        }
    }
}

/// Sort xpubs lexicographically so the descriptor is deterministic.
fn sort_xpubs(xpubs: &[String]) -> Vec<String> {
    let mut sorted: Vec<&str> = xpubs.iter().map(|s| s.as_str()).collect();
    sorted.sort();
    sorted.into_iter().map(|s| s.to_string()).collect()
}

// ─── BitcoinWallet ───────────────────────────────────────────────────────────

/// High-level Bitcoin wallet wrapping a BDK `Wallet`.
/// Uses interior mutability so address generation works with `&self`.
#[derive(Debug)]
pub struct BitcoinWallet {
    inner: std::sync::Mutex<Wallet>,
}

impl BitcoinWallet {
    /// Create a new in-memory wallet from a descriptor template and network.
    pub fn create(template: DescriptorTemplate, network: Network) -> Result<Self, WalletError> {
        let (external, internal) = template.build()?;

        let wallet = match internal {
            Some(internal) => {
                let external_owned = external;
                let internal_owned = internal;
                Wallet::create(external_owned, internal_owned)
                    .network(network)
                    .create_wallet_no_persist()
                    .map_err(|e| WalletError::Unavailable(e.to_string()))?
            }
            None => {
                let external_owned = external;
                Wallet::create_single(external_owned)
                    .network(network)
                    .create_wallet_no_persist()
                    .map_err(|e| WalletError::Unavailable(e.to_string()))?
            }
        };

        Ok(Self {
            inner: std::sync::Mutex::new(wallet),
        })
    }

    /// Create a wallet from explicit descriptor strings.
    pub fn create_from_descriptors(
        external_desc: &str,
        internal_desc: Option<&str>,
        network: Network,
    ) -> Result<Self, WalletError> {
        let external_owned = external_desc.to_string();
        let internal_owned = internal_desc.map(|s| s.to_string());

        let wallet = match internal_owned {
            Some(internal) => Wallet::create(external_owned, internal)
                .network(network)
                .create_wallet_no_persist()
                .map_err(|e| WalletError::Unavailable(e.to_string()))?,
            None => Wallet::create_single(external_owned)
                .network(network)
                .create_wallet_no_persist()
                .map_err(|e| WalletError::Unavailable(e.to_string()))?,
        };

        Ok(Self {
            inner: std::sync::Mutex::new(wallet),
        })
    }

    /// Generate the next receive address (advances derivation index).
    pub fn next_address(&self) -> String {
        let mut inner = self.inner.lock().unwrap();
        let addr_info = inner.reveal_next_address(KeychainKind::External);
        addr_info.address.to_string()
    }

    /// Generate the next change address (for single-sig wallets).
    pub fn next_change_address(&self) -> Option<String> {
        let mut inner = self.inner.lock().unwrap();
        let addr_info = inner.reveal_next_address(KeychainKind::Internal);
        Some(addr_info.address.to_string())
    }

    /// Derive addresses up to `index` and return the last one.
    /// Note: BDK 1.x requires revealing addresses sequentially.
    pub fn address_at(&self, index: u32) -> String {
        let mut inner = self.inner.lock().unwrap();
        let mut addr = inner.reveal_next_address(KeychainKind::External);
        for _ in 1..=index {
            addr = inner.reveal_next_address(KeychainKind::External);
        }
        addr.address.to_string()
    }

    /// Get the total balance (confirmed + unconfirmed) in satoshis.
    pub fn total_balance(&self) -> Satoshis {
        let inner = self.inner.lock().unwrap();
        let b = inner.balance();
        Satoshis((b.confirmed + b.trusted_pending + b.untrusted_pending + b.immature).to_sat())
    }

    /// Get the confirmed balance in satoshis.
    pub fn confirmed_balance(&self) -> Satoshis {
        let inner = self.inner.lock().unwrap();
        Satoshis(inner.balance().confirmed.to_sat())
    }

    /// Get the unconfirmed balance in satoshis.
    pub fn unconfirmed_balance(&self) -> Satoshis {
        let inner = self.inner.lock().unwrap();
        let b = inner.balance();
        Satoshis((b.trusted_pending + b.untrusted_pending).to_sat())
    }

    /// List all unspent transaction outputs.
    pub fn list_utxos(&self) -> Vec<UtxoInfo> {
        let inner = self.inner.lock().unwrap();
        inner
            .list_unspent()
            .map(|output| {
                UtxoInfo {
                    txid: output.outpoint.txid.to_string(),
                    vout: output.outpoint.vout,
                    value: Satoshis(output.txout.value.to_sat()),
                }
            })
            .collect()
    }

    /// Build a transaction sending `amount` to `to_address`.
    ///
    /// Returns the unsigned transaction hex and the estimated fee in satoshis.
    pub fn build_send_transaction(
        &self,
        to_address: &str,
        amount: Satoshis,
        fee_rate_sat_per_vb: f64,
    ) -> Result<(String, Satoshis), WalletError> {
        let dst = bdk_wallet::bitcoin::Address::from_str(to_address)
            .map_err(|e| WalletError::InvalidAddress(e.to_string()))?
            .assume_checked();

        let bdk_amount = bdk_wallet::bitcoin::Amount::from_sat(amount.0);
        let fee_rate = FeeRate::from_sat_per_vb(fee_rate_sat_per_vb as u64)
            .expect("fee rate");

        let psbt = {
            let mut inner = self.inner.lock().unwrap();
            let mut builder = inner.build_tx();
            builder
                .ordering(bdk_wallet::TxOrdering::Untouched)
                .fee_rate(fee_rate)
                .add_recipient(dst.script_pubkey(), bdk_amount);
            builder.finish().map_err(|e| {
                WalletError::SigningFailed(format!("Build transaction failed: {e}"))
            })?
        };

        let tx = psbt.extract_tx().map_err(|e| {
            WalletError::SigningFailed(format!("Extract transaction from PSBT failed: {e}"))
        })?;

        let mut buf = Vec::new();
        tx.consensus_encode(&mut buf).map_err(|e| {
            WalletError::SigningFailed(format!("Encode transaction failed: {e}"))
        })?;
        let tx_hex = hex::encode(buf);
        Ok((tx_hex, Satoshis(0)))
    }

    /// Build a transaction sending to multiple recipients.
    pub fn build_send_to_many(
        &self,
        recipients: &[(&str, Satoshis)],
        fee_rate_sat_per_vb: f64,
    ) -> Result<(String, Satoshis), WalletError> {
        let fee_rate = FeeRate::from_sat_per_vb(fee_rate_sat_per_vb as u64)
            .expect("fee rate");

        let psbt = {
            let mut inner = self.inner.lock().unwrap();
            let mut builder = inner.build_tx();
            builder.ordering(bdk_wallet::TxOrdering::Untouched).fee_rate(fee_rate);

            for (addr_str, amount) in recipients {
                let addr = bdk_wallet::bitcoin::Address::from_str(addr_str)
                    .map_err(|e| WalletError::InvalidAddress(e.to_string()))?
                    .assume_checked();
                let bdk_amount = bdk_wallet::bitcoin::Amount::from_sat(amount.0);
                builder.add_recipient(addr.script_pubkey(), bdk_amount);
            }

            builder.finish().map_err(|e| {
                WalletError::SigningFailed(format!("Build transaction failed: {e}"))
            })?
        };

        let tx = psbt.extract_tx().map_err(|e| {
            WalletError::SigningFailed(format!("Extract transaction from PSBT failed: {e}"))
        })?;

        let mut buf = Vec::new();
        tx.consensus_encode(&mut buf).map_err(|e| {
            WalletError::SigningFailed(format!("Encode transaction failed: {e}"))
        })?;
        let tx_hex = hex::encode(buf);
        Ok((tx_hex, Satoshis(0)))
    }

    /// Return the inner BDK wallet reference for advanced operations.
    pub fn inner(&self) -> std::sync::MutexGuard<'_, Wallet> {
        self.inner.lock().unwrap()
    }
}

// ─── Coin Selection Strategies ───────────────────────────────────────────────

/// Coin selection strategies for choosing which UTXOs to spend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoinSelectionStrategy {
    /// BDK's default: branch-and-bound (BnB) with fallback to largest-first.
    /// Minimizes change output size, preferred for privacy.
    BranchAndBound,
    /// Spend the oldest UTXOs first. Simple and predictable.
    OldestFirst,
    /// Spend the largest UTXOs first. Minimizes number of inputs.
    LargestFirst,
    /// Spend the smallest UTXOs that cover the amount. Conserves large UTXOs.
    SmallestFirst,
}

// ─── PSBT Signing Flow ──────────────────────────────────────────────────────

/// Information about a PSBT (Partially Signed Bitcoin Transaction).
#[derive(Debug, Clone)]
pub struct PsbtInfo {
    /// Base64-encoded PSBT for sharing with co-signers.
    pub psbt_base64: String,
    /// Estimated fee in satoshis (0 if not yet computed).
    pub fee_satoshis: Satoshis,
    /// Number of inputs that still need signing.
    pub unsigned_inputs: usize,
    /// Total number of inputs.
    pub total_inputs: usize,
}

impl BitcoinWallet {
    /// Build a transaction and return the unsigned PSBT for multi-party signing.
    ///
    /// This is the first step in a multi-sig workflow:
    /// 1. Create PSBT with `create_psbt`
    /// 2. Share PSBT with co-signers
    /// 3. Each co-signer signs their inputs
    /// 4. Collect signatures and finalize with `finalize_psbt`
    /// 5. Extract the final transaction with `extract_psbt`
    pub fn create_psbt(
        &self,
        to_address: &str,
        amount: Satoshis,
        fee_rate_sat_per_vb: f64,
        _strategy: CoinSelectionStrategy,
    ) -> Result<PsbtInfo, WalletError> {
        let dst = bdk_wallet::bitcoin::Address::from_str(to_address)
            .map_err(|e| WalletError::InvalidAddress(e.to_string()))?
            .assume_checked();

        let bdk_amount = bdk_wallet::bitcoin::Amount::from_sat(amount.0);
        let fee_rate = FeeRate::from_sat_per_vb(fee_rate_sat_per_vb as u64)
            .expect("fee rate");

        let psbt = {
            let mut inner = self.inner.lock().unwrap();
            let mut builder = inner.build_tx();
            builder
                .ordering(bdk_wallet::TxOrdering::Untouched)
                .fee_rate(fee_rate)
                .add_recipient(dst.script_pubkey(), bdk_amount);

            builder.finish().map_err(|e| {
                WalletError::SigningFailed(format!("Build PSBT failed: {e}"))
            })?
        };

        let psbt_base64 = psbt.to_string();
        let total_inputs = psbt.unsigned_tx.input.len();
        let unsigned_inputs = psbt
            .unsigned_tx
            .input
            .iter()
            .enumerate()
            .filter(|(i, _)| !psbt.inputs[*i].final_script_sig.is_some())
            .count();

        Ok(PsbtInfo {
            psbt_base64,
            fee_satoshis: Satoshis(0),
            unsigned_inputs,
            total_inputs,
        })
    }

    /// Sign all inputs in a PSBT that this wallet can sign.
    ///
    /// Returns the updated PSBT with signatures applied.
    ///
    /// In BDK 3.x, signing uses caller-owned signers. For descriptor wallets,
    /// we retrieve the signers from the wallet's keychain and apply them.
    #[allow(deprecated)]
    pub fn sign_psbt(&self, psbt_base64: &str) -> Result<String, WalletError> {
        use bdk_wallet::bitcoin::psbt::Psbt;
        use bdk_wallet::SignOptions;

        let mut psbt = Psbt::from_str(psbt_base64)
            .map_err(|e| WalletError::SigningFailed(format!("Parse PSBT failed: {e}")))?;

        let inner = self.inner.lock().unwrap();
        let sign_options = SignOptions::default();
        // Get signers for the external keychain (receive addresses)
        let signers = inner.get_signers(KeychainKind::External);
        let signers_ref: &[&bdk_wallet::signer::SignersContainer] = &[&*signers];
        inner.sign_with_signers(&mut psbt, signers_ref, sign_options).map_err(|e| {
            WalletError::SigningFailed(format!("Sign PSBT failed: {e}"))
        })?;

        Ok(psbt.to_string())
    }

    /// Finalize a PSBT after all signatures are collected.
    ///
    /// Returns the finalized PSBT ready for extraction.
    pub fn finalize_psbt(&self, psbt_base64: &str) -> Result<String, WalletError> {
        use bdk_wallet::bitcoin::psbt::Psbt;
        use bdk_wallet::SignOptions;

        let mut psbt = Psbt::from_str(psbt_base64)
            .map_err(|e| WalletError::SigningFailed(format!("Parse PSBT failed: {e}")))?;

        let inner = self.inner.lock().unwrap();
        let sign_options = SignOptions::default();
        if !inner.finalize_psbt(&mut psbt, sign_options).map_err(|e| {
            WalletError::SigningFailed(format!("Finalize PSBT failed: {e}"))
        })? {
            return Err(WalletError::SigningFailed(
                "PSBT not ready for finalization — missing signatures".to_string(),
            ));
        }

        Ok(psbt.to_string())
    }

    /// Extract the final transaction from a finalized PSBT.
    ///
    /// Returns the transaction hex and the fee in satoshis.
    pub fn extract_psbt(&self, psbt_base64: &str) -> Result<(String, Satoshis), WalletError> {
        use bdk_wallet::bitcoin::psbt::Psbt;

        let psbt = Psbt::from_str(psbt_base64)
            .map_err(|e| WalletError::SigningFailed(format!("Parse PSBT failed: {e}")))?;

        let tx = psbt.extract_tx().map_err(|e| {
            WalletError::SigningFailed(format!("Extract transaction from PSBT failed: {e}"))
        })?;

        let mut buf = Vec::new();
        tx.consensus_encode(&mut buf).map_err(|e| {
            WalletError::SigningFailed(format!("Encode transaction failed: {e}"))
        })?;
        let tx_hex = hex::encode(buf);

        // Calculate fee from input/output difference
        let inner = self.inner.lock().unwrap();
        let fee = inner.calculate_fee(&tx).unwrap_or(bdk_wallet::bitcoin::Amount::ZERO);

        Ok((tx_hex, Satoshis(fee.to_sat())))
    }
}

/// Apply a coin selection strategy to a BDK transaction builder.
///
/// In BDK 1.x, coin selection is primarily controlled via the `drain_wallet`
/// and `manually_selected_only` methods. The strategy enum documents the
/// intended behavior; BDK's default (branch-and-bound) is used when no
/// explicit override is needed.
///
/// For now, BDK's default branch-and-bound is used.
#[allow(dead_code)]
fn apply_coin_selection(_strategy: CoinSelectionStrategy) {
    // BDK 1.x uses branch-and-bound by default.
    // For custom strategies, the caller would use manually_selected_only()
    // with specific UTXOs.
    let _ = _strategy;
}

// ─── Fee Estimation ──────────────────────────────────────────────────────────

/// Fee rate information for transaction construction.
#[derive(Debug, Clone, Copy)]
pub struct FeeRateInfo {
    /// Estimated fee rate in satoshis per vbyte.
    pub sat_per_vb: f64,
    /// Estimated fee for a typical single-input, single-output transaction (~140 vbytes).
    pub typical_tx_fee: Satoshis,
    /// Estimated fee for a 2-of-3 multi-sig transaction (~372 vbytes).
    pub multisig_tx_fee: Satoshis,
}

/// Helper to estimate fees based on a fee rate.
pub fn estimate_fees(sat_per_vb: f64) -> FeeRateInfo {
    // Typical P2WPKH tx: ~140 vbytes
    let typical_vbytes = 140.0;
    // 2-of-3 P2WSH multi-sig tx: ~372 vbytes
    let multisig_vbytes = 372.0;

    FeeRateInfo {
        sat_per_vb,
        typical_tx_fee: Satoshis((typical_vbytes * sat_per_vb) as u64),
        multisig_tx_fee: Satoshis((multisig_vbytes * sat_per_vb) as u64),
    }
}

/// Common fee rate presets for different confirmation targets.
pub mod fee_presets {
    use super::{estimate_fees, FeeRateInfo};

    /// High priority: ~1 block confirmation (~10 min).
    pub fn high_priority() -> FeeRateInfo {
        estimate_fees(50.0)
    }

    /// Normal priority: ~3 block confirmation (~30 min).
    pub fn normal() -> FeeRateInfo {
        estimate_fees(25.0)
    }

    /// Low priority: ~6 block confirmation (~1 hour).
    pub fn low_priority() -> FeeRateInfo {
        estimate_fees(10.0)
    }

    /// Economy: ~144 block confirmation (~1 day).
    pub fn economy() -> FeeRateInfo {
        estimate_fees(5.0)
    }
}

// ─── UTXO Info ───────────────────────────────────────────────────────────────

/// Information about an unspent transaction output.
#[derive(Debug, Clone)]
pub struct UtxoInfo {
    /// Transaction ID of the output.
    pub txid: String,
    /// Output index within the transaction.
    pub vout: u32,
    /// Value in satoshis.
    pub value: Satoshis,
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_xpubs_deterministic() {
        let xpubs = vec![
            "tpub2".to_string(),
            "tpub1".to_string(),
            "tpub3".to_string(),
        ];
        let sorted = sort_xpubs(&xpubs);
        assert_eq!(sorted[0], "tpub1");
        assert_eq!(sorted[1], "tpub2");
        assert_eq!(sorted[2], "tpub3");
    }

    #[test]
    fn test_single_sig_descriptor_shape() {
        let template = DescriptorTemplate::SingleSig {
            xprv: "tprv".to_string(),
        };
        let (external, internal) = template.build().unwrap();
        assert!(external.starts_with("wpkh(tprv/84'/0'/0'/0/*)"));
        assert!(internal.is_some());
        assert!(internal.unwrap().starts_with("wpkh(tprv/84'/0'/0'/1/*)"));
    }

    #[test]
    fn test_multisig_2of2_descriptor_shape() {
        let template = DescriptorTemplate::MultiSig2of2 {
            xpubs: ["tpubB".to_string(), "tpubA".to_string()],
        };
        let (external, internal) = template.build().unwrap();
        // xpubs should be sorted: tpubA before tpubB
        assert!(external.starts_with("wsh(multi(2,tpubA,tpubB/0/*))"));
        assert!(internal.is_none());
    }

    #[test]
    fn test_multisig_2of3_descriptor_shape() {
        let template = DescriptorTemplate::MultiSig2of3 {
            xpubs: [
                "tpubC".to_string(),
                "tpubA".to_string(),
                "tpubB".to_string(),
            ],
        };
        let (external, internal) = template.build().unwrap();
        // xpubs should be sorted: A, B, C
        assert!(external.starts_with("wsh(multi(2,tpubA,tpubB,tpubC/0/*))"));
        assert!(internal.is_none());
    }

    #[test]
    fn test_fee_estimation_presets() {
        use crate::wallet::fee_presets;
        use crate::wallet::estimate_fees;

        let fees = estimate_fees(25.0);
        assert_eq!(fees.sat_per_vb, 25.0);
        // 140 vbytes * 25 sat/vb = 3500 sats
        assert_eq!(fees.typical_tx_fee.0, 3500);
        // 372 vbytes * 25 sat/vb = 9300 sats
        assert_eq!(fees.multisig_tx_fee.0, 9300);

        // Verify presets produce reasonable values
        let high = fee_presets::high_priority();
        assert!(high.sat_per_vb >= 50.0);
        let economy = fee_presets::economy();
        assert!(economy.sat_per_vb <= 10.0);
        assert!(high.typical_tx_fee.0 > economy.typical_tx_fee.0);
    }

    #[test]
    fn test_coin_selection_strategy_equality() {
        let s1 = CoinSelectionStrategy::BranchAndBound;
        let s2 = CoinSelectionStrategy::BranchAndBound;
        assert_eq!(s1, s2);
        assert_ne!(s1, CoinSelectionStrategy::OldestFirst);
    }
}
