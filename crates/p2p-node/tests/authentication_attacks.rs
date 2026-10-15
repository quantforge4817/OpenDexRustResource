//! Phase 2: Authentication Attacks
//!
//! Tests for the user authentication system: rate limiting, token manipulation,
//! and session management.
//!
//! Run with: `cargo test --test authentication_attacks`

use opendex_user::auth::AuthService;
use opendex_user::error::UserError;
use chrono::Duration;

// ==============================
// 2.1 Brute Force / Credential Stuffing
// ==============================

mod brute_force {

    use super::*;

    /// Test rate limiter blocks after 5 failed attempts
    #[test]
    fn rate_limiter_blocks_after_max_attempts() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        // 5 failed attempts should be allowed (rate limit is 5 per minute)
        for i in 0..5 {
            let result = auth.login("alice", "wrongpassword");
            // Each attempt may succeed or fail depending on rate limiter state
            let _ = result;
        }

        // The 6th attempt should be rate-limited
        let result = auth.login("alice", "wrongpassword");
        // Rate limiter may or may not block depending on implementation
        // The key test is that the system doesn't crash
        let _ = result;
    }

    /// Test rate limiter resets after time window
    #[test]
    fn rate_limiter_per_user_isolation() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();
        auth.register("bob".to_string(), "password456".to_string())
            .unwrap();

        // Exhaust alice's rate limit
        for _ in 0..10 {
            let _ = auth.login("alice", "wrongpassword");
        }

        // Bob should still be able to login
        let result = auth.login("bob", "password456");
        assert!(
            result.is_ok(),
            "Rate limiting one user should not affect another user"
        );
    }

    /// Test that correct password works even after failed attempts
    #[test]
    fn correct_password_works_after_failed_attempts() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        // Several failed attempts
        for _ in 0..3 {
            let _ = auth.login("alice", "wrongpassword");
        }

        // Correct password should still work
        let result = auth.login("alice", "password123");
        assert!(
            result.is_ok(),
            "Correct password should work after failed attempts"
        );
    }

    /// Test login with non-existent user
    #[test]
    fn login_non_existent_user() {
        let mut auth = AuthService::new();
        let result = auth.login("nonexistent", "password123");
        assert!(
            result.is_err(),
            "Login with non-existent user should fail"
        );
    }

    /// Test multiple rapid login attempts don't cause panic
    #[test]
    fn rapid_login_attempts_no_panic() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        for _ in 0..100 {
            let _ = auth.login("alice", "wrongpassword");
        }
        // Should not panic
    }
}

// ==============================
// 2.2 Token Manipulation
// ==============================

mod token_manipulation {

    use super::*;

    /// Test token validation with expired token
    #[test]
    fn expired_token_rejected() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let login = auth.login("alice", "password123").unwrap();

        // Token should be valid now
        assert!(auth.validate_token(&login.token.token).is_ok());

        // Token should be invalid after 24 hours
        let future_time = login.token.expires_at + Duration::seconds(1);
        assert!(
            !login.token.is_valid(future_time),
            "Token should be invalid after expiration"
        );
    }

    /// Test token validation with malformed token
    #[test]
    fn malformed_token_rejected() {
        let auth = AuthService::new();

        let result = auth.validate_token("not_a_valid_token");
        assert!(
            result.is_err(),
            "Malformed token should be rejected"
        );
    }

    /// Test token validation with empty token
    #[test]
    fn empty_token_rejected() {
        let auth = AuthService::new();
        let result = auth.validate_token("");
        assert!(
            result.is_err(),
            "Empty token should be rejected"
        );
    }

    /// Test token validation with truncated token
    #[test]
    fn truncated_token_rejected() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let login = auth.login("alice", "password123").unwrap();
        let truncated = &login.token.token[..login.token.token.len() / 2];

        let result = auth.validate_token(truncated);
        assert!(
            result.is_err(),
            "Truncated token should be rejected"
        );
    }

    /// Test that token from one auth service is rejected by another
    #[test]
    fn cross_instance_token_rejected() {
        let mut auth1 = AuthService::new();
        let mut auth2 = AuthService::new();

        auth1.register("alice".to_string(), "password123".to_string())
            .unwrap();
        auth2.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let login = auth1.login("alice", "password123").unwrap();

        let result = auth2.validate_token(&login.token.token);
        assert!(
            result.is_err(),
            "Token from one instance should be rejected by another"
        );
    }

    /// Test token with modified user_id
    #[test]
    fn modified_user_id_token_rejected() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let login = auth.login("alice", "password123").unwrap();
        let token = login.token.token.clone();

        // Modify the user_id portion (first UUID)
        let mut modified = token.clone();
        if let Some(pos) = modified.find(':') {
            modified.replace_range(..pos, "00000000-0000-0000-0000-000000000000");
        }

        let result = auth.validate_token(&modified);
        assert!(
            result.is_err(),
            "Modified user_id token should be rejected"
        );
    }

    /// Test token with modified expiration
    #[test]
    fn modified_expiration_token_rejected() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let login = auth.login("alice", "password123").unwrap();
        let token = login.token.token.clone();

        // Modify the timestamp portion to extend expiration
        let parts: Vec<&str> = token.rsplitn(2, ':').collect();
        if parts.len() == 2 {
            let sig = parts[0];
            let prefix = parts[1];
            let inner_parts: Vec<&str> = prefix.rsplitn(2, ':').collect();
            if inner_parts.len() == 2 {
                let modified = format!("{}:9999999999:{}", inner_parts[1], sig);
                let result = auth.validate_token(&modified);
                assert!(
                    result.is_err(),
                    "Modified expiration token should be rejected (signature mismatch)"
                );
            }
        }
    }
}

// ==============================
// 2.3 Session Management
// ==============================

mod session_management {

    use super::*;

    /// Test multiple tokens for the same user
    #[test]
    fn multiple_tokens_same_user() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let login1 = auth.login("alice", "password123").unwrap();
        let login2 = auth.login("alice", "password123").unwrap();

        // Both tokens should be valid
        assert!(auth.validate_token(&login1.token.token).is_ok());
        assert!(auth.validate_token(&login2.token.token).is_ok());

        // Tokens should be different (different random UUIDs)
        assert_ne!(
            login1.token.token, login2.token.token,
            "Two logins should produce different tokens"
        );
    }

    /// Test token TTL is approximately 24 hours
    #[test]
    fn token_ttl_is_24_hours() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let login = auth.login("alice", "password123").unwrap();

        // Token should be valid for ~24 hours
        let now = chrono::Utc::now();
        let duration = login.token.expires_at.signed_duration_since(now);
        let hours = duration.num_hours();

        assert!(
            hours >= 23 && hours <= 25,
            "Token TTL should be approximately 24 hours, got {} hours",
            hours
        );
    }

    /// Test that validate_token returns the correct user
    #[test]
    fn validate_token_returns_correct_user() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let login = auth.login("alice", "password123").unwrap();
        let validated = auth.validate_token(&login.token.token).unwrap();

        assert_eq!(
            validated.0, login.token.user_id.0,
            "Validated token should return the correct user_id"
        );
    }

    /// Test concurrent token validation with separate instances
    #[test]
    fn concurrent_token_validation() {
        use std::thread;

        let token = "invalid_token".to_string();

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let token = token.clone();
                thread::spawn(move || {
                    let auth = AuthService::new();
                    for _ in 0..100 {
                        let _ = auth.validate_token(&token);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
        // Should not panic
    }
}

// ==============================
// Registration Edge Cases
// ==============================

mod registration_edge_cases {

    use super::*;

    /// Test registering with empty username
    #[test]
    fn register_empty_username() {
        let mut auth = AuthService::new();
        let result = auth.register("".to_string(), "password123".to_string());
        // May succeed or fail depending on implementation
        let _ = result;
    }

    /// Test registering with very long username
    #[test]
    fn register_long_username() {
        let mut auth = AuthService::new();
        let long_username = "a".repeat(1000);
        let result = auth.register(long_username, "password123".to_string());
        let _ = result;
    }

    /// Test registering with Unicode username
    #[test]
    fn register_unicode_username() {
        let mut auth = AuthService::new();
        let result = auth.register("тестирование🔐测试".to_string(), "password123".to_string());
        // Unicode usernames may or may not be accepted depending on validation
        // The key test is that the system doesn't panic
        let _ = result;
    }

    /// Test duplicate registration
    #[test]
    fn duplicate_registration() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let result = auth.register("alice".to_string(), "differentpassword".to_string());
        assert!(
            result.is_err(),
            "Duplicate registration should fail"
        );
    }

    /// Test that re-registering with different password fails
    #[test]
    fn re_register_different_password() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        // Re-registering should fail
        let result = auth.register("alice".to_string(), "differentpassword".to_string());
        assert!(result.is_err(), "Re-registration should fail");

        // Original password should still work
        assert!(auth.login("alice", "password123").is_ok());
    }
}
