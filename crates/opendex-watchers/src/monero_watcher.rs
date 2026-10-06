//! Monero blockchain watcher.

use std::collections::HashSet;
use std::sync::Mutex;

use opendex_monero::MoneroChain;
use opendex_protocol_types::ChainObservation;
use opendex_types::SwapId;

/// Polls a Monero chain for funding, redemption, and refund transactions.
pub struct MoneroWatcher {
    chain: Box<dyn MoneroChain>,
    watched_swaps: Mutex<Vec<SwapId>>,
    seen_txids: Mutex<HashSet<String>>,
    pending_observations: Mutex<Vec<ChainObservation>>,
}

impl MoneroWatcher {
    pub fn new(chain: Box<dyn MoneroChain>) -> Self {
        Self {
            chain,
            watched_swaps: Mutex::new(Vec::new()),
            seen_txids: Mutex::new(HashSet::new()),
            pending_observations: Mutex::new(Vec::new()),
        }
    }

    /// Add a swap to watch.
    pub fn watch_swap(&self, swap_id: SwapId) {
        self.watched_swaps.lock().unwrap().push(swap_id);
    }

    /// Poll the chain for new observations.
    pub fn poll(&self) -> Vec<ChainObservation> {
        let mut pending = self.pending_observations.lock().unwrap();
        std::mem::take(&mut *pending)
    }

    /// Check a specific transaction and emit observations if needed.
    pub fn check_transaction(&self, swap_id: SwapId, txid: &str) {
        if self.seen_txids.lock().unwrap().contains(txid) {
            return;
        }

        if let Ok(Some(tx)) = self.chain.transaction(txid) {
            let mut observations = self.pending_observations.lock().unwrap();
            observations.push(ChainObservation::XmrFundingSeen {
                swap_id,
                txid: tx.txid.clone(),
                amount: tx.amount,
                confirmations: tx.confirmations,
            });

            if tx.confirmations >= 10 {
                observations.push(ChainObservation::XmrFundingConfirmed {
                    swap_id,
                    txid: tx.txid.clone(),
                    confirmations: tx.confirmations,
                });
            }

            drop(observations);
            self.seen_txids.lock().unwrap().insert(txid.to_string());
        }
    }

    /// Inject an observation directly (for testing).
    pub fn inject(&self, observation: ChainObservation) {
        self.pending_observations.lock().unwrap().push(observation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendex_monero::MockMoneroChain;
    use opendex_types::Piconero;

    #[test]
    fn test_watcher_watch_swap() {
        let chain = Box::new(MockMoneroChain::new());
        let watcher = MoneroWatcher::new(chain);
        let swap_id = SwapId(uuid::Uuid::new_v4());
        watcher.watch_swap(swap_id);
        assert_eq!(watcher.watched_swaps.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_watcher_check_transaction() {
        let chain = Box::new(MockMoneroChain::new());
        chain.register_transaction("abc123".to_string(), Piconero(2_000_000_000_000), 5);
        let watcher = MoneroWatcher::new(chain);
        let swap_id = SwapId(uuid::Uuid::new_v4());
        watcher.check_transaction(swap_id, "abc123");
        let observations = watcher.poll();
        assert_eq!(observations.len(), 1);
    }

    #[test]
    fn test_watcher_confirmed_transaction() {
        let chain = Box::new(MockMoneroChain::new());
        chain.register_transaction("def456".to_string(), Piconero(3_000_000_000_000), 10);
        let watcher = MoneroWatcher::new(chain);
        let swap_id = SwapId(uuid::Uuid::new_v4());
        watcher.check_transaction(swap_id, "def456");
        let observations = watcher.poll();
        assert_eq!(observations.len(), 2); // Seen + Confirmed
    }

    #[test]
    fn test_watcher_deduplicates_transactions() {
        let chain = Box::new(MockMoneroChain::new());
        chain.register_transaction("ghi789".to_string(), Piconero(1_000_000_000_000), 1);
        let watcher = MoneroWatcher::new(chain);
        let swap_id = SwapId(uuid::Uuid::new_v4());
        watcher.check_transaction(swap_id, "ghi789");
        watcher.check_transaction(swap_id, "ghi789");
        let observations = watcher.poll();
        assert_eq!(observations.len(), 1);
    }

    #[test]
    fn test_watcher_inject_observation() {
        let chain = Box::new(MockMoneroChain::new());
        let watcher = MoneroWatcher::new(chain);
        let swap_id = SwapId(uuid::Uuid::new_v4());
        watcher.inject(ChainObservation::XmrFundingSeen {
            swap_id,
            txid: "injected".to_string(),
            amount: Piconero(2_000_000_000_000),
            confirmations: 1,
        });
        let observations = watcher.poll();
        assert_eq!(observations.len(), 1);
    }
}
