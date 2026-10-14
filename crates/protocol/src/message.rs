use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PRICE_SCALE: u64 = 100_000_000;
pub const AMOUNT_SCALE: u64 = 100_000_000;

/// Current protocol schema version.
pub const SCHEMA_VERSION: u32 = 1;
/// Maximum supported schema version (reject older/unsupported).
pub const MAX_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum MessageType {
    PeerAnnouncement,
    OrderCreated,
    OrderCancelled,
    TradeCreated,
    TradeStateChanged,
    StateRequest,
    StateResponse,
    // P2P trading messages
    P2POfferCreated,
    P2POfferCancelled,
    P2PTradeCreated,
    P2PTradeStateChanged,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub message_type: MessageType,
    pub sender: String,
    pub payload: String,
    pub timestamp: u64,
    /// Protocol schema version for forward/backward compatibility.
    /// Current version is 1.
    pub schema_version: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Asset {
    BTC,
    XMR,
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Asset::BTC => write!(f, "BTC"),
            Asset::XMR => write!(f, "XMR"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Market {
    pub base: Asset,
    pub quote: Asset,
}

impl std::fmt::Display for Market {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub enum Side {
    #[default]
    Buy,
    Sell,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "Buy"),
            Side::Sell => write!(f, "Sell"),
        }
    }
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TradeState {
    Created,
    Accepted,
    EscrowFunding,
    EscrowFunded,
    PaymentPending,
    PaymentSent,
    PaymentConfirmed,
    PayoutPending,
    Settled,

    Cancelled,
    Expired,
    Disputed,
}

impl TradeState {
    pub fn can_transition_to(&self, next: &TradeState) -> bool {
        use TradeState::*;

        matches!(
            (self, next),
            (Created, Accepted)
                | (Created, Cancelled)
                | (Created, Expired)
                | (Accepted, EscrowFunding)
                | (Accepted, Cancelled)
                | (EscrowFunding, EscrowFunded)
                | (EscrowFunding, Disputed)
                | (EscrowFunding, Cancelled)
                | (EscrowFunded, PaymentPending)
                | (EscrowFunded, Disputed)
                | (PaymentPending, PaymentSent)
                | (PaymentPending, Disputed)
                | (PaymentPending, Expired)
                | (PaymentSent, PaymentConfirmed)
                | (PaymentSent, Disputed)
                | (PaymentConfirmed, PayoutPending)
                | (PaymentConfirmed, Disputed)
                | (PayoutPending, Settled)
                | (PayoutPending, Disputed)
                | (Disputed, Settled)
                | (Disputed, Cancelled)
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    pub id: Uuid,

    pub buy_order: Uuid,
    pub sell_order: Uuid,

    pub maker: String,
    pub taker: String,

    pub buyer: String,
    pub seller: String,
    pub market: Market,
    pub price: u64,
    pub amount: u64,
    pub fill_sequence: u64,
    pub state: TradeState,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeStateChanged {
    pub trade_id: Uuid,
    pub state: TradeState,
    pub timestamp: u64,
}

impl Trade {
    pub fn transition(&mut self, next: TradeState, timestamp: u64) -> anyhow::Result<()> {
        // Duplicate state events are harmless and should be idempotent.
        if self.state == next {
            return Ok(());
        }

        if !self.state.can_transition_to(&next) {
            return Err(anyhow::anyhow!(
                "invalid trade state transition: {:?} -> {:?}",
                self.state,
                next
            ));
        }

        self.state = next;
        self.updated_at = timestamp;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateRequest {
    pub request_id: Uuid,
    pub since: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateResponse {
    pub request_id: Uuid,
    pub orders: Vec<Order>,
    pub trades: Vec<Trade>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderCancelled {
    pub order_id: Uuid,
    pub trader: String,
}

#[cfg(test)]
mod tests {
    use super::*;

fn trade() -> Trade {
    Trade {
        id: Uuid::new_v4(),
        buy_order: Uuid::new_v4(),
        sell_order: Uuid::new_v4(),
        maker: "seller".to_string(),
        taker: "buyer".to_string(),
        buyer: "buyer".to_string(),
        seller: "seller".to_string(),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        price: 100 * PRICE_SCALE,
        amount: AMOUNT_SCALE,
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    }
}

    #[test]
    fn valid_trade_transition() {
        let mut trade = trade();

        assert!(trade.transition(TradeState::Accepted, 2).is_ok());

        assert_eq!(trade.state, TradeState::Accepted);
        assert_eq!(trade.updated_at, 2);
    }

    #[test]
    fn invalid_trade_transition_is_rejected() {
        let mut trade = trade();

        assert!(trade.transition(TradeState::Settled, 2).is_err());

        assert_eq!(trade.state, TradeState::Created);
    }

    #[test]
    fn complete_trade_lifecycle() {
        let mut trade = trade();

        let states = [
            TradeState::Accepted,
            TradeState::EscrowFunding,
            TradeState::EscrowFunded,
            TradeState::PaymentPending,
            TradeState::PaymentSent,
            TradeState::PaymentConfirmed,
            TradeState::PayoutPending,
            TradeState::Settled,
        ];
        for (timestamp, state) in (2..).zip(states) {
            trade.transition(state, timestamp).unwrap();
        }

        assert_eq!(trade.state, TradeState::Settled);
    }
}
