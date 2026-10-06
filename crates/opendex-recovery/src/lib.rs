//! Deterministic recovery engine for interrupted swaps.
//!
//! Evaluates swap state combined with blockchain state and returns
//! a recovery decision (Wait, Redeem, Refund, Punish, Abort).

pub mod decision;
pub mod engine;

pub use decision::*;
pub use engine::*;
