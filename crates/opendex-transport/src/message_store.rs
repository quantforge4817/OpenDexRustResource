//! Message store for replay protection.
//!
//! Tracks which message IDs have been seen for each swap, preventing
//! duplicate message processing. Also tracks sequence numbers for
//! gap detection on reconnection.

use std::collections::{HashMap, HashSet};

use opendex_types::{MessageId, SwapId};

/// Tracks seen message IDs per swap for replay protection.
///
/// The swap engine checks this store before processing any inbound
/// message. If a message ID has already been seen, it is rejected
/// as a replay attempt.
pub struct MessageStore {
    seen_messages: HashMap<SwapId, HashSet<MessageId>>,
    last_sequence: HashMap<SwapId, u64>,
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            seen_messages: HashMap::new(),
            last_sequence: HashMap::new(),
        }
    }

    /// Check if a message ID has already been seen for this swap.
    ///
    /// Returns `true` if the message is a duplicate (already seen).
    pub fn is_seen(&self, swap_id: SwapId, message_id: MessageId) -> bool {
        self.seen_messages
            .get(&swap_id)
            .map(|ids| ids.contains(&message_id))
            .unwrap_or(false)
    }

    /// Mark a message ID as seen and update the last sequence number.
    pub fn mark_seen(&mut self, swap_id: SwapId, message_id: MessageId, sequence: u64) {
        self.seen_messages
            .entry(swap_id)
            .or_default()
            .insert(message_id);
        self.last_sequence.insert(swap_id, sequence);
    }

    /// Get the last seen sequence number for a swap.
    ///
    /// Used on reconnection to request missing messages.
    pub fn last_sequence(&self, swap_id: SwapId) -> Option<u64> {
        self.last_sequence.get(&swap_id).copied()
    }

    /// Get all seen message IDs for a swap.
    pub fn seen_ids(&self, swap_id: SwapId) -> HashSet<MessageId> {
        self.seen_messages
            .get(&swap_id)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for MessageStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_swap_id() -> SwapId {
        SwapId(Uuid::new_v4())
    }

    fn test_message_id() -> MessageId {
        MessageId(Uuid::new_v4())
    }

    #[test]
    fn test_new_store_empty() {
        let store = MessageStore::new();
        let swap_id = test_swap_id();
        let message_id = test_message_id();
        assert!(!store.is_seen(swap_id, message_id));
        assert_eq!(store.last_sequence(swap_id), None);
    }

    #[test]
    fn test_mark_seen() {
        let mut store = MessageStore::new();
        let swap_id = test_swap_id();
        let message_id = test_message_id();

        assert!(!store.is_seen(swap_id, message_id));
        store.mark_seen(swap_id, message_id, 1);
        assert!(store.is_seen(swap_id, message_id));
        assert_eq!(store.last_sequence(swap_id), Some(1));
    }

    #[test]
    fn test_replay_detection() {
        let mut store = MessageStore::new();
        let swap_id = test_swap_id();
        let message_id = test_message_id();

        store.mark_seen(swap_id, message_id, 1);
        assert!(store.is_seen(swap_id, message_id));
    }

    #[test]
    fn test_different_swaps_isolated() {
        let mut store = MessageStore::new();
        let swap_id_1 = test_swap_id();
        let swap_id_2 = test_swap_id();
        let message_id = test_message_id();

        store.mark_seen(swap_id_1, message_id, 1);
        assert!(store.is_seen(swap_id_1, message_id));
        assert!(!store.is_seen(swap_id_2, message_id));
    }

    #[test]
    fn test_sequence_tracking() {
        let mut store = MessageStore::new();
        let swap_id = test_swap_id();

        store.mark_seen(swap_id, test_message_id(), 1);
        assert_eq!(store.last_sequence(swap_id), Some(1));

        store.mark_seen(swap_id, test_message_id(), 5);
        assert_eq!(store.last_sequence(swap_id), Some(5));
    }

    #[test]
    fn test_seen_ids() {
        let mut store = MessageStore::new();
        let swap_id = test_swap_id();
        let id1 = test_message_id();
        let id2 = test_message_id();

        store.mark_seen(swap_id, id1, 1);
        store.mark_seen(swap_id, id2, 2);

        let seen = store.seen_ids(swap_id);
        assert_eq!(seen.len(), 2);
        assert!(seen.contains(&id1));
        assert!(seen.contains(&id2));
    }
}
