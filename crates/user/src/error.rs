//! Error types for the user accounts system.

use std::fmt;

/// Errors that can occur when creating or managing user accounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserError {
    /// Username already taken.
    UsernameTaken,
    /// Username too short (minimum 3 characters).
    UsernameTooShort,
    /// Username too long (maximum 32 characters).
    UsernameTooLong,
    /// Username contains invalid characters.
    InvalidUsername,
    /// Password too short (minimum 8 characters).
    PasswordTooShort,
    /// Password too long (maximum 128 characters).
    PasswordTooLong,
    /// User not found.
    UserNotFound,
    /// Invalid username or password.
    InvalidCredentials,
    /// Account is suspended.
    AccountSuspended,
    /// Account already exists.
    AccountExists,
    /// Token expired or invalid.
    InvalidToken,
    /// Review already submitted for this trade.
    ReviewAlreadySubmitted,
    /// Cannot review yourself.
    SelfReview,
    /// Invalid rating value.
    InvalidRating,
    /// Password hashing failed.
    PasswordHashError(String),
    /// Rate limit exceeded.
    RateLimited,
    /// Email already registered.
    EmailTaken,
    /// Email format is invalid.
    InvalidEmail,
    /// Email not verified.
    EmailNotVerified,
    /// Email verification code is invalid or expired.
    InvalidVerificationCode,
    /// Wallet address format is invalid.
    InvalidWalletAddress,
    /// Bitcoin wallet address is invalid.
    InvalidBtcWalletAddress,
    /// Monero wallet address is invalid.
    InvalidXmrWalletAddress,
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserError::UsernameTaken => write!(f, "username already taken"),
            UserError::UsernameTooShort => write!(f, "username must be at least 3 characters"),
            UserError::UsernameTooLong => write!(f, "username must be at most 32 characters"),
            UserError::InvalidUsername => write!(f, "username contains invalid characters"),
            UserError::PasswordTooShort => write!(f, "password must be at least 8 characters"),
            UserError::PasswordTooLong => write!(f, "password must be at most 128 characters"),
            UserError::UserNotFound => write!(f, "user not found"),
            UserError::InvalidCredentials => write!(f, "invalid username or password"),
            UserError::AccountSuspended => write!(f, "account is suspended"),
            UserError::AccountExists => write!(f, "account already exists"),
            UserError::InvalidToken => write!(f, "invalid or expired token"),
            UserError::ReviewAlreadySubmitted => {
                write!(f, "review already submitted for this trade")
            }
            UserError::SelfReview => write!(f, "cannot review yourself"),
            UserError::InvalidRating => write!(f, "rating must be between 1 and 5"),
            UserError::PasswordHashError(msg) => write!(f, "password hashing failed: {}", msg),
            UserError::RateLimited => write!(f, "rate limit exceeded, please try again later"),
            UserError::EmailTaken => write!(f, "email already registered"),
            UserError::InvalidEmail => write!(f, "invalid email format"),
            UserError::EmailNotVerified => write!(f, "email not verified"),
            UserError::InvalidVerificationCode => write!(f, "invalid or expired verification code"),
            UserError::InvalidWalletAddress => write!(f, "invalid wallet address format"),
            UserError::InvalidBtcWalletAddress => write!(f, "invalid Bitcoin wallet address"),
            UserError::InvalidXmrWalletAddress => write!(f, "invalid Monero wallet address"),
        }
    }
}

impl std::error::Error for UserError {}
