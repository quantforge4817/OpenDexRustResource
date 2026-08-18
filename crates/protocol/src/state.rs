use crate::message::{Order, Trade};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub orders: Vec<Order>,
    pub trades: Vec<Trade>,

    #[serde(default)]
    pub removed_orders: Vec<Uuid>,

    #[serde(default)]
    pub applied_trades: Vec<String>,
}
