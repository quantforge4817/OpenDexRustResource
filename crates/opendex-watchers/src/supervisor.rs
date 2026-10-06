//! Watcher supervisor that coordinates Bitcoin and Monero watchers.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use opendex_protocol_types::ChainObservation;

use crate::bitcoin_watcher::BitcoinWatcher;
use crate::monero_watcher::MoneroWatcher;

/// Channel message from a watcher to the supervisor.
#[derive(Debug, Clone)]
pub struct WatcherMessage {
    pub source: WatcherSource,
    pub observation: ChainObservation,
}

/// Source of a watcher message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherSource {
    Bitcoin,
    Monero,
}

/// Coordinates multiple blockchain watchers.
pub struct WatcherSupervisor {
    btc_watcher: Option<BitcoinWatcher>,
    xmr_watcher: Option<MoneroWatcher>,
    poll_interval: Duration,
}

impl WatcherSupervisor {
    pub fn new(poll_interval: Duration) -> Self {
        Self {
            btc_watcher: None,
            xmr_watcher: None,
            poll_interval,
        }
    }

    pub fn with_bitcoin(mut self, watcher: BitcoinWatcher) -> Self {
        self.btc_watcher = Some(watcher);
        self
    }

    pub fn with_monero(mut self, watcher: MoneroWatcher) -> Self {
        self.xmr_watcher = Some(watcher);
        self
    }

    /// Run all watchers, sending observations to the provided channel.
    pub fn run(self, tx: mpsc::Sender<WatcherMessage>) {
        let poll_interval = self.poll_interval;

        if let Some(btc) = self.btc_watcher {
            let tx = tx.clone();
            thread::spawn(move || {
                loop {
                    thread::sleep(poll_interval);
                    for obs in btc.poll() {
                        let _ = tx.send(WatcherMessage {
                            source: WatcherSource::Bitcoin,
                            observation: obs,
                        });
                    }
                }
            });
        }

        if let Some(xmr) = self.xmr_watcher {
            let tx = tx;
            thread::spawn(move || {
                loop {
                    thread::sleep(poll_interval);
                    for obs in xmr.poll() {
                        let _ = tx.send(WatcherMessage {
                            source: WatcherSource::Monero,
                            observation: obs,
                        });
                    }
                }
            });
        }
    }
}

impl Default for WatcherSupervisor {
    fn default() -> Self {
        Self::new(Duration::from_secs(10))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_watcher::MockWatcher;

    #[test]
    fn test_supervisor_construction() {
        let supervisor = WatcherSupervisor::new(Duration::from_secs(5));
        assert!(supervisor.btc_watcher.is_none());
        assert!(supervisor.xmr_watcher.is_none());
    }

    #[test]
    fn test_watcher_message_creation() {
        let msg = WatcherMessage {
            source: WatcherSource::Bitcoin,
            observation: ChainObservation::BtcFundingSeen {
                swap_id: opendex_types::SwapId(uuid::Uuid::new_v4()),
                txid: "abc123".to_string(),
                amount: opendex_types::Satoshis(100_000),
                confirmations: 1,
            },
        };
        assert_eq!(msg.source, WatcherSource::Bitcoin);
    }

    #[test]
    fn test_mock_watcher_emits_observations() {
        let watcher = MockWatcher::new();
        let obs = opendex_protocol_types::ChainObservation::BtcFundingSeen {
            swap_id: opendex_types::SwapId(uuid::Uuid::new_v4()),
            txid: "abc123".to_string(),
            amount: opendex_types::Satoshis(100_000),
            confirmations: 1,
        };
        watcher.inject(obs.clone());
        let observations = watcher.poll();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0], obs);
    }
}
