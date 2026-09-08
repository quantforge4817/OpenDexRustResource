use crate::OpenDexBehaviour;
use crate::balances::BalanceBook;
use crate::cli::Command;
use crate::exchange::Exchange;
use crate::network::publish_message;
use crate::order_factory::create_order;
use crate::orderbook::OrderBook;
use crate::settlement::settle_local_trade;
use crate::trade_state::advance_trade_to_state;
use crate::trades::TradeBook;
use anyhow::Result;
use libp2p::{PeerId, Swarm, gossipsub};
use protocol::message::{
    AMOUNT_SCALE, Asset, MessageType, OpenDexMessage, OrderCancelled, TradeState, TradeStateChanged,
};

fn format_amount(amount: u64) -> String {
    let whole = amount / AMOUNT_SCALE;
    let fractional = amount % AMOUNT_SCALE;

    if fractional == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fractional:08}")
            .trim_end_matches('0')
            .to_string()
    }
}

pub fn handle_command(
    command: Command,
    swarm: &mut Swarm<OpenDexBehaviour>,
    topic: &gossipsub::IdentTopic,
    local_peer_id: &PeerId,
    order_book: &mut OrderBook,
    trade_book: &mut TradeBook,
    balance_book: &mut BalanceBook,
) -> Result<()> {
    match command {
        Command::Fund { asset, amount } => {
            let trader = local_peer_id.to_string();

            let scaled_amount = amount
                .checked_mul(AMOUNT_SCALE)
                .ok_or_else(|| anyhow::anyhow!("fund amount overflow"))?;

            balance_book.credit(&trader, asset.clone(), scaled_amount)?;
            balance_book.save()?;

            let balance = balance_book.balance(&trader, &asset);

            println!(
                "Funded {:?}: available={}, locked={}",
                asset,
                format_amount(balance.available),
                format_amount(balance.locked)
            );
        }

        Command::ShowBalances => {
            let trader = local_peer_id.to_string();

            println!("=== Balances ===");

            for asset in [Asset::BTC, Asset::XMR] {
                let balance = balance_book.balance(&trader, &asset);

                println!(
                    "{:?}: available={}, locked={}",
                    asset,
                    format_amount(balance.available),
                    format_amount(balance.locked)
                );
            }
        }

        Command::Order {
            side,
            market,
            price,
            amount,
        } => {
            let order = create_order(local_peer_id.to_string(), market, side, price, amount);

            println!("Created order: {}", order.id);

            if let Err(e) = balance_book.lock_for_order(&order) {
                eprintln!("Cannot create order {}: {}", order.id, e);
                return Ok(());
            }

            balance_book.save()?;

            println!("Locked funds for order {}", order.id);

            let order_message = OpenDexMessage {
                message_type: MessageType::OrderCreated,
                sender: local_peer_id.to_string(),
                payload: serde_json::to_string(&order)?,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };

            match publish_message(swarm, topic, &order_message) {
                Ok(_) => {
                    println!("Published order");
                }

                Err(e) => {
                    println!("Publish failed: {:?}", e);

                    if let Err(unlock_error) = balance_book.unlock_for_order(&order) {
                        eprintln!(
                            "CRITICAL: failed to unlock funds for order {}: {}",
                            order.id, unlock_error
                        );
                    }

                    balance_book.save()?;

                    println!("Order {} was not created", order.id);
                    return Ok(());
                }
            }

            let mut exchange = Exchange::new(order_book, trade_book);
            let trades = exchange.submit_order(order.clone())?;

            if trades.is_empty() {
                order_book.save_orders()?;
            } else {
                for trade in trades {
                    println!("=== Trade Created ===");
                    println!("{:?}", trade);

                    let timestamp = chrono::Utc::now().timestamp() as u64;

                    let message = OpenDexMessage {
                        message_type: MessageType::TradeCreated,
                        sender: local_peer_id.to_string(),
                        payload: serde_json::to_string(&trade)?,
                        timestamp,
                    };

                    match publish_message(swarm, topic, &message) {
                        Ok(_) => {
                            println!("Published TradeCreated: {}", trade.id);
                        }
                        Err(e) => {
                            eprintln!("Failed to publish TradeCreated {}: {:?}", trade.id, e);
                        }
                    }
                }
            }
        }

        Command::ShowOrders => {
            order_book.print_orders();
        }

        Command::ShowTrades => {
            trade_book.print_trades();
        }

        Command::TradeState { id, state } => {
            let timestamp = chrono::Utc::now().timestamp() as u64;

            let current_trade = match trade_book.get_trade(id).cloned() {
                Some(trade) => trade,
                None => {
                    eprintln!("Trade not found locally: {}", id);
                    return Ok(());
                }
            };

            if current_trade.state == state {
                println!("Trade {} is already {:?}", id, state);
                return Ok(());
            }

            advance_trade_to_state(trade_book, id, state.clone(), timestamp)?;

            let updated_trade = match trade_book.get_trade(id).cloned() {
                Some(trade) => trade,
                None => {
                    eprintln!("Trade disappeared after state transition: {}", id);
                    return Ok(());
                }
            };

            if state == TradeState::Settled {
                settle_local_trade(balance_book, local_peer_id, &updated_trade)?;
            }

            let change = TradeStateChanged {
                trade_id: id,
                state: state.clone(),
                timestamp,
            };

            let message = OpenDexMessage {
                message_type: MessageType::TradeStateChanged,
                sender: local_peer_id.to_string(),
                payload: serde_json::to_string(&change)?,
                timestamp,
            };

            match publish_message(swarm, topic, &message) {
                Ok(_) => {
                    println!("Published TradeStateChanged: {} -> {:?}", id, state);
                }
                Err(e) => {
                    eprintln!(
                        "Failed to publish TradeStateChanged {} -> {:?}: {:?}",
                        id, state, e
                    );
                }
            }
        }

        Command::Cancel { id } => {
            if let Some(order) = order_book
                .orders
                .iter()
                .find(|order| order.id == id)
                .cloned()
            {
                if order_book.remove_order(id) {
                    balance_book.unlock_for_order(&order)?;
                    balance_book.save()?;
                    order_book.save_orders()?;

                    println!("Removed local order: {}", id);

                    let cancelled = OrderCancelled {
                        order_id: id,
                        trader: order.trader.clone(),
                    };

                    let message = OpenDexMessage {
                        message_type: MessageType::OrderCancelled,
                        sender: local_peer_id.to_string(),
                        payload: serde_json::to_string(&cancelled)?,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    };

                    publish_message(swarm, topic, &message)?;
                }
            } else {
                println!("Order not found locally: {}", id);
            }
        }

        Command::Peers => {
            println!("=== Connected Peers ===");

            for peer in swarm.connected_peers() {
                println!("{}", peer);
            }
        }

        Command::Unknown => {
            println!("Unknown command");
            println!("Example: buy BTC XMR 12.5 0.1");
        }
    }

    Ok(())
}
