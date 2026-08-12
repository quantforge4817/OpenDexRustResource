use protocol::message::{AMOUNT_SCALE, Market, Order, PRICE_SCALE, Side};
use uuid::Uuid;

pub fn create_order(trader: String, market: Market, side: Side, price: f64, amount: f64) -> Order {
    let price = (price * PRICE_SCALE as f64).round() as u64;
    let amount = (amount * AMOUNT_SCALE as f64).round() as u64;

    Order {
        id: Uuid::new_v4(),
        trader,
        market,
        side,
        price,
        amount,
    }
}
