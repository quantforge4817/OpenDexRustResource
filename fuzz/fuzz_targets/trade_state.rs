#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::message::{Asset, Market, Trade, TradeState};

fuzz_target!(|data: &[u8]| {
    // Parse a sequence of TradeState indices from arbitrary bytes.
    // Each byte maps to one of the 13 TradeState variants.
    let states: Vec<TradeState> = data
        .iter()
        .filter_map(|&b| match b % 13 {
            0 => Some(TradeState::Created),
            1 => Some(TradeState::Accepted),
            2 => Some(TradeState::EscrowFunding),
            3 => Some(TradeState::EscrowFunded),
            4 => Some(TradeState::PaymentPending),
            5 => Some(TradeState::PaymentSent),
            6 => Some(TradeState::PaymentConfirmed),
            7 => Some(TradeState::PayoutPending),
            8 => Some(TradeState::Settled),
            9 => Some(TradeState::Cancelled),
            10 => Some(TradeState::Expired),
            11 => Some(TradeState::Disputed),
            _ => None,
        })
        .collect();

    if states.is_empty() {
        return;
    }

    // Build a minimal trade starting in Created state.
    let mut trade = Trade {
        id: Default::default(),
        buy_order: Default::default(),
        sell_order: Default::default(),
        maker: "seller".to_string(),
        taker: "buyer".to_string(),
        buyer: "buyer".to_string(),
        seller: "seller".to_string(),
        market: Market { base: Asset::BTC, quote: Asset::XMR },
        price: 0,
        amount: 0,
        fill_sequence: 0,
        state: TradeState::Created,
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
