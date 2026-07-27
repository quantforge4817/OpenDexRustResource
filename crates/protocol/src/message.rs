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

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    pub trader: String,
    pub pair: String,
    pub side: String,
    pub price: f64,
    pub amount: f64,
}
