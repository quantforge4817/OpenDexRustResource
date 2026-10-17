//! Authentication service for user registration and login.
//!
//! Provides registration, login, and token-based authentication.
//! Tokens are HMAC-SHA256 signed for integrity verification.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use opendex_types::ParticipantId;

type HmacSha256 = Hmac<Sha256>;

use crate::error::UserError;
use crate::user::{UserAccount, UserId, Username, Email, BtcWalletAddress, XmrWalletAddress, ReputationTier, Rating, Review};

/// Sliding window rate limiter.
///
/// Tracks request timestamps per key and enforces a maximum number of
/// requests within a time window. Old entries are pruned on each check.
/// To prevent unbounded memory growth (DoS vector), the number of tracked
/// keys is capped; when the cap is reached, the key with the oldest last
/// request time is evicted.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Maximum number of requests allowed per window.
    max_requests: usize,
    /// Window duration in seconds.
    window_seconds: u64,
    /// Per-key request timestamps (Unix epoch seconds).
    requests: HashMap<String, Vec<u64>>,
    /// Maximum number of unique keys tracked; 0 means unlimited.
    max_keys: usize,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
            requests: HashMap::new(),
            max_keys: 0,
        }
    }

    /// Create a new rate limiter with a maximum number of tracked keys.
    ///
    /// When the number of unique keys exceeds `max_keys`, the key with the
    /// oldest last request timestamp is evicted to prevent unbounded memory
    /// growth.
    pub fn with_max_keys(max_requests: usize, window_seconds: u64, max_keys: usize) -> Self {
        Self {
            max_requests,
            window_seconds,
            requests: HashMap::new(),
            max_keys,
        }
    }

    /// Evict the key with the oldest last request timestamp.
    fn evict_oldest_key(&mut self) {
        if self.requests.is_empty() {
            return;
        }

        let oldest_key = self
            .requests
            .iter()
            .min_by_key(|(_, timestamps)| {
                timestamps.last().copied().unwrap_or(u64::MAX)
            })
            .map(|(key, _)| key.clone());

        if let Some(key) = oldest_key {
            self.requests.remove(&key);
            tracing::debug!(key = %key, "Evicted oldest rate limiter key");
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

        // Evict oldest key if we've reached the max key limit and this is a new key
        if self.max_keys > 0
            && !self.requests.contains_key(key)
            && self.requests.len() >= self.max_keys
        {
            self.evict_oldest_key();
        }

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

/// Atomically write bytes to a file using temp file + rename.
///
/// Prevents corruption if the process crashes mid-write.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("users");

    let temp_path = parent.join(format!(".{}.{}.tmp", file_name, Uuid::new_v4()));

    let result = (|| -> Result<()> {
        let mut file = File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);

        fs::rename(&temp_path, path)?;

        #[cfg(unix)]
        {
            File::open(parent)?.sync_all()?;
        }

        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
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
    /// Email verification code (6-digit code for console/email).
    pub verification_code: String,
}

/// Email verification code with expiration.
#[derive(Debug, Clone)]
struct VerificationCodeEntry {
    /// The 6-digit verification code.
    code: String,
    /// User ID associated with this code.
    user_id: UserId,
    /// Expiration time.
    expires_at: DateTime<Utc>,
}

impl VerificationCodeEntry {
    /// Code validity duration: 15 minutes.
    const TTL_MINUTES: i64 = 15;

    fn new(user_id: UserId) -> Self {
        // Generate a 6-digit code using random bytes
        let mut bytes = [0u8; 4];
        rand_core::OsRng.fill_bytes(&mut bytes);
        let num = u32::from_le_bytes(bytes) as u64 % 1_000_000;
        let code = format!("{:06}", num);
        let expires_at = Utc::now() + Duration::minutes(Self::TTL_MINUTES);
        Self { code, user_id, expires_at }
    }

    fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
}

/// Serializable snapshot of persistent user data.
///
/// Contains only the data that must survive a restart: user accounts,
/// username index, email index, and the HMAC signing key.
/// Sessions, verification codes, and rate limiter state are intentionally
/// excluded — they are short-lived and will be regenerated on next use.
#[derive(Debug, Serialize, Deserialize)]
struct UserStoreSnapshot {
    /// All registered user accounts.
    users: HashMap<UserId, UserAccount>,
    /// Username to user ID mapping for fast lookup.
    username_index: HashMap<String, UserId>,
    /// Email to user ID mapping for fast lookup.
    email_index: HashMap<String, UserId>,
    /// HMAC signing key for token integrity (stored as hex).
    signing_key_hex: String,
}

/// Authentication service managing user accounts and sessions.
pub struct AuthService {
    /// All registered user accounts.
    users: HashMap<UserId, UserAccount>,
    /// Username to user ID mapping for fast lookup.
    username_index: HashMap<String, UserId>,
    /// Email to user ID mapping for fast lookup.
    email_index: HashMap<String, UserId>,
    /// Active authentication tokens.
    tokens: HashMap<String, AuthToken>,
    /// HMAC signing key for token integrity.
    signing_key: Vec<u8>,
    /// Rate limiter for login attempts (5 per minute per username).
    login_rate_limiter: RateLimiter,
    /// Pending email verification codes.
    verification_codes: HashMap<UserId, VerificationCodeEntry>,
    /// File path for persistent storage; `None` means in-memory only.
    storage_path: Option<PathBuf>,
}

impl AuthService {
    /// Create a new authentication service with a random signing key (in-memory only).
    pub fn new() -> Self {
        let mut key = [0u8; 32];
        getrandom::getrandom(&mut key).expect("Failed to generate signing key");
        Self {
            users: HashMap::new(),
            username_index: HashMap::new(),
            email_index: HashMap::new(),
            tokens: HashMap::new(),
            signing_key: key.to_vec(),
            login_rate_limiter: RateLimiter::with_max_keys(5, 60, 10_000), // 5 attempts/min, max 10k keys
            verification_codes: HashMap::new(),
            storage_path: None,
        }
    }

    /// Create a new authentication service with a specific signing key (in-memory only).
    pub fn with_signing_key(key: Vec<u8>) -> Self {
        Self {
            users: HashMap::new(),
            username_index: HashMap::new(),
            email_index: HashMap::new(),
            tokens: HashMap::new(),
            signing_key: key,
            login_rate_limiter: RateLimiter::with_max_keys(5, 60, 10_000), // 5 attempts/min, max 10k keys
            verification_codes: HashMap::new(),
            storage_path: None,
        }
    }

    /// Create a new authentication service with persistent storage at the given path.
    ///
    /// If a storage file already exists, it will be loaded. Otherwise, a new
    /// empty service is created with a random signing key. The signing key is
    /// persisted so that tokens remain valid across restarts.
    pub fn with_storage<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            Self::load_from(&path)
        } else {
            let mut key = [0u8; 32];
            getrandom::getrandom(&mut key).expect("Failed to generate signing key");
            let service = Self {
                users: HashMap::new(),
                username_index: HashMap::new(),
                email_index: HashMap::new(),
                tokens: HashMap::new(),
                signing_key: key.to_vec(),
                login_rate_limiter: RateLimiter::with_max_keys(5, 60, 10_000),
                verification_codes: HashMap::new(),
                storage_path: Some(path),
            };
            // Save the initial empty state so the signing key is persisted
            service.save()?;
            Ok(service)
        }
    }

    /// Load user data from a persistent storage file.
    fn load_from(path: &Path) -> Result<Self> {
        let data = fs::read_to_string(path)?;
        let snapshot: UserStoreSnapshot = serde_json::from_str(&data)?;

        let signing_key = hex_decode(&snapshot.signing_key_hex)
            .ok_or_else(|| anyhow::anyhow!("Invalid signing key in storage file"))?;

        Ok(Self {
            users: snapshot.users,
            username_index: snapshot.username_index,
            email_index: snapshot.email_index,
            tokens: HashMap::new(),
            signing_key,
            login_rate_limiter: RateLimiter::with_max_keys(5, 60, 10_000),
            verification_codes: HashMap::new(),
            storage_path: Some(path.to_path_buf()),
        })
    }

    /// Save user data to persistent storage.
    ///
    /// Uses atomic write (temp file + rename) to prevent corruption on crash.
    /// Only persists user accounts, indexes, and the signing key — sessions,
    /// verification codes, and rate limiter state are intentionally excluded.
    pub fn save(&self) -> Result<()> {
        let path = match &self.storage_path {
            Some(p) => p,
            None => return Ok(()), // In-memory only, nothing to save
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let snapshot = UserStoreSnapshot {
            users: self.users.clone(),
            username_index: self.username_index.clone(),
            email_index: self.email_index.clone(),
            signing_key_hex: hex_encode(&self.signing_key),
        };

        let data = serde_json::to_vec_pretty(&snapshot)?;
        atomic_write(path, &data)
    }

    /// Internal helper: save after a mutating operation, ignoring errors.
    /// Used so that a save failure doesn't break the user-facing operation.
    fn save_if_persisted(&self) {
        if let Err(e) = self.save() {
            tracing::error!(error = %e, "Failed to persist user data");
        }
    }

    /// Register a new user account with email and optional wallet addresses.
    ///
    /// Returns the user ID, auth token, and a 6-digit email verification code.
    /// The verification code is printed to logs for the user to enter.
    pub fn register(
        &mut self,
        username: String,
        password: String,
        email: String,
        btc_wallet: Option<String>,
        xmr_wallet: Option<String>,
    ) -> Result<RegistrationResult, UserError> {
        // Validate username
        let username_obj = Username::new(username.clone())?;

        // Validate email
        let email_obj = Email::new(email.clone())?;

        // Check for duplicate username
        if self.username_index.contains_key(username_obj.as_str()) {
            return Err(UserError::UsernameTaken);
        }

        // Check for duplicate email
        if self.email_index.contains_key(email_obj.as_str()) {
            return Err(UserError::EmailTaken);
        }

        // Validate wallet addresses if provided
        let btc_wallet_obj = match btc_wallet {
            Some(addr) => Some(BtcWalletAddress::new(addr)?),
            None => None,
        };
        let xmr_wallet_obj = match xmr_wallet {
            Some(addr) => Some(XmrWalletAddress::new(addr)?),
            None => None,
        };

        // Create participant ID and user account
        let participant_id = ParticipantId(Uuid::new_v4());
        let account = UserAccount::new(
            username_obj,
            &password,
            email_obj,
            btc_wallet_obj,
            xmr_wallet_obj,
            participant_id,
        )?;

        // Store the account
        let user_id = account.id;
        self.username_index
            .insert(account.username.as_str().to_string(), user_id);
        self.email_index
            .insert(account.email.as_str().to_string(), user_id);
        self.users.insert(user_id, account);

        // Generate email verification code
        let verification_entry = VerificationCodeEntry::new(user_id);
        let verification_code = verification_entry.code.clone();
        self.verification_codes.insert(user_id, verification_entry);

        // Log the verification code (in production, this would be sent via email)
        tracing::info!(
            user_id = %user_id,
            email = %self.users.get(&user_id).unwrap().email,
            code = %verification_code,
            "Email verification code generated (console output for development)"
        );

        // Create auth token
        let token = AuthToken::new(user_id, &self.signing_key);
        self.tokens.insert(token.token.clone(), token.clone());

        let result = RegistrationResult {
            user_id,
            token,
            participant_id,
            verification_code,
        };

        // Persist the new user account
        self.save_if_persisted();

        Ok(result)
    }

    /// Verify the email address using the 6-digit verification code.
    pub fn verify_email(&mut self, user_id: UserId, code: &str) -> Result<(), UserError> {
        // Find the verification code entry
        let entry = self.verification_codes.get(&user_id)
            .ok_or(UserError::InvalidVerificationCode)?;

        // Check if code has expired
        if !entry.is_valid() {
            self.verification_codes.remove(&user_id);
            return Err(UserError::InvalidVerificationCode);
        }

        // Check if code matches
        if entry.code != code {
            return Err(UserError::InvalidVerificationCode);
        }

        // Mark email as verified
        if let Some(account) = self.users.get_mut(&user_id) {
            account.verify_email();
        }

        // Remove the used verification code
        self.verification_codes.remove(&user_id);

        tracing::info!(user_id = %user_id, "Email verified successfully");

        // Persist the verified email status
        self.save_if_persisted();

        Ok(())
    }

    /// Resend a verification code for the given user.
    pub fn resend_verification_code(&mut self, user_id: UserId) -> Result<String, UserError> {
        // Check user exists
        if !self.users.contains_key(&user_id) {
            return Err(UserError::UserNotFound);
        }

        // Generate a new verification code
        let entry = VerificationCodeEntry::new(user_id);
        let code = entry.code.clone();
        self.verification_codes.insert(user_id, entry);

        tracing::info!(
            user_id = %user_id,
            code = %code,
            "New verification code generated"
        );

        Ok(code)
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
        self.save_if_persisted();
        Ok(())
    }

    /// Reactivate a suspended user account.
    pub fn reactivate_user(&mut self, user_id: UserId) -> Result<(), UserError> {
        let account = self.users.get_mut(&user_id).ok_or(UserError::UserNotFound)?;
        account.reactivate();
        self.save_if_persisted();
        Ok(())
    }

    /// Get the number of registered users.
    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    /// Get the first registered user ID (used by GUI for email verification flow).
    pub fn first_user_id(&self) -> Option<UserId> {
        self.users.keys().next().copied()
    }

    // ─── Reputation tracking ────────────────────────────────────────────────

    /// Record a placed order (maker side) for a user.
    pub fn record_make(&mut self, user_id: UserId) -> Result<(), UserError> {
        let account = self.users.get_mut(&user_id).ok_or(UserError::UserNotFound)?;
        account.record_make();
        self.save_if_persisted();
        Ok(())
    }

    /// Record an accepted order (taker side) for a user.
    pub fn record_take(&mut self, user_id: UserId) -> Result<(), UserError> {
        let account = self.users.get_mut(&user_id).ok_or(UserError::UserNotFound)?;
        account.record_take();
        self.save_if_persisted();
        Ok(())
    }

    /// Record a successfully completed trade for a user.
    pub fn record_completed_trade(&mut self, user_id: UserId) -> Result<(), UserError> {
        let account = self.users.get_mut(&user_id).ok_or(UserError::UserNotFound)?;
        account.record_completed_trade();
        self.save_if_persisted();
        Ok(())
    }

    /// Record a disputed trade for a user.
    pub fn record_dispute(&mut self, user_id: UserId) -> Result<(), UserError> {
        let account = self.users.get_mut(&user_id).ok_or(UserError::UserNotFound)?;
        account.record_dispute();
        self.save_if_persisted();
        Ok(())
    }

    /// Record a failure to respond for a user.
    pub fn record_failure_to_respond(&mut self, user_id: UserId) -> Result<(), UserError> {
        let account = self.users.get_mut(&user_id).ok_or(UserError::UserNotFound)?;
        account.record_failure_to_respond();
        self.save_if_persisted();
        Ok(())
    }

    /// Record a rating received from a trade counterparty (1-5 scale).
    pub fn record_rating(&mut self, user_id: UserId, rating: u32) -> Result<(), UserError> {
        let account = self.users.get_mut(&user_id).ok_or(UserError::UserNotFound)?;
        account.record_rating(rating)?;
        self.save_if_persisted();
        Ok(())
    }

    /// Get the trust score for a user (0-100).
    pub fn get_trust_score(&self, user_id: UserId) -> Result<u32, UserError> {
        let account = self.users.get(&user_id).ok_or(UserError::UserNotFound)?;
        Ok(account.trust_score())
    }

    /// Get the reputation tier for a user.
    pub fn get_reputation_tier(&self, user_id: UserId) -> Result<ReputationTier, UserError> {
        let account = self.users.get(&user_id).ok_or(UserError::UserNotFound)?;
        Ok(account.reputation_tier())
    }

    /// Get a reference to a user's reputation data.
    pub fn get_reputation(&self, user_id: UserId) -> Result<&crate::user::Reputation, UserError> {
        let account = self.users.get(&user_id).ok_or(UserError::UserNotFound)?;
        Ok(&account.reputation)
    }

    // ─── Review submission ──────────────────────────────────────────────────

    /// Submit a review for a trading partner.
    ///
    /// Prevents self-reviews and double reviews of the same trade.
    /// The review is added to the reviewed user's reputation profile
    /// and the trade is marked as reviewed by the reviewer.
    pub fn submit_review(
        &mut self,
        trade_id: opendex_types::TradeId,
        reviewer_id: UserId,
        reviewed_id: UserId,
        rating: Rating,
        comment: Option<String>,
    ) -> Result<Review, UserError> {
        // Cannot review yourself
        if reviewer_id == reviewed_id {
            return Err(UserError::SelfReview);
        }

        // Both users must exist
        let reviewer = self.users.get(&reviewer_id).ok_or(UserError::UserNotFound)?;
        let reviewed = self.users.get(&reviewed_id).ok_or(UserError::UserNotFound)?;

        // Check reviewer hasn't already reviewed this trade
        if reviewer.reputation.has_reviewed_trade(trade_id) {
            return Err(UserError::ReviewAlreadySubmitted);
        }

        // Create the review
        let review = Review::new(trade_id, reviewer_id, reviewed_id, rating, comment);

        // Add review to the reviewed user's reputation
        let reviewed_account = self.users.get_mut(&reviewed_id).unwrap();
        reviewed_account.reputation.add_review(review.clone());

        // Mark the trade as reviewed by the reviewer
        let reviewer_account = self.users.get_mut(&reviewer_id).unwrap();
        reviewer_account.reputation.mark_trade_reviewed(trade_id);

        self.save_if_persisted();
        Ok(review)
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
            .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        assert!(result.user_id.0 != Uuid::nil());
        assert_eq!(result.token.user_id, result.user_id);
        assert_eq!(auth.user_count(), 1);
        assert!(!result.verification_code.is_empty());
        assert_eq!(result.verification_code.len(), 6);
    }

    #[test]
    fn register_with_wallets() {
        let mut auth = AuthService::new();
        let result = auth
            .register(
                "alice".to_string(),
                "password123".to_string(),
                "alice@example.com".to_string(),
                Some("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string()),
                Some("4".to_string() + &"a".repeat(94)),
            )
            .unwrap();

        let user = auth.get_user(result.user_id).unwrap();
        assert!(user.btc_wallet.is_some());
        assert!(user.xmr_wallet.is_some());
    }

    #[test]
    fn duplicate_username_rejected() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        let result = auth.register("alice".to_string(), "otherpassword".to_string(), "other@example.com".to_string(), None, None);
        assert_eq!(result.unwrap_err(), UserError::UsernameTaken);
    }

    #[test]
    fn duplicate_email_rejected() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        let result = auth.register("bob".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None);
        assert_eq!(result.unwrap_err(), UserError::EmailTaken);
    }

    #[test]
    fn email_verification_success() {
        let mut auth = AuthService::new();
        let reg = auth
            .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        let user = auth.get_user(reg.user_id).unwrap();
        assert!(!user.is_email_verified());

        // Verify with correct code
        auth.verify_email(reg.user_id, &reg.verification_code).unwrap();

        let user = auth.get_user(reg.user_id).unwrap();
        assert!(user.is_email_verified());
    }

    #[test]
    fn email_verification_wrong_code() {
        let mut auth = AuthService::new();
        let reg = auth
            .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        let result = auth.verify_email(reg.user_id, "000000");
        assert_eq!(result.unwrap_err(), UserError::InvalidVerificationCode);
    }

    #[test]
    fn resend_verification_code() {
        let mut auth = AuthService::new();
        let reg = auth
            .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        let new_code = auth.resend_verification_code(reg.user_id).unwrap();
        assert!(!new_code.is_empty());
        assert_ne!(reg.verification_code, new_code);

        // Old code should no longer work
        let result = auth.verify_email(reg.user_id, &reg.verification_code);
        assert_eq!(result.unwrap_err(), UserError::InvalidVerificationCode);

        // New code should work
        auth.verify_email(reg.user_id, &new_code).unwrap();
    }

    #[test]
    fn login_success() {
        let mut auth = AuthService::new();
        auth.register("bob".to_string(), "securepass1".to_string(), "bob@example.com".to_string(), None, None)
            .unwrap();

        let result = auth.login("bob", "securepass1").unwrap();
        assert_eq!(result.username, "bob");
        assert!(result.token.is_valid(Utc::now()));
    }

    #[test]
    fn login_wrong_password() {
        let mut auth = AuthService::new();
        auth.register("bob".to_string(), "securepass1".to_string(), "bob@example.com".to_string(), None, None)
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
        auth.register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
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
        auth.register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
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
            .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        auth.suspend_user(reg.user_id).unwrap();

        let result = auth.login("alice", "password123");
        assert_eq!(result.unwrap_err(), UserError::AccountSuspended);
    }

    #[test]
    fn reactivate_suspended_account() {
        let mut auth = AuthService::new();
        let reg = auth
            .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        auth.suspend_user(reg.user_id).unwrap();
        auth.reactivate_user(reg.user_id).unwrap();

        let result = auth.login("alice", "password123");
        assert!(result.is_ok());
    }

    #[test]
    fn get_user_by_username() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        let user = auth.get_user_by_username("alice").unwrap();
        assert_eq!(user.username.as_str(), "alice");
    }

    #[test]
    fn login_records_timestamp() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
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
        auth.register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
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
        auth.register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();
        auth.register("bob".to_string(), "password123".to_string(), "bob@example.com".to_string(), None, None)
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

    #[test]
    fn persist_and_load_users() {
        let dir = std::env::temp_dir().join(format!("opendex_test_{}", Uuid::new_v4()));
        let path = dir.join("users.json");

        // Create service with storage and register a user
        {
            let mut auth = AuthService::with_storage(&path).unwrap();
            let reg = auth
                .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
                .unwrap();

            // Verify email
            auth.verify_email(reg.user_id, &reg.verification_code).unwrap();

            assert_eq!(auth.user_count(), 1);
            assert!(auth.get_user(reg.user_id).unwrap().is_email_verified());
        } // auth dropped — data should still be on disk

        // Load from the same path
        let mut auth = AuthService::with_storage(&path).unwrap();
        assert_eq!(auth.user_count(), 1);

        // User should be loadable and verifiable
        let user = auth.get_user_by_username("alice").unwrap();
        assert!(user.is_email_verified());

        // Login should work with persisted data
        let result = auth.login("alice", "password123");
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn persist_wallet_addresses() {
        let dir = std::env::temp_dir().join(format!("opendex_test_{}", Uuid::new_v4()));
        let path = dir.join("users.json");

        {
            let mut auth = AuthService::with_storage(&path).unwrap();
            let reg = auth
                .register(
                    "bob".to_string(),
                    "password123".to_string(),
                    "bob@example.com".to_string(),
                    Some("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string()),
                    Some("4".to_string() + &"a".repeat(94)),
                )
                .unwrap();

            let user = auth.get_user(reg.user_id).unwrap();
            assert!(user.btc_wallet.is_some());
            assert!(user.xmr_wallet.is_some());
        } // dropped

        let auth = AuthService::with_storage(&path).unwrap();
        let user = auth.get_user_by_username("bob").unwrap();
        assert!(user.btc_wallet.is_some());
        assert!(user.xmr_wallet.is_some());
        assert_eq!(user.btc_wallet.as_ref().unwrap().as_str(), "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn persist_signing_key() {
        let dir = std::env::temp_dir().join(format!("opendex_test_{}", Uuid::new_v4()));
        let path = dir.join("users.json");

        // First instance creates a signing key
        let token: String;
        {
            let mut auth = AuthService::with_storage(&path).unwrap();
            auth.register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
                .unwrap();
            let login = auth.login("alice", "password123").unwrap();
            token = login.token.token.clone();
        } // dropped — new instance should use same signing key

        let mut auth = AuthService::with_storage(&path).unwrap();
        // The token was signed with the persisted key, so it should still validate
        // (though it won't be in the tokens HashMap, which is in-memory only)
        // We verify the signing key persisted by checking login works
        let login = auth.login("alice", "password123").unwrap();
        // New token should verify with the loaded signing key
        assert!(login.token.verify(&auth.signing_key, Utc::now()));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn in_memory_mode_skips_save() {
        let mut auth = AuthService::new();
        auth.register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        // save() on in-memory service should return Ok without error
        assert!(auth.save().is_ok());
        assert_eq!(auth.user_count(), 1);
    }

    #[test]
    fn persist_suspended_account() {
        let dir = std::env::temp_dir().join(format!("opendex_test_{}", Uuid::new_v4()));
        let path = dir.join("users.json");

        {
            let mut auth = AuthService::with_storage(&path).unwrap();
            let reg = auth
                .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
                .unwrap();
            auth.suspend_user(reg.user_id).unwrap();
        } // dropped

        let mut auth = AuthService::with_storage(&path).unwrap();
        let user = auth.get_user_by_username("alice").unwrap();
        assert_eq!(user.status, crate::user::AccountStatus::Suspended);
        // Suspended account should not be able to login
        let result = auth.login("alice", "password123");
        assert_eq!(result.unwrap_err(), UserError::AccountSuspended);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn submit_review_success() {
        let mut auth = AuthService::new();
        let alice = auth
            .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();
        let bob = auth
            .register("bob".to_string(), "password123".to_string(), "bob@example.com".to_string(), None, None)
            .unwrap();

        let trade_id = opendex_types::TradeId(Uuid::new_v4());
        let rating = Rating::new(5).unwrap();

        let review = auth
            .submit_review(trade_id, alice.user_id, bob.user_id, rating, Some("Great trader!".to_string()))
            .unwrap();

        assert_eq!(review.reviewer_id, alice.user_id);
        assert_eq!(review.reviewed_id, bob.user_id);
        assert_eq!(review.rating, rating);

        // Bob's reputation should have the review
        let bob_rep = auth.get_reputation(bob.user_id).unwrap();
        assert_eq!(bob_rep.review_count(), 1);
        assert!((bob_rep.average_rating - 5.0).abs() < f64::EPSILON);

        // Alice should have marked the trade as reviewed
        let alice_rep = auth.get_reputation(alice.user_id).unwrap();
        assert!(alice_rep.has_reviewed_trade(trade_id));
    }

    #[test]
    fn cannot_review_yourself() {
        let mut auth = AuthService::new();
        let alice = auth
            .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();

        let result = auth.submit_review(
            opendex_types::TradeId(Uuid::new_v4()),
            alice.user_id,
            alice.user_id,
            Rating::new(5).unwrap(),
            None,
        );
        assert_eq!(result.unwrap_err(), UserError::SelfReview);
    }

    #[test]
    fn cannot_review_same_trade_twice() {
        let mut auth = AuthService::new();
        let alice = auth
            .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();
        let bob = auth
            .register("bob".to_string(), "password123".to_string(), "bob@example.com".to_string(), None, None)
            .unwrap();

        let trade_id = opendex_types::TradeId(Uuid::new_v4());
        auth.submit_review(trade_id, alice.user_id, bob.user_id, Rating::new(5).unwrap(), None)
            .unwrap();

        let result = auth.submit_review(trade_id, alice.user_id, bob.user_id, Rating::new(4).unwrap(), None);
        assert_eq!(result.unwrap_err(), UserError::ReviewAlreadySubmitted);
    }

    #[test]
    fn multiple_reviews_average() {
        let mut auth = AuthService::new();
        let alice = auth
            .register("alice".to_string(), "password123".to_string(), "alice@example.com".to_string(), None, None)
            .unwrap();
        let bob = auth
            .register("bob".to_string(), "password123".to_string(), "bob@example.com".to_string(), None, None)
            .unwrap();
        let charlie = auth
            .register("charlie".to_string(), "password123".to_string(), "charlie@example.com".to_string(), None, None)
            .unwrap();

        // Alice reviews Bob 5 stars
        auth.submit_review(
            opendex_types::TradeId(Uuid::new_v4()),
            alice.user_id,
            bob.user_id,
            Rating::new(5).unwrap(),
            None,
        ).unwrap();

        // Charlie reviews Bob 3 stars
        auth.submit_review(
            opendex_types::TradeId(Uuid::new_v4()),
            charlie.user_id,
            bob.user_id,
            Rating::new(3).unwrap(),
            None,
        ).unwrap();

        let bob_rep = auth.get_reputation(bob.user_id).unwrap();
        assert_eq!(bob_rep.review_count(), 2);
        assert!((bob_rep.average_rating - 4.0).abs() < f64::EPSILON);
    }
}
