use anyhow::{Result, bail};
use protocol::message::{Asset, Market, Side, TradeState};
use protocol::p2p::{CryptoAsset, FiatCurrency, PaymentMethod};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliArgs {
    pub port: u16,
    pub peer: Option<String>,
    pub new_identity: bool,
    pub gui: bool,
    /// Monero wallet RPC URL, e.g., "http://127.0.0.1:18082"
    pub monero_rpc_url: Option<String>,
    /// Monero wallet RPC username
    pub monero_rpc_user: Option<String>,
    /// Monero wallet RPC password
    pub monero_rpc_pass: Option<String>,
    /// Bitcoin wallet RPC URL, e.g., "http://127.0.0.1:8332"
    pub btc_rpc_url: Option<String>,
    /// Bitcoin wallet RPC username
    pub btc_rpc_user: Option<String>,
    /// Bitcoin wallet RPC password
    pub btc_rpc_pass: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    Run(CliArgs),
    Help,
}

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            port: 4001,
            peer: None,
            new_identity: false,
            gui: false,
            monero_rpc_url: None,
            monero_rpc_user: None,
            monero_rpc_pass: None,
            btc_rpc_url: None,
            btc_rpc_user: None,
            btc_rpc_pass: None,
        }
    }
}

impl CliArgs {
    pub fn usage() -> &'static str {
        "\
OpenDEX P2P node

USAGE:
    p2p-node [OPTIONS]

OPTIONS:
    --port <PORT>          Listening TCP port [default: 4001]
    --peer <MULTIADDR>     Dial a peer on startup
    --new-identity         Generate a new identity instead of loading the saved one
    --gui                  Launch the GUI application
    --monero-rpc <URL>     Monero wallet RPC URL [default: http://127.0.0.1:18082]
    --monero-rpc-user <U>  Monero wallet RPC username
    --monero-rpc-pass <P>  Monero wallet RPC password
    --btc-rpc <URL>        Bitcoin wallet RPC URL [default: http://127.0.0.1:8332]
    --btc-rpc-user <U>     Bitcoin wallet RPC username
    --btc-rpc-pass <P>     Bitcoin wallet RPC password
    -h, --help             Print this help message
"
    }
}

impl fmt::Display for CliArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "port={}, peer={:?}, new_identity={}",
            self.port, self.peer, self.new_identity
        )
    }
}

pub fn parse_args(args: &[String]) -> Result<CliAction> {
    let mut config = CliArgs::default();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => return Ok(CliAction::Help),

            "--port" => {
                i += 1;

                let value = args
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("--port requires a value"))?;

                config.port = value
                    .parse::<u16>()
                    .map_err(|_| anyhow::anyhow!("invalid port: {value}"))?;

                if config.port == 0 {
                    bail!("port must be between 1 and 65535");
                }
            }

            "--peer" => {
                i += 1;

                let value = args
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("--peer requires a multiaddr"))?;

                if value.trim().is_empty() {
                    bail!("--peer requires a multiaddr");
                }

                config.peer = Some(value.clone());
            }

            "--new-identity" => {
                config.new_identity = true;
            }

            "--gui" => {
                config.gui = true;
            }

            "--monero-rpc" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("--monero-rpc requires a URL"))?;
                config.monero_rpc_url = Some(value.clone());
            }

            "--monero-rpc-user" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("--monero-rpc-user requires a value"))?;
                config.monero_rpc_user = Some(value.clone());
            }

            "--monero-rpc-pass" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("--monero-rpc-pass requires a value"))?;
                config.monero_rpc_pass = Some(value.clone());
            }

            "--btc-rpc" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("--btc-rpc requires a URL"))?;
                config.btc_rpc_url = Some(value.clone());
            }

            "--btc-rpc-user" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("--btc-rpc-user requires a value"))?;
                config.btc_rpc_user = Some(value.clone());
            }

            "--btc-rpc-pass" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("--btc-rpc-pass requires a value"))?;
                config.btc_rpc_pass = Some(value.clone());
            }

            value => {
                bail!("unknown argument: {value}\n\n{}", CliArgs::usage());
            }
        }

        i += 1;
    }

    Ok(CliAction::Run(config))
}

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
    /// Show the on-chain Monero wallet address
    WalletAddress,
    /// Show the on-chain Monero wallet balance
    WalletBalance,
    /// Show the on-chain Bitcoin wallet address
    BtcWalletAddress,
    /// Show the on-chain Bitcoin wallet balance
    BtcWalletBalance,
    /// Create a P2P offer to buy/sell crypto (XMR or BTC) for fiat
    P2POffer {
        crypto_asset: CryptoAsset,
        side: Side,
        amount: f64,
        price: f64,
        currency: FiatCurrency,
        payment_method: PaymentMethod,
        payment_instructions: String,
        timeout_secs: u64,
    },
    /// Take an existing P2P offer
    P2PTakeOffer {
        offer_id: Uuid,
        amount: f64,
    },
    /// Show active P2P offers
    ShowP2POffers,
    /// Show active P2P trades
    ShowP2PTrades,
    /// Advance a P2P trade state
    P2PTradeState {
        trade_id: Uuid,
        state: String,
    },
    /// Cancel a P2P offer
    P2PCancelOffer {
        offer_id: Uuid,
    },
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

    if input.trim() == "wallet-address" {
        return Command::WalletAddress;
    }

    if input.trim() == "wallet-balance" {
        return Command::WalletBalance;
    }

    if input.trim() == "btc-wallet-address" {
        return Command::BtcWalletAddress;
    }

    if input.trim() == "btc-wallet-balance" {
        return Command::BtcWalletBalance;
    }

    if parts.len() == 2 && parts[0].eq_ignore_ascii_case("cancel") {
        return match Uuid::parse_str(parts[1]) {
            Ok(id) => Command::Cancel { id },
            Err(_) => Command::Unknown,
        };
    }

    // --- P2P commands ---

    if input.trim() == "p2p-offers" {
        return Command::ShowP2POffers;
    }

    if input.trim() == "p2p-trades" {
        return Command::ShowP2PTrades;
    }

    // p2p-offer <xmr|btc> <buy|sell> <amount> <price> <currency> <payment_method> <instructions> [timeout]
    if parts.len() >= 8 && parts[0].eq_ignore_ascii_case("p2p-offer") {
        let crypto_asset = match parts[1].to_uppercase().as_str() {
            "XMR" => CryptoAsset::XMR,
            "BTC" => CryptoAsset::BTC,
            _ => return Command::Unknown,
        };

        let side = match parts[2].to_lowercase().as_str() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            _ => return Command::Unknown,
        };

        let amount = match parts[3].parse::<f64>() {
            Ok(v) if v > 0.0 => v,
            _ => return Command::Unknown,
        };

        let price = match parts[4].parse::<f64>() {
            Ok(v) if v > 0.0 => v,
            _ => return Command::Unknown,
        };

        let currency = match parts[5].to_uppercase().as_str() {
            "USD" => FiatCurrency::USD,
            "EUR" => FiatCurrency::EUR,
            "GBP" => FiatCurrency::GBP,
            "CAD" => FiatCurrency::CAD,
            "AUD" => FiatCurrency::AUD,
            "CHF" => FiatCurrency::CHF,
            "JPY" => FiatCurrency::JPY,
            "CNY" => FiatCurrency::CNY,
            "INR" => FiatCurrency::INR,
            "BRL" => FiatCurrency::BRL,
            "RUB" => FiatCurrency::RUB,
            "KRW" => FiatCurrency::KRW,
            "TRY" => FiatCurrency::TRY,
            other => FiatCurrency::Other(other.to_string()),
        };

        let payment_method = match parts[6].to_lowercase().as_str() {
            "banktransfer" | "bank" => PaymentMethod::BankTransfer,
            "cash" => PaymentMethod::Cash,
            "revolut" => PaymentMethod::Revolut,
            "wise" => PaymentMethod::Wise,
            "paypal" => PaymentMethod::PayPal,
            "zelle" => PaymentMethod::Zelle,
            "cashbymail" => PaymentMethod::CashByMail,
            "westernunion" => PaymentMethod::WesternUnion,
            other => PaymentMethod::Custom(other.to_string()),
        };

        // Collect remaining parts as payment instructions (join with spaces)
        let instructions = parts[7..parts.len() - 1].join(" ");
        let timeout_secs = if parts.len() > 8 {
            parts[parts.len() - 1].parse::<u64>().unwrap_or(3600)
        } else {
            3600
        };

        return Command::P2POffer {
            crypto_asset,
            side,
            amount,
            price,
            currency,
            payment_method,
            payment_instructions: instructions,
            timeout_secs,
        };
    }

    // p2p-take <offer_id> <amount>
    if parts.len() == 3 && parts[0].eq_ignore_ascii_case("p2p-take") {
        let offer_id = match Uuid::parse_str(parts[1]) {
            Ok(id) => id,
            Err(_) => return Command::Unknown,
        };
        let amount = match parts[2].parse::<f64>() {
            Ok(v) if v > 0.0 => v,
            _ => return Command::Unknown,
        };
        return Command::P2PTakeOffer { offer_id, amount };
    }

    // p2p-trade-state <trade_id> <state>
    if parts.len() == 3 && parts[0].eq_ignore_ascii_case("p2p-trade-state") {
        let trade_id = match Uuid::parse_str(parts[1]) {
            Ok(id) => id,
            Err(_) => return Command::Unknown,
        };
        return Command::P2PTradeState {
            trade_id,
            state: parts[2].to_string(),
        };
    }

    // p2p-cancel <offer_id>
    if parts.len() == 2 && parts[0].eq_ignore_ascii_case("p2p-cancel") {
        let offer_id = match Uuid::parse_str(parts[1]) {
            Ok(id) => id,
            Err(_) => return Command::Unknown,
        };
        return Command::P2PCancelOffer { offer_id };
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
