#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::p2p::P2PTrade;

fuzz_target!(|data: &[u8]| {
    // Fuzz deserialization of P2P trades received from the network.
    // Malformed JSON should not panic, only return an error.
    let _ = serde_json::from_slice::<P2PTrade>(data);
});
