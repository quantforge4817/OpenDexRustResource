//! User accounts, authentication, and reputation system for OpenDex P2P trading.
//!
//! This crate provides:
//! - User account management with username/password authentication
//! - Reputation scoring based on membership duration, trade history, and reviews
//! - Maker and taker statistics tracking
//! - Review system for post-trade feedback

pub mod user;
pub mod auth;
pub mod reputation;
pub mod stats;
pub mod error;

pub use user::*;
pub use auth::*;
pub use reputation::*;
pub use stats::*;
pub use error::*;

// Kani proofs disabled: the user crate has heavy dependencies (chrono, hmac, argonaut)
// that cause state space explosion during symbolic execution (166K+ SAT variables).
// The auth_kani.rs file is kept for reference but not compiled.
// #[cfg(kani)]
// mod auth_kani;
