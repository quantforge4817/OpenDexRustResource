//! Blockchain watcher infrastructure.
//!
//! Watchers are read-only observers that poll blockchain nodes and emit
//! `ChainObservation` events to the swap engine.

pub mod bitcoin_watcher;
pub mod mock_watcher;
pub mod monero_watcher;
pub mod supervisor;

pub use bitcoin_watcher::*;
pub use mock_watcher::*;
pub use monero_watcher::*;
pub use supervisor::*;
