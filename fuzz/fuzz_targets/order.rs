#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::message::Order;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<Order>(data);
});
