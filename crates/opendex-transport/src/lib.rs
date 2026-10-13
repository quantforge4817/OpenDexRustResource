//! P2P transport abstraction for OpenDex.
//!
//! This crate provides the `Transport` trait that decouples the swap state
//! machine from any specific network implementation (libp2p, Tor, direct P2P,
//! relay). It also provides `ProtocolEnvelope` for message routing and
//! `MessageStore` for replay protection.
//!
//! Real implementations:
//! - `Libp2pTransport` — Production P2P transport using libp2p
//! - `MockTransport` — In-memory transport for testing

pub mod envelope;
pub mod transport;
pub mod message_store;
pub mod mock;
pub mod libp2p_transport;

pub use envelope::*;
pub use transport::*;
pub use message_store::*;
pub use mock::*;
pub use libp2p_transport::*;

