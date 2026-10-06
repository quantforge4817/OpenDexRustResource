//! Mock watcher for testing without real blockchain connections.

use std::sync::Mutex;

use opendex_protocol_types::ChainObservation;

/// Injects observations for testing without real chains.
pub struct MockWatcher {
    pending: Mutex<Vec<ChainObservation>>,
}

impl MockWatcher {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
        }
    }

    /// Inject an observation to be returned by poll().
    pub fn inject(&self, observation: ChainObservation) {
        self.pending.lock().unwrap().push(observation);
    }

    /// Return all pending observations and clear the queue.
    pub fn poll(&self) -> Vec<ChainObservation> {
        let mut pending = self.pending.lock().unwrap();
        std::mem::take(&mut *pending)
    }
}

impl Default for MockWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendex_types::{Satoshis, SwapId};

    #[test]
    fn test_mock_watcher_empty_poll() {
        let watcher = MockWatcher::new();
        let observations = watcher.poll();
        assert!(observations.is_empty());
    }

    #[test]
    fn test_mock_watcher_inject_and_poll() {
        let watcher = MockWatcher::new();
        let obs = ChainObservation::BtcFundingSeen {
            swap_id: SwapId(uuid::Uuid::new_v4()),
            txid: "abc123".to_string(),
            amount: Satoshis(100_000),
            confirmations: 1,
        };
        watcher.inject(obs.clone());
        let observations = watcher.poll();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0], obs);
    }

    #[test]
    fn test_mock_watcher_multiple_observations() {
        let watcher = MockWatcher::new();
        let swap_id = SwapId(uuid::Uuid::new_v4());
        watcher.inject(ChainObservation::BtcFundingSeen {
            swap_id,
            txid: "abc123".to_string(),
            amount: Satoshis(100_000),
            confirmations: 1,
        });
        watcher.inject(ChainObservation::BtcFundingConfirmed {
            swap_id,
            txid: "abc123".to_string(),
            confirmations: 6,
        });
        let observations = watcher.poll();
        assert_eq!(observations.len(), 2);
    }

    #[test]
    fn test_mock_watcher_poll_clears_queue() {
        let watcher = MockWatcher::new();
        watcher.inject(ChainObservation::BtcFundingSeen {
            swap_id: SwapId(uuid::Uuid::new_v4()),
            txid: "abc123".to_string(),
            amount: Satoshis(100_000),
            confirmations: 1,
        });
        watcher.poll();
        let observations = watcher.poll();
        assert!(observations.is_empty());
    }
}
