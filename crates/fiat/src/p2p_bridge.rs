//! Bridge between fiat settlement types and P2P protocol types.
//!
//! Converts between the fiat crate's FiatOffer/FiatTrade and the P2P protocol's
//! P2POffer/P2PTrade, enabling fiat trades to flow through the existing P2P network.

use uuid::Uuid;
use protocol::message::{Side, AMOUNT_SCALE};
use protocol::p2p::{
    CryptoAsset, FiatCurrency as P2PFiatCurrency, PaymentMethod as P2PPaymentMethod,
    P2POffer, P2PTrade, P2PTradeState, FIAT_SCALE,
};

use opendex_types::{Asset, FiatCurrency, ParticipantId, TradeId};

use crate::payment::FiatAmount;
use crate::trade::{FiatOffer, FiatTrade, FiatTradeState, SettlementMethod};

/// Convert a fiat offer to a P2P offer for broadcasting.
pub fn fiat_offer_to_p2p(offer: &FiatOffer) -> P2POffer {
    let crypto_asset = match offer.crypto_asset {
        Asset::Btc => CryptoAsset::BTC,
        Asset::Xmr => CryptoAsset::XMR,
        Asset::Fiat(_) => CryptoAsset::XMR, // Should not happen
    };

    let side = Side::Sell; // Maker sells crypto for fiat

    let currency = fiat_currency_to_p2p(offer.amount.currency.clone());
    let payment_method = settlement_method_to_p2p(&offer.settlement_method);

    // Calculate price in P2P format (fiat per unit of crypto, scaled by FIAT_SCALE)
    let price = if offer.price.denominator > 0 {
        (offer.price.numerator as f64 / offer.price.denominator as f64 * FIAT_SCALE as f64) as u64
    } else {
        0
    };

    // Calculate amount in P2P format (scaled by AMOUNT_SCALE)
    let amount = offer.amount.minor_units;

    P2POffer {
        id: offer.trade_id.0,
        maker: "fiat-maker".to_string(), // Placeholder — set by caller
        crypto_asset,
        side,
        amount,
        price,
        currency,
        payment_method,
        payment_instructions: "".to_string(), // Set by maker
        payment_timeout_secs: offer.settlement_window,
        min_amount: amount / 10,
        max_amount: amount,
        expires_at: 0, // Set by caller
        created_at: 0, // Set by caller
    }
}

/// Convert a P2P offer to a fiat offer.
pub fn p2p_to_fiat_offer(offer: &P2POffer) -> FiatOffer {
    let crypto_asset = match offer.crypto_asset {
        CryptoAsset::BTC => Asset::Btc,
        CryptoAsset::XMR => Asset::Xmr,
    };

    let currency = p2p_to_fiat_currency(&offer.currency);
    let settlement_method = p2p_to_settlement_method(&offer.payment_method, currency.clone());

    let amount = FiatAmount::new(offer.amount, currency.clone());

    // Calculate price from P2P format
    let price = opendex_types::Price::new(
        crypto_asset.clone(),
        Asset::Fiat(currency.clone()),
        offer.price as u128,
        FIAT_SCALE as u128,
    );

    FiatOffer {
        trade_id: TradeId(offer.id),
        maker: ParticipantId(offer.id), // P2P offer ID as maker placeholder
        currency,
        crypto_asset,
        amount,
        price,
        settlement_method,
        settlement_window: offer.payment_timeout_secs,
        bond_requirement: 100, // Default 1%
    }
}

/// Convert a fiat trade to a P2P trade for broadcasting.
pub fn fiat_trade_to_p2p(trade: &FiatTrade) -> P2PTrade {
    let crypto_asset = match trade.crypto_asset {
        Asset::Btc => CryptoAsset::BTC,
        Asset::Xmr => CryptoAsset::XMR,
        Asset::Fiat(_) => CryptoAsset::XMR,
    };

    let currency = fiat_currency_to_p2p(trade.fiat_currency.clone());
    let payment_method = settlement_method_to_p2p(&trade.settlement_method);
    let state = fiat_trade_state_to_p2p(&trade.state);

    // Calculate fiat amount: crypto_amount * price / AMOUNT_SCALE * FIAT_SCALE
    let fiat_amount = trade.fiat_amount.minor_units;

    P2PTrade {
        id: trade.id.0,
        offer_id: Uuid::nil(), // Set by caller
        maker: trade.maker.0.to_string(),
        taker: trade.taker.0.to_string(),
        crypto_asset,
        side: Side::Sell,
        amount: trade.crypto_amount,
        price: trade.fiat_amount.minor_units, // Simplified — should use actual price
        fiat_amount,
        currency,
        payment_method,
        state,
        taker_crypto_address: "".to_string(), // Set by caller
        escrow_address: "".to_string(), // Set by caller
        escrow_tx_hash: None,
        payment_id: trade.payment_id.map(|p| p.0.to_string()),
        fiat_payment_proof: None,
        release_tx_hash: None,
        created_at: trade.created_at,
        updated_at: trade.updated_at,
    }
}

/// Convert a P2P trade state to a fiat trade state.
pub fn p2p_to_fiat_trade_state(state: &P2PTradeState) -> FiatTradeState {
    match state {
        P2PTradeState::OfferTaken => FiatTradeState::Created,
        P2PTradeState::EscrowFunded => FiatTradeState::CryptoReserved,
        P2PTradeState::FiatPaymentPending => FiatTradeState::PaymentPending,
        P2PTradeState::FiatPaymentSent => FiatTradeState::Detected,
        P2PTradeState::FiatPaymentConfirmed => FiatTradeState::Confirmed,
        P2PTradeState::CryptoReleased => FiatTradeState::CryptoRelease,
        P2PTradeState::Settled => FiatTradeState::Complete,
        P2PTradeState::Cancelled => FiatTradeState::Cancelled,
        P2PTradeState::Disputed => FiatTradeState::Disputed,
        P2PTradeState::Expired => FiatTradeState::Failed,
    }
}

/// Convert a fiat trade state to a P2P trade state.
pub fn fiat_trade_state_to_p2p(state: &FiatTradeState) -> P2PTradeState {
    match state {
        FiatTradeState::Created => P2PTradeState::OfferTaken,
        FiatTradeState::Accepted => P2PTradeState::OfferTaken,
        FiatTradeState::CryptoReserved => P2PTradeState::EscrowFunded,
        FiatTradeState::PaymentRequested => P2PTradeState::FiatPaymentPending,
        FiatTradeState::PaymentSubmitted => P2PTradeState::FiatPaymentSent,
        FiatTradeState::PaymentPending => P2PTradeState::FiatPaymentPending,
        FiatTradeState::Detected => P2PTradeState::FiatPaymentSent,
        FiatTradeState::Confirming => P2PTradeState::FiatPaymentSent,
        FiatTradeState::MakerConfirmed => P2PTradeState::FiatPaymentConfirmed,
        FiatTradeState::Confirmed => P2PTradeState::FiatPaymentConfirmed,
        FiatTradeState::CryptoRelease => P2PTradeState::CryptoReleased,
        FiatTradeState::Complete => P2PTradeState::Settled,
        FiatTradeState::ReversalDetected => P2PTradeState::Disputed,
        FiatTradeState::Disputed => P2PTradeState::Disputed,
        FiatTradeState::Failed => P2PTradeState::Expired,
        FiatTradeState::Cancelled => P2PTradeState::Cancelled,
    }
}

/// Convert fiat currency to P2P currency.
pub fn fiat_currency_to_p2p(currency: FiatCurrency) -> P2PFiatCurrency {
    match currency {
        FiatCurrency::GBP => P2PFiatCurrency::GBP,
        FiatCurrency::EUR => P2PFiatCurrency::EUR,
        FiatCurrency::USD => P2PFiatCurrency::USD,
    }
}

/// Convert P2P currency to fiat currency.
pub fn p2p_to_fiat_currency(currency: &P2PFiatCurrency) -> FiatCurrency {
    match currency {
        P2PFiatCurrency::GBP => FiatCurrency::GBP,
        P2PFiatCurrency::EUR => FiatCurrency::EUR,
        P2PFiatCurrency::USD => FiatCurrency::USD,
        P2PFiatCurrency::Other(code) => {
            // Map unknown currencies to GBP as default
            FiatCurrency::GBP
        }
        _ => FiatCurrency::GBP,
    }
}

/// Convert settlement method to P2P payment method.
pub fn settlement_method_to_p2p(method: &SettlementMethod) -> P2PPaymentMethod {
    match method {
        SettlementMethod::BankTransfer { .. } => P2PPaymentMethod::BankTransfer,
        SettlementMethod::CashDeposit { .. } => P2PPaymentMethod::Cash,
        SettlementMethod::CashInPerson { .. } => P2PPaymentMethod::Cash,
    }
}

/// Convert P2P payment method to settlement method.
pub fn p2p_to_settlement_method(
    method: &P2PPaymentMethod,
    currency: FiatCurrency,
) -> SettlementMethod {
    match method {
        P2PPaymentMethod::BankTransfer => SettlementMethod::BankTransfer {
            currency,
        },
        P2PPaymentMethod::Cash => SettlementMethod::CashInPerson {
            currency,
        },
        P2PPaymentMethod::Revolut
        | P2PPaymentMethod::Wise
        | P2PPaymentMethod::PayPal
        | P2PPaymentMethod::Zelle
        | P2PPaymentMethod::WesternUnion => SettlementMethod::BankTransfer {
            currency,
        },
        P2PPaymentMethod::CashByMail => SettlementMethod::CashDeposit {
            currency,
        },
        P2PPaymentMethod::Custom(_) => SettlementMethod::BankTransfer {
            currency,
        },
    }
}

/// Calculate the crypto amount from fiat amount and price.
pub fn crypto_amount_from_fiat(
    fiat_amount: u64,
    price: u64,
) -> u64 {
    if price == 0 {
        return 0;
    }
    // crypto_amount = fiat_amount * AMOUNT_SCALE / price
    (fiat_amount as u128 * AMOUNT_SCALE as u128 / price as u128) as u64
}

/// Calculate the fiat amount from crypto amount and price.
pub fn fiat_amount_from_crypto(
    crypto_amount: u64,
    price: u64,
) -> u64 {
    // fiat_amount = crypto_amount * price / AMOUNT_SCALE
    (crypto_amount as u128 * price as u128 / AMOUNT_SCALE as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fiat_offer() -> FiatOffer {
        FiatOffer {
            trade_id: TradeId(Uuid::new_v4()),
            maker: ParticipantId(Uuid::new_v4()),
            currency: FiatCurrency::GBP,
            crypto_asset: Asset::Btc,
            amount: FiatAmount::new(100000, FiatCurrency::GBP),
            price: opendex_types::Price::new(
                Asset::Btc,
                Asset::Fiat(FiatCurrency::GBP),
                5000000, // 50000.00 GBP/BTC
                1,
            ),
            settlement_method: SettlementMethod::BankTransfer {
                currency: FiatCurrency::GBP,
            },
            settlement_window: 3600,
            bond_requirement: 100,
        }
    }

    #[test]
    fn fiat_offer_to_p2p_conversion() {
        let offer = make_fiat_offer();
        let p2p = fiat_offer_to_p2p(&offer);

        assert_eq!(p2p.crypto_asset, CryptoAsset::BTC);
        assert_eq!(p2p.currency, P2PFiatCurrency::GBP);
        assert_eq!(p2p.payment_method, P2PPaymentMethod::BankTransfer);
    }

    #[test]
    fn p2p_to_fiat_offer_conversion() {
        let offer = make_fiat_offer();
        let p2p = fiat_offer_to_p2p(&offer);
        let fiat = p2p_to_fiat_offer(&p2p);

        assert_eq!(fiat.crypto_asset, Asset::Btc);
        assert_eq!(fiat.currency, FiatCurrency::GBP);
    }

    #[test]
    fn fiat_trade_state_to_p2p_mapping() {
        assert_eq!(
            fiat_trade_state_to_p2p(&FiatTradeState::Created),
            P2PTradeState::OfferTaken,
        );
        assert_eq!(
            fiat_trade_state_to_p2p(&FiatTradeState::CryptoReserved),
            P2PTradeState::EscrowFunded,
        );
        assert_eq!(
            fiat_trade_state_to_p2p(&FiatTradeState::Complete),
            P2PTradeState::Settled,
        );
        assert_eq!(
            fiat_trade_state_to_p2p(&FiatTradeState::Disputed),
            P2PTradeState::Disputed,
        );
    }

    #[test]
    fn p2p_to_fiat_trade_state_mapping() {
        assert_eq!(
            p2p_to_fiat_trade_state(&P2PTradeState::OfferTaken),
            FiatTradeState::Created,
        );
        assert_eq!(
            p2p_to_fiat_trade_state(&P2PTradeState::EscrowFunded),
            FiatTradeState::CryptoReserved,
        );
        assert_eq!(
            p2p_to_fiat_trade_state(&P2PTradeState::Settled),
            FiatTradeState::Complete,
        );
    }

    #[test]
    fn currency_conversion_round_trip() {
        for currency in [FiatCurrency::GBP, FiatCurrency::EUR, FiatCurrency::USD] {
            let p2p = fiat_currency_to_p2p(currency.clone());
            let back = p2p_to_fiat_currency(&p2p);
            assert_eq!(currency, back);
        }
    }

    #[test]
    fn settlement_method_conversion() {
        let method = SettlementMethod::BankTransfer {
            currency: FiatCurrency::GBP,
        };
        let p2p = settlement_method_to_p2p(&method);
        assert_eq!(p2p, P2PPaymentMethod::BankTransfer);
    }

    #[test]
    fn crypto_amount_calculation() {
        // 100000 GBP at 50000 GBP/BTC = 0.002 BTC
        let crypto = crypto_amount_from_fiat(100000, 5000000);
        assert!(crypto > 0);
    }

    #[test]
    fn fiat_amount_calculation() {
        // crypto_amount * price / AMOUNT_SCALE
        // 200000 * 5000000 / 100000000 = 10000
        let fiat = fiat_amount_from_crypto(200000, 5000000);
        assert_eq!(fiat, 10000);
    }
}
