use protocol::message::{Asset, Market, Side};

use uuid::Uuid;

pub enum Command {
    Order {
        side: Side,
        market: Market,
        price: f64,
        amount: f64,
    },

    Cancel {
        id: Uuid,
    },

    ShowOrders,

    Unknown,
}

pub fn parse_command(input: &str) -> Command {
    let parts: Vec<&str> = input.trim().split_whitespace().collect();

    if input.trim() == "orders" {
        return Command::ShowOrders;
    }

    if parts.len() == 2 && parts[0] == "cancel" {
        return match Uuid::parse_str(parts[1]) {
            Ok(id) => Command::Cancel { id },
            Err(_) => Command::Unknown,
        };
    }

    if parts.len() != 5 {
        return Command::Unknown;
    }

    let side = match parts[0].to_lowercase().as_str() {
        "buy" => Side::Buy,
        "sell" => Side::Sell,
        _ => return Command::Unknown,
    };

    let base = match parts[1].to_uppercase().as_str() {
        "BTC" => Asset::BTC,
        "XMR" => Asset::XMR,
        _ => return Command::Unknown,
    };

    let quote = match parts[2].to_uppercase().as_str() {
        "BTC" => Asset::BTC,
        "XMR" => Asset::XMR,
        _ => return Command::Unknown,
    };

    let price = match parts[3].parse::<f64>() {
        Ok(v) => v,
        Err(_) => return Command::Unknown,
    };

    let amount = match parts[4].parse::<f64>() {
        Ok(v) => v,
        Err(_) => return Command::Unknown,
    };

    Command::Order {
        side,
        market: Market { base, quote },
        price,
        amount,
    }
}
