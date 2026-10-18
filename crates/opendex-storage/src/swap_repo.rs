//! Swap repository trait and implementations.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use opendex_errors::StorageError;
use opendex_protocol_types::{Swap, SwapState};
use opendex_types::SwapId;

use crate::storage::{atomic_write, id_to_path};

/// Repository for swap records.
pub trait SwapRepository: Send + Sync {
    fn create(&self, swap: &Swap) -> Result<(), StorageError>;
    fn get(&self, id: SwapId) -> Result<Option<Swap>, StorageError>;
    fn update_state(&self, id: SwapId, state: SwapState) -> Result<(), StorageError>;
    fn list_incomplete(&self) -> Result<Vec<Swap>, StorageError>;
}

// ─── File-Based Implementation ──────────────────────────────────────────────

/// File-based swap repository using atomic JSON writes.
pub struct FileSwapRepository {
    base_dir: PathBuf,
    cache: Mutex<HashMap<SwapId, Swap>>,
}

impl FileSwapRepository {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn swap_path(&self, id: SwapId) -> PathBuf {
        id_to_path(&self.base_dir, &id.0.to_string(), "json")
    }

    /// Load all swaps from disk into the cache.
    fn load_all(&self) -> Result<(), StorageError> {
        let mut cache = self.cache.lock().unwrap();
        if self.base_dir.exists() {
            for entry in std::fs::read_dir(&self.base_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(swap) = serde_json::from_str::<Swap>(&data) {
                            cache.insert(swap.id, swap);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl SwapRepository for FileSwapRepository {
    fn create(&self, swap: &Swap) -> Result<(), StorageError> {
        let path = self.swap_path(swap.id);
        let data = serde_json::to_vec_pretty(swap)?;
        atomic_write(&path, &data)?;
        self.cache.lock().unwrap().insert(swap.id, swap.clone());
        Ok(())
    }

    fn get(&self, id: SwapId) -> Result<Option<Swap>, StorageError> {
        // Try cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(swap) = cache.get(&id) {
                return Ok(Some(swap.clone()));
            }
        }
        // Fall back to disk
        let path = self.swap_path(id);
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            let swap: Swap = serde_json::from_str(&data)?;
            self.cache.lock().unwrap().insert(id, swap.clone());
            Ok(Some(swap))
        } else {
            Ok(None)
        }
    }

    fn update_state(&self, id: SwapId, state: SwapState) -> Result<(), StorageError> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(swap) = cache.get_mut(&id) {
            swap.state = state.clone();
            let path = self.swap_path(id);
            let data = serde_json::to_vec_pretty(swap)?;
            atomic_write(&path, &data)?;
            Ok(())
        } else {
            Err(StorageError::NotFound(format!("swap {} not found", id.0)))
        }
    }

    fn list_incomplete(&self) -> Result<Vec<Swap>, StorageError> {
        self.load_all()?;
        let cache = self.cache.lock().unwrap();
        let incomplete = cache
            .values()
            .filter(|swap| {
                !matches!(
                    swap.state,
                    SwapState::Completed | SwapState::Refunded | SwapState::Failed
                )
            })
            .cloned()
            .collect();
        Ok(incomplete)
    }
}

// ─── In-Memory Implementation (for testing) ─────────────────────────────────

/// In-memory swap repository for testing without disk I/O.
pub struct InMemorySwapRepository {
    swaps: Mutex<HashMap<SwapId, Swap>>,
}

impl InMemorySwapRepository {
    pub fn new() -> Self {
        Self {
            swaps: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySwapRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl SwapRepository for InMemorySwapRepository {
    fn create(&self, swap: &Swap) -> Result<(), StorageError> {
        self.swaps.lock().unwrap().insert(swap.id, swap.clone());
        Ok(())
    }

    fn get(&self, id: SwapId) -> Result<Option<Swap>, StorageError> {
        Ok(self.swaps.lock().unwrap().get(&id).cloned())
    }

    fn update_state(&self, id: SwapId, state: SwapState) -> Result<(), StorageError> {
        let mut swaps = self.swaps.lock().unwrap();
        if let Some(swap) = swaps.get_mut(&id) {
            swap.state = state;
            Ok(())
        } else {
            Err(StorageError::NotFound(format!("swap {} not found", id.0)))
        }
    }

    fn list_incomplete(&self) -> Result<Vec<Swap>, StorageError> {
        let swaps = self.swaps.lock().unwrap();
        let incomplete = swaps
            .values()
            .filter(|swap| {
                !matches!(
                    swap.state,
                    SwapState::Completed | SwapState::Refunded | SwapState::Failed
                )
            })
            .cloned()
            .collect();
        Ok(incomplete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendex_types::{Asset, ParticipantId, Piconero, ProtocolVersion, Satoshis, TradeId};
    use std::sync::Arc;
    use tempfile::tempdir;
    use uuid::Uuid;

    fn test_swap(id: SwapId) -> Swap {
        Swap {
            id,
            trade_id: TradeId(uuid::Uuid::new_v4()),
            maker: ParticipantId(uuid::Uuid::new_v4()),
            taker: ParticipantId(uuid::Uuid::new_v4()),
            sell_asset: Asset::Btc,
            buy_asset: Asset::Xmr,
            btc_amount: Satoshis(100_000),
            xmr_amount: Piconero(2_000_000_000_000),
            protocol: ProtocolVersion::btc_xmr_v1(),
            state: SwapState::Created,
        }
    }

    #[test]
    fn test_in_memory_create_and_get() {
        let repo = InMemorySwapRepository::new();
        let swap = test_swap(SwapId(uuid::Uuid::new_v4()));
        repo.create(&swap).unwrap();
        let loaded = repo.get(swap.id).unwrap().unwrap();
        assert_eq!(loaded.id, swap.id);
        assert_eq!(loaded.state, SwapState::Created);
    }

    #[test]
    fn test_in_memory_update_state() {
        let repo = InMemorySwapRepository::new();
        let swap = test_swap(SwapId(uuid::Uuid::new_v4()));
        repo.create(&swap).unwrap();
        repo.update_state(swap.id, SwapState::Negotiating).unwrap();
        let loaded = repo.get(swap.id).unwrap().unwrap();
        assert_eq!(loaded.state, SwapState::Negotiating);
    }

    #[test]
    fn test_in_memory_list_incomplete() {
        let repo = InMemorySwapRepository::new();
        let swap1 = test_swap(SwapId(uuid::Uuid::new_v4()));
        repo.create(&swap1).unwrap();
        repo.update_state(swap1.id, SwapState::Funding).unwrap();

        let swap2 = test_swap(SwapId(uuid::Uuid::new_v4()));
        repo.create(&swap2).unwrap();
        repo.update_state(swap2.id, SwapState::Completed).unwrap();

        let incomplete = repo.list_incomplete().unwrap();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].id, swap1.id);
    }

    #[test]
    fn test_in_memory_get_nonexistent() {
        let repo = InMemorySwapRepository::new();
        let result = repo.get(SwapId(uuid::Uuid::new_v4())).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_file_repo_create_and_get() {
        let dir = tempdir().unwrap();
        let repo = Arc::new(FileSwapRepository::new(dir.path().to_path_buf()));
        let swap = test_swap(SwapId(uuid::Uuid::new_v4()));
        repo.create(&swap).unwrap();

        // Reload from a new instance to verify persistence
        let repo2 = Arc::new(FileSwapRepository::new(dir.path().to_path_buf()));
        let loaded = repo2.get(swap.id).unwrap().unwrap();
        assert_eq!(loaded.id, swap.id);
        assert_eq!(loaded.state, SwapState::Created);
    }

    #[test]
    fn test_file_repo_update_state() {
        let dir = tempdir().unwrap();
        let repo = Arc::new(FileSwapRepository::new(dir.path().to_path_buf()));
        let swap = test_swap(SwapId(uuid::Uuid::new_v4()));
        repo.create(&swap).unwrap();
        repo.update_state(swap.id, SwapState::Funding).unwrap();

        let loaded = repo.get(swap.id).unwrap().unwrap();
        assert_eq!(loaded.state, SwapState::Funding);
    }

    #[test]
    fn test_file_repo_list_incomplete() {
        let dir = tempdir().unwrap();
        let repo = Arc::new(FileSwapRepository::new(dir.path().to_path_buf()));

        let swap1 = test_swap(SwapId(uuid::Uuid::new_v4()));
        repo.create(&swap1).unwrap();
        repo.update_state(swap1.id, SwapState::Funding).unwrap();

        let swap2 = test_swap(SwapId(uuid::Uuid::new_v4()));
        repo.create(&swap2).unwrap();
        repo.update_state(swap2.id, SwapState::Completed).unwrap();

        let incomplete = repo.list_incomplete().unwrap();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].id, swap1.id);
    }

    #[test]
    fn test_file_repo_persistence_across_instances() {
        let dir = tempdir().unwrap();
        let swap_id = SwapId(uuid::Uuid::new_v4());

        {
            let repo = FileSwapRepository::new(dir.path().to_path_buf());
            let swap = test_swap(swap_id);
            repo.create(&swap).unwrap();
            repo.update_state(swap_id, SwapState::Redeeming).unwrap();
        } // repo dropped — simulating crash

        {
            let repo = FileSwapRepository::new(dir.path().to_path_buf());
            let loaded = repo.get(swap_id).unwrap().unwrap();
            assert_eq!(loaded.id, swap_id);
            assert_eq!(loaded.state, SwapState::Redeeming);
        }
    }
}
