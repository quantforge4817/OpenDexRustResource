//! Transaction intent repository trait and implementations.

use std::path::PathBuf;
use std::sync::Mutex;

use opendex_errors::StorageError;
use opendex_protocol_types::TransactionIntent;
use opendex_types::SwapId;

use crate::storage::{atomic_write, id_to_path};

/// Repository for recording transaction broadcast intents.
pub trait TransactionIntentRepository: Send + Sync {
    fn create(&self, intent: &TransactionIntent) -> Result<(), StorageError>;
    fn find(&self, swap_id: SwapId) -> Result<Vec<TransactionIntent>, StorageError>;
}

// ─── File-Based Implementation ──────────────────────────────────────────────

/// File-based transaction intent repository.
pub struct FileTransactionIntentRepository {
    base_dir: PathBuf,
}

impl FileTransactionIntentRepository {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn intent_path(&self, intent: &TransactionIntent) -> PathBuf {
        let key = format!("{}_{}", intent.swap_id.0, format!("{:?}", intent.tx_type));
        id_to_path(&self.base_dir, &key, "json")
    }
}

impl TransactionIntentRepository for FileTransactionIntentRepository {
    fn create(&self, intent: &TransactionIntent) -> Result<(), StorageError> {
        let path = self.intent_path(intent);
        let data = serde_json::to_vec_pretty(intent)?;
        atomic_write(&path, &data)?;
        Ok(())
    }

    fn find(&self, swap_id: SwapId) -> Result<Vec<TransactionIntent>, StorageError> {
        let mut intents = Vec::new();
        if self.base_dir.exists() {
            for entry in std::fs::read_dir(&self.base_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(intent) = serde_json::from_str::<TransactionIntent>(&data) {
                            if intent.swap_id == swap_id {
                                intents.push(intent);
                            }
                        }
                    }
                }
            }
        }
        Ok(intents)
    }
}

// ─── In-Memory Implementation (for testing) ─────────────────────────────────

/// In-memory transaction intent repository for testing.
pub struct InMemoryTransactionIntentRepository {
    intents: Mutex<Vec<TransactionIntent>>,
}

impl InMemoryTransactionIntentRepository {
    pub fn new() -> Self {
        Self {
            intents: Mutex::new(Vec::new()),
        }
    }
}

impl Default for InMemoryTransactionIntentRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionIntentRepository for InMemoryTransactionIntentRepository {
    fn create(&self, intent: &TransactionIntent) -> Result<(), StorageError> {
        self.intents.lock().unwrap().push(intent.clone());
        Ok(())
    }

    fn find(&self, swap_id: SwapId) -> Result<Vec<TransactionIntent>, StorageError> {
        let intents = self.intents.lock().unwrap();
        Ok(intents
            .iter()
            .filter(|i| i.swap_id == swap_id)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendex_protocol_types::TransactionType;
    use opendex_types::Asset;
    use tempfile::tempdir;

    fn test_swap_id() -> SwapId {
        SwapId(uuid::Uuid::new_v4())
    }

    #[test]
    fn test_in_memory_create_and_find() {
        let repo = InMemoryTransactionIntentRepository::new();
        let swap_id = test_swap_id();
        let intent = TransactionIntent {
            swap_id,
            tx_type: TransactionType::BtcFunding,
            chain: Asset::Btc,
            txid: "test-txid".to_string(),
        };
        repo.create(&intent).unwrap();
        let found = repo.find(swap_id).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].tx_type, TransactionType::BtcFunding);
    }

    #[test]
    fn test_in_memory_multiple_intents() {
        let repo = InMemoryTransactionIntentRepository::new();
        let swap_id = test_swap_id();

        repo.create(&TransactionIntent {
            swap_id,
            tx_type: TransactionType::BtcFunding,
            chain: Asset::Btc,
            txid: "btc-txid".to_string(),
        })
        .unwrap();

        repo.create(&TransactionIntent {
            swap_id,
            tx_type: TransactionType::XmrFunding,
            chain: Asset::Xmr,
            txid: "xmr-txid".to_string(),
        })
        .unwrap();

        let found = repo.find(swap_id).unwrap();
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_in_memory_find_nonexistent() {
        let repo = InMemoryTransactionIntentRepository::new();
        let found = repo.find(test_swap_id()).unwrap();
        assert_eq!(found.len(), 0);
    }

    #[test]
    fn test_file_repo_create_and_find() {
        let dir = tempdir().unwrap();
        let repo = FileTransactionIntentRepository::new(dir.path().to_path_buf());
        let swap_id = test_swap_id();

        repo.create(&TransactionIntent {
            swap_id,
            tx_type: TransactionType::BtcFunding,
            chain: Asset::Btc,
            txid: "btc-txid".to_string(),
        })
        .unwrap();

        let found = repo.find(swap_id).unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_file_repo_persistence_across_instances() {
        let dir = tempdir().unwrap();
        let swap_id = test_swap_id();

        {
            let repo = FileTransactionIntentRepository::new(dir.path().to_path_buf());
            repo.create(&TransactionIntent {
                swap_id,
                tx_type: TransactionType::BtcRedemption,
                chain: Asset::Btc,
                txid: "btc-redemption-txid".to_string(),
            })
            .unwrap();
        }

        {
            let repo = FileTransactionIntentRepository::new(dir.path().to_path_buf());
            let found = repo.find(swap_id).unwrap();
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].tx_type, TransactionType::BtcRedemption);
        }
    }
}
