//! Central error definitions for OpenDex.
//!
//! All OpenDex crates use this error hierarchy. Lower-level errors retain
//! their source information and are wrapped by higher-level errors.

use thiserror::Error;

/// Top-level OpenDex error type.
#[derive(Debug, Error)]
pub enum OpenDexError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("invalid state transition: {0}")]
    InvalidState(String),

    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("wallet error: {0}")]
    Wallet(#[from] WalletError),

    #[error("chain error: {0}")]
    Chain(#[from] ChainError),

    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("security error: {0}")]
    Security(#[from] SecurityError),
}

/// Errors from the settlement protocol layer.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(String),

    #[error("no common protocol found between participants")]
    NoCommonProtocol,

    #[error("protocol message rejected: {0}")]
    MessageRejected(String),

    #[error("cryptographic verification failed: {0}")]
    CryptoVerification(String),

    #[error("protocol state machine error: {0}")]
    StateMachine(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("recovery failed: {0}")]
    Recovery(String),

    #[error("upstream protocol error: {0}")]
    Upstream(String),
}

/// Errors from wallet operations.
#[derive(Debug, Error)]
pub enum WalletError {
    #[error("insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u64, need: u64 },

    #[error("signing failed: {0}")]
    SigningFailed(String),

    #[error("wallet unavailable: {0}")]
    Unavailable(String),

    #[error("invalid address: {0}")]
    InvalidAddress(String),
}

/// Errors from blockchain chain operations.
#[derive(Debug, Error)]
pub enum ChainError {
    #[error("transaction not found: {0}")]
    TransactionNotFound(String),

    #[error("broadcast failed: {0}")]
    BroadcastFailed(String),

    #[error("chain backend unavailable: {0}")]
    BackendUnavailable(String),

    #[error("confirmation check failed: {0}")]
    ConfirmationFailed(String),

    #[error("chain reorganization detected")]
    Reorg,

    #[error("chain state unknown (backends disagree)")]
    StateUnknown,

    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),
}

/// Errors from storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("record not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::Serialization(err.to_string())
    }
}

/// Errors from transport/messaging.
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("send failed: {0}")]
    SendFailed(String),

    #[error("receive failed: {0}")]
    ReceiveFailed(String),

    #[error("peer not found: {0}")]
    PeerNotFound(String),

    #[error("message too large: {size} bytes")]
    MessageTooLarge { size: usize },

    #[error("connection lost")]
    ConnectionLost,

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("peer disconnected")]
    Disconnected,

    #[error("replayed message: {0}")]
    ReplayedMessage(String),
}

/// Security-critical errors.
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("invalid signature: {0}")]
    InvalidSignature(String),

    #[error("replay attack detected: message_id={0}")]
    ReplayAttack(String),

    #[error("commitment mismatch: {0}")]
    CommitmentMismatch(String),

    #[error("unauthorized access: {0}")]
    Unauthorized(String),

    #[error("witness extraction failed: {0}")]
    WitnessExtraction(String),
}
