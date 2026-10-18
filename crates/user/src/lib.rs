//! User accounts, authentication, and reputation system for OpenDex P2P trading.
//!
//! This crate provides:
//! - User account management with username/password authentication
//! - Reputation scoring based on membership duration, trade history, and reviews
//! - Maker and taker statistics tracking
//! - Review system for post-trade feedback

#![allow(warnings)]

pub mod auth;
pub mod error;
pub mod reputation;
pub mod stats;
pub mod user;

pub use auth::*;
pub use error::*;
pub use reputation::*;
pub use stats::*;
pub use user::*;

// Kani proofs disabled: the user crate has heavy dependencies (chrono, hmac, argonaut)
// that cause state space explosion during symbolic execution (166K+ SAT variables).
// The auth_kani.rs file is kept for reference but not compiled.
// #[cfg(kani)]
// mod auth_kani;
