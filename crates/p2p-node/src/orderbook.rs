use protocol::message::Order;

#[derive(Debug)]
pub struct OrderBook {
    pub orders: Vec<Order>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            orders: Vec::new(),
        }
    }

    pub fn add_order(&mut self, order: Order) {
        self.orders.push(order);
    }

    pub fn print_orders(&self) {
        println!("=== OpenDEX OrderBook ===");

        for order in &self.orders {
            println!("{:?}", order);
        }
    }
}
