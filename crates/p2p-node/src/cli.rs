use protocol::message::{Asset, Market, Side, TradeState};

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

    Fund {
        asset: Asset,
        amount: u64,
    },

    TradeState {
        id: Uuid,
        state: TradeState,
    },

    ShowBalances,
    ShowOrders,
    ShowTrades,
    Peers,
    Unknown,
}

pub fn parse_command(input: &str) -> Command {
    let parts: Vec<&str> = input.split_whitespace().collect();

    if input.trim() == "balances" {
        return Command::ShowBalances;
    }

    if parts.len() == 3 && parts[0].eq_ignore_ascii_case("trade-state") {
        let id = match Uuid::parse_str(parts[1]) {
            Ok(id) => id,
            Err(_) => return Command::Unknown,
        };

        let state = match parts[2].to_lowercase().as_str() {
            "created" => TradeState::Created,
            "accepted" => TradeState::Accepted,
            "escrow-funding" => TradeState::EscrowFunding,
            "escrow-funded" => TradeState::EscrowFunded,
            "payment-pending" => TradeState::PaymentPending,
            "payment-sent" => TradeState::PaymentSent,
            "payment-confirmed" => TradeState::PaymentConfirmed,
            "payout-pending" => TradeState::PayoutPending,
            "settled" => TradeState::Settled,
            "cancelled" => TradeState::Cancelled,
            "expired" => TradeState::Expired,
            "disputed" => TradeState::Disputed,
            _ => return Command::Unknown,
        };

        return Command::TradeState { id, state };
    }

    if parts.len() == 3 && parts[0].eq_ignore_ascii_case("fund") {
        let asset = match parts[1].to_uppercase().as_str() {
            "BTC" => Asset::BTC,
            "XMR" => Asset::XMR,
            _ => return Command::Unknown,
        };

        let amount = match parts[2].parse::<u64>() {
            Ok(v) if v > 0 => v,
            _ => return Command::Unknown,
        };

        return Command::Fund { asset, amount };
    }

    if input.trim() == "orders" {
        return Command::ShowOrders;
    }

    if input.trim() == "trades" {
        return Command::ShowTrades;
    }

    if input.trim() == "peers" {
        return Command::Peers;
    }

    if parts.len() == 2 && parts[0].eq_ignore_ascii_case("cancel") {
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
