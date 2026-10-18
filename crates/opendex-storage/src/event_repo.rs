//! Event repository trait and implementations.

use std::path::PathBuf;
use std::sync::Mutex;

use opendex_errors::StorageError;
use opendex_events::{EventId, EventRecord, SwapEvent};
use opendex_types::SwapId;

use crate::storage::atomic_append_line;

/// Repository for append-only event records.
pub trait EventRepository: Send + Sync {
    fn append(&self, event: SwapEvent) -> Result<EventId, StorageError>;
    fn load_after(&self, sequence: u64) -> Result<Vec<EventRecord>, StorageError>;
    fn load_for_swap(&self, swap_id: SwapId) -> Result<Vec<EventRecord>, StorageError>;
}

// ─── File-Based Implementation ──────────────────────────────────────────────

/// File-based event repository using append-only JSON lines.
pub struct FileEventRepository {
    log_path: PathBuf,
    sequence: Mutex<u64>,
}

impl FileEventRepository {
    pub fn new(log_path: impl Into<PathBuf>) -> Self {
        let log_path = log_path.into();
        // Count existing lines to initialize sequence number
        let initial_sequence = if log_path.exists() {
            let content = std::fs::read_to_string(&log_path).unwrap_or_default();
            content.lines().count() as u64
        } else {
            0
        };
        Self {
            log_path,
            sequence: Mutex::new(initial_sequence),
        }
    }
}

impl EventRepository for FileEventRepository {
    fn append(&self, event: SwapEvent) -> Result<EventId, StorageError> {
        let id = EventId::new();
        let mut seq = self.sequence.lock().unwrap();
        *seq += 1;
        let sequence = *seq;

        let timestamp = chrono::Utc::now().to_rfc3339();
        let record = EventRecord::new(id, sequence, timestamp, event);
        let line = serde_json::to_string(&record)?;

        atomic_append_line(&self.log_path, line.as_bytes())?;
        Ok(id)
    }

    fn load_after(&self, sequence: u64) -> Result<Vec<EventRecord>, StorageError> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&self.log_path)?;
        let mut records = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<EventRecord>(line) {
                if record.sequence > sequence {
                    records.push(record);
                }
            }
        }
        Ok(records)
    }

    fn load_for_swap(&self, swap_id: SwapId) -> Result<Vec<EventRecord>, StorageError> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&self.log_path)?;
        let mut records = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<EventRecord>(line) {
                if record.event.swap_id() == swap_id {
                    records.push(record);
                }
            }
        }
        Ok(records)
    }
}

// ─── In-Memory Implementation (for testing) ─────────────────────────────────

/// In-memory event repository for testing without disk I/O.
pub struct InMemoryEventRepository {
    records: Mutex<Vec<EventRecord>>,
    sequence: Mutex<u64>,
}

impl InMemoryEventRepository {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
            sequence: Mutex::new(0),
        }
    }
}

impl Default for InMemoryEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl EventRepository for InMemoryEventRepository {
    fn append(&self, event: SwapEvent) -> Result<EventId, StorageError> {
        let id = EventId::new();
        let mut seq = self.sequence.lock().unwrap();
        *seq += 1;
        let sequence = *seq;

        let timestamp = chrono::Utc::now().to_rfc3339();
        let record = EventRecord::new(id, sequence, timestamp, event);
        self.records.lock().unwrap().push(record);
        Ok(id)
    }

    fn load_after(&self, sequence: u64) -> Result<Vec<EventRecord>, StorageError> {
        let records = self.records.lock().unwrap();
        Ok(records
            .iter()
            .filter(|r| r.sequence > sequence)
            .cloned()
            .collect())
    }

    fn load_for_swap(&self, swap_id: SwapId) -> Result<Vec<EventRecord>, StorageError> {
        let records = self.records.lock().unwrap();
        Ok(records
            .iter()
            .filter(|r| r.event.swap_id() == swap_id)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendex_protocol_types::SwapState;
    use opendex_types::{Piconero, Satoshis, TradeId};
    use std::sync::Arc;
    use tempfile::tempdir;

    fn test_swap_id() -> SwapId {
        SwapId(uuid::Uuid::new_v4())
    }

    #[test]
    fn test_in_memory_append_and_load() {
        let repo = InMemoryEventRepository::new();
        let swap_id = test_swap_id();
        let event = SwapEvent::SwapCreated {
            swap_id,
            trade_id: TradeId(uuid::Uuid::new_v4()),
            btc_amount: Satoshis(100_000),
            xmr_amount: Piconero(2_000_000_000_000),
        };
        let id = repo.append(event).unwrap();
        let records = repo.load_for_swap(swap_id).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, id);
        assert_eq!(records[0].sequence, 1);
    }

    #[test]
    fn test_in_memory_load_after_sequence() {
        let repo = InMemoryEventRepository::new();
        let swap_id = test_swap_id();

        repo.append(SwapEvent::SwapCreated {
            swap_id,
            trade_id: TradeId(uuid::Uuid::new_v4()),
            btc_amount: Satoshis(100_000),
            xmr_amount: Piconero(2_000_000_000_000),
        })
        .unwrap();

        repo.append(SwapEvent::SwapStateChanged {
            swap_id,
            from_state: SwapState::Created,
            to_state: SwapState::Negotiating,
        })
        .unwrap();

        repo.append(SwapEvent::SwapStateChanged {
            swap_id,
            from_state: SwapState::Negotiating,
            to_state: SwapState::ProtocolSetup,
        })
        .unwrap();

        // Load after sequence 1 — should return 2 records
        let records = repo.load_after(1).unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_in_memory_multiple_swaps() {
        let repo = InMemoryEventRepository::new();
        let swap1 = test_swap_id();
        let swap2 = test_swap_id();

        repo.append(SwapEvent::SwapCreated {
            swap_id: swap1,
            trade_id: TradeId(uuid::Uuid::new_v4()),
            btc_amount: Satoshis(100_000),
            xmr_amount: Piconero(2_000_000_000_000),
        })
        .unwrap();

        repo.append(SwapEvent::SwapCreated {
            swap_id: swap2,
            trade_id: TradeId(uuid::Uuid::new_v4()),
            btc_amount: Satoshis(200_000),
            xmr_amount: Piconero(4_000_000_000_000),
        })
        .unwrap();

        let records1 = repo.load_for_swap(swap1).unwrap();
        let records2 = repo.load_for_swap(swap2).unwrap();
        assert_eq!(records1.len(), 1);
        assert_eq!(records2.len(), 1);
    }

    #[test]
    fn test_file_repo_append_and_load() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("events.jsonl");
        let repo = Arc::new(FileEventRepository::new(log_path.clone()));

        let swap_id = test_swap_id();
        let event = SwapEvent::SwapCreated {
            swap_id,
            trade_id: TradeId(uuid::Uuid::new_v4()),
            btc_amount: Satoshis(100_000),
            xmr_amount: Piconero(2_000_000_000_000),
        };
        let id = repo.append(event).unwrap();

        // Reload from a new instance
        let repo2 = Arc::new(FileEventRepository::new(log_path));
        let records = repo2.load_for_swap(swap_id).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, id);
    }

    #[test]
    fn test_file_repo_persistence_across_instances() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("events.jsonl");
        let swap_id = test_swap_id();

        {
            let repo = FileEventRepository::new(log_path.clone());
            repo.append(SwapEvent::SwapCreated {
                swap_id,
                trade_id: TradeId(uuid::Uuid::new_v4()),
                btc_amount: Satoshis(100_000),
                xmr_amount: Piconero(2_000_000_000_000),
            })
            .unwrap();
            repo.append(SwapEvent::SwapStateChanged {
                swap_id,
                from_state: SwapState::Created,
                to_state: SwapState::Negotiating,
            })
            .unwrap();
        } // dropped — simulating crash

        {
            let repo = FileEventRepository::new(log_path);
            let records = repo.load_for_swap(swap_id).unwrap();
            assert_eq!(records.len(), 2);
            assert_eq!(records[0].sequence, 1);
            assert_eq!(records[1].sequence, 2);
        }
    }
}
