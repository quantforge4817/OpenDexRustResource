//! L33: End-to-end fiat settlement integration test.
//!
//! Covers the full lifecycle: offer creation → trade creation → crypto reserve
//! → payment confirm → crypto release → complete.

use opendex_fiat::adapter::MockPaymentProvider;
use opendex_fiat::settlement_engine::{CryptoWallet, SettlementEngine};
use opendex_fiat::trade::{FiatOffer, FiatTradeState, SettlementMethod};
use opendex_fiat::error::SettlementError;
use opendex_fiat::payment::FiatAmount;
use opendex_types::{Asset, FiatCurrency, ParticipantId, Price, TradeId};
use uuid::Uuid;

/// Mock crypto wallet for E2E testing.
#[derive(Default)]
struct MockCryptoWallet;

impl CryptoWallet for MockCryptoWallet {
    fn reserve_escrow(
        &self,
        trade_id: TradeId,
        _asset: &Asset,
        _amount: u64,
    ) -> Result<String, SettlementError> {
        Ok(format!("mock_escrow_{}", trade_id.0))
    }

    fn release_escrow(
        &self,
        trade_id: TradeId,
        _recipient_address: &str,
    ) -> Result<String, SettlementError> {
        Ok(format!("mock_release_{}", trade_id.0))
    }

    fn return_escrow(
        &self,
        trade_id: TradeId,
    ) -> Result<String, SettlementError> {
        Ok(format!("mock_return_{}", trade_id.0))
    }
}

fn make_offer(maker: ParticipantId) -> FiatOffer {
    FiatOffer {
        trade_id: TradeId(Uuid::new_v4()),
        maker,
        currency: FiatCurrency::GBP,
        crypto_asset: Asset::Btc,
        amount: FiatAmount { minor_units: 100_000, currency: FiatCurrency::GBP },
        price: Price { base: Asset::Btc, quote: Asset::Fiat(FiatCurrency::GBP), numerator: 100_000_000, denominator: 1 },
        settlement_method: SettlementMethod::BankTransfer { currency: FiatCurrency::GBP },
        settlement_window: 3600,
        bond_requirement: 1000,
    }
}

#[test]
fn e2e_fiat_trade_lifecycle() {
    let maker = ParticipantId(Uuid::new_v4());
    let taker = ParticipantId(Uuid::new_v4());
    let offer = make_offer(maker);

    let payment_adapter = MockPaymentProvider::auto_confirm();
    let crypto_wallet = MockCryptoWallet;
    let mut engine = SettlementEngine::new(payment_adapter, crypto_wallet);

    // 1. Create trade
    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtest1234567890abcdef".to_string(), 1000)
        .unwrap();

    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::Created);

    // 2. Accept trade
    engine.accept_trade(trade_id, 1001).unwrap();
    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::Accepted);

    // 3. Reserve crypto
    let escrow_tx = engine.reserve_crypto(trade_id, 1002).unwrap();
    assert!(!escrow_tx.is_empty());
    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::CryptoReserved);

    // 4. Request payment (maker provides bank details)
    let bank_details = opendex_fiat::payment::PaymentRecipient {
        name: "Maker".to_string(),
        account_number: "12345678".to_string(),
        routing_code: "12-34-56".to_string(),
        bank_name: "Test Bank".to_string(),
        iban: None,
        swift_code: None,
    };
    let _instructions = engine.request_payment(trade_id, bank_details, 1003).unwrap();
    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::PaymentRequested);

    // 5. Taker confirms payment
    engine.taker_confirm_payment(trade_id, taker, 1004).unwrap();
    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::PaymentSubmitted);

    // 6. Maker confirms payment
    engine.maker_confirm_payment(trade_id, maker, 1005).unwrap();
    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::MakerConfirmed);

    // 7. Release crypto
    let release_tx = engine.release_crypto(trade_id, maker, 1006).unwrap();
    assert!(!release_tx.is_empty());
    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::CryptoRelease);

    // 8. Complete trade
    engine.complete_trade(trade_id, maker, 1007).unwrap();
    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::Complete);

    // Verify events were emitted
    let events = engine.take_events();
    assert!(!events.is_empty(), "Events should have been emitted");
}

#[test]
fn e2e_fiat_trade_cancel() {
    let maker = ParticipantId(Uuid::new_v4());
    let taker = ParticipantId(Uuid::new_v4());
    let offer = make_offer(maker);

    let payment_adapter = MockPaymentProvider::new();
    let crypto_wallet = MockCryptoWallet;
    let mut engine = SettlementEngine::new(payment_adapter, crypto_wallet);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtest1234567890abcdef".to_string(), 1000)
        .unwrap();

    engine.accept_trade(trade_id, 1001).unwrap();

    // Cancel by maker (needs timestamp)
    engine.cancel_trade(trade_id, maker, 1002).unwrap();

    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::Cancelled);
}

#[test]
fn e2e_fiat_trade_dispute() {
    let maker = ParticipantId(Uuid::new_v4());
    let taker = ParticipantId(Uuid::new_v4());
    let offer = make_offer(maker);

    let payment_adapter = MockPaymentProvider::new();
    let crypto_wallet = MockCryptoWallet;
    let mut engine = SettlementEngine::new(payment_adapter, crypto_wallet);

    let trade_id = engine
        .create_trade(&offer, taker, "bc1qtest1234567890abcdef".to_string(), 1000)
        .unwrap();

    // Open dispute
    let dispute_id = engine
        .open_dispute(
            trade_id,
            taker,
            opendex_fiat::dispute::DisputeReason::PaymentNotReceived,
            1001,
        )
        .unwrap();

    assert!(dispute_id != Uuid::nil());
    let trade = engine.get_trade_public(trade_id).unwrap();
    assert_eq!(trade.state, FiatTradeState::Disputed);
}
