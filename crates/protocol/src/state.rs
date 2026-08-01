use crate::message::{Order, Trade};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub orders: Vec<Order>,
    pub trades: Vec<Trade>,
}
