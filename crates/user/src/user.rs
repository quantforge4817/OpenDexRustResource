//! User account data model.
//!
//! Defines the core user account structure including username, password hash,
//! account status, and associated metadata.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use opendex_types::{ParticipantId, TradeId};

use crate::error::UserError;

/// Star rating for a review (1-5 stars).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rating(u32);

impl Rating {
    /// Minimum rating.
    pub const MIN: u32 = 1;

    /// Maximum rating.
    pub const MAX: u32 = 5;

    /// Create a new rating.
    pub fn new(value: u32) -> Result<Self, UserError> {
        if value < Self::MIN || value > Self::MAX {
            return Err(UserError::InvalidRating);
        }
        Ok(Self(value))
    }

    /// Get the rating value.
    pub fn value(&self) -> u32 {
        self.0
    }

    /// Check if this is a positive rating (4-5 stars).
    pub fn is_positive(&self) -> bool {
        self.0 >= 4
    }

    /// Check if this is a negative rating (1-2 stars).
    pub fn is_negative(&self) -> bool {
        self.0 <= 2
    }
}

impl fmt::Display for Rating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}★", self.0)
    }
}

/// Unique identifier for a user account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl UserId {
    /// Generate a new random user ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Validated email address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    /// Maximum email length.
    pub const MAX_LENGTH: usize = 254;

    /// Create a new email, validating the format.
    pub fn new(address: String) -> Result<Self, UserError> {
        if address.is_empty() || address.len() > Self::MAX_LENGTH {
            return Err(UserError::InvalidEmail);
        }
        // Basic email validation: must contain exactly one @ with text on both sides
        let parts: Vec<&str> = address.split('@').collect();
        if parts.len() != 2 {
            return Err(UserError::InvalidEmail);
        }
        let local = parts[0];
        let domain = parts[1];
        if local.is_empty() || domain.is_empty() {
            return Err(UserError::InvalidEmail);
        }
        // Domain must contain at least one dot
        if !domain.contains('.') {
            return Err(UserError::InvalidEmail);
        }
        Ok(Self(address))
    }

    /// Get the email as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Human-readable username chosen by the user.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Username(String);

impl Username {
    /// Minimum username length.
    pub const MIN_LENGTH: usize = 3;

    /// Maximum username length.
    pub const MAX_LENGTH: usize = 32;

    /// Create a new username, validating the input.
    pub fn new(name: String) -> Result<Self, UserError> {
        if name.len() < Self::MIN_LENGTH {
            return Err(UserError::UsernameTooShort);
        }
        if name.len() > Self::MAX_LENGTH {
            return Err(UserError::UsernameTooLong);
        }
        // Allow alphanumeric, underscores, and hyphens
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(UserError::InvalidUsername);
        }
        Ok(Self(name))
    }

    /// Get the username as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Hashed password for secure storage using Argon2id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordHash(String);

impl PasswordHash {
    /// Minimum password length.
    pub const MIN_LENGTH: usize = 8;

    /// Maximum password length.
    pub const MAX_LENGTH: usize = 128;

    /// Hash a plaintext password using Argon2id with a random salt.
    pub fn from_password(password: &str) -> Result<Self, UserError> {
        use argon2::PasswordHasher;
        if password.len() < Self::MIN_LENGTH {
            return Err(UserError::PasswordTooShort);
        }
        if password.len() > Self::MAX_LENGTH {
            return Err(UserError::PasswordTooLong);
        }
        let argon2 = argon2::Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::default(),
        );
        // SaltString generates a cryptographically random salt
        let salt = password_hash::SaltString::generate(&mut rand_core::OsRng);
        let hash = argon2
            .hash_password(password.as_bytes(), password_hash::Salt::from(&salt))
            .map_err(|e| UserError::PasswordHashError(e.to_string()))?;
        Ok(Self(hash.to_string()))
    }

    /// Verify a plaintext password against the stored Argon2 hash.
    pub fn verify(&self, password: &str) -> bool {
        use argon2::PasswordVerifier;
        let argon2 = argon2::Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::default(),
        );
        if let Ok(parsed_hash) = argon2::PasswordHash::new(&self.0) {
            argon2
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok()
        } else {
            false
        }
    }
}

/// Status of a user account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    /// Active and able to trade.
    Active,
    /// Suspended — cannot create or accept trades.
    Suspended,
}

/// Email verification status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmailVerificationStatus {
    /// Email has not been verified yet.
    Unverified,
    /// Email has been verified successfully.
    Verified,
}

/// Bitcoin wallet address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtcWalletAddress(String);

impl BtcWalletAddress {
    /// Create a new Bitcoin wallet address with basic validation.
    pub fn new(address: String) -> Result<Self, UserError> {
        if address.is_empty() || address.len() < 26 {
            return Err(UserError::InvalidBtcWalletAddress);
        }
        // Bitcoin addresses start with 1, 3, or bc1
        if !(address.starts_with('1') || address.starts_with('3') || address.starts_with("bc1")) {
            return Err(UserError::InvalidBtcWalletAddress);
        }
        Ok(Self(address))
    }

    /// Get the address as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BtcWalletAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Monero wallet address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XmrWalletAddress(String);

impl XmrWalletAddress {
    /// Create a new Monero wallet address with basic validation.
    pub fn new(address: String) -> Result<Self, UserError> {
        if address.is_empty() || address.len() != 95 {
            return Err(UserError::InvalidXmrWalletAddress);
        }
        // Monero addresses start with 4 (mainnet) or 5 (stagenet/testnet)
        if !(address.starts_with('4')
            || address.starts_with('5')
            || address.starts_with('7')
            || address.starts_with('8'))
        {
            return Err(UserError::InvalidXmrWalletAddress);
        }
        Ok(Self(address))
    }

    /// Get the address as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for XmrWalletAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for AccountStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountStatus::Active => write!(f, "Active"),
            AccountStatus::Suspended => write!(f, "Suspended"),
        }
    }
}

/// Reputation tier based on activity and trust score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReputationTier {
    /// New user with no trade history.
    Newcomer,
    /// Completed at least 1 trade.
    Trader,
    /// Completed at least 10 trades with good standing.
    Trusted,
    /// Completed at least 50 trades with good standing.
    Verified,
    /// Completed at least 200 trades with excellent standing.
    Elite,
}

impl fmt::Display for ReputationTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReputationTier::Newcomer => write!(f, "Newcomer"),
            ReputationTier::Trader => write!(f, "Trader"),
            ReputationTier::Trusted => write!(f, "Trusted"),
            ReputationTier::Verified => write!(f, "Verified"),
            ReputationTier::Elite => write!(f, "Elite"),
        }
    }
}

/// A written review left by one user about another after a trade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Review {
    /// Unique review identifier.
    pub id: Uuid,
    /// Trade this review is for.
    pub trade_id: TradeId,
    /// User who wrote the review.
    pub reviewer_id: UserId,
    /// User being reviewed.
    pub reviewed_id: UserId,
    /// Star rating (1-5).
    pub rating: Rating,
    /// Optional text comment.
    pub comment: Option<String>,
    /// Review creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl Review {
    /// Create a new review.
    pub fn new(
        trade_id: TradeId,
        reviewer_id: UserId,
        reviewed_id: UserId,
        rating: Rating,
        comment: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            trade_id,
            reviewer_id,
            reviewed_id,
            rating,
            comment,
            created_at: Utc::now(),
        }
    }
}

/// Tracks a user's on-protocol activity and reputation.
///
/// Reputation is built from actual trade outcomes: completed trades increase
/// your score, while disputes and failures to respond decrease it.
/// Written reviews from trading partners provide qualitative context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reputation {
    /// Number of orders placed (maker side).
    pub makes: u32,
    /// Number of orders accepted (taker side).
    pub takes: u32,
    /// Number of trades successfully completed (settled).
    pub completed_trades: u32,
    /// Number of trades that ended in dispute.
    pub disputes: u32,
    /// Number of trades cancelled or expired by this user (failures to respond).
    pub failures_to_respond: u32,
    /// Average rating received from trade counterparts (1-5 scale, 0 if unrated).
    pub average_rating: f64,
    /// Number of ratings received.
    pub rating_count: u32,
    /// Written reviews from trading partners.
    pub reviews: Vec<Review>,
    /// Trades that have already been reviewed by this user (prevents double reviews).
    pub reviewed_trades: Vec<TradeId>,
    /// Last updated timestamp.
    pub last_updated: DateTime<Utc>,
}

impl Reputation {
    /// Create a new empty reputation record.
    pub fn new() -> Self {
        Self {
            makes: 0,
            takes: 0,
            completed_trades: 0,
            disputes: 0,
            failures_to_respond: 0,
            average_rating: 0.0,
            rating_count: 0,
            reviews: Vec::new(),
            reviewed_trades: Vec::new(),
            last_updated: Utc::now(),
        }
    }

    /// Record a placed order (maker side).
    pub fn record_make(&mut self) {
        self.makes += 1;
        self.last_updated = Utc::now();
    }

    /// Record an accepted order (taker side).
    pub fn record_take(&mut self) {
        self.takes += 1;
        self.last_updated = Utc::now();
    }

    /// Record a successfully completed trade.
    pub fn record_completed_trade(&mut self) {
        self.completed_trades += 1;
        self.last_updated = Utc::now();
    }

    /// Record a disputed trade.
    pub fn record_dispute(&mut self) {
        self.disputes += 1;
        self.last_updated = Utc::now();
    }

    /// Record a failure to respond (cancelled or expired trade).
    pub fn record_failure_to_respond(&mut self) {
        self.failures_to_respond += 1;
        self.last_updated = Utc::now();
    }

    /// Record a rating received from a trade counterparty (1-5 scale).
    /// Returns an error if the rating is out of range.
    pub fn record_rating(&mut self, rating: u32) -> Result<(), UserError> {
        if rating < 1 || rating > 5 {
            return Err(UserError::InvalidRating);
        }
        // Update running average incrementally
        let new_rating = rating as f64;
        let current_count = self.rating_count as f64;
        self.average_rating =
            (self.average_rating * current_count + new_rating) / (current_count + 1.0);
        self.rating_count += 1;
        self.last_updated = Utc::now();
        Ok(())
    }

    /// Calculate the overall trust score (0-100).
    ///
    /// Score is based on:
    /// - Completed trades (positive, up to 40 points)
    /// - Average rating (positive, up to 30 points)
    /// - Disputes (negative, -10 per dispute)
    /// - Failures to respond (negative, -5 per failure)
    /// - Account age bonus (up to 10 points for 365+ days)
    /// - Response rate bonus (up to 20 points)
    pub fn trust_score(&self, account_age_days: u64) -> u32 {
        let mut score: f64 = 0.0;

        // Completed trades: up to 40 points (1 point per trade, capped at 40)
        score += (self.completed_trades as f64).min(40.0);

        // Average rating: up to 30 points (6 points per rating point)
        if self.rating_count > 0 {
            score += self.average_rating * 6.0;
        }

        // Disputes: -10 per dispute
        score -= (self.disputes as f64) * 10.0;

        // Failures to respond: -5 per failure
        score -= (self.failures_to_respond as f64) * 5.0;

        // Account age bonus: up to 10 points for 365+ days
        let age_days = account_age_days.min(365);
        score += (age_days as f64 / 365.0) * 10.0;

        // Response rate bonus: up to 20 points
        let total_actions = self.completed_trades + self.disputes + self.failures_to_respond;
        if total_actions > 0 {
            let response_rate = self.completed_trades as f64 / total_actions as f64;
            score += response_rate * 20.0;
        }

        // Clamp to 0-100
        score.max(0.0).min(100.0) as u32
    }

    /// Determine the reputation tier based on completed trades and trust score.
    pub fn tier(&self, account_age_days: u64) -> ReputationTier {
        let score = self.trust_score(account_age_days);
        let completed = self.completed_trades;

        // Must have a minimum trust score of 30 to qualify for any tier above Newcomer
        if score < 30 {
            return ReputationTier::Newcomer;
        }

        match completed {
            0 => ReputationTier::Newcomer,
            1..=9 => ReputationTier::Trader,
            10..=49 => ReputationTier::Trusted,
            50..=199 => ReputationTier::Verified,
            _ => ReputationTier::Elite,
        }
    }

    /// Total number of trades (completed + disputes + failures).
    pub fn total_trades(&self) -> u32 {
        self.completed_trades + self.disputes + self.failures_to_respond
    }

    /// Response rate as a percentage (0-100).
    pub fn response_rate(&self) -> f64 {
        let total = self.total_trades();
        if total == 0 {
            return 100.0; // New users start at 100%
        }
        (self.completed_trades as f64 / total as f64) * 100.0
    }

    /// Check if this user has already reviewed a specific trade.
    pub fn has_reviewed_trade(&self, trade_id: TradeId) -> bool {
        self.reviewed_trades.contains(&trade_id)
    }

    /// Add a review received from a trading partner.
    pub fn add_review(&mut self, review: Review) {
        self.reviews.push(review);
        // Recalculate average rating from all reviews
        if !self.reviews.is_empty() {
            let avg: f64 = self
                .reviews
                .iter()
                .map(|r| r.rating.value() as f64)
                .sum::<f64>()
                / self.reviews.len() as f64;
            self.average_rating = avg;
            self.rating_count = self.reviews.len() as u32;
        }
        self.last_updated = Utc::now();
    }

    /// Mark a trade as reviewed by this user.
    pub fn mark_trade_reviewed(&mut self, trade_id: TradeId) {
        if !self.reviewed_trades.contains(&trade_id) {
            self.reviewed_trades.push(trade_id);
        }
    }

    /// Get the average rating as a display string.
    pub fn average_rating_str(&self) -> String {
        if self.reviews.is_empty() {
            "No reviews".to_string()
        } else {
            format!("{:.1}/5.0", self.average_rating)
        }
    }

    /// Get the number of written reviews.
    pub fn review_count(&self) -> usize {
        self.reviews.len()
    }

    /// Get recent reviews (most recent first, limited to `limit`).
    pub fn recent_reviews(&self, limit: usize) -> impl Iterator<Item = &Review> {
        self.reviews.iter().rev().take(limit)
    }
}

impl Default for Reputation {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete user account record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserAccount {
    /// Unique user identifier.
    pub id: UserId,
    /// Human-readable username.
    pub username: Username,
    /// Hashed password.
    pub password_hash: PasswordHash,
    /// Email address for verification and recovery.
    pub email: Email,
    /// Email verification status.
    pub email_verified: EmailVerificationStatus,
    /// Bitcoin wallet address for receiving BTC.
    pub btc_wallet: Option<BtcWalletAddress>,
    /// Monero wallet address for receiving XMR.
    pub xmr_wallet: Option<XmrWalletAddress>,
    /// Associated P2P participant ID.
    pub participant_id: ParticipantId,
    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last login timestamp.
    pub last_login: Option<DateTime<Utc>>,
    /// Current account status.
    pub status: AccountStatus,
    /// On-protocol activity and reputation tracking.
    pub reputation: Reputation,
}

impl UserAccount {
    /// Create a new user account with email and optional wallet addresses.
    pub fn new(
        username: Username,
        password: &str,
        email: Email,
        btc_wallet: Option<BtcWalletAddress>,
        xmr_wallet: Option<XmrWalletAddress>,
        participant_id: ParticipantId,
    ) -> Result<Self, UserError> {
        let password_hash = PasswordHash::from_password(password)?;
        let now = Utc::now();
        Ok(Self {
            id: UserId::new(),
            username,
            password_hash,
            email,
            email_verified: EmailVerificationStatus::Unverified,
            btc_wallet,
            xmr_wallet,
            participant_id,
            created_at: now,
            last_login: None,
            status: AccountStatus::Active,
            reputation: Reputation::new(),
        })
    }

    /// Check if the account is active.
    pub fn is_active(&self) -> bool {
        self.status == AccountStatus::Active
    }

    /// Check if the email has been verified.
    pub fn is_email_verified(&self) -> bool {
        self.email_verified == EmailVerificationStatus::Verified
    }

    /// Verify the email address.
    pub fn verify_email(&mut self) {
        self.email_verified = EmailVerificationStatus::Verified;
    }

    /// Verify the provided password.
    pub fn verify_password(&self, password: &str) -> bool {
        self.password_hash.verify(password)
    }

    /// Get the account age in days.
    pub fn age_days(&self, now: DateTime<Utc>) -> u64 {
        now.signed_duration_since(self.created_at).num_days().max(0) as u64
    }

    /// Set the Bitcoin wallet address.
    pub fn set_btc_wallet(&mut self, address: BtcWalletAddress) {
        self.btc_wallet = Some(address);
    }

    /// Set the Monero wallet address.
    pub fn set_xmr_wallet(&mut self, address: XmrWalletAddress) {
        self.xmr_wallet = Some(address);
    }

    /// Suspend the account.
    pub fn suspend(&mut self) {
        self.status = AccountStatus::Suspended;
    }

    /// Reactivate the account.
    pub fn reactivate(&mut self) {
        self.status = AccountStatus::Active;
    }

    /// Update the last login timestamp.
    pub fn record_login(&mut self) {
        self.last_login = Some(Utc::now());
    }

    /// Get the current reputation trust score (0-100).
    pub fn trust_score(&self) -> u32 {
        self.reputation.trust_score(self.age_days(Utc::now()))
    }

    /// Get the current reputation tier.
    pub fn reputation_tier(&self) -> ReputationTier {
        self.reputation.tier(self.age_days(Utc::now()))
    }

    /// Record a placed order (maker side) in reputation.
    pub fn record_make(&mut self) {
        self.reputation.record_make();
    }

    /// Record an accepted order (taker side) in reputation.
    pub fn record_take(&mut self) {
        self.reputation.record_take();
    }

    /// Record a successfully completed trade in reputation.
    pub fn record_completed_trade(&mut self) {
        self.reputation.record_completed_trade();
    }

    /// Record a disputed trade in reputation.
    pub fn record_dispute(&mut self) {
        self.reputation.record_dispute();
    }

    /// Record a failure to respond in reputation.
    pub fn record_failure_to_respond(&mut self) {
        self.reputation.record_failure_to_respond();
    }

    /// Record a rating received from a trade counterparty (1-5 scale).
    pub fn record_rating(&mut self, rating: u32) -> Result<(), UserError> {
        self.reputation.record_rating(rating)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_validation() {
        // Valid username
        assert!(Username::new("alice".to_string()).is_ok());
        assert!(Username::new("bob-123".to_string()).is_ok());
        assert!(Username::new("charlie_456".to_string()).is_ok());

        // Too short
        assert_eq!(
            Username::new("ab".to_string()).unwrap_err(),
            UserError::UsernameTooShort
        );

        // Too long
        assert_eq!(
            Username::new("a".repeat(33).to_string()).unwrap_err(),
            UserError::UsernameTooLong
        );

        // Invalid characters
        assert_eq!(
            Username::new("user@name".to_string()).unwrap_err(),
            UserError::InvalidUsername
        );
    }

    #[test]
    fn password_hashing() {
        let password = "mysecurepassword123";
        let hash = PasswordHash::from_password(password).unwrap();
        assert!(hash.verify(password));
        assert!(!hash.verify("wrongpassword"));
    }

    #[test]
    fn password_validation() {
        // Too short
        assert_eq!(
            PasswordHash::from_password("short").unwrap_err(),
            UserError::PasswordTooShort
        );

        // Too long
        assert_eq!(
            PasswordHash::from_password(&"a".repeat(129)).unwrap_err(),
            UserError::PasswordTooLong
        );
    }

    #[test]
    fn email_validation() {
        // Valid emails
        assert!(Email::new("alice@example.com".to_string()).is_ok());
        assert!(Email::new("bob.test@domain.org".to_string()).is_ok());

        // Invalid emails
        assert_eq!(
            Email::new("".to_string()).unwrap_err(),
            UserError::InvalidEmail
        );
        assert_eq!(
            Email::new("noatsign".to_string()).unwrap_err(),
            UserError::InvalidEmail
        );
        assert_eq!(
            Email::new("@nodomain.com".to_string()).unwrap_err(),
            UserError::InvalidEmail
        );
        assert_eq!(
            Email::new("user@".to_string()).unwrap_err(),
            UserError::InvalidEmail
        );
        assert_eq!(
            Email::new("user@nodot".to_string()).unwrap_err(),
            UserError::InvalidEmail
        );
    }

    #[test]
    fn btc_wallet_address_validation() {
        // Valid Bitcoin addresses
        assert!(BtcWalletAddress::new("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string()).is_ok());
        assert!(
            BtcWalletAddress::new("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_string()).is_ok()
        );
        assert!(BtcWalletAddress::new("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy".to_string()).is_ok());

        // Invalid Bitcoin addresses
        assert_eq!(
            BtcWalletAddress::new("".to_string()).unwrap_err(),
            UserError::InvalidBtcWalletAddress
        );
        assert_eq!(
            BtcWalletAddress::new("short".to_string()).unwrap_err(),
            UserError::InvalidBtcWalletAddress
        );
        assert_eq!(
            BtcWalletAddress::new("4InvalidAddress12345678901234567890".to_string()).unwrap_err(),
            UserError::InvalidBtcWalletAddress
        );
    }

    #[test]
    fn xmr_wallet_address_validation() {
        // Valid Monero address (95 chars, starts with 4)
        let valid_xmr = "4".to_string() + &"a".repeat(94);
        assert!(XmrWalletAddress::new(valid_xmr.clone()).is_ok());

        // Invalid Monero addresses
        assert_eq!(
            XmrWalletAddress::new("".to_string()).unwrap_err(),
            UserError::InvalidXmrWalletAddress
        );
        assert_eq!(
            XmrWalletAddress::new("short".to_string()).unwrap_err(),
            UserError::InvalidXmrWalletAddress
        );
        // Wrong length
        let wrong_len = "4".to_string() + &"a".repeat(93);
        assert_eq!(
            XmrWalletAddress::new(wrong_len).unwrap_err(),
            UserError::InvalidXmrWalletAddress
        );
    }

    #[test]
    fn create_user_account() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let participant = ParticipantId(Uuid::new_v4());
        let account =
            UserAccount::new(username, "password123", email, None, None, participant).unwrap();

        assert!(account.is_active());
        assert_eq!(account.status, AccountStatus::Active);
        assert!(account.last_login.is_none());
        assert_eq!(account.participant_id, participant);
        assert!(!account.is_email_verified());
        assert!(account.btc_wallet.is_none());
        assert!(account.xmr_wallet.is_none());
    }

    #[test]
    fn create_user_account_with_wallets() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let btc = BtcWalletAddress::new("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string()).unwrap();
        let xmr = XmrWalletAddress::new("4".to_string() + &"a".repeat(94));
        let participant = ParticipantId(Uuid::new_v4());
        let account = UserAccount::new(
            username,
            "password123",
            email,
            Some(btc),
            xmr.ok(),
            participant,
        )
        .unwrap();

        assert!(account.btc_wallet.is_some());
        assert!(account.xmr_wallet.is_some());
    }

    #[test]
    fn email_verification() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        assert!(!account.is_email_verified());
        account.verify_email();
        assert!(account.is_email_verified());
    }

    #[test]
    fn account_suspension() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        assert!(account.is_active());
        account.suspend();
        assert!(!account.is_active());
        assert_eq!(account.status, AccountStatus::Suspended);
        account.reactivate();
        assert!(account.is_active());
    }

    #[test]
    fn account_age_calculation() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        // Fresh account — 0 days old
        assert_eq!(account.age_days(Utc::now()), 0);

        // Simulate 30 days old
        account.created_at = Utc::now() - chrono::Duration::days(30);
        assert_eq!(account.age_days(Utc::now()), 30);
    }

    #[test]
    fn reputation_starts_empty() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        assert_eq!(account.reputation.makes, 0);
        assert_eq!(account.reputation.takes, 0);
        assert_eq!(account.reputation.completed_trades, 0);
        assert_eq!(account.reputation.disputes, 0);
        assert_eq!(account.reputation.failures_to_respond, 0);
        assert_eq!(account.reputation.average_rating, 0.0);
        assert_eq!(account.reputation.rating_count, 0);
        assert!(account.reputation.reviews.is_empty());
        assert!(account.reputation.reviewed_trades.is_empty());
    }

    #[test]
    fn reputation_records_activity() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        account.record_make();
        account.record_make();
        account.record_take();
        account.record_completed_trade();
        account.record_completed_trade();
        account.record_completed_trade();
        account.record_dispute();
        account.record_failure_to_respond();

        assert_eq!(account.reputation.makes, 2);
        assert_eq!(account.reputation.takes, 1);
        assert_eq!(account.reputation.completed_trades, 3);
        assert_eq!(account.reputation.disputes, 1);
        assert_eq!(account.reputation.failures_to_respond, 1);
        assert_eq!(account.reputation.total_trades(), 5);
    }

    #[test]
    fn reputation_rating_average() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        account.record_rating(5).unwrap();
        assert!((account.reputation.average_rating - 5.0).abs() < f64::EPSILON);

        account.record_rating(3).unwrap();
        assert!((account.reputation.average_rating - 4.0).abs() < f64::EPSILON);

        account.record_rating(4).unwrap();
        assert!((account.reputation.average_rating - 4.0).abs() < 0.01);

        assert_eq!(account.reputation.rating_count, 3);
    }

    #[test]
    fn reputation_rating_validation() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        assert_eq!(
            account.record_rating(0).unwrap_err(),
            UserError::InvalidRating
        );
        assert_eq!(
            account.record_rating(6).unwrap_err(),
            UserError::InvalidRating
        );
        assert!(account.record_rating(1).is_ok());
        assert!(account.record_rating(5).is_ok());
    }

    #[test]
    fn reputation_trust_score_increases_with_trades() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        // Fresh account has low score (no trades, no ratings)
        let initial_score = account.trust_score();
        assert!(initial_score < 10);

        // Complete 10 trades with good ratings
        for _ in 0..10 {
            account.record_completed_trade();
        }
        account.record_rating(5).unwrap();
        account.record_rating(4).unwrap();

        let improved_score = account.trust_score();
        assert!(
            improved_score > initial_score,
            "Score should increase: {} > {}",
            improved_score,
            initial_score
        );
    }

    #[test]
    fn reputation_disputes_lower_score() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        // 10 completed trades
        for _ in 0..10 {
            account.record_completed_trade();
        }
        let good_score = account.trust_score();

        // Add 3 disputes
        account.record_dispute();
        account.record_dispute();
        account.record_dispute();
        let bad_score = account.trust_score();

        assert!(
            bad_score < good_score,
            "Disputes should lower score: {} < {}",
            bad_score,
            good_score
        );
    }

    #[test]
    fn reputation_tier_progression() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        // Newcomer: no trades
        assert_eq!(account.reputation_tier(), ReputationTier::Newcomer);

        // Trader: 1+ completed trades (need trust score >= 30)
        for _ in 0..5 {
            account.record_completed_trade();
        }
        account.record_rating(5).unwrap();
        // With 5 trades and a 5-star rating, score should be >= 30
        let tier = account.reputation_tier();
        assert!(
            tier == ReputationTier::Trader || tier == ReputationTier::Trusted,
            "Expected Trader or Trusted, got {:?}",
            tier
        );
    }

    #[test]
    fn reputation_response_rate() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        // New user starts at 100%
        assert_eq!(account.reputation.response_rate(), 100.0);

        // 9 completed, 1 dispute = 90% response rate
        for _ in 0..9 {
            account.record_completed_trade();
        }
        account.record_dispute();
        assert!((account.reputation.response_rate() - 90.0).abs() < 0.01);
    }

    #[test]
    fn reputation_tier_display() {
        assert_eq!(format!("{}", ReputationTier::Newcomer), "Newcomer");
        assert_eq!(format!("{}", ReputationTier::Trader), "Trader");
        assert_eq!(format!("{}", ReputationTier::Trusted), "Trusted");
        assert_eq!(format!("{}", ReputationTier::Verified), "Verified");
        assert_eq!(format!("{}", ReputationTier::Elite), "Elite");
    }

    #[test]
    fn reputation_persists_with_account() {
        let username = Username::new("alice".to_string()).unwrap();
        let email = Email::new("alice@example.com".to_string()).unwrap();
        let mut account = UserAccount::new(
            username,
            "password123",
            email,
            None,
            None,
            ParticipantId(Uuid::new_v4()),
        )
        .unwrap();

        account.record_completed_trade();
        account.record_completed_trade();
        account.record_rating(4).unwrap();

        assert_eq!(account.reputation.completed_trades, 2);
        assert!((account.reputation.average_rating - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn review_adds_to_reputation() {
        let alice = UserId::new();
        let bob = UserId::new();
        let trade_id = opendex_types::TradeId(Uuid::new_v4());
        let rating = Rating::new(5).unwrap();

        let mut rep = Reputation::new();
        assert!(rep.reviews.is_empty());
        assert_eq!(rep.review_count(), 0);

        let review = Review::new(
            trade_id,
            alice,
            bob,
            rating,
            Some("Great trader!".to_string()),
        );
        rep.add_review(review);

        assert_eq!(rep.review_count(), 1);
        assert!((rep.average_rating - 5.0).abs() < f64::EPSILON);
        assert_eq!(rep.rating_count, 1);
    }

    #[test]
    fn review_average_calculation() {
        let alice = UserId::new();
        let bob = UserId::new();
        let mut rep = Reputation::new();

        rep.add_review(Review::new(
            opendex_types::TradeId(Uuid::new_v4()),
            alice,
            bob,
            Rating::new(5).unwrap(),
            None,
        ));
        rep.add_review(Review::new(
            opendex_types::TradeId(Uuid::new_v4()),
            alice,
            bob,
            Rating::new(3).unwrap(),
            None,
        ));
        rep.add_review(Review::new(
            opendex_types::TradeId(Uuid::new_v4()),
            alice,
            bob,
            Rating::new(4).unwrap(),
            None,
        ));

        assert_eq!(rep.review_count(), 3);
        assert!((rep.average_rating - 4.0).abs() < 0.01);
    }

    #[test]
    fn review_prevents_double_review() {
        let trade_id = opendex_types::TradeId(Uuid::new_v4());
        let mut rep = Reputation::new();

        assert!(!rep.has_reviewed_trade(trade_id));
        rep.mark_trade_reviewed(trade_id);
        assert!(rep.has_reviewed_trade(trade_id));
        // Marking again should not duplicate
        rep.mark_trade_reviewed(trade_id);
        assert_eq!(rep.reviewed_trades.len(), 1);
    }

    #[test]
    fn rating_display() {
        let rating = Rating::new(5).unwrap();
        assert_eq!(format!("{}", rating), "5★");
    }

    #[test]
    fn rating_positive_negative() {
        assert!(Rating::new(5).unwrap().is_positive());
        assert!(Rating::new(4).unwrap().is_positive());
        assert!(Rating::new(2).unwrap().is_negative());
        assert!(Rating::new(1).unwrap().is_negative());
    }

    #[test]
    fn average_rating_string() {
        let rep = Reputation::new();
        assert_eq!(rep.average_rating_str(), "No reviews");

        let alice = UserId::new();
        let bob = UserId::new();
        let mut rep = Reputation::new();
        rep.add_review(Review::new(
            opendex_types::TradeId(Uuid::new_v4()),
            alice,
            bob,
            Rating::new(4).unwrap(),
            None,
        ));
        assert_eq!(rep.average_rating_str(), "4.0/5.0");
    }
}
