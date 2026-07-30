use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageType {
    PeerAnnouncement,
    OrderCreated,
    TradeCreated,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenDexMessage {
    pub message_type: MessageType,
    pub sender: String,
    pub payload: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Asset {
    BTC,
    XMR,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Market {
    pub base: Asset,
    pub quote: Asset,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
