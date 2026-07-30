use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageType {
    PeerAnnouncement,
    OrderCreated,
    OrderCancelled,
    TradeCreated,
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
    pub price: f64,
    pub amount: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    pub id: String,
    pub buy_order: Uuid,
    pub sell_order: Uuid,
    pub market: Market,
    pub price: f64,
    pub amount: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderCancelled {
    pub order_id: Uuid,
}
