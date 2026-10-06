#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::message::StateResponse;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<StateResponse>(data);
});
