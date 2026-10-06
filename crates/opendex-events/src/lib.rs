//! Append-only event types for OpenDex event sourcing.
//!
//! Every swap state change is recorded as an immutable event. The event log
//! allows full reconstruction of swap state without trusting in-memory state.
//! Events are never modified once created.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use opendex_types::{Asset, Piconero, Satoshis, SwapId, TradeId};
use opendex_protocol_types::SwapState;

// ─── Event ID ───────────────────────────────────────────────────────────────

/// Unique identifier for an event in the append-only log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

// ─── Swap Event ─────────────────────────────────────────────────────────────

/// Immutable events that record every swap state change.
///
/// Events are append-only and form the source of truth for swap state.
/// The swap state can be reconstructed by replaying events from the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwapEvent {
    /// A new swap was created.
    SwapCreated {
        swap_id: SwapId,
        trade_id: TradeId,
        btc_amount: Satoshis,
        xmr_amount: Piconero,
    },

    /// Swap state changed from one state to another.
    SwapStateChanged {
        swap_id: SwapId,
        from_state: SwapState,
        to_state: SwapState,
    },

    /// Funding transaction detected on a blockchain.
    FundingDetected {
        swap_id: SwapId,
        chain: Asset,
        txid: String,
        amount: u64,
        confirmations: u32,
    },

    /// Redemption transaction detected on a blockchain.
    RedemptionDetected {
        swap_id: SwapId,
        chain: Asset,
        txid: String,
    },

    /// Refund transaction detected on a blockchain.
    RefundDetected {
        swap_id: SwapId,
        chain: Asset,
        txid: String,
    },

    /// Swap completed successfully.
    SwapCompleted {
        swap_id: SwapId,
    },

    /// Swap failed with a reason.
    SwapFailed {
        swap_id: SwapId,
        reason: String,
    },
}

impl SwapEvent {
    /// Returns the swap ID associated with this event.
    pub fn swap_id(&self) -> SwapId {
        match self {
            SwapEvent::SwapCreated { swap_id, .. }
            | SwapEvent::SwapStateChanged { swap_id, .. }
            | SwapEvent::FundingDetected { swap_id, .. }
            | SwapEvent::RedemptionDetected { swap_id, .. }
            | SwapEvent::RefundDetected { swap_id, .. }
            | SwapEvent::SwapCompleted { swap_id }
            | SwapEvent::SwapFailed { swap_id, .. } => *swap_id,
        }
    }
}

// ─── Event Record ───────────────────────────────────────────────────────────

/// A persisted event with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    /// Unique event identifier.
    pub id: EventId,
    /// Monotonically increasing sequence number for ordering.
    pub sequence: u64,
    /// ISO 8601 timestamp when the event was created.
    pub timestamp: String,
    /// The event payload.
    pub event: SwapEvent,
}

impl EventRecord {
    pub fn new(id: EventId, sequence: u64, timestamp: String, event: SwapEvent) -> Self {
        Self { id, sequence, timestamp, event }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendex_types::{SwapId, TradeId, Satoshis, Piconero, Asset};

    fn test_swap_id() -> SwapId {
        SwapId(Uuid::new_v4())
    }

    fn test_trade_id() -> TradeId {
        TradeId(Uuid::new_v4())
    }

    #[test]
    fn test_event_id_generation() {
        let id1 = EventId::new();
        let id2 = EventId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_swap_created_event() {
        let swap_id = test_swap_id();
        let trade_id = test_trade_id();
        let event = SwapEvent::SwapCreated {
            swap_id,
            trade_id,
            btc_amount: Satoshis(100_000),
            xmr_amount: Piconero(2_000_000_000_000),
        };
        assert_eq!(event.swap_id(), swap_id);
    }

    #[test]
    fn test_swap_state_changed_event() {
        let swap_id = test_swap_id();
        let event = SwapEvent::SwapStateChanged {
            swap_id,
            from_state: SwapState::Created,
            to_state: SwapState::Negotiating,
        };
        assert_eq!(event.swap_id(), swap_id);
    }

    #[test]
    fn test_funding_detected_event() {
        let swap_id = test_swap_id();
        let event = SwapEvent::FundingDetected {
            swap_id,
            chain: Asset::Btc,
            txid: "abc123".to_string(),
            amount: 100_000,
            confirmations: 1,
        };
        assert_eq!(event.swap_id(), swap_id);
    }

    #[test]
    fn test_redemption_detected_event() {
        let swap_id = test_swap_id();
        let event = SwapEvent::RedemptionDetected {
            swap_id,
            chain: Asset::Xmr,
            txid: "def456".to_string(),
        };
        assert_eq!(event.swap_id(), swap_id);
    }

    #[test]
    fn test_refund_detected_event() {
        let swap_id = test_swap_id();
        let event = SwapEvent::RefundDetected {
            swap_id,
            chain: Asset::Btc,
            txid: "ghi789".to_string(),
        };
        assert_eq!(event.swap_id(), swap_id);
    }

    #[test]
    fn test_swap_completed_event() {
        let swap_id = test_swap_id();
        let event = SwapEvent::SwapCompleted { swap_id };
        assert_eq!(event.swap_id(), swap_id);
    }

    #[test]
    fn test_swap_failed_event() {
        let swap_id = test_swap_id();
        let event = SwapEvent::SwapFailed {
            swap_id,
            reason: "timeout".to_string(),
        };
        assert_eq!(event.swap_id(), swap_id);
    }

    #[test]
    fn test_event_record_creation() {
        let id = EventId::new();
        let event = SwapEvent::SwapCompleted { swap_id: test_swap_id() };
        let record = EventRecord::new(id, 1, "2024-01-01T00:00:00Z".to_string(), event);
        assert_eq!(record.id, id);
        assert_eq!(record.sequence, 1);
    }

    #[test]
    fn test_event_serialization() {
        let swap_id = test_swap_id();
        let event = SwapEvent::SwapStateChanged {
            swap_id,
            from_state: SwapState::Created,
            to_state: SwapState::Negotiating,
        };
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: SwapEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.swap_id(), swap_id);
    }

    #[test]
    fn test_event_record_serialization() {
        let id = EventId::new();
        let event = SwapEvent::SwapCreated {
            swap_id: test_swap_id(),
            trade_id: test_trade_id(),
            btc_amount: Satoshis(100_000),
            xmr_amount: Piconero(2_000_000_000_000),
        };
        let record = EventRecord::new(id, 1, "2024-01-01T00:00:00Z".to_string(), event);
        let serialized = serde_json::to_string(&record).unwrap();
        let deserialized: EventRecord = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, id);
        assert_eq!(deserialized.sequence, 1);
    }
}
