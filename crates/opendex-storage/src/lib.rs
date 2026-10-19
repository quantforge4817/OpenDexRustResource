//! Persistent storage for OpenDex swap state and events.
#![allow(warnings)]
//!
//! This crate provides repository traits and file-based implementations
//! for swap records, events, and transaction intents.
//!
//! All persisted data can be encrypted at rest using AES-256-GCM
//! (see the `encryption` module).

pub mod encryption;
pub mod event_repo;
pub mod storage;
pub mod swap_repo;
pub mod tx_intent_repo;

pub use encryption::*;
pub use event_repo::*;
pub use storage::*;
pub use swap_repo::*;
pub use tx_intent_repo::*;
