use crate::message::{Order, Trade};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Monotonically increasing snapshot version number (AUDIT-010).
///
/// Each time a node produces a new snapshot, it increments the version.
/// Receivers reject snapshots whose version is not strictly greater than
/// the previously accepted version, preventing stale or replayed snapshots
/// from overwriting newer local state.
///
/// Version starts at 1 for the first snapshot. A value of 0 indicates an
/// uninitialized snapshot and is rejected during validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Monotonic version counter; must be strictly greater than the
    /// previously accepted snapshot version. Starts at 1.
    #[serde(default)]
    pub version: u64,

    pub orders: Vec<Order>,
    pub trades: Vec<Trade>,

    #[serde(default)]
    pub removed_orders: HashSet<Uuid>,

    #[serde(default)]
    pub applied_trades: HashSet<Uuid>,
}
