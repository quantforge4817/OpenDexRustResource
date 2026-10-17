//! Reputation and review system for user accounts.
//!
//! This module re-exports the unified reputation types from [`crate::user`].
//! The reputation system tracks:
//! - On-protocol activity (makes, takes, completed trades, disputes, failures)
//! - Written reviews from trading partners (1-5 star ratings with comments)
//! - Trust score (0-100) based on activity and reviews
//! - Reputation tier (Newcomer → Trader → Trusted → Verified → Elite)

// Re-export all reputation types from user.rs for backward compatibility
pub use crate::user::{Rating, Review, Reputation, ReputationTier};
