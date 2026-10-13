//! User account data model.
//!
//! Defines the core user account structure including username, password hash,
//! account status, and associated metadata.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use opendex_types::ParticipantId;

use crate::error::UserError;

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

impl fmt::Display for AccountStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountStatus::Active => write!(f, "Active"),
            AccountStatus::Suspended => write!(f, "Suspended"),
        }
    }
}

/// Complete user account record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAccount {
    /// Unique user identifier.
    pub id: UserId,
    /// Human-readable username.
    pub username: Username,
    /// Hashed password.
    pub password_hash: PasswordHash,
    /// Associated P2P participant ID.
    pub participant_id: ParticipantId,
    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last login timestamp.
    pub last_login: Option<DateTime<Utc>>,
    /// Current account status.
    pub status: AccountStatus,
}

impl UserAccount {
    /// Create a new user account.
    pub fn new(
        username: Username,
        password: &str,
        participant_id: ParticipantId,
    ) -> Result<Self, UserError> {
        let password_hash = PasswordHash::from_password(password)?;
        let now = Utc::now();
        Ok(Self {
            id: UserId::new(),
            username,
            password_hash,
            participant_id,
            created_at: now,
            last_login: None,
            status: AccountStatus::Active,
        })
    }

    /// Check if the account is active.
    pub fn is_active(&self) -> bool {
        self.status == AccountStatus::Active
    }

    /// Verify the provided password.
    pub fn verify_password(&self, password: &str) -> bool {
        self.password_hash.verify(password)
    }

    /// Get the account age in days.
    pub fn age_days(&self, now: DateTime<Utc>) -> u64 {
        now.signed_duration_since(self.created_at)
            .num_days()
            .max(0) as u64
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
    fn create_user_account() {
        let username = Username::new("alice".to_string()).unwrap();
        let participant = ParticipantId(Uuid::new_v4());
        let account = UserAccount::new(username, "password123", participant).unwrap();

        assert!(account.is_active());
        assert_eq!(account.status, AccountStatus::Active);
        assert!(account.last_login.is_none());
        assert_eq!(account.participant_id, participant);
    }

    #[test]
    fn account_suspension() {
        let username = Username::new("alice".to_string()).unwrap();
        let mut account = UserAccount::new(username, "password123", ParticipantId(Uuid::new_v4())).unwrap();

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
        let mut account = UserAccount::new(username, "password123", ParticipantId(Uuid::new_v4())).unwrap();

        // Fresh account — 0 days old
        assert_eq!(account.age_days(Utc::now()), 0);

        // Simulate 30 days old
        account.created_at = Utc::now() - chrono::Duration::days(30);
        assert_eq!(account.age_days(Utc::now()), 30);
    }
}
