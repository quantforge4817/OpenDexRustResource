#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::message::Trade;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<Trade>(data);
});
