#![no_main]

use libfuzzer_sys::fuzz_target;
use opendex_fiat::trade::{FiatTrade, FiatTradeState};

fuzz_target!(|data: &[u8]| {
    // Fuzz sequences of state transitions starting from a parsed trade
    if let Ok(mut trade) = serde_json::from_slice::<FiatTrade>(data) {
        // Try a sequence of transitions derived from input bytes
        for chunk in data.chunks(2).take(20) {
            let state_idx = chunk[0] as usize % 16;
            let timestamp = if chunk.len() > 1 { chunk[1] as u64 } else { 1 };

            let state = match state_idx {
                0 => FiatTradeState::Created,
                1 => FiatTradeState::Accepted,
                2 => FiatTradeState::CryptoReserved,
                3 => FiatTradeState::PaymentRequested,
                4 => FiatTradeState::PaymentSubmitted,
                5 => FiatTradeState::PaymentPending,
                6 => FiatTradeState::Detected,
                7 => FiatTradeState::Confirming,
                8 => FiatTradeState::MakerConfirmed,
                9 => FiatTradeState::Confirmed,
                10 => FiatTradeState::CryptoRelease,
                11 => FiatTradeState::Complete,
                12 => FiatTradeState::ReversalDetected,
                13 => FiatTradeState::Disputed,
                14 => FiatTradeState::Failed,
                _ => FiatTradeState::Cancelled,
            };
            let _ = trade.transition(state, timestamp);
        }
    }
});
