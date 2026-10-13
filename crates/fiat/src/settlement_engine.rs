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
        }
    }

    /// Get all events emitted since the last call.
    pub fn take_events(&mut self) -> Vec<SettlementEvent> {
        std::mem::take(&mut self.events)
    }

    /// Create a trade from a fiat offer.
    pub fn create_trade(
        &mut self,
        offer: &FiatOffer,
        taker: ParticipantId,
        taker_crypto_address: String,
        timestamp: u64,
    ) -> Result<TradeId, SettlementError> {
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

        if let Some(ref _payment_id) = trade.payment_id {
            // In a real implementation, we would check the adapter
            // For now, return None to indicate payment not yet detected
            Ok(None)
        } else {
            Ok(None)
        }
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
        {
            let trade = self.get_trade_mut(trade_id)?;
            if trade.maker != caller && trade.taker != caller {
                return Err(SettlementError::Unauthorized);
            }
            trade.transition(FiatTradeState::Cancelled, timestamp)?;
        }

        // Return crypto from escrow
        self.crypto_wallet.return_escrow(trade_id)?;

        // Release bonds
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
        let mut to_fail: Vec<(TradeId, FiatTradeState)> = Vec::new();

        for (id, trade) in &self.trades {
            match trade.state {
                FiatTradeState::PaymentPending
                    if current_time > trade.deadlines.payment_deadline =>
                {
                    to_fail.push((TradeId(*id), FiatTradeState::Failed));
                }
                FiatTradeState::PaymentSubmitted
                    if current_time > trade.deadlines.payment_deadline =>
                {
                    to_fail.push((TradeId(*id), FiatTradeState::Failed));
                }
                FiatTradeState::Confirming
                    if current_time > trade.deadlines.confirmation_deadline =>
                {
                    to_fail.push((TradeId(*id), FiatTradeState::Failed));
                }
                _ => {}
            }
        }

        // Apply the failures
        for (trade_id, state) in to_fail {
            if let Some(trade) = self.trades.get_mut(&trade_id.0) {
                let _ = trade.transition(state.clone(), current_time);
                self.emit(SettlementEvent::TradeStateChanged {
                    trade_id,
                    state,
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
}
