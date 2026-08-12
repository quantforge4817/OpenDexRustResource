use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PRICE_SCALE: u64 = 100_000_000;
pub const AMOUNT_SCALE: u64 = 100_000_000;

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageType {
    PeerAnnouncement,
    OrderCreated,
    OrderCancelled,
    TradeCreated,
    StateRequest,
    StateResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenDexMessage {
    pub message_type: MessageType,
    pub sender: String,
    pub payload: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Asset {
    BTC,
    XMR,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Market {
    pub base: Asset,
    pub quote: Asset,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub id: Uuid,
    pub trader: String,
    pub market: Market,
    pub side: Side,
    pub price: u64,
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    pub id: String,
    pub buy_order: Uuid,
    pub sell_order: Uuid,
    pub market: Market,
    pub price: u64,
    pub amount: u64,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateRequest {
    pub since: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateResponse {
    pub orders: Vec<Order>,
    pub trades: Vec<Trade>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderCancelled {
    pub order_id: Uuid,
}
