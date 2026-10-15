#![no_main]

use libfuzzer_sys::fuzz_target;
use opendex_fiat::trade::{FiatTrade, FiatTradeState};

fuzz_target!(|data: &[u8]| {
    // Fuzz JSON deserialization
    if let Ok(mut trade) = serde_json::from_slice::<FiatTrade>(data) {
        // Fuzz state transitions with arbitrary states derived from data
        for byte in data.iter().take(10) {
            let state = match byte % 16 {
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
            let _ = trade.transition(state, 1);
        }
    }
});
