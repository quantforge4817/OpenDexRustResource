//! Settlement engine for fiat-to-crypto trades.
//!
//! Orchestrates the complete fiat trade lifecycle:
//! - Creates trades from offers
//! - Manages payment adapter interactions
//! - Coordinates crypto escrow and release
//! - Handles bonds and disputes
//! - Monitors deadlines and timeouts

use std::collections::HashMap;
use uuid::Uuid;

use opendex_types::{Asset, ParticipantId, TradeId};

use crate::adapter::PaymentAdapter;
use crate::bond::SecurityBond;
use crate::dispute::{Dispute, DisputeReason};
use crate::error::{FiatError, SettlementError};
use crate::payment::PaymentId;
use crate::trade::{FeeSchedule, FiatOffer, FiatTrade, FiatTradeState, SettlementDeadlines};
use crate::verification::VerifiedPayment;

/// Sliding window rate limiter for settlement operations.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    max_requests: usize,
    window_seconds: u64,
    requests: HashMap<String, Vec<u64>>,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
            requests: HashMap::new(),
        }
    }

    pub fn is_allowed(&mut self, key: &str, current_time: u64) -> bool {
        let cutoff = if current_time >= self.window_seconds {
            current_time - self.window_seconds
        } else {
            0
        };
        let timestamps = self
            .requests
            .entry(key.to_string())
            .or_insert_with(Vec::new);
        timestamps.retain(|&t| t > cutoff);
        if timestamps.len() >= self.max_requests {
            false
        } else {
            timestamps.push(current_time);
            true
        }
    }
}

/// Crypto wallet operations for escrow and release.
pub trait CryptoWallet: Send + Sync {
    /// Reserve crypto funds in escrow for a trade.
    fn reserve_escrow(
        &self,
        trade_id: TradeId,
        asset: &Asset,
        amount: u64,
    ) -> Result<String, SettlementError>;

    /// Release crypto from escrow to the taker's address.
    fn release_escrow(
        &self,
        trade_id: TradeId,
        recipient_address: &str,
    ) -> Result<String, SettlementError>;

    /// Return crypto from escrow back to the maker.
    fn return_escrow(
        &self,
        trade_id: TradeId,
    ) -> Result<String, SettlementError>;
}

/// Events emitted by the settlement engine.
#[derive(Debug, Clone)]
pub enum SettlementEvent {
    /// Trade state changed.
    TradeStateChanged {
        trade_id: TradeId,
        state: FiatTradeState,
    },
    /// Payment instructions sent to taker (maker's bank details provided).
    PaymentInstructionsSent {
        trade_id: TradeId,
        payment_reference: crate::payment::PaymentReference,
    },
    /// Taker confirmed payment (clicked "I've paid").
    TakerPaymentConfirmed {
        trade_id: TradeId,
    },
    /// Maker confirmed receiving payment (clicked "I confirm payment").
    MakerPaymentConfirmed {
        trade_id: TradeId,
    },
    /// Payment detected.
    PaymentDetected {
        trade_id: TradeId,
        payment_id: PaymentId,
    },
    /// Payment confirmed.
    PaymentConfirmed {
        trade_id: TradeId,
        payment_id: PaymentId,
    },
    /// Crypto released to taker.
    CryptoReleased {
        trade_id: TradeId,
        tx_hash: String,
    },
    /// Trade completed.
    TradeCompleted {
        trade_id: TradeId,
    },
    /// Dispute opened.
    DisputeOpened {
        trade_id: TradeId,
        dispute_id: uuid::Uuid,
    },
    /// Error occurred.
    Error {
        trade_id: TradeId,
        error: SettlementError,
    },
}

/// The settlement engine manages fiat trades from creation to completion.
pub struct SettlementEngine<PA, CW>
where
    PA: PaymentAdapter,
    CW: CryptoWallet,
{
    /// Active trades indexed by ID.
    trades: HashMap<Uuid, FiatTrade>,
    /// Security bonds indexed by trade ID.
    bonds: HashMap<Uuid, (SecurityBond, SecurityBond)>,
    /// Active disputes indexed by trade ID.
    disputes: HashMap<Uuid, Dispute>,
    /// Payment adapter for fiat payments.
    payment_adapter: PA,
    /// Crypto wallet for escrow operations.
    crypto_wallet: CW,
    /// Events emitted by the engine.
    events: Vec<SettlementEvent>,
    /// Rate limiter for trade creation (10 per hour per user).
    trade_creation_limiter: RateLimiter,
    /// Rate limiter for payment confirmation (5 per minute per user).
    payment_confirm_limiter: RateLimiter,
}

impl<PA, CW> SettlementEngine<PA, CW>
where
    PA: PaymentAdapter,
    CW: CryptoWallet,
{
    /// Create a new settlement engine.
    pub fn new(payment_adapter: PA, crypto_wallet: CW) -> Self {
        Self {
            trades: HashMap::new(),
            bonds: HashMap::new(),
            disputes: HashMap::new(),
            payment_adapter,
            crypto_wallet,
            events: Vec::new(),
            trade_creation_limiter: RateLimiter::new(10, 3600), // 10 per hour
            payment_confirm_limiter: RateLimiter::new(5, 60), // 5 per minute
        }
    }

    /// Get all events emitted since the last call.
    pub fn take_events(&mut self) -> Vec<SettlementEvent> {
        std::mem::take(&mut self.events)
    }

    /// M24: Validate a crypto address for the given asset.
    fn validate_crypto_address(asset: &Asset, address: &str) -> Result<(), SettlementError> {
        if address.is_empty() {
            return Err(SettlementError::VerificationFailed(
                "Crypto address is empty".to_string(),
            ));
        }
        match asset {
            Asset::Btc => {
                // Validate Bitcoin address format
                use std::str::FromStr;
                // Try parsing as a Bitcoin address (mainnet or testnet)
                if let Ok(addr) = bitcoin::Address::from_str(address) {
                    if addr.is_valid_for_network(bitcoin::Network::Bitcoin)
                        || addr.is_valid_for_network(bitcoin::Network::Testnet)
                    {
                        return Ok(());
                    }
                }
                // Fallback: accept common address prefixes for flexibility
                if address.starts_with('1')
                    || address.starts_with('3')
                    || address.starts_with("bc1")
                    || address.starts_with("tb1")
                    || address.starts_with('m')
                    || address.starts_with('n')
                {
                    return Ok(());
                }
                Err(SettlementError::VerificationFailed(
                    format!("Invalid Bitcoin address: {}", address),
                ))
            }
            Asset::Xmr => {
                // Validate Monero address: mainnet starts with '4', testnet with '8'
                // Length is 95 characters
                if address.len() != 95 {
                    return Err(SettlementError::VerificationFailed(
                        format!("Invalid Monero address length: {}", address.len()),
                    ));
                }
                if !address.starts_with('4') && !address.starts_with('8') {
                    return Err(SettlementError::VerificationFailed(
                        format!("Invalid Monero address prefix: {}", address.chars().next().unwrap_or(' ')),
                    ));
                }
                Ok(())
            }
            Asset::Fiat(_) => {
                // Fiat assets don't have crypto addresses
                Ok(())
            }
        }
    }

    /// Create a trade from a fiat offer.
    pub fn create_trade(
        &mut self,
        offer: &FiatOffer,
        taker: ParticipantId,
        taker_crypto_address: String,
        timestamp: u64,
    ) -> Result<TradeId, SettlementError> {
        // Rate limit: 10 trades per hour per taker
        if !self.trade_creation_limiter.is_allowed(&taker.0.to_string(), timestamp) {
            return Err(SettlementError::RateLimited);
        }

        // M24: Validate crypto address before storing
        Self::validate_crypto_address(&offer.crypto_asset, &taker_crypto_address)?;

        let trade_id = TradeId(Uuid::new_v4());

        // No KYC policy — skip identity verification
        let maker = offer.maker;

        // AUDIT-C06: Calculate crypto amount from price and fiat amount
        let crypto_amount = offer.calculate_crypto_amount();

        let payment_deadline = timestamp + offer.settlement_window;
        let confirmation_deadline = payment_deadline + 3600; // +1 hour
        let dispute_deadline = confirmation_deadline + 86400; // +24 hours

        let trade = FiatTrade {
            id: trade_id,
            maker,
            taker,
            crypto_asset: offer.crypto_asset.clone(),
            fiat_currency: offer.amount.currency.clone(),
            crypto_amount,
            fiat_amount: offer.amount.clone(),
            settlement_method: offer.settlement_method.clone(),
            fee_schedule: FeeSchedule::bank_transfer_default(),
            state: FiatTradeState::Created,
            maker_bank_details: None,
            taker_crypto_address,
            payment_id: None,
            payment_reference: None,
            deadlines: SettlementDeadlines {
                payment_deadline,
                confirmation_deadline,
                dispute_deadline,
            },
            created_at: timestamp,
            updated_at: timestamp,
        };

        // Create security bonds for both parties
        let maker_bond = SecurityBond::new(
            trade_id,
            trade.maker,
            trade.crypto_asset.clone(),
            Self::calculate_bond_amount(trade.crypto_amount, offer.bond_requirement),
            timestamp,
        );
        let taker_bond = SecurityBond::new(
            trade_id,
            taker,
            trade.crypto_asset.clone(),
            Self::calculate_bond_amount(trade.crypto_amount, offer.bond_requirement),
            timestamp,
        );

        self.trades.insert(trade_id.0, trade);
        self.bonds.insert(trade_id.0, (maker_bond, taker_bond));

        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::Created,
        });

        Ok(trade_id)
    }

    /// Accept a trade (both parties agree).
    pub fn accept_trade(
        &mut self,
        trade_id: TradeId,
        timestamp: u64,
    ) -> Result<(), SettlementError> {
        let trade = self.get_trade_mut(trade_id)?;
        trade.transition(FiatTradeState::Accepted, timestamp)?;

        // Lock bonds
        if let Some((maker_bond, taker_bond)) = self.bonds.get_mut(&trade_id.0) {
            maker_bond.lock(timestamp).map_err(|e| {
                SettlementError::Fiat(FiatError::BondError(e.to_string()))
            })?;
            taker_bond.lock(timestamp).map_err(|e| {
                SettlementError::Fiat(FiatError::BondError(e.to_string()))
            })?;
        }

        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::Accepted,
        });

        Ok(())
    }

    /// Reserve crypto in escrow.
    pub fn reserve_crypto(
        &mut self,
        trade_id: TradeId,
        timestamp: u64,
    ) -> Result<String, SettlementError> {
        let (crypto_asset, crypto_amount) = {
            let trade = self.get_trade_mut(trade_id)?;
            trade.transition(FiatTradeState::CryptoReserved, timestamp)?;
            (trade.crypto_asset.clone(), trade.crypto_amount)
        };

        let escrow_tx = self.crypto_wallet.reserve_escrow(
            trade_id,
            &crypto_asset,
            crypto_amount,
        )?;

        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::CryptoReserved,
        });

        Ok(escrow_tx)
    }

    /// Request payment from the taker by providing maker's bank details.
    ///
    /// This is the first step of the manual flow: maker sends their bank details
    /// to the taker, who will then make the payment externally.
    pub fn request_payment(
        &mut self,
        trade_id: TradeId,
        maker_bank_details: crate::payment::PaymentRecipient,
        timestamp: u64,
    ) -> Result<crate::payment::PaymentInstructions, SettlementError> {
        let (fiat_amount, taker, maker) = {
            let trade = self.get_trade_mut(trade_id)?;
            trade.transition(FiatTradeState::PaymentRequested, timestamp)?;
            // Store maker's bank details with the trade
            trade.maker_bank_details = Some(maker_bank_details.clone());
            (trade.fiat_amount.clone(), trade.taker, trade.maker)
        };

        let reference = crate::payment::PaymentReference::generate();
        let payment_id = crate::payment::PaymentId(Uuid::new_v4());

        {
            let trade = self.get_trade_mut(trade_id)?;
            trade.payment_id = Some(payment_id);
            trade.payment_reference = Some(reference.clone());
        }

        let instructions = crate::payment::PaymentInstructions {
            trade_id,
            payment_id,
            currency: fiat_amount.currency.clone(),
            amount: fiat_amount,
            reference: reference.clone(),
            recipient: maker_bank_details,
            deadline: {
                let trade = self.get_trade(trade_id)?;
                trade.deadlines.payment_deadline
            },
        };

        self.emit(SettlementEvent::PaymentInstructionsSent {
            trade_id,
            payment_reference: reference,
        });
        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::PaymentRequested,
        });

        Ok(instructions)
    }

    /// Taker confirms they have made the payment (manual flow).
    ///
    /// Taker clicks "I've paid" button after making the bank transfer externally.
    /// Trade moves to PaymentSubmitted, waiting for maker to confirm receipt.
    /// AUDIT-C02: Verifies caller is the taker.
    pub fn taker_confirm_payment(
        &mut self,
        trade_id: TradeId,
        caller: ParticipantId,
        timestamp: u64,
    ) -> Result<(), SettlementError> {
        // Rate limit: 5 payment confirmations per minute per user
        if !self.payment_confirm_limiter.is_allowed(&caller.0.to_string(), timestamp) {
            return Err(SettlementError::RateLimited);
        }

        {
            let trade = self.get_trade_mut(trade_id)?;
            if trade.taker != caller {
                return Err(SettlementError::Unauthorized);
            }
            trade.transition(FiatTradeState::PaymentSubmitted, timestamp)?;
        }

        self.emit(SettlementEvent::TakerPaymentConfirmed { trade_id });
        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::PaymentSubmitted,
        });

        Ok(())
    }

    /// Maker confirms they have received the payment (manual flow).
    ///
    /// Maker clicks "I confirm payment" button after checking their bank account.
    /// Trade moves to MakerConfirmed, ready for crypto release.
    /// AUDIT-C02: Verifies caller is the maker.
    pub fn maker_confirm_payment(
        &mut self,
        trade_id: TradeId,
        caller: ParticipantId,
        timestamp: u64,
    ) -> Result<(), SettlementError> {
        {
            let trade = self.get_trade_mut(trade_id)?;
            if trade.maker != caller {
                return Err(SettlementError::Unauthorized);
            }
            trade.transition(FiatTradeState::MakerConfirmed, timestamp)?;
        }

        self.emit(SettlementEvent::MakerPaymentConfirmed { trade_id });
        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::MakerConfirmed,
        });

        Ok(())
    }

    /// Mark payment as pending (taker has sent payment).
    pub fn mark_payment_pending(
        &mut self,
        trade_id: TradeId,
        timestamp: u64,
    ) -> Result<(), SettlementError> {
        {
            let trade = self.get_trade_mut(trade_id)?;
            trade.transition(FiatTradeState::PaymentPending, timestamp)?;
        }

        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::PaymentPending,
        });

        Ok(())
    }

    /// Check for payment detection via the adapter.
    pub fn check_payment(
        &mut self,
        trade_id: TradeId,
    ) -> Result<Option<VerifiedPayment>, SettlementError> {
        let trade = self.get_trade(trade_id)?;

        if let Some(ref payment_id) = trade.payment_id {
            // Check the payment adapter for status updates
            if let Some(status) = self.payment_adapter.check_payment_status(&payment_id.0)
                .map_err(|e| SettlementError::AdapterError(e.to_string()))?
            {
                if status == crate::payment::PaymentStatus::Confirmed {
                    return Ok(Some(VerifiedPayment {
                        payment_id: *payment_id,
                        currency: trade.fiat_amount.currency.clone(),
                        amount: trade.fiat_amount.clone(),
                        sender_reference: crate::payment::PaymentReference(String::new()),
                        recipient_reference: crate::payment::PaymentReference(String::new()),
                        verified_at: chrono::Utc::now(),
                        status: crate::payment::PaymentStatus::Confirmed,
                    }));
                }
            }
        }
        Ok(None)
    }

    /// Confirm a detected payment.
    pub fn confirm_payment(
        &mut self,
        trade_id: TradeId,
        verified: VerifiedPayment,
        timestamp: u64,
    ) -> Result<(), SettlementError> {
        let fiat_amount = {
            let trade = self.get_trade_mut(trade_id)?;
            trade.transition(FiatTradeState::Confirming, timestamp)?;
            trade.fiat_amount.clone()
        };
        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::Confirming,
        });

        // Verify the payment amount matches
        if verified.amount != fiat_amount {
            return Err(SettlementError::Fiat(FiatError::AmountMismatch {
                expected: fiat_amount.minor_units,
                actual: verified.amount.minor_units,
            }));
        }

        // Confirm the payment
        {
            let trade = self.get_trade_mut(trade_id)?;
            trade.transition(FiatTradeState::Confirmed, timestamp)?;
        }
        self.emit(SettlementEvent::PaymentConfirmed {
            trade_id,
            payment_id: verified.payment_id,
        });
        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::Confirmed,
        });

        Ok(())
    }

    /// Release crypto to the taker.
    /// AUDIT-C02: Verifies caller is the maker.
    pub fn release_crypto(
        &mut self,
        trade_id: TradeId,
        caller: ParticipantId,
        timestamp: u64,
    ) -> Result<String, SettlementError> {
        let taker_addr = {
            let trade = self.get_trade_mut(trade_id)?;
            if trade.maker != caller {
                return Err(SettlementError::Unauthorized);
            }
            trade.transition(FiatTradeState::CryptoRelease, timestamp)?;
            trade.taker_crypto_address.clone()
        };

        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::CryptoRelease,
        });

        // Release crypto from escrow
        let tx_hash = self.crypto_wallet.release_escrow(
            trade_id,
            &taker_addr,
        )?;

        self.emit(SettlementEvent::CryptoReleased {
            trade_id,
            tx_hash: tx_hash.clone(),
        });

        Ok(tx_hash)
    }

    /// Complete a trade.
    /// AUDIT-C02: Verifies caller is either maker or taker.
    pub fn complete_trade(
        &mut self,
        trade_id: TradeId,
        caller: ParticipantId,
        timestamp: u64,
    ) -> Result<(), SettlementError> {
        {
            let trade = self.get_trade_mut(trade_id)?;
            if trade.maker != caller && trade.taker != caller {
                return Err(SettlementError::Unauthorized);
            }
            trade.transition(FiatTradeState::Complete, timestamp)?;
        }

        // Release bonds
        if let Some((maker_bond, taker_bond)) = self.bonds.get_mut(&trade_id.0) {
            maker_bond.release(timestamp).map_err(|e| {
                SettlementError::Fiat(FiatError::BondError(e.to_string()))
            })?;
            taker_bond.release(timestamp).map_err(|e| {
                SettlementError::Fiat(FiatError::BondError(e.to_string()))
            })?;
        }

        self.emit(SettlementEvent::TradeCompleted { trade_id });
        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::Complete,
        });

        Ok(())
    }

    /// Open a dispute for a trade.
    pub fn open_dispute(
        &mut self,
        trade_id: TradeId,
        opened_by: ParticipantId,
        reason: DisputeReason,
        timestamp: u64,
    ) -> Result<uuid::Uuid, SettlementError> {
        {
            let trade = self.get_trade_mut(trade_id)?;

            // Check if already disputed
            if trade.state == FiatTradeState::Disputed {
                return Err(SettlementError::Fiat(FiatError::DisputeAlreadyOpen));
            }

            trade.transition(FiatTradeState::Disputed, timestamp)?;
        }

        let dispute = Dispute::new(trade_id, opened_by, reason, timestamp);
        let dispute_id = dispute.id.0;

        self.disputes.insert(trade_id.0, dispute);

        self.emit(SettlementEvent::DisputeOpened {
            trade_id,
            dispute_id,
        });
        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::Disputed,
        });

        Ok(dispute_id)
    }

    /// Cancel a trade.
    /// AUDIT-C02: Verifies caller is either maker or taker.
    pub fn cancel_trade(
        &mut self,
        trade_id: TradeId,
        caller: ParticipantId,
        timestamp: u64,
    ) -> Result<(), SettlementError> {
        // Check whether crypto was reserved so we know if we need to return escrow
        let crypto_reserved = {
            let trade = self.get_trade_mut(trade_id)?;
            if trade.maker != caller && trade.taker != caller {
                return Err(SettlementError::Unauthorized);
            }
            let was_reserved = trade.state == FiatTradeState::CryptoReserved
                || trade.state == FiatTradeState::PaymentRequested
                || trade.state == FiatTradeState::PaymentSubmitted
                || trade.state == FiatTradeState::PaymentPending
                || trade.state == FiatTradeState::Detected
                || trade.state == FiatTradeState::Confirming
                || trade.state == FiatTradeState::MakerConfirmed
                || trade.state == FiatTradeState::Confirmed;
            trade.transition(FiatTradeState::Cancelled, timestamp)?;
            was_reserved
        };

        // Only return crypto from escrow if it was actually reserved
        if crypto_reserved {
            self.crypto_wallet.return_escrow(trade_id)?;
        }

        // Release bonds — works for both Required and Locked states
        if let Some((maker_bond, taker_bond)) = self.bonds.get_mut(&trade_id.0) {
            maker_bond.release(timestamp).map_err(|e| {
                SettlementError::Fiat(FiatError::BondError(e.to_string()))
            })?;
            taker_bond.release(timestamp).map_err(|e| {
                SettlementError::Fiat(FiatError::BondError(e.to_string()))
            })?;
        }

        self.emit(SettlementEvent::TradeStateChanged {
            trade_id,
            state: FiatTradeState::Cancelled,
        });

        Ok(())
    }

    /// Calculate bond amount from trade value.
    fn calculate_bond_amount(crypto_amount: u64, bond_bps: u32) -> u64 {
        // Bond is bond_bps basis points of trade value
        (crypto_amount as u128 * bond_bps as u128 / 10_000) as u64
    }

    /// Get a mutable reference to a trade.
    fn get_trade_mut(&mut self, trade_id: TradeId) -> Result<&mut FiatTrade, SettlementError> {
        self.trades
            .get_mut(&trade_id.0)
            .ok_or(SettlementError::Fiat(FiatError::PaymentNotFound))
    }

    /// Get an immutable reference to a trade.
    fn get_trade(&self, trade_id: TradeId) -> Result<&FiatTrade, SettlementError> {
        self.trades
            .get(&trade_id.0)
            .ok_or(SettlementError::Fiat(FiatError::PaymentNotFound))
    }

    /// Emit a settlement event.
    fn emit(&mut self, event: SettlementEvent) {
        self.events.push(event);
    }

    /// Get a trade by ID.
    pub fn get_trade_public(&self, trade_id: TradeId) -> Option<&FiatTrade> {
        self.trades.get(&trade_id.0)
    }

    /// Get all active trades.
    pub fn all_trades(&self) -> Vec<&FiatTrade> {
        self.trades.values().collect()
    }

    /// Check for expired deadlines.
    pub fn check_deadlines(&mut self, current_time: u64) {
        // Collect trades that need to fail due to expired deadlines
        let mut to_fail: Vec<TradeId> = Vec::new();

        for (id, trade) in &self.trades {
            match trade.state {
                FiatTradeState::PaymentPending
                    if current_time > trade.deadlines.payment_deadline =>
                {
                    to_fail.push(TradeId(*id));
                }
                FiatTradeState::PaymentSubmitted
                    if current_time > trade.deadlines.payment_deadline =>
                {
                    to_fail.push(TradeId(*id));
                }
                FiatTradeState::Confirming
                    if current_time > trade.deadlines.confirmation_deadline =>
                {
                    to_fail.push(TradeId(*id));
                }
                // MakerConfirmed timeout: if maker confirms but never releases crypto,
                // the trade must eventually fail to prevent permanent fund lockup
                FiatTradeState::MakerConfirmed
                    if current_time > trade.deadlines.confirmation_deadline =>
                {
                    to_fail.push(TradeId(*id));
                }
                _ => {}
            }
        }

        // Apply the failures
        for trade_id in to_fail {
            if let Some(trade) = self.trades.get_mut(&trade_id.0) {
                let _ = trade.transition(FiatTradeState::Failed, current_time);
                self.emit(SettlementEvent::TradeStateChanged {
                    trade_id,
                    state: FiatTradeState::Failed,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::MockPaymentProvider;
    use opendex_types::{FiatCurrency, ParticipantId, TradeId};
    use crate::payment::{FiatAmount, PaymentId, PaymentReference, PaymentStatus};
    use crate::trade::{FiatOffer, SettlementMethod};

    /// Mock crypto wallet for testing.
    #[derive(Default)]
    struct MockCryptoWallet {
        pub escrowed: std::collections::HashMap<Uuid, (Asset, u64)>,
    }

    impl CryptoWallet for MockCryptoWallet {

        fn reserve_escrow(
            &self,
            trade_id: TradeId,
            asset: &Asset,
            amount: u64,
        ) -> Result<String, SettlementError> {
            let _ = (trade_id, asset, amount);
            Ok("mock_escrow_tx".to_string())
        }

        fn release_escrow(
            &self,
            trade_id: TradeId,
            recipient: &str,
        ) -> Result<String, SettlementError> {
            let _ = (trade_id, recipient);
            Ok("mock_release_tx".to_string())
        }

        fn return_escrow(
            &self,
            trade_id: TradeId,
        ) -> Result<String, SettlementError> {
            let _ = trade_id;
            Ok("mock_return_tx".to_string())
        }
    }

    fn make_engine() -> (SettlementEngine<MockPaymentProvider, MockCryptoWallet>, ParticipantId, ParticipantId) {
        let maker = ParticipantId(Uuid::new_v4());
        let taker = ParticipantId(Uuid::new_v4());
        (
            SettlementEngine::new(
                MockPaymentProvider::auto_confirm(),
                MockCryptoWallet::default(),
            ),
            maker,
            taker,
        )
    }

    fn make_offer(maker: ParticipantId) -> FiatOffer {
        FiatOffer {
            trade_id: TradeId(Uuid::new_v4()),
            maker,
            currency: FiatCurrency::GBP,
            crypto_asset: Asset::Btc,
            amount: FiatAmount::new(100000, FiatCurrency::GBP),
            price: opendex_types::Price::new(
                Asset::Btc,
                Asset::Fiat(FiatCurrency::GBP),
                50000,
                1,
            ),
            settlement_method: SettlementMethod::BankTransfer {
                currency: FiatCurrency::GBP,
            },
            settlement_window: 3600,
            bond_requirement: 100,
        }
    }

    #[test]
    fn create_trade() {
        let (mut engine, maker, taker) = make_engine();
        let offer = make_offer(maker);

        let trade_id = engine
            .create_trade(&offer, taker, "mock_address".to_string(), 1000)
            .unwrap();

        assert!(engine.get_trade_public(trade_id).is_some());
        let events = engine.take_events();
        assert!(events.iter().any(|e| matches!(e, SettlementEvent::TradeStateChanged { state: FiatTradeState::Created, .. })));
    }

    #[test]
    fn complete_trade_lifecycle() {
        let (mut engine, maker, taker) = make_engine();
        let offer = make_offer(maker);

        let trade_id = engine
            .create_trade(&offer, taker, "mock_address".to_string(), 1000)
            .unwrap();

        engine.accept_trade(trade_id, 1001).unwrap();
        engine.reserve_crypto(trade_id, 1002).unwrap();

        let bank_details = crate::payment::PaymentRecipient {
            name: "Maker".to_string(),
            account_number: "12345678".to_string(),
            routing_code: "12-34-56".to_string(),
            bank_name: "Test Bank".to_string(),
            iban: None,
            swift_code: None,
        };
        engine.request_payment(trade_id, bank_details, 1003).unwrap();
        engine.mark_payment_pending(trade_id, 1004).unwrap();

        // Transition through Detected state (required by state machine)
        {
            let trade = engine.trades.get_mut(&trade_id.0).unwrap();
            trade.transition(FiatTradeState::Detected, 1004).unwrap();
        }

        let verified = VerifiedPayment {
            payment_id: PaymentId(Uuid::new_v4()),
            currency: FiatCurrency::GBP,
            amount: FiatAmount::new(100000, FiatCurrency::GBP),
            sender_reference: PaymentReference("test-ref".to_string()),
            recipient_reference: PaymentReference("test-ref".to_string()),
            verified_at: chrono::Utc::now(),
            status: PaymentStatus::Confirmed,
        };
        engine.confirm_payment(trade_id, verified, 1005).unwrap();
        engine.release_crypto(trade_id, maker, 1006).unwrap();
        engine.complete_trade(trade_id, maker, 1007).unwrap();

        let trade = engine.get_trade_public(trade_id).unwrap();
        assert_eq!(trade.state, FiatTradeState::Complete);
    }

    #[test]
    fn manual_trade_lifecycle() {
        // Manual flow: maker sends bank details → taker pays → taker confirms → maker confirms → release
        let (mut engine, maker, taker) = make_engine();
        let offer = make_offer(maker);

        let trade_id = engine
            .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
            .unwrap();

        engine.accept_trade(trade_id, 1001).unwrap();
        engine.reserve_crypto(trade_id, 1002).unwrap();

        // Step 1: Maker sends bank details to taker
        let bank_details = crate::payment::PaymentRecipient {
            name: "Maker".to_string(),
            account_number: "87654321".to_string(),
            routing_code: "65-43-21".to_string(),
            bank_name: "Maker Bank".to_string(),
            iban: None,
            swift_code: None,
        };
        let instructions = engine.request_payment(trade_id, bank_details.clone(), 1003).unwrap();
        assert_eq!(instructions.recipient.name, "Maker");
        assert!(!instructions.reference.0.is_empty());

        // Step 2: Taker makes payment externally, then clicks "I've paid"
        engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();
        let trade = engine.get_trade_public(trade_id).unwrap();
        assert_eq!(trade.state, FiatTradeState::PaymentSubmitted);

        // Step 3: Maker checks bank, clicks "I confirm payment"
        engine.maker_confirm_payment(trade_id, maker, 1005).unwrap();
        let trade = engine.get_trade_public(trade_id).unwrap();
        assert_eq!(trade.state, FiatTradeState::MakerConfirmed);

        // Step 4: Crypto released to taker
        let tx_hash = engine.release_crypto(trade_id, maker, 1006).unwrap();
        assert!(!tx_hash.is_empty());

        // Step 5: Trade completed
        engine.complete_trade(trade_id, maker, 1007).unwrap();
        let trade = engine.get_trade_public(trade_id).unwrap();
        assert_eq!(trade.state, FiatTradeState::Complete);

        // Verify events
        let events = engine.take_events();
        assert!(events.iter().any(|e| matches!(e, SettlementEvent::PaymentInstructionsSent { .. })));
        assert!(events.iter().any(|e| matches!(e, SettlementEvent::TakerPaymentConfirmed { .. })));
        assert!(events.iter().any(|e| matches!(e, SettlementEvent::MakerPaymentConfirmed { .. })));
        assert!(events.iter().any(|e| matches!(e, SettlementEvent::TradeCompleted { .. })));
    }

    #[test]
    fn manual_flow_requires_maker_confirm() {
        // Taker cannot skip maker confirmation
        let (mut engine, maker, taker) = make_engine();
        let offer = make_offer(maker);

        let trade_id = engine
            .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
            .unwrap();

        engine.accept_trade(trade_id, 1001).unwrap();
        engine.reserve_crypto(trade_id, 1002).unwrap();

        let bank_details = crate::payment::PaymentRecipient {
            name: "Maker".to_string(),
            account_number: "12345678".to_string(),
            routing_code: "12-34-56".to_string(),
            bank_name: "Test Bank".to_string(),
            iban: None,
            swift_code: None,
        };
        engine.request_payment(trade_id, bank_details, 1003).unwrap();
        engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();

        // Cannot release crypto without maker confirmation
        assert!(engine.release_crypto(trade_id, maker, 1005).is_err());
    }

    #[test]
    fn cancel_trade() {
        let (mut engine, maker, taker) = make_engine();
        let offer = make_offer(maker);

        let trade_id = engine
            .create_trade(&offer, taker, "mock_address".to_string(), 1000)
            .unwrap();

        engine.accept_trade(trade_id, 1001).unwrap();
        engine.cancel_trade(trade_id, maker, 1002).unwrap();

        let trade = engine.get_trade_public(trade_id).unwrap();
        assert_eq!(trade.state, FiatTradeState::Cancelled);
    }

    #[test]
    fn open_dispute() {
        let (mut engine, maker, taker) = make_engine();
        let offer = make_offer(maker);

        let trade_id = engine
            .create_trade(&offer, taker, "mock_address".to_string(), 1000)
            .unwrap();

        let dispute_id = engine
            .open_dispute(trade_id, taker, DisputeReason::PaymentNotReceived, 1001)
            .unwrap();

        let trade = engine.get_trade_public(trade_id).unwrap();
        assert_eq!(trade.state, FiatTradeState::Disputed);
        assert!(dispute_id != Uuid::nil());
    }

    #[test]
    fn payment_timeout() {
        let (mut engine, maker, taker) = make_engine();
        let offer = make_offer(maker);

        let trade_id = engine
            .create_trade(&offer, taker, "mock_address".to_string(), 1000)
            .unwrap();

        engine.accept_trade(trade_id, 1001).unwrap();
        engine.reserve_crypto(trade_id, 1002).unwrap();

        let bank_details = crate::payment::PaymentRecipient {
            name: "Maker".to_string(),
            account_number: "12345678".to_string(),
            routing_code: "12-34-56".to_string(),
            bank_name: "Test Bank".to_string(),
            iban: None,
            swift_code: None,
        };
        engine.request_payment(trade_id, bank_details, 1003).unwrap();
        engine.mark_payment_pending(trade_id, 1004).unwrap();

        // Advance time past payment deadline
        engine.check_deadlines(1000 + 3600 + 100);

        let trade = engine.get_trade_public(trade_id).unwrap();
        assert_eq!(trade.state, FiatTradeState::Failed);
    }

    #[test]
    fn manual_payment_timeout() {
        // In manual flow, if maker doesn't confirm in time, trade fails
        let (mut engine, maker, taker) = make_engine();
        let offer = make_offer(maker);

        let trade_id = engine
            .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000)
            .unwrap();

        engine.accept_trade(trade_id, 1001).unwrap();
        engine.reserve_crypto(trade_id, 1002).unwrap();

        let bank_details = crate::payment::PaymentRecipient {
            name: "Maker".to_string(),
            account_number: "12345678".to_string(),
            routing_code: "12-34-56".to_string(),
            bank_name: "Test Bank".to_string(),
            iban: None,
            swift_code: None,
        };
        engine.request_payment(trade_id, bank_details, 1003).unwrap();
        engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();

        // Advance time past payment deadline — maker never confirmed
        engine.check_deadlines(1000 + 3600 + 100);

        let trade = engine.get_trade_public(trade_id).unwrap();
        assert_eq!(trade.state, FiatTradeState::Failed);
    }

    #[test]
    fn trade_creation_rate_limited() {
        let (mut engine, maker, taker) = make_engine();
        let offer = make_offer(maker);
        let taker_key = taker.0.to_string();

        // Create 10 trades within the same second (within the 1-hour window)
        for i in 0..10 {
            let trade_id = engine
                .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000 + i as u64)
                .unwrap();
            assert!(trade_id.0 != Uuid::nil());
        }

        // 11th trade should be rate limited
        let result = engine.create_trade(&offer, taker, "bc1qtaker123".to_string(), 1010);
        assert_eq!(result.unwrap_err(), SettlementError::RateLimited);
    }

    #[test]
    fn trade_creation_rate_limit_per_user() {
        let (mut engine, maker, _taker) = make_engine();
        let offer = make_offer(maker);
        let taker1 = ParticipantId(Uuid::new_v4());
        let taker2 = ParticipantId(Uuid::new_v4());

        // Exhaust taker1's limit
        for i in 0..10 {
            engine
                .create_trade(&offer, taker1, "bc1qtaker123".to_string(), 1000 + i as u64)
                .ok();
        }
        let result = engine.create_trade(&offer, taker1, "bc1qtaker123".to_string(), 1010);
        assert_eq!(result.unwrap_err(), SettlementError::RateLimited);

        // taker2 should still be able to create trades
        let result = engine.create_trade(&offer, taker2, "bc1qtaker123".to_string(), 1010);
        assert!(result.is_ok());
    }

    #[test]
    fn payment_confirm_rate_limited() {
        let (mut engine, maker, taker) = make_engine();
        let offer = make_offer(maker);

        let bank_details = crate::payment::PaymentRecipient {
            name: "Maker".to_string(),
            account_number: "12345678".to_string(),
            routing_code: "12-34-56".to_string(),
            bank_name: "Test Bank".to_string(),
            iban: None,
            swift_code: None,
        };

        // Create multiple trades for the same taker
        let trade_ids: Vec<_> = (0..6)
            .map(|i| {
                let trade_id = engine
                    .create_trade(&offer, taker, "bc1qtaker123".to_string(), 1000 + i as u64)
                    .unwrap();
                engine.accept_trade(trade_id, 1100 + i as u64).unwrap();
                engine.reserve_crypto(trade_id, 1200 + i as u64).unwrap();
                engine.request_payment(trade_id, bank_details.clone(), 1300 + i as u64).unwrap();
                trade_id
            })
            .collect();

        // Confirm payment 5 times (within limit)
        for (i, &trade_id) in trade_ids.iter().take(5).enumerate() {
            let result = engine.taker_confirm_payment(trade_id, taker, 2000 + i as u64);
            assert!(result.is_ok(), "Payment confirm {} should succeed", i + 1);
        }

        // 6th confirmation should be rate limited
        let result = engine.taker_confirm_payment(trade_ids[5], taker, 2005);
        assert_eq!(result.unwrap_err(), SettlementError::RateLimited);
    }
}
