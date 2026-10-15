#![no_main]

use libfuzzer_sys::fuzz_target;
use opendex_user::auth::AuthService;

fuzz_target!(|data: &[u8]| {
    // Fuzz auth token validation with arbitrary byte strings as token input
    let auth = AuthService::new();

    // Try to interpret the data as a token string
    if let Ok(token_str) = core::str::from_utf8(data) {
        // Validate the token — this exercises HMAC verification and expiration checks
        let _ = auth.validate_token(token_str);
    }

    // Also try registration with arbitrary username/password
    if data.len() >= 4 {
        let mut auth = AuthService::new();
        let username = if let Ok(s) = core::str::from_utf8(&data[..data.len() / 2]) {
            s.to_string()
        } else {
            "fuzz_user".to_string()
        };
        let password = if let Ok(s) = core::str::from_utf8(&data[data.len() / 2..]) {
            s.to_string()
        } else {
            "fuzz_pass".to_string()
        };
        let _ = auth.register(username, password);
    }
});
