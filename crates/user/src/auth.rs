//! Authentication service for user registration and login.
//!
//! Provides registration, login, and token-based authentication.
//! Tokens are HMAC-SHA256 signed for integrity verification.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use opendex_types::ParticipantId;

type HmacSha256 = Hmac<Sha256>;

use crate::error::UserError;
use crate::user::{UserAccount, UserId, Username};

/// Sliding window rate limiter.
///
/// Tracks request timestamps per key and enforces a maximum number of
/// requests within a time window. Old entries are pruned on each check.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Maximum number of requests allowed per window.
    max_requests: usize,
    /// Window duration in seconds.
    window_seconds: u64,
    /// Per-key request timestamps (Unix epoch seconds).
    requests: HashMap<String, Vec<u64>>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
            requests: HashMap::new(),
        }
    }

    /// Check whether a request from the given key is allowed.
    ///
    /// Returns `true` if the request is within the rate limit, `false` otherwise.
    /// If allowed, the timestamp is recorded.
    pub fn is_allowed(&mut self, key: &str, current_time: u64) -> bool {
        let cutoff = if current_time >= self.window_seconds {
            current_time - self.window_seconds
        } else {
            0
        };

        let timestamps = self
            .requests
            .entry(key.to_string())
            .or_insert_with(Vec::new);

        // Prune old entries outside the window
        timestamps.retain(|&t| t > cutoff);

        if timestamps.len() >= self.max_requests {
            false
        } else {
            timestamps.push(current_time);
            true
        }
    }
}

/// Authentication token returned after successful login.
///
/// The token is an HMAC-SHA256 signed string containing:
/// `{random_uuid}:{expires_at_timestamp}:{signature}`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthToken {
    /// Full signed token string.
    pub token: String,
    /// Associated user ID.
    pub user_id: UserId,
    /// Token expiration time.
    pub expires_at: DateTime<Utc>,
}

impl AuthToken {
    /// Token validity duration: 24 hours.
    pub const TTL: i64 = 24 * 60 * 60; // seconds

    /// Create a new HMAC-signed authentication token.
    pub fn new(user_id: UserId, signing_key: &[u8]) -> Self {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(Self::TTL);
        let expires_ts = expires_at.timestamp();
        let random_part = Uuid::new_v4().to_string();
        let payload = format!("{}:{}:{}", user_id.0, random_part, expires_ts);

        let mut mac = HmacSha256::new_from_slice(signing_key)
            .expect("HMAC can accept any key size");
        mac.update(payload.as_bytes());
        let signature = mac.finalize();
        let sig_bytes = signature.into_bytes();
        let sig_hex = hex_encode(&sig_bytes);

        Self {
            token: format!("{}:{}", payload, sig_hex),
            user_id,
            expires_at,
        }
    }

    /// Verify the token signature and check expiration.
    pub fn verify(&self, signing_key: &[u8], now: DateTime<Utc>) -> bool {
        if now >= self.expires_at {
            return false;
        }
        let parts: Vec<&str> = self.token.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            return false;
        }
        let signature_hex = parts[0];
        let payload = parts[1];

        let mut mac = match HmacSha256::new_from_slice(signing_key) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac.update(payload.as_bytes());

        let sig_bytes = match hex_decode(signature_hex) {
            Some(b) => b,
            None => return false,
        };
        mac.verify_slice(&sig_bytes).is_ok()
    }

    /// Check if the token is still valid (not expired).
    pub fn is_valid(&self, now: DateTime<Utc>) -> bool {
        now < self.expires_at
    }
}

/// Simple hex encoding for HMAC signatures.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Simple hex decoding for HMAC signature verification.
fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let mut chars = hex.chars().peekable();
    while let Some(c1) = chars.next() {
        let hi = c1.to_digit(16)?;
        let c2 = chars.next()?;
        let lo = c2.to_digit(16)?;
        bytes.push((hi * 16 + lo) as u8);
    }
    Some(bytes)
}

/// Result of a login attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    /// Authentication token.
    pub token: AuthToken,
    /// User's username.
    pub username: String,
}

/// Result of a registration attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResult {
    /// Created user ID.
    pub user_id: UserId,
    /// Authentication token.
    pub token: AuthToken,
    /// Associated participant ID.
    pub participant_id: ParticipantId,
}

/// Authentication service managing user accounts and sessions.
pub struct AuthService {
    /// All registered user accounts.
    users: HashMap<UserId, UserAccount>,
    /// Username to user ID mapping for fast lookup.
    username_index: HashMap<String, UserId>,
    /// Active authentication tokens.
    tokens: HashMap<String, AuthToken>,
    /// HMAC signing key for token integrity.
    signing_key: Vec<u8>,
    /// Rate limiter for login attempts (5 per minute per username).
    login_rate_limiter: RateLimiter,
}

impl AuthService {
    /// Create a new authentication service with a random signing key.
    pub fn new() -> Self {
        let mut key = [0u8; 32];
        getrandom::getrandom(&mut key).expect("Failed to generate signing key");
        Self {
            users: HashMap::new(),
            username_index: HashMap::new(),
            tokens: HashMap::new(),
            signing_key: key.to_vec(),
            login_rate_limiter: RateLimiter::new(5, 60), // 5 attempts per minute
        }
    }

    /// Create a new authentication service with a specific signing key.
    pub fn with_signing_key(key: Vec<u8>) -> Self {
        Self {
            users: HashMap::new(),
            username_index: HashMap::new(),
            tokens: HashMap::new(),
            signing_key: key,
            login_rate_limiter: RateLimiter::new(5, 60), // 5 attempts per minute
        }
    }

    /// Register a new user account.
    ///
    /// Returns the user ID and an authentication token on success.
    pub fn register(
        &mut self,
        username: String,
        password: String,
    ) -> Result<RegistrationResult, UserError> {
        // Validate username
        let username_obj = Username::new(username.clone())?;

        // Check for duplicate username
        if self.username_index.contains_key(username_obj.as_str()) {
            return Err(UserError::UsernameTaken);
        }

        // Create participant ID and user account
        let participant_id = ParticipantId(Uuid::new_v4());
        let account = UserAccount::new(username_obj, &password, participant_id)?;

        // Store the account
        let user_id = account.id;
        self.username_index
            .insert(account.username.as_str().to_string(), user_id);
        self.users.insert(user_id, account);

        // Create auth token
        let token = AuthToken::new(user_id, &self.signing_key);
        self.tokens.insert(token.token.clone(), token.clone());

        Ok(RegistrationResult {
            user_id,
            token,
            participant_id,
        })
    }

    /// Log in with username and password.
    ///
    /// Returns an authentication token on success.
    pub fn login(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<LoginResult, UserError> {
        // Rate limit check: 5 attempts per minute per username
        let now = chrono::Utc::now().timestamp() as u64;
        if !self.login_rate_limiter.is_allowed(username, now) {
            return Err(UserError::RateLimited);
        }

        let user_id = *self
            .username_index
            .get(username)
            .ok_or(UserError::InvalidCredentials)?;

        let account = self
            .users
            .get_mut(&user_id)
            .ok_or(UserError::InvalidCredentials)?;

        // Check account status
        if !account.is_active() {
            return Err(UserError::AccountSuspended);
        }

        // Verify password
        if !account.verify_password(password) {
            return Err(UserError::InvalidCredentials);
        }

        // Record login
        account.record_login();

        // Create new token
        let token = AuthToken::new(user_id, &self.signing_key);
        self.tokens.insert(token.token.clone(), token.clone());

        Ok(LoginResult {
            token,
            username: account.username.to_string(),
        })
    }

    /// Validate an authentication token and return the associated user ID.
    /// Verifies the HMAC signature and checks expiration.
    pub fn validate_token(&self, token: &str) -> Result<UserId, UserError> {
        let auth_token = self
            .tokens
            .get(token)
            .ok_or(UserError::InvalidToken)?;

        // Verify HMAC signature
        if !auth_token.verify(&self.signing_key, Utc::now()) {
            return Err(UserError::InvalidToken);
        }

        if !auth_token.is_valid(Utc::now()) {
            return Err(UserError::InvalidToken);
        }

        Ok(auth_token.user_id)
    }

    /// Get a user account by user ID.
    pub fn get_user(&self, user_id: UserId) -> Option<&UserAccount> {
        self.users.get(&user_id)
    }

    /// Get a user account by username.
    pub fn get_user_by_username(&self, username: &str) -> Option<&UserAccount> {
        let user_id = self.username_index.get(username)?;
        self.users.get(user_id)
    }

    /// Log out by invalidating the token.
    pub fn logout(&mut self, token: &str) {
        self.tokens.remove(token);
    }

    /// Suspend a user account.
    pub fn suspend_user(&mut self, user_id: UserId) -> Result<(), UserError> {
        let account = self.users.get_mut(&user_id).ok_or(UserError::UserNotFound)?;
        account.suspend();
        // Invalidate all tokens for this user
        self.tokens.retain(|_, t| t.user_id != user_id);
        Ok(())
    }

    /// Reactivate a suspended user account.
    pub fn reactivate_user(&mut self, user_id: UserId) -> Result<(), UserError> {
        let account = self.users.get_mut(&user_id).ok_or(UserError::UserNotFound)?;
        account.reactivate();
        Ok(())
    }

    /// Get the number of registered users.
    pub fn user_count(&self) -> usize {
        self.users.len()
    }
}

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_new_user() {
        let mut auth = AuthService::new();
        let result = auth
            .register("alice".to_string(), "password123".to_string())
            .unwrap();

        assert!(result.user_id.0 != Uuid::nil());
        assert_eq!(result.token.user_id, result.user_id);
        assert_eq!(auth.user_count(), 1);
    }

    #[test]
    fn duplicate_username_rejected() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let result = auth.register("alice".to_string(), "otherpassword".to_string());
        assert_eq!(result.unwrap_err(), UserError::UsernameTaken);
    }

    #[test]
    fn login_success() {
        let mut auth = AuthService::new();
        auth.register("bob".to_string(), "securepass1".to_string())
            .unwrap();

        let result = auth.login("bob", "securepass1").unwrap();
        assert_eq!(result.username, "bob");
        assert!(result.token.is_valid(Utc::now()));
    }

    #[test]
    fn login_wrong_password() {
        let mut auth = AuthService::new();
        auth.register("bob".to_string(), "securepass1".to_string())
            .unwrap();

        let result = auth.login("bob", "wrongpassword");
        assert_eq!(result.unwrap_err(), UserError::InvalidCredentials);
    }

    #[test]
    fn login_nonexistent_user() {
        let mut auth = AuthService::new();
        let result = auth.login("nobody", "password123");
        assert_eq!(result.unwrap_err(), UserError::InvalidCredentials);
    }

    #[test]
    fn token_validation() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let login = auth.login("alice", "password123").unwrap();
        let user_id = auth.validate_token(&login.token.token).unwrap();
        assert_eq!(user_id, login.token.user_id);
    }

    #[test]
    fn invalid_token_rejected() {
        let auth = AuthService::new();
        let result = auth.validate_token("nonexistent-token");
        assert_eq!(result.unwrap_err(), UserError::InvalidToken);
    }

    #[test]
    fn logout_invalidates_token() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let login = auth.login("alice", "password123").unwrap();
        auth.logout(&login.token.token);

        let result = auth.validate_token(&login.token.token);
        assert_eq!(result.unwrap_err(), UserError::InvalidToken);
    }

    #[test]
    fn suspended_account_cannot_login() {
        let mut auth = AuthService::new();
        let reg = auth
            .register("alice".to_string(), "password123".to_string())
            .unwrap();

        auth.suspend_user(reg.user_id).unwrap();

        let result = auth.login("alice", "password123");
        assert_eq!(result.unwrap_err(), UserError::AccountSuspended);
    }

    #[test]
    fn reactivate_suspended_account() {
        let mut auth = AuthService::new();
        let reg = auth
            .register("alice".to_string(), "password123".to_string())
            .unwrap();

        auth.suspend_user(reg.user_id).unwrap();
        auth.reactivate_user(reg.user_id).unwrap();

        let result = auth.login("alice", "password123");
        assert!(result.is_ok());
    }

    #[test]
    fn get_user_by_username() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        let user = auth.get_user_by_username("alice").unwrap();
        assert_eq!(user.username.as_str(), "alice");
    }

    #[test]
    fn login_records_timestamp() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        auth.login("alice", "password123").unwrap();
        let user = auth.get_user_by_username("alice").unwrap();
        assert!(user.last_login.is_some());
    }

    #[test]
    fn rate_limiter_allows_within_limit() {
        let mut limiter = RateLimiter::new(3, 60);
        assert!(limiter.is_allowed("user1", 1000));
        assert!(limiter.is_allowed("user1", 1001));
        assert!(limiter.is_allowed("user1", 1002));
    }

    #[test]
    fn rate_limiter_blocks_over_limit() {
        let mut limiter = RateLimiter::new(3, 60);
        assert!(limiter.is_allowed("user1", 1000));
        assert!(limiter.is_allowed("user1", 1001));
        assert!(limiter.is_allowed("user1", 1002));
        // 4th request should be blocked
        assert!(!limiter.is_allowed("user1", 1003));
    }

    #[test]
    fn rate_limiter_window_expiry() {
        let mut limiter = RateLimiter::new(2, 60);
        assert!(limiter.is_allowed("user1", 1000));
        assert!(limiter.is_allowed("user1", 1001));
        assert!(!limiter.is_allowed("user1", 1002));
        // After window expires, should allow again
        assert!(limiter.is_allowed("user1", 1061));
    }

    #[test]
    fn rate_limiter_per_user_isolation() {
        let mut limiter = RateLimiter::new(2, 60);
        assert!(limiter.is_allowed("user1", 1000));
        assert!(limiter.is_allowed("user1", 1001));
        assert!(!limiter.is_allowed("user1", 1002));
        // Different user should not be affected
        assert!(limiter.is_allowed("user2", 1000));
        assert!(limiter.is_allowed("user2", 1001));
    }

    #[test]
    fn login_rate_limited_after_5_attempts() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();

        // 5 successful logins should succeed
        for _ in 0..5 {
            let result = auth.login("alice", "password123");
            assert!(result.is_ok(), "Login should succeed within limit");
        }

        // 6th attempt should be rate limited
        let result = auth.login("alice", "password123");
        assert_eq!(result.unwrap_err(), UserError::RateLimited);
    }

    #[test]
    fn login_rate_limit_per_username() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string())
            .unwrap();
        auth.register("bob".to_string(), "password123".to_string())
            .unwrap();

        // Exhaust Alice's limit
        for _ in 0..5 {
            auth.login("alice", "password123").ok();
        }
        assert_eq!(auth.login("alice", "password123").unwrap_err(), UserError::RateLimited);

        // Bob should still be able to login
        let result = auth.login("bob", "password123");
        assert!(result.is_ok());
    }
}
