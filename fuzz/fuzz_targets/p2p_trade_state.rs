#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::message::Side;
use protocol::p2p::{CryptoAsset, FiatCurrency, PaymentMethod, P2PTrade, P2PTradeState};

fuzz_target!(|data: &[u8]| {
    // Parse a sequence of P2PTradeState indices from arbitrary bytes.
    // Each byte maps to one of the 9 P2PTradeState variants.
    let states: Vec<P2PTradeState> = data
        .iter()
        .filter_map(|&b| match b % 9 {
            0 => Some(P2PTradeState::OfferTaken),
            1 => Some(P2PTradeState::EscrowFunded),
            2 => Some(P2PTradeState::FiatPaymentPending),
            3 => Some(P2PTradeState::FiatPaymentSent),
            4 => Some(P2PTradeState::FiatPaymentConfirmed),
            5 => Some(P2PTradeState::CryptoReleased),
            6 => Some(P2PTradeState::Settled),
            7 => Some(P2PTradeState::Cancelled),
            _ => Some(P2PTradeState::Disputed),
        })
        .collect();

    if states.is_empty() {
        return;
    }

    // Build a minimal P2P trade starting in OfferTaken state.
    let mut trade = P2PTrade {
        id: Default::default(),
        offer_id: Default::default(),
        maker: "maker".to_string(),
        taker: "taker".to_string(),
        crypto_asset: CryptoAsset::XMR,
        side: Side::Buy,
        amount: 0,
        price: 0,
        fiat_amount: 0,
        currency: FiatCurrency::USD,
        payment_method: PaymentMethod::BankTransfer,
        state: P2PTradeState::OfferTaken,
        taker_crypto_address: "".to_string(),
        escrow_address: "".to_string(),
        escrow_tx_hash: None,
        payment_id: None,
        fiat_payment_proof: None,
        release_tx_hash: None,
        created_at: 1,
        updated_at: 1,
    };

    let mut timestamp: u64 = 2;
    for next in &states {
        // Invalid transitions must return an error, not panic.
        let _ = trade.transition(next.clone(), timestamp);
        timestamp += 1;
    }
});
