#![no_main]

use libfuzzer_sys::fuzz_target;
use opendex_fiat::bond::{SecurityBond, BondState};
use opendex_types::{Asset, ParticipantId, TradeId};
use uuid::Uuid;

fuzz_target!(|data: &[u8]| {
    // Fuzz SecurityBond state machine transitions
    if data.is_empty() {
        return;
    }

    let trade_id = TradeId(Uuid::new_v4());
    let participant = ParticipantId(Uuid::new_v4());
    let amount = data.iter().take(8).fold(0u64, |acc, b| (acc << 8) | (*b as u64));

    let mut bond = SecurityBond::new(
        trade_id,
        participant,
        Asset::Btc,
        amount,
        1,
    );

    // Try a sequence of operations derived from input bytes
    for byte in data.iter().skip(1).take(20) {
        let op = byte % 5;
        let timestamp = 1;
        match op {
            0 => { let _ = bond.lock(timestamp); }
            1 => { let _ = bond.release(timestamp); }
            2 => { let _ = bond.slash("fuzz violation".to_string(), "rule1".to_string(), timestamp); }
            _ => {
                // Direct state assignment for fuzzing edge cases
                bond.state = match byte % 6 {
                    0 => BondState::Required,
                    1 => BondState::Locked,
                    2 => BondState::Released,
                    3 => BondState::PartiallySlashed,
                    4 => BondState::Slashed,
                    _ => BondState::Refunded,
                };
            }
        }
    }
});
