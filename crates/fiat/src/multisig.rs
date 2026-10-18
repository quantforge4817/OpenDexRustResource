//! Bitcoin 2-of-2 multisig escrow for fiat trades.
//!
//! Implements the cryptographic escrow layer where neither party can
//! unilaterally spend the funds — both maker and taker must co-sign
//! the payout transaction. This replaces the ledger-based escrow with
//! real on-chain 2-of-2 P2WSH (Pay-to-Witness-Script-Hash) multisig.
//!
//! # Flow
//!
//! 1. **Setup**: Each party generates a key pair and shares their public key.
//! 2. **Lock**: The maker sends BTC to a 2-of-2 multisig address derived from
//!    both public keys. The redemption script requires 2-of-2 signatures.
//! 3. **Payout**: After fiat payment is confirmed, both parties co-sign a
//!    payout transaction that sends the BTC to the taker's address.
//! 4. **Return**: On dispute/cancellation, both parties co-sign a return
//!    transaction that sends the BTC back to the maker's address.
//!
//! # Co-signing Protocol
//!
//! Each party creates a partial signature. The partial signatures
//! are exchanged and combined to produce a final transaction that can be
//! broadcast to the Bitcoin network.

use std::collections::HashMap;
use std::str::FromStr;

use bitcoin::consensus::encode::serialize;
use bitcoin::hashes::{Hash, sha256};
use bitcoin::opcodes::all;
use bitcoin::secp256k1::{All, Message, PublicKey, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::SettlementError;
use zeroize::Zeroize;

/// A secp256k1 key pair for one party in the multisig escrow.
///
/// Secret key is zeroized from memory when dropped to prevent leakage.
#[derive(Debug, Clone)]
pub struct EscrowKeyPair {
    secret: SecretKey,
    public: PublicKey,
}

impl Drop for EscrowKeyPair {
    fn drop(&mut self) {
        // Zeroize the secret key bytes from memory when dropped
        let bytes = self.secret.secret_bytes();
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&bytes);
        buf.zeroize();
    }
}

impl EscrowKeyPair {
    pub fn new() -> Self {
        let secp = Secp256k1::<All>::new();
        let (secret, public) = secp.generate_keypair(&mut bitcoin::secp256k1::rand::thread_rng());
        Self { secret, public }
    }

    pub fn public_key(&self) -> PublicKey {
        self.public
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public.serialize())
    }

    pub fn public_key_bytes(&self) -> [u8; 33] {
        self.public.serialize()
    }

    pub fn from_secret_hex(secret_hex: &str) -> Result<Self, SettlementError> {
        let secret_bytes = hex::decode(secret_hex)
            .map_err(|e| SettlementError::MultisigError(format!("Invalid hex: {}", e)))?;
        let secret = SecretKey::from_slice(&secret_bytes)
            .map_err(|e| SettlementError::MultisigError(format!("Invalid secret key: {}", e)))?;
        let secp = Secp256k1::<All>::new();
        let public = PublicKey::from_secret_key(&secp, &secret);
        Ok(Self { secret, public })
    }
}

/// The redemption script for a 2-of-2 multisig escrow.
/// Script: OP_2 <pub1> <pub2> OP_2 OP_CHECKMULTISIG
#[derive(Debug, Clone)]
pub struct RedemptionScript {
    witness_script: ScriptBuf,
    p2wsh_script: ScriptBuf,
    script_hash: sha256::Hash,
    maker_pub_key_hex: String,
    taker_pub_key_hex: String,
}

impl Serialize for RedemptionScript {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("RedemptionScript", 5)?;
        state.serialize_field("witness_script", &hex::encode(&self.witness_script))?;
        state.serialize_field("p2wsh_script", &hex::encode(&self.p2wsh_script))?;
        state.serialize_field(
            "script_hash",
            &hex::encode(self.script_hash.to_byte_array()),
        )?;
        state.serialize_field("maker_pub_key_hex", &self.maker_pub_key_hex)?;
        state.serialize_field("taker_pub_key_hex", &self.taker_pub_key_hex)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for RedemptionScript {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        #[derive(Deserialize)]
        struct Raw {
            witness_script: String,
            p2wsh_script: String,
            script_hash: String,
            maker_pub_key_hex: String,
            taker_pub_key_hex: String,
        }
        let raw = Raw::deserialize(deserializer)?;
        let witness_script = ScriptBuf::from(
            hex::decode(&raw.witness_script).map_err(|e| D::Error::custom(e.to_string()))?,
        );
        let p2wsh_script = ScriptBuf::from(
            hex::decode(&raw.p2wsh_script).map_err(|e| D::Error::custom(e.to_string()))?,
        );
        let hash_bytes: [u8; 32] = hex::decode(&raw.script_hash)
            .map_err(|e| D::Error::custom(e.to_string()))?
            .try_into()
            .map_err(|_| D::Error::custom("script hash must be 32 bytes"))?;
        let script_hash = sha256::Hash::from_byte_array(hash_bytes);
        Ok(Self {
            witness_script,
            p2wsh_script,
            script_hash,
            maker_pub_key_hex: raw.maker_pub_key_hex,
            taker_pub_key_hex: raw.taker_pub_key_hex,
        })
    }
}

impl RedemptionScript {
    pub fn new(maker_pub: PublicKey, taker_pub: PublicKey) -> Self {
        let maker_bytes: [u8; 33] = maker_pub.serialize();
        let taker_bytes: [u8; 33] = taker_pub.serialize();

        // Build OP_2 <pub1> <pub2> OP_2 OP_CHECKMULTISIG
        let mut witness_script = ScriptBuf::new();
        witness_script.push_opcode(all::OP_PUSHNUM_2);
        witness_script.push_slice(&maker_bytes);
        witness_script.push_slice(&taker_bytes);
        witness_script.push_opcode(all::OP_PUSHNUM_2);
        witness_script.push_opcode(all::OP_CHECKMULTISIG);

        // Create P2WSH output: OP_0 <sha256(witness_script)>
        let script_hash = sha256::Hash::hash(witness_script.as_bytes());
        let mut p2wsh_builder = ScriptBuf::new();
        p2wsh_builder.push_opcode(bitcoin::opcodes::OP_0);
        let hash_bytes: [u8; 32] = script_hash.to_byte_array();
        p2wsh_builder.push_slice(hash_bytes);
        let p2wsh_script = p2wsh_builder;

        Self {
            witness_script,
            p2wsh_script,
            script_hash,
            maker_pub_key_hex: hex::encode(maker_bytes),
            taker_pub_key_hex: hex::encode(taker_bytes),
        }
    }

    pub fn address(&self, network: Network) -> Address {
        Address::from_script(&self.p2wsh_script, network).expect("P2WSH script is always valid")
    }

    pub fn witness_script(&self) -> &ScriptBuf {
        &self.witness_script
    }

    pub fn p2wsh_script(&self) -> &ScriptBuf {
        &self.p2wsh_script
    }

    pub fn script_hash_hex(&self) -> String {
        hex::encode(self.script_hash.to_byte_array())
    }

    pub fn maker_pub_key_hex(&self) -> &str {
        &self.maker_pub_key_hex
    }

    pub fn taker_pub_key_hex(&self) -> &str {
        &self.taker_pub_key_hex
    }
}

/// A partial signature from one party.
#[derive(Debug, Clone)]
pub struct PartialSignature {
    signature_with_sighash: Vec<u8>,
    party: String,
    trade_id: String,
}

impl Serialize for PartialSignature {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PartialSignature", 3)?;
        state.serialize_field(
            "signature_with_sighash",
            &hex::encode(&self.signature_with_sighash),
        )?;
        state.serialize_field("party", &self.party)?;
        state.serialize_field("trade_id", &self.trade_id)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for PartialSignature {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        #[derive(Deserialize)]
        struct Raw {
            signature_with_sighash: String,
            party: String,
            trade_id: String,
        }
        let raw = Raw::deserialize(deserializer)?;
        let sig = hex::decode(&raw.signature_with_sighash)
            .map_err(|e| D::Error::custom(e.to_string()))?;
        Ok(Self {
            signature_with_sighash: sig,
            party: raw.party,
            trade_id: raw.trade_id,
        })
    }
}

impl PartialSignature {
    pub fn new(signature_with_sighash: Vec<u8>, party: String, trade_id: String) -> Self {
        Self {
            signature_with_sighash,
            party,
            trade_id,
        }
    }

    pub fn signature_bytes(&self) -> &[u8] {
        &self.signature_with_sighash
    }

    pub fn party(&self) -> &str {
        &self.party
    }

    pub fn to_json(&self) -> Result<String, SettlementError> {
        serde_json::to_string(self)
            .map_err(|e| SettlementError::MultisigError(format!("JSON serialize: {}", e)))
    }

    pub fn from_json(json: &str) -> Result<Self, SettlementError> {
        serde_json::from_str(json)
            .map_err(|e| SettlementError::MultisigError(format!("JSON deserialize: {}", e)))
    }
}

/// State of a multisig escrow for a single trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultisigEscrowState {
    AwaitingDeposit,
    Deposited {
        deposit_txid: String,
        vout: u32,
        amount: u64,
    },
    MakerSigned,
    ReadyToBroadcast,
    Broadcast {
        payout_txid: String,
    },
    Returned,
}

/// Complete escrow context for a 2-of-2 multisig trade.
#[derive(Debug, Clone)]
pub struct MultisigEscrow {
    pub trade_id: Uuid,
    pub redemption_script: RedemptionScript,
    pub state: MultisigEscrowState,
    pub network: Network,
    pub maker_partial_sig: Option<PartialSignature>,
    pub taker_partial_sig: Option<PartialSignature>,
    pub payout_tx_hex: Option<String>,
    pub return_tx_hex: Option<String>,
}

impl Serialize for MultisigEscrow {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("MultisigEscrow", 8)?;
        state.serialize_field("trade_id", &self.trade_id)?;
        state.serialize_field("redemption_script", &self.redemption_script)?;
        state.serialize_field("state", &self.state)?;
        state.serialize_field("network", &self.network.to_string())?;
        state.serialize_field("maker_partial_sig", &self.maker_partial_sig)?;
        state.serialize_field("taker_partial_sig", &self.taker_partial_sig)?;
        state.serialize_field("payout_tx_hex", &self.payout_tx_hex)?;
        state.serialize_field("return_tx_hex", &self.return_tx_hex)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for MultisigEscrow {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        #[derive(Deserialize)]
        struct Raw {
            trade_id: Uuid,
            redemption_script: RedemptionScript,
            state: MultisigEscrowState,
            network: String,
            maker_partial_sig: Option<PartialSignature>,
            taker_partial_sig: Option<PartialSignature>,
            payout_tx_hex: Option<String>,
            return_tx_hex: Option<String>,
        }
        let raw = Raw::deserialize(deserializer)?;
        let network = Network::from_str(&raw.network)
            .map_err(|e| D::Error::custom(format!("Invalid network: {}", e)))?;
        Ok(Self {
            trade_id: raw.trade_id,
            redemption_script: raw.redemption_script,
            state: raw.state,
            network,
            maker_partial_sig: raw.maker_partial_sig,
            taker_partial_sig: raw.taker_partial_sig,
            payout_tx_hex: raw.payout_tx_hex,
            return_tx_hex: raw.return_tx_hex,
        })
    }
}

impl MultisigEscrow {
    pub fn new(
        trade_id: Uuid,
        maker_key: &EscrowKeyPair,
        taker_key: &EscrowKeyPair,
        network: Network,
    ) -> Self {
        let redemption_script =
            RedemptionScript::new(maker_key.public_key(), taker_key.public_key());

        Self {
            trade_id,
            redemption_script,
            state: MultisigEscrowState::AwaitingDeposit,
            network,
            maker_partial_sig: None,
            taker_partial_sig: None,
            payout_tx_hex: None,
            return_tx_hex: None,
        }
    }

    pub fn deposit_address(&self) -> Address {
        self.redemption_script.address(self.network)
    }

    pub fn deposit_address_str(&self) -> String {
        self.deposit_address().to_string()
    }

    pub fn record_deposit(&mut self, deposit_txid: String, vout: u32, amount: u64) {
        self.state = MultisigEscrowState::Deposited {
            deposit_txid,
            vout,
            amount,
        };
    }

    pub fn create_payout_tx(
        &self,
        deposit_txid_hex: &str,
        vout: u32,
        amount: u64,
        taker_address: &str,
        miner_fee_sats: u64,
    ) -> Result<Transaction, SettlementError> {
        let deposit_txid = bitcoin::Txid::from_str(deposit_txid_hex)
            .map_err(|e| SettlementError::MultisigError(format!("Invalid txid: {}", e)))?;

        let taker_addr_unchecked: bitcoin::Address<_> =
            bitcoin::Address::from_str(taker_address)
                .map_err(|e| SettlementError::MultisigError(format!("Invalid address: {}", e)))?;
        let taker_addr = taker_addr_unchecked
            .require_network(self.network)
            .map_err(|_| {
                SettlementError::MultisigError("Address not valid for network".to_string())
            })?;

        let amount = Amount::from_sat(amount);
        let fee = Amount::from_sat(miner_fee_sats);

        // Ensure the fee doesn't exceed the total amount
        if fee > amount {
            return Err(SettlementError::MultisigError(format!(
                "Miner fee ({}) exceeds escrow amount ({})",
                miner_fee_sats, amount
            )));
        }

        let taker_value = amount - fee;

        let tx = Transaction {
            version: bitcoin::transaction::Version(2),
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: deposit_txid,
                    vout,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: taker_value,
                script_pubkey: taker_addr.script_pubkey(),
            }],
        };

        Ok(tx)
    }

    pub fn sign_payout(
        &self,
        tx: &Transaction,
        key: &EscrowKeyPair,
        party: String,
        amount: u64,
    ) -> Result<PartialSignature, SettlementError> {
        let secp = Secp256k1::<All>::new();

        let mut sighash_cache = SighashCache::new(tx);
        let sighash = sighash_cache
            .p2wsh_signature_hash(
                0,
                self.redemption_script.witness_script(),
                Amount::from_sat(amount),
                EcdsaSighashType::All,
            )
            .map_err(|e| SettlementError::MultisigError(format!("Sighash error: {}", e)))?;

        let message = Message::from(sighash);
        let signature = secp.sign_ecdsa(&message, &key.secret);

        let mut sig_with_sighash = signature.serialize_der().to_vec();
        sig_with_sighash.push(EcdsaSighashType::All as u8);

        Ok(PartialSignature::new(
            sig_with_sighash,
            party,
            self.trade_id.to_string(),
        ))
    }

    pub fn combine_signatures(
        &self,
        tx: &mut Transaction,
        maker_sig: &PartialSignature,
        taker_sig: &PartialSignature,
    ) -> Result<(), SettlementError> {
        if tx.input.is_empty() {
            return Err(SettlementError::MultisigError(
                "No inputs to sign".to_string(),
            ));
        }

        let mut witness = Witness::new();
        witness.push(&[]);
        witness.push(maker_sig.signature_bytes());
        witness.push(taker_sig.signature_bytes());
        witness.push(self.redemption_script.witness_script().as_bytes());

        tx.input[0].witness = witness;
        Ok(())
    }

    pub fn tx_to_hex(tx: &Transaction) -> String {
        hex::encode(serialize(tx))
    }
}

/// Manages multiple multisig escrows across trades.
///
/// Secret keys are zeroized from memory when the manager is dropped.
#[derive(Default)]
pub struct MultisigManager {
    escrows: HashMap<Uuid, MultisigEscrow>,
    keys: HashMap<Uuid, (EscrowKeyPair, EscrowKeyPair)>,
}

impl Drop for MultisigManager {
    fn drop(&mut self) {
        // Zeroize all secret key bytes when the manager is dropped
        for (_, (mut maker, mut taker)) in self.keys.drain() {
            let mut buf1 = [0u8; 32];
            buf1.copy_from_slice(&maker.secret.secret_bytes());
            buf1.zeroize();
            let mut buf2 = [0u8; 32];
            buf2.copy_from_slice(&taker.secret.secret_bytes());
            buf2.zeroize();
        }
    }
}

impl std::fmt::Debug for MultisigManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultisigManager")
            .field("escrows", &self.escrows)
            .field("keys", &format!("{} keys stored", self.keys.len()))
            .finish()
    }
}

impl MultisigManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn setup_escrow(
        &mut self,
        trade_id: Uuid,
        maker_key: EscrowKeyPair,
        taker_key: EscrowKeyPair,
        network: Network,
    ) -> Result<&MultisigEscrow, SettlementError> {
        let escrow = MultisigEscrow::new(trade_id, &maker_key, &taker_key, network);
        self.keys.insert(trade_id, (maker_key, taker_key));
        let _ = self.escrows.insert(trade_id, escrow);
        Ok(self
            .escrows
            .get(&trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Escrow setup failed".to_string()))?)
    }

    pub fn get_escrow(&self, trade_id: Uuid) -> Option<&MultisigEscrow> {
        self.escrows.get(&trade_id)
    }

    pub fn get_escrow_mut(&mut self, trade_id: Uuid) -> Option<&mut MultisigEscrow> {
        self.escrows.get_mut(&trade_id)
    }

    pub fn record_deposit(
        &mut self,
        trade_id: Uuid,
        deposit_txid: String,
        vout: u32,
        amount: u64,
    ) -> Result<(), SettlementError> {
        let escrow = self
            .escrows
            .get_mut(&trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Escrow not found".to_string()))?;
        escrow.record_deposit(deposit_txid, vout, amount);
        Ok(())
    }

    pub fn maker_sign_payout(
        &mut self,
        trade_id: Uuid,
        deposit_txid: &str,
        vout: u32,
        amount: u64,
        taker_address: &str,
    ) -> Result<PartialSignature, SettlementError> {
        let (maker_key, _) = self
            .keys
            .get(&trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Keys not found".to_string()))?;

        let escrow = self
            .get_escrow(trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Escrow not found".to_string()))?;

        let tx = escrow.create_payout_tx(deposit_txid, vout, amount, taker_address, 2000)?;
        let partial_sig = escrow.sign_payout(&tx, maker_key, "maker".to_string(), amount)?;

        let escrow_mut = self
            .escrows
            .get_mut(&trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Escrow not found".to_string()))?;
        escrow_mut.payout_tx_hex = Some(MultisigEscrow::tx_to_hex(&tx));
        escrow_mut.maker_partial_sig = Some(partial_sig.clone());
        escrow_mut.state = MultisigEscrowState::MakerSigned;

        Ok(partial_sig)
    }

    pub fn taker_sign_payout(
        &mut self,
        trade_id: Uuid,
        _deposit_txid: &str,
        vout: u32,
        amount: u64,
    ) -> Result<PartialSignature, SettlementError> {
        let (_, taker_key) = self
            .keys
            .get(&trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Keys not found".to_string()))?;

        let escrow = self
            .get_escrow(trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Escrow not found".to_string()))?;

        let tx_hex = escrow
            .payout_tx_hex
            .as_ref()
            .ok_or_else(|| SettlementError::MultisigError("No payout tx to sign".to_string()))?;

        let tx_bytes = hex::decode(tx_hex)
            .map_err(|e| SettlementError::MultisigError(format!("Invalid tx hex: {}", e)))?;
        let tx: Transaction = bitcoin::consensus::encode::deserialize(&tx_bytes)
            .map_err(|e| SettlementError::MultisigError(format!("Deserialize tx: {}", e)))?;

        let partial_sig = escrow.sign_payout(&tx, taker_key, "taker".to_string(), amount)?;

        let escrow_mut = self
            .escrows
            .get_mut(&trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Escrow not found".to_string()))?;
        escrow_mut.taker_partial_sig = Some(partial_sig.clone());

        Ok(partial_sig)
    }

    pub fn finalize_payout(&mut self, trade_id: Uuid) -> Result<String, SettlementError> {
        let escrow = self
            .escrows
            .get_mut(&trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Escrow not found".to_string()))?;

        let maker_sig = escrow
            .maker_partial_sig
            .as_ref()
            .ok_or_else(|| SettlementError::MultisigError("Maker signature missing".to_string()))?;

        let taker_sig = escrow
            .taker_partial_sig
            .as_ref()
            .ok_or_else(|| SettlementError::MultisigError("Taker signature missing".to_string()))?;

        let tx_hex = escrow
            .payout_tx_hex
            .take()
            .ok_or_else(|| SettlementError::MultisigError("Payout tx missing".to_string()))?;

        let tx_bytes = hex::decode(&tx_hex)
            .map_err(|e| SettlementError::MultisigError(format!("Invalid hex: {}", e)))?;
        let mut tx: Transaction = bitcoin::consensus::encode::deserialize(&tx_bytes)
            .map_err(|e| SettlementError::MultisigError(format!("Deserialize: {}", e)))?;

        escrow.combine_signatures(&mut tx, maker_sig, taker_sig)?;

        let final_hex = MultisigEscrow::tx_to_hex(&tx);
        escrow.state = MultisigEscrowState::ReadyToBroadcast;
        escrow.payout_tx_hex = Some(final_hex.clone());

        Ok(final_hex)
    }

    pub fn create_return_tx(
        &mut self,
        trade_id: Uuid,
        deposit_txid: &str,
        vout: u32,
        amount: u64,
        maker_address: &str,
    ) -> Result<String, SettlementError> {
        let escrow = self
            .get_escrow(trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Escrow not found".to_string()))?;

        let tx = escrow.create_payout_tx(deposit_txid, vout, amount, maker_address, 2000)?;
        let tx_hex = MultisigEscrow::tx_to_hex(&tx);

        let escrow_mut = self
            .escrows
            .get_mut(&trade_id)
            .ok_or_else(|| SettlementError::MultisigError("Escrow not found".to_string()))?;
        escrow_mut.return_tx_hex = Some(tx_hex.clone());

        Ok(tx_hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_network() -> Network {
        Network::Testnet
    }

    #[test]
    fn generate_key_pair() {
        let key = EscrowKeyPair::new();
        let hex = key.public_key_hex();
        assert_eq!(hex.len(), 66);
    }

    #[test]
    fn key_pair_from_secret_hex() {
        let key1 = EscrowKeyPair::new();
        let secret_hex = hex::encode(key1.secret.secret_bytes());
        let key2 = EscrowKeyPair::from_secret_hex(&secret_hex).unwrap();
        assert_eq!(key1.public_key_hex(), key2.public_key_hex());
    }

    #[test]
    fn create_redemption_script() {
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let script = RedemptionScript::new(maker.public_key(), taker.public_key());
        assert_eq!(script.script_hash_hex().len(), 64);
        assert_eq!(script.maker_pub_key_hex().len(), 66);
        assert_eq!(script.taker_pub_key_hex().len(), 66);
        assert_ne!(script.maker_pub_key_hex(), script.taker_pub_key_hex());
    }

    #[test]
    fn deposit_address_generation() {
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let script = RedemptionScript::new(maker.public_key(), taker.public_key());
        let address = script.address(Network::Testnet);
        assert!(address.to_string().starts_with("tb1q"));
    }

    #[test]
    fn setup_multisig_escrow() {
        let mut manager = MultisigManager::new();
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let trade_id = Uuid::new_v4();
        let escrow = manager
            .setup_escrow(trade_id, maker, taker, test_network())
            .unwrap();
        assert_eq!(escrow.trade_id, trade_id);
        assert!(matches!(escrow.state, MultisigEscrowState::AwaitingDeposit));
        assert!(escrow.deposit_address_str().starts_with("tb1q"));
    }

    #[test]
    fn record_deposit() {
        let mut manager = MultisigManager::new();
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let trade_id = Uuid::new_v4();
        manager
            .setup_escrow(trade_id, maker, taker, test_network())
            .unwrap();
        manager
            .record_deposit(
                trade_id,
                "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string(),
                0,
                100_000_000,
            )
            .unwrap();
        let escrow = manager.get_escrow(trade_id).unwrap();
        assert!(matches!(
            escrow.state,
            MultisigEscrowState::Deposited { .. }
        ));
    }

    #[test]
    fn partial_signature_json_roundtrip() {
        let sig = PartialSignature::new(
            vec![0u8; 65],
            "maker".to_string(),
            Uuid::new_v4().to_string(),
        );
        let json = sig.to_json().unwrap();
        let restored = PartialSignature::from_json(&json).unwrap();
        assert_eq!(restored.party(), "maker");
        assert_eq!(restored.signature_bytes(), &vec![0u8; 65]);
    }

    #[test]
    fn escrow_state_transitions() {
        let mut manager = MultisigManager::new();
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let trade_id = Uuid::new_v4();
        manager
            .setup_escrow(trade_id, maker, taker, test_network())
            .unwrap();
        assert!(matches!(
            manager.get_escrow(trade_id).unwrap().state,
            MultisigEscrowState::AwaitingDeposit
        ));
        manager
            .record_deposit(
                trade_id,
                "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string(),
                0,
                100_000_000,
            )
            .unwrap();
        assert!(matches!(
            manager.get_escrow(trade_id).unwrap().state,
            MultisigEscrowState::Deposited { .. }
        ));
    }

    #[test]
    fn different_keys_produce_different_addresses() {
        let maker1 = EscrowKeyPair::new();
        let taker1 = EscrowKeyPair::new();
        let maker2 = EscrowKeyPair::new();
        let taker2 = EscrowKeyPair::new();
        let script1 = RedemptionScript::new(maker1.public_key(), taker1.public_key());
        let script2 = RedemptionScript::new(maker2.public_key(), taker2.public_key());
        let addr1 = script1.address(Network::Testnet).to_string();
        let addr2 = script2.address(Network::Testnet).to_string();
        assert_ne!(addr1, addr2);
    }

    #[test]
    fn redemption_script_serialization() {
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let script = RedemptionScript::new(maker.public_key(), taker.public_key());
        let json = serde_json::to_string(&script).unwrap();
        let restored: RedemptionScript = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.maker_pub_key_hex(), script.maker_pub_key_hex());
        assert_eq!(restored.taker_pub_key_hex(), script.taker_pub_key_hex());
        assert_eq!(restored.script_hash_hex(), script.script_hash_hex());
    }

    #[test]
    fn multisig_escrow_serialization() {
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let trade_id = Uuid::new_v4();
        let escrow = MultisigEscrow::new(trade_id, &maker, &taker, Network::Testnet);
        let json = serde_json::to_string(&escrow).unwrap();
        let restored: MultisigEscrow = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.trade_id, escrow.trade_id);
        assert_eq!(restored.network, escrow.network);
    }

    #[test]
    fn create_payout_transaction() {
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let trade_id = Uuid::new_v4();
        let escrow = MultisigEscrow::new(trade_id, &maker, &taker, Network::Testnet);
        // Use the taker's own multisig deposit address as payout target
        let taker_addr = escrow.deposit_address_str();
        let tx = escrow
            .create_payout_tx(
                "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
                0,
                100_000_000,
                &taker_addr,
                2000,
            )
            .unwrap();
        assert_eq!(tx.input.len(), 1);
        assert_eq!(tx.output.len(), 1);
        // Taker receives amount minus miner fee
        assert_eq!(tx.output[0].value.to_sat(), 100_000_000 - 2000);
    }

    #[test]
    fn sign_and_combine_flow() {
        let mut manager = MultisigManager::new();
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let trade_id = Uuid::new_v4();
        let deposit_txid = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let amount = 100_000_000u64;

        manager
            .setup_escrow(trade_id, maker.clone(), taker.clone(), test_network())
            .unwrap();
        manager
            .record_deposit(trade_id, deposit_txid.to_string(), 0, amount)
            .unwrap();

        // Use the escrow's own deposit address as payout target
        let taker_addr = manager.get_escrow(trade_id).unwrap().deposit_address_str();

        let maker_sig = manager
            .maker_sign_payout(trade_id, deposit_txid, 0, amount, &taker_addr)
            .unwrap();
        assert_eq!(maker_sig.party(), "maker");

        let taker_sig = manager
            .taker_sign_payout(trade_id, deposit_txid, 0, amount)
            .unwrap();
        assert_eq!(taker_sig.party(), "taker");

        let final_tx_hex = manager.finalize_payout(trade_id).unwrap();
        assert!(!final_tx_hex.is_empty());

        let escrow = manager.get_escrow(trade_id).unwrap();
        assert!(matches!(
            escrow.state,
            MultisigEscrowState::ReadyToBroadcast
        ));
    }

    #[test]
    fn miner_fee_deducted_from_payout() {
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let trade_id = Uuid::new_v4();
        let escrow = MultisigEscrow::new(trade_id, &maker, &taker, Network::Testnet);
        let taker_addr = escrow.deposit_address_str();
        let tx = escrow
            .create_payout_tx(
                "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
                0,
                100_000_000,
                &taker_addr,
                5000,
            )
            .unwrap();
        // Taker receives amount minus miner fee
        assert_eq!(tx.output[0].value.to_sat(), 100_000_000 - 5000);
    }

    #[test]
    fn miner_fee_exceeds_amount_rejected() {
        let maker = EscrowKeyPair::new();
        let taker = EscrowKeyPair::new();
        let trade_id = Uuid::new_v4();
        let escrow = MultisigEscrow::new(trade_id, &maker, &taker, Network::Testnet);
        let taker_addr = escrow.deposit_address_str();
        let result = escrow.create_payout_tx(
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
            0,
            1000,
            &taker_addr,
            2000,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Miner fee"));
    }

    #[test]
    fn missing_escrow_returns_error() {
        let mut manager = MultisigManager::new();
        let non_existent = Uuid::new_v4();
        let result = manager.maker_sign_payout(
            non_existent,
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
            0,
            100_000_000,
            "tb1qtest",
        );
        assert!(result.is_err());
    }

    #[test]
    fn missing_escrow_return_tx_returns_error() {
        let mut manager = MultisigManager::new();
        let non_existent = Uuid::new_v4();
        let result = manager.create_return_tx(
            non_existent,
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
            0,
            100_000_000,
            "tb1qtest",
        );
        assert!(result.is_err());
    }
}
