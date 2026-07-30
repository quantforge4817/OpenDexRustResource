use protocol::message::{Market, Order, Side};
use uuid::Uuid;

pub fn create_order(trader: String, market: Market, side: Side, price: f64, amount: f64) -> Order {
    Order {
        id: Uuid::new_v4(),
        trader,
        market,
        side,
        price,
        amount,
    }
}
