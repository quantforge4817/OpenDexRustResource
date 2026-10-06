use crate::OpenDexBehaviour;
use crate::balances::BalanceBook;
use crate::btc_wallet_client::BitcoinWalletClient;
use crate::cli::Command;
use crate::exchange::Exchange;
use crate::network::publish_message;
use crate::order_factory::create_order;
use crate::orderbook::OrderBook;
use crate::p2p_escrow::{P2PEscrowContext, fund_escrow, release_crypto, return_escrow};
use crate::p2p_offer_book::P2POfferBook;
use crate::settlement::settle_local_trade;
use crate::trade_state::advance_trade_to_state;
use crate::trades::TradeBook;
use crate::wallet_client::MoneroWalletClient;
use anyhow::Result;
use libp2p::{PeerId, Swarm, gossipsub};
use protocol::message::{
    AMOUNT_SCALE, Asset, MessageType, Message, OrderCancelled, TradeState, TradeStateChanged,
};
use protocol::p2p::{P2POffer, P2POfferCancelled, P2PTrade, P2PTradeStateChanged, P2PTradeState};

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

pub async fn handle_command(
    command: Command,
    swarm: &mut Swarm<OpenDexBehaviour>,
    topic: &gossipsub::IdentTopic,
    local_peer_id: &PeerId,
    order_book: &mut OrderBook,
    trade_book: &mut TradeBook,
    balance_book: &mut BalanceBook,
    p2p_offer_book: &mut P2POfferBook,
    monero_wallet: Option<&MoneroWalletClient>,
    btc_wallet: Option<&BitcoinWalletClient>,
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

            tracing::info!(
                asset = ?asset,
                available = format_amount(balance.available),
                locked = format_amount(balance.locked),
                escrowed = format_amount(balance.escrowed),
                "Funded"
            );
        }

        Command::ShowBalances => {
            let trader = local_peer_id.to_string();

            tracing::info!("=== Balances ===");

            for asset in [Asset::BTC, Asset::XMR] {
                let balance = balance_book.balance(&trader, &asset);

                tracing::info!(
                    asset = ?asset,
                    available = format_amount(balance.available),
                    locked = format_amount(balance.locked),
                    escrowed = format_amount(balance.escrowed)
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

            tracing::info!(order_id = %order.id, "Created order");

            if let Err(e) = balance_book.lock_for_order(&order) {
                tracing::error!(order_id = %order.id, error = %e, "Cannot create order");
                return Ok(());
            }

            balance_book.save()?;

            tracing::info!(order_id = %order.id, "Locked funds for order");

            let order_message = Message {
                message_type: MessageType::OrderCreated,
                sender: local_peer_id.to_string(),
                payload: serde_json::to_string(&order)?,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };

            match publish_message(swarm, topic, &order_message) {
                Ok(_) => {
                    tracing::info!("Published order");
                }

                Err(e) => {
                    tracing::error!(error = %e, "Publish failed");

                    if let Err(unlock_error) = balance_book.unlock_for_order(&order) {
                        tracing::error!(
                            order_id = %order.id,
                            error = %unlock_error,
                            "CRITICAL: failed to unlock funds for order"
                        );
                    }

                    balance_book.save()?;

                    tracing::error!(order_id = %order.id, "Order was not created");
                    return Ok(());
                }
            }

            let mut exchange = Exchange::new(order_book, trade_book, balance_book);
            let trades = exchange.submit_order(crate::exchange::SubmitOrder {
                actor: local_peer_id.to_string(),
                order: order.clone(),
            })?;

            if trades.is_empty() {
                order_book.save_orders()?;
            } else {
                for trade in trades {
                    tracing::info!(trade_id = %trade.id, "=== Trade Created ===");
                    tracing::debug!("{:?}", trade);

                    let timestamp = chrono::Utc::now().timestamp() as u64;

                    let message = Message {
                        message_type: MessageType::TradeCreated,
                        sender: local_peer_id.to_string(),
                        payload: serde_json::to_string(&trade)?,
                        timestamp,
                    };

                    match publish_message(swarm, topic, &message) {
                        Ok(_) => {
                            tracing::info!(trade_id = %trade.id, "Published TradeCreated");
                        }
                        Err(e) => {
                            tracing::error!(trade_id = %trade.id, error = %e, "Failed to publish TradeCreated");
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
                    tracing::error!(trade_id = %id, "Trade not found locally");
                    return Ok(());
                }
            };

            if current_trade.state == state {
                tracing::info!(trade_id = %id, state = ?state, "Trade is already in target state");
                return Ok(());
            }

            advance_trade_to_state(trade_book, id, state.clone(), timestamp)?;

            let updated_trade = match trade_book.get_trade(id).cloned() {
                Some(trade) => trade,
                None => {
                    tracing::error!(trade_id = %id, "Trade disappeared after state transition");
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

            let message = Message {
                message_type: MessageType::TradeStateChanged,
                sender: local_peer_id.to_string(),
                payload: serde_json::to_string(&change)?,
                timestamp,
            };

            match publish_message(swarm, topic, &message) {
                Ok(_) => {
                    tracing::info!(trade_id = %id, state = ?state, "Published TradeStateChanged");
                }
                Err(e) => {
                    tracing::error!(
                        trade_id = %id, state = ?state, error = %e,
                        "Failed to publish TradeStateChanged"
                    );
                }
            }
        }

        Command::Cancel { id } => {
            if let Some(order) = order_book.get_order(&id).cloned() {
                if order_book.remove_order(id) {
                    balance_book.unlock_for_order(&order)?;
                    balance_book.save()?;
                    order_book.save_orders()?;

                    tracing::info!(order_id = %id, "Removed local order");

                    let cancelled = OrderCancelled {
                        order_id: id,
                        trader: order.trader.clone(),
                    };

                    let message = Message {
                        message_type: MessageType::OrderCancelled,
                        sender: local_peer_id.to_string(),
                        payload: serde_json::to_string(&cancelled)?,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    };

                    publish_message(swarm, topic, &message)?;
                }
            } else {
                tracing::debug!(order_id = %id, "Order not found locally");
            }
        }

        Command::Peers => {
            tracing::info!("=== Connected Peers ===");

            for peer in swarm.connected_peers() {
                tracing::info!(peer = %peer);
            }
        }

        Command::WalletAddress => {
            tracing::info!("Use 'wallet-address' in CLI mode (handled by main loop)");
        }

        Command::WalletBalance => {
            tracing::info!("Use 'wallet-balance' in CLI mode (handled by main loop)");
        }

        Command::BtcWalletAddress => {
            tracing::info!("Use 'btc-wallet-address' in CLI mode (handled by main loop)");
        }

        Command::BtcWalletBalance => {
            tracing::info!("Use 'btc-wallet-balance' in CLI mode (handled by main loop)");
        }

        Command::ShowP2POffers => {
            p2p_offer_book.print_offers();
        }

        Command::ShowP2PTrades => {
            p2p_offer_book.print_trades();
        }

        Command::P2POffer {
            crypto_asset,
            side,
            amount,
            price,
            currency,
            payment_method,
            payment_instructions,
            timeout_secs,
        } => {
            let maker = local_peer_id.to_string();
            let scaled_amount = (amount * AMOUNT_SCALE as f64) as u64;
            let scaled_price = (price * protocol::p2p::FIAT_SCALE as f64) as u64;
            let now = chrono::Utc::now().timestamp() as u64;
            let expires_at = now + timeout_secs;

            let offer = P2POffer {
                id: uuid::Uuid::new_v4(),
                maker: maker.clone(),
                crypto_asset: crypto_asset.clone(),
                side: side.clone(),
                amount: scaled_amount,
                price: scaled_price,
                currency: currency.clone(),
                payment_method: payment_method.clone(),
                payment_instructions: payment_instructions.clone(),
                payment_timeout_secs: timeout_secs,
                min_amount: scaled_amount,
                max_amount: scaled_amount,
                expires_at,
                created_at: now,
            };

            tracing::info!(
                offer_id = %offer.id,
                "Created P2P offer: {} {} {} @ {} {} via {}",
                offer.side,
                amount,
                offer.crypto_asset,
                price,
                offer.currency,
                offer.payment_method
            );

            p2p_offer_book.add_offer(offer.clone());
            p2p_offer_book.save_offers()?;

            let message = Message {
                message_type: MessageType::P2POfferCreated,
                sender: maker,
                payload: serde_json::to_string(&offer)?,
                timestamp: now,
            };

            publish_message(swarm, topic, &message)?;
            tracing::info!("Published P2P offer");
        }

        Command::P2PTakeOffer { offer_id, amount } => {
            let offer = match p2p_offer_book.get_offer(offer_id) {
                Some(offer) => offer.clone(),
                None => {
                    tracing::error!(offer_id = %offer_id, "P2P offer not found");
                    return Ok(());
                }
            };

            let taker = local_peer_id.to_string();
            let scaled_amount = (amount * AMOUNT_SCALE as f64) as u64;
            let scaled_price = offer.price;
            let fiat_amount = scaled_amount as u128 * scaled_price as u128 / AMOUNT_SCALE as u128;

            if fiat_amount > u64::MAX as u128 {
                tracing::error!("Fiat amount overflow");
                return Ok(());
            }

            let now = chrono::Utc::now().timestamp() as u64;

            let trade = P2PTrade {
                id: uuid::Uuid::new_v4(),
                offer_id,
                maker: offer.maker.clone(),
                taker: taker.clone(),
                crypto_asset: offer.crypto_asset.clone(),
                side: offer.side.clone(),
                amount: scaled_amount,
                price: scaled_price,
                fiat_amount: fiat_amount as u64,
                currency: offer.currency.clone(),
                payment_method: offer.payment_method.clone(),
                state: P2PTradeState::OfferTaken,
                taker_crypto_address: String::new(),
                escrow_address: String::new(),
                escrow_tx_hash: None,
                payment_id: None,
                fiat_payment_proof: None,
                release_tx_hash: None,
                created_at: now,
                updated_at: now,
            };

            tracing::info!(
                trade_id = %trade.id,
                "Took P2P offer: {} {} @ {} {}",
                amount,
                offer.crypto_asset,
                offer.price as f64 / protocol::p2p::FIAT_SCALE as f64,
                offer.currency
            );

            p2p_offer_book.add_trade(trade.clone());
            p2p_offer_book.save_trades()?;

            let message = Message {
                message_type: MessageType::P2PTradeCreated,
                sender: taker,
                payload: serde_json::to_string(&trade)?,
                timestamp: now,
            };

            publish_message(swarm, topic, &message)?;
            tracing::info!("Published P2P trade");
        }

        Command::P2PTradeState { trade_id, state } => {
            let state_enum = match state.to_lowercase().as_str() {
                "offertaken" => P2PTradeState::OfferTaken,
                "escrowfunded" => P2PTradeState::EscrowFunded,
                "fiatpaymentpending" => P2PTradeState::FiatPaymentPending,
                "fiatpaymentsent" => P2PTradeState::FiatPaymentSent,
                "fiatpaymentconfirmed" => P2PTradeState::FiatPaymentConfirmed,
                "cryptoreleased" => P2PTradeState::CryptoReleased,
                "settled" => P2PTradeState::Settled,
                "cancelled" => P2PTradeState::Cancelled,
                "disputed" => P2PTradeState::Disputed,
                "expired" => P2PTradeState::Expired,
                _ => {
                    tracing::error!(state = %state, "Unknown P2P trade state");
                    return Ok(());
                }
            };

            let now = chrono::Utc::now().timestamp() as u64;

            let trade = match p2p_offer_book.get_trade(trade_id) {
                Some(trade) => trade.clone(),
                None => {
                    tracing::error!(trade_id = %trade_id, "P2P trade not found");
                    return Ok(());
                }
            };

            let mut trade = trade;
            if trade.transition(state_enum.clone(), now).is_ok() {
                // Perform on-chain escrow operations for key state transitions
                let escrow_ctx = P2PEscrowContext::new(monero_wallet, btc_wallet);

                match &state_enum {
                    P2PTradeState::EscrowFunded => {
                        if let Err(e) = fund_escrow(&escrow_ctx, &mut trade).await {
                            tracing::error!(
                                trade_id = %trade_id,
                                error = %e,
                                "Failed to fund P2P escrow on-chain"
                            );
                        }
                    }
                    P2PTradeState::CryptoReleased => {
                        if let Err(e) = release_crypto(&escrow_ctx, &mut trade).await {
                            tracing::error!(
                                trade_id = %trade_id,
                                error = %e,
                                "Failed to release P2P crypto on-chain"
                            );
                        }
                    }
                    P2PTradeState::Cancelled => {
                        if let Err(e) = return_escrow(&escrow_ctx, &trade).await {
                            tracing::error!(
                                trade_id = %trade_id,
                                error = %e,
                                "Failed to return P2P escrow on-chain"
                            );
                        }
                    }
                    _ => {}
                }

                p2p_offer_book.trades.insert(trade_id, trade);
                p2p_offer_book.save_trades()?;

                let change = P2PTradeStateChanged {
                    trade_id,
                    state: state_enum,
                    timestamp: now,
                };

                let message = Message {
                    message_type: MessageType::P2PTradeStateChanged,
                    sender: local_peer_id.to_string(),
                    payload: serde_json::to_string(&change)?,
                    timestamp: now,
                };

                publish_message(swarm, topic, &message)?;
                tracing::info!(trade_id = %trade_id, "Published P2P trade state change");
            }
        }

        Command::P2PCancelOffer { offer_id } => {
            if let Some(offer) = p2p_offer_book.remove_offer(offer_id) {
                p2p_offer_book.save_offers()?;

                let cancelled = P2POfferCancelled {
                    offer_id,
                    maker: offer.maker.clone(),
                };

                let message = Message {
                    message_type: MessageType::P2POfferCancelled,
                    sender: local_peer_id.to_string(),
                    payload: serde_json::to_string(&cancelled)?,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                };

                publish_message(swarm, topic, &message)?;
                tracing::info!(offer_id = %offer_id, "Cancelled P2P offer");
            } else {
                tracing::error!(offer_id = %offer_id, "P2P offer not found");
            }
        }

        Command::Unknown => {
            tracing::info!("Unknown command");
            tracing::info!("Example: buy BTC XMR 12.5 0.1");
            tracing::info!("P2P: p2p-offer XMR sell 0.1 6000 EUR banktransfer 'Account: 123456' 3600");
        }
    }

    Ok(())
}
