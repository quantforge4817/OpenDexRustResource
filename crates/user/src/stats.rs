//! Maker and taker trade statistics tracking.
//!
//! Tracks the number of successful trades as maker and taker,
//! providing a complete picture of each user's trading activity.

use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use opendex_types::{Asset, TradeId};

use crate::user::UserId;

/// Role played in a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TradeRole {
    /// Created the offer (selling crypto).
    Maker,
    /// Accepted the offer (buying crypto).
    Taker,
}

impl fmt::Display for TradeRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TradeRole::Maker => write!(f, "Maker"),
            TradeRole::Taker => write!(f, "Taker"),
        }
    }
}

/// Outcome of a completed trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TradeOutcome {
    /// Trade completed successfully.
    Completed,
    /// Trade failed or was cancelled.
    Failed,
    /// Trade ended in a dispute.
    Disputed,
}

impl fmt::Display for TradeOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TradeOutcome::Completed => write!(f, "Completed"),
            TradeOutcome::Failed => write!(f, "Failed"),
            TradeOutcome::Disputed => write!(f, "Disputed"),
        }
    }
}

/// Record of a single trade for statistics purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    /// Trade identifier.
    pub trade_id: TradeId,
    /// User's role in the trade.
    pub role: TradeRole,
    /// Trade outcome.
    pub outcome: TradeOutcome,
    /// Crypto asset traded.
    pub asset: Asset,
    /// Completion timestamp.
    pub completed_at: DateTime<Utc>,
}

impl TradeRecord {
    /// Create a new trade record.
    pub fn new(trade_id: TradeId, role: TradeRole, outcome: TradeOutcome, asset: Asset) -> Self {
        Self {
            trade_id,
            role,
            outcome,
            asset,
            completed_at: Utc::now(),
        }
    }

    /// Check if this trade was successful.
    pub fn is_successful(&self) -> bool {
        self.outcome == TradeOutcome::Completed
    }
}

/// Complete statistics profile for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    /// User ID.
    pub user_id: UserId,
    /// Total trades as maker.
    pub maker_trades: usize,
    /// Successful trades as maker.
    pub maker_successful: usize,
    /// Total trades as taker.
    pub taker_trades: usize,
    /// Successful trades as taker.
    pub taker_successful: usize,
    /// Total disputed trades.
    pub disputed_trades: usize,
    /// Individual trade records.
    pub trade_records: Vec<TradeRecord>,
}

impl UserStats {
    /// Create new empty statistics for a user.
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            maker_trades: 0,
            maker_successful: 0,
            taker_trades: 0,
            taker_successful: 0,
            disputed_trades: 0,
            trade_records: Vec::new(),
        }
    }

    /// Record a completed trade.
    pub fn record_trade(
        &mut self,
        trade_id: TradeId,
        role: TradeRole,
        outcome: TradeOutcome,
        asset: Asset,
    ) {
        let record = TradeRecord::new(trade_id, role, outcome, asset);

        match role {
            TradeRole::Maker => {
                self.maker_trades += 1;
                if outcome == TradeOutcome::Completed {
                    self.maker_successful += 1;
                }
            }
            TradeRole::Taker => {
                self.taker_trades += 1;
                if outcome == TradeOutcome::Completed {
                    self.taker_successful += 1;
                }
            }
        }

        if outcome == TradeOutcome::Disputed {
            self.disputed_trades += 1;
        }

        self.trade_records.push(record);
    }

    /// Get the total number of trades.
    pub fn total_trades(&self) -> usize {
        self.maker_trades + self.taker_trades
    }

    /// Get the total number of successful trades.
    pub fn total_successful(&self) -> usize {
        self.maker_successful + self.taker_successful
    }

    /// Get the maker success rate as a percentage (0.0-100.0).
    pub fn maker_success_rate(&self) -> f64 {
        if self.maker_trades == 0 {
            0.0
        } else {
            self.maker_successful as f64 / self.maker_trades as f64 * 100.0
        }
    }

    /// Get the taker success rate as a percentage (0.0-100.0).
    pub fn taker_success_rate(&self) -> f64 {
        if self.taker_trades == 0 {
            0.0
        } else {
            self.taker_successful as f64 / self.taker_trades as f64 * 100.0
        }
    }

    /// Get the overall success rate as a percentage (0.0-100.0).
    pub fn overall_success_rate(&self) -> f64 {
        if self.total_trades() == 0 {
            0.0
        } else {
            self.total_successful() as f64 / self.total_trades() as f64 * 100.0
        }
    }

    /// Get the number of trades for a specific asset.
    pub fn trades_for_asset(&self, asset: &Asset) -> usize {
        self.trade_records
            .iter()
            .filter(|r| &r.asset == asset)
            .count()
    }
}

impl Default for UserStats {
    fn default() -> Self {
        Self::new(UserId::new())
    }
}

/// Service for tracking user trade statistics.
pub struct StatsService {
    /// Statistics for all users.
    stats: HashMap<UserId, UserStats>,
}

impl StatsService {
    /// Create a new statistics service.
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    /// Register a user for statistics tracking.
    pub fn register_user(&mut self, user_id: UserId) {
        let user_stats = UserStats::new(user_id);
        self.stats.insert(user_id, user_stats);
    }

    /// Record a trade outcome for a user.
    pub fn record_trade(
        &mut self,
        user_id: UserId,
        trade_id: TradeId,
        role: TradeRole,
        outcome: TradeOutcome,
        asset: Asset,
    ) {
        // Ensure user has a stats entry
        if !self.stats.contains_key(&user_id) {
            self.register_user(user_id);
        }

        if let Some(user_stats) = self.stats.get_mut(&user_id) {
            user_stats.record_trade(trade_id, role, outcome, asset);
        }
    }

    /// Get a user's statistics.
    pub fn get_stats(&self, user_id: UserId) -> Option<&UserStats> {
        self.stats.get(&user_id)
    }

    /// Get a user's total trade count.
    pub fn get_total_trades(&self, user_id: UserId) -> usize {
        self.stats
            .get(&user_id)
            .map(|s| s.total_trades())
            .unwrap_or(0)
    }

    /// Get a user's success rate.
    pub fn get_success_rate(&self, user_id: UserId) -> f64 {
        self.stats
            .get(&user_id)
            .map(|s| s.overall_success_rate())
            .unwrap_or(0.0)
    }
}

impl Default for StatsService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn record_maker_trade() {
        let mut service = StatsService::new();
        let user_id = UserId::new();
        service.register_user(user_id);

        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Maker,
            TradeOutcome::Completed,
            Asset::Btc,
        );

        let stats = service.get_stats(user_id).unwrap();
        assert_eq!(stats.maker_trades, 1);
        assert_eq!(stats.maker_successful, 1);
        assert_eq!(stats.total_trades(), 1);
        assert!((stats.maker_success_rate() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn record_taker_trade() {
        let mut service = StatsService::new();
        let user_id = UserId::new();
        service.register_user(user_id);

        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Taker,
            TradeOutcome::Completed,
            Asset::Xmr,
        );

        let stats = service.get_stats(user_id).unwrap();
        assert_eq!(stats.taker_trades, 1);
        assert_eq!(stats.taker_successful, 1);
    }

    #[test]
    fn failed_trade_not_counted_as_successful() {
        let mut service = StatsService::new();
        let user_id = UserId::new();
        service.register_user(user_id);

        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Maker,
            TradeOutcome::Failed,
            Asset::Btc,
        );

        let stats = service.get_stats(user_id).unwrap();
        assert_eq!(stats.maker_trades, 1);
        assert_eq!(stats.maker_successful, 0);
        assert!((stats.maker_success_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn disputed_trade_counted() {
        let mut service = StatsService::new();
        let user_id = UserId::new();
        service.register_user(user_id);

        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Taker,
            TradeOutcome::Disputed,
            Asset::Btc,
        );

        let stats = service.get_stats(user_id).unwrap();
        assert_eq!(stats.disputed_trades, 1);
        assert_eq!(stats.taker_successful, 0);
    }

    #[test]
    fn mixed_success_rate() {
        let mut service = StatsService::new();
        let user_id = UserId::new();
        service.register_user(user_id);

        // 3 successful maker trades
        for _ in 0..3 {
            service.record_trade(
                user_id,
                TradeId(Uuid::new_v4()),
                TradeRole::Maker,
                TradeOutcome::Completed,
                Asset::Btc,
            );
        }

        // 1 failed maker trade
        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Maker,
            TradeOutcome::Failed,
            Asset::Btc,
        );

        let stats = service.get_stats(user_id).unwrap();
        assert_eq!(stats.maker_trades, 4);
        assert_eq!(stats.maker_successful, 3);
        assert!((stats.maker_success_rate() - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn overall_success_rate() {
        let mut service = StatsService::new();
        let user_id = UserId::new();
        service.register_user(user_id);

        // 2 successful maker trades
        for _ in 0..2 {
            service.record_trade(
                user_id,
                TradeId(Uuid::new_v4()),
                TradeRole::Maker,
                TradeOutcome::Completed,
                Asset::Btc,
            );
        }

        // 1 successful taker trade
        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Taker,
            TradeOutcome::Completed,
            Asset::Xmr,
        );

        // 1 failed taker trade
        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Taker,
            TradeOutcome::Failed,
            Asset::Xmr,
        );

        let stats = service.get_stats(user_id).unwrap();
        assert_eq!(stats.total_trades(), 4);
        assert_eq!(stats.total_successful(), 3);
        assert!((stats.overall_success_rate() - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn auto_register_on_trade() {
        let mut service = StatsService::new();
        let user_id = UserId::new();

        // Record trade without explicitly registering
        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Maker,
            TradeOutcome::Completed,
            Asset::Btc,
        );

        assert!(service.get_stats(user_id).is_some());
    }

    #[test]
    fn trades_for_asset() {
        let mut service = StatsService::new();
        let user_id = UserId::new();
        service.register_user(user_id);

        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Maker,
            TradeOutcome::Completed,
            Asset::Btc,
        );
        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Maker,
            TradeOutcome::Completed,
            Asset::Btc,
        );
        service.record_trade(
            user_id,
            TradeId(Uuid::new_v4()),
            TradeRole::Taker,
            TradeOutcome::Completed,
            Asset::Xmr,
        );

        let stats = service.get_stats(user_id).unwrap();
        assert_eq!(stats.trades_for_asset(&Asset::Btc), 2);
        assert_eq!(stats.trades_for_asset(&Asset::Xmr), 1);
    }

    #[test]
    fn display_impls() {
        assert_eq!(format!("{}", TradeRole::Maker), "Maker");
        assert_eq!(format!("{}", TradeRole::Taker), "Taker");
        assert_eq!(format!("{}", TradeOutcome::Completed), "Completed");
        assert_eq!(format!("{}", TradeOutcome::Failed), "Failed");
        assert_eq!(format!("{}", TradeOutcome::Disputed), "Disputed");
    }

    #[test]
    fn trade_record_is_successful() {
        let completed = TradeRecord::new(
            TradeId(Uuid::new_v4()),
            TradeRole::Maker,
            TradeOutcome::Completed,
            Asset::Btc,
        );
        assert!(completed.is_successful());

        let failed = TradeRecord::new(
            TradeId(Uuid::new_v4()),
            TradeRole::Maker,
            TradeOutcome::Failed,
            Asset::Btc,
        );
        assert!(!failed.is_successful());
    }
}
