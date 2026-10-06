//! P2P transport abstraction for OpenDex.
//!
//! This crate provides the `Transport` trait that decouples the swap state
//! machine from any specific network implementation (libp2p, Tor, direct P2P,
//! relay). It also provides `ProtocolEnvelope` for message routing and
//! `MessageStore` for replay protection.

pub mod envelope;
pub mod transport;
pub mod message_store;
pub mod mock;

pub use envelope::*;
pub use transport::*;
pub use message_store::*;
pub use mock::*;

