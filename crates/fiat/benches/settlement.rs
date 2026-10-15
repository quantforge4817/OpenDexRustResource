use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use opendex_fiat::{
    FeeSchedule, FiatAmount, FiatOffer, FiatTrade, FiatTradeState, SettlementDeadlines,
    SettlementMethod,
};
use opendex_types::{Asset, FiatCurrency, ParticipantId, Price, TradeId};
use uuid::Uuid;

fn make_offer() -> FiatOffer {
    FiatOffer {
        trade_id: TradeId(Uuid::new_v4()),
        maker: ParticipantId(Uuid::new_v4()),
        currency: FiatCurrency::GBP,
        crypto_asset: Asset::Btc,
        amount: FiatAmount::new(7_500_000, FiatCurrency::GBP),
        price: Price::new(
            Asset::Btc,
            Asset::Fiat(FiatCurrency::GBP),
            75_000,
            1,
        ),
        settlement_method: SettlementMethod::BankTransfer {
            currency: FiatCurrency::GBP,
        },
        settlement_window: 3600,
        bond_requirement: 500,
    }
}

fn make_trade() -> FiatTrade {
    FiatTrade {
        id: TradeId(Uuid::new_v4()),
        maker: ParticipantId(Uuid::new_v4()),
        taker: ParticipantId(Uuid::new_v4()),
        crypto_asset: Asset::Btc,
        fiat_currency: FiatCurrency::GBP,
        crypto_amount: 1_000_000,
        fiat_amount: FiatAmount::new(7_500_000, FiatCurrency::GBP),
        settlement_method: SettlementMethod::BankTransfer {
            currency: FiatCurrency::GBP,
        },
        fee_schedule: FeeSchedule::bank_transfer_default(),
        state: FiatTradeState::Created,
        maker_bank_details: None,
        taker_crypto_address: "bc1qtest123".to_string(),
        payment_id: None,
        payment_reference: None,
        deadlines: SettlementDeadlines {
            payment_deadline: 1_000_000,
            confirmation_deadline: 2_000_000,
            dispute_deadline: 3_000_000,
        },
        created_at: 1,
        updated_at: 1,
    }
}

/// Benchmark the full happy-path state transition lifecycle
fn bench_trade_lifecycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("settlement");

    group.bench_function("trade_lifecycle_transitions", |b| {
        b.iter(|| {
            let mut trade = make_trade();
            // Full happy path: Created → Accepted → CryptoReserved → PaymentRequested
            // → PaymentPending → Detected → Confirming → Confirmed → CryptoRelease → Complete
            let _ = trade.transition(FiatTradeState::Accepted, 2);
            let _ = trade.transition(FiatTradeState::CryptoReserved, 3);
            let _ = trade.transition(FiatTradeState::PaymentRequested, 4);
            let _ = trade.transition(FiatTradeState::PaymentPending, 5);
            let _ = trade.transition(FiatTradeState::Detected, 6);
            let _ = trade.transition(FiatTradeState::Confirming, 7);
            let _ = trade.transition(FiatTradeState::Confirmed, 8);
            let _ = trade.transition(FiatTradeState::CryptoRelease, 9);
            let _ = trade.transition(FiatTradeState::Complete, 10);
        })
    });
}

/// Benchmark single state transition calls
fn bench_single_transition(c: &mut Criterion) {
    let mut group = c.benchmark_group("settlement");

    group.bench_function("single_state_transition", |b| {
        b.iter_custom(|iters| {
            let mut time = std::time::Duration::ZERO;
            for _ in 0..iters {
                let mut trade = make_trade();
                let start = std::time::Instant::now();
                let _ = trade.transition(FiatTradeState::Accepted, 2);
                time += start.elapsed();
            }
            time
        })
    });
}

/// Benchmark FiatTrade JSON serialization
fn bench_trade_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("settlement");

    let trade = make_trade();

    group.bench_function("serialize_fiat_trade", |b| {
        b.iter(|| {
            serde_json::to_string(&trade).unwrap();
        })
    });
}

/// Benchmark FiatTrade JSON deserialization
fn bench_trade_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("settlement");

    let trade = make_trade();
    let json = serde_json::to_string(&trade).unwrap();

    group.bench_function("deserialize_fiat_trade", |b| {
        b.iter(|| {
            serde_json::from_str::<FiatTrade>(&json).unwrap();
        })
    });
}

/// Benchmark crypto amount calculation from offer
fn bench_crypto_amount_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("settlement");

    let offer = make_offer();

    group.bench_function("calculate_crypto_amount", |b| {
        b.iter(|| {
            offer.calculate_crypto_amount();
        })
    });
}

/// Benchmark batch trade creation (simulating high-throughput scenario)
fn bench_batch_trade_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("settlement");

    for &count in &[10, 100, 1_000] {
        group.bench_with_input(
            BenchmarkId::new("batch_create_trades", count),
            &count,
            |b, &n| {
                b.iter(|| {
                    for _ in 0..n {
                        let _trade = make_trade();
                    }
                })
            },
        );
    }
}

criterion_group!(
    benches,
    bench_trade_lifecycle,
    bench_single_transition,
    bench_trade_serialization,
    bench_trade_deserialization,
    bench_crypto_amount_calculation,
    bench_batch_trade_creation,
);
criterion_main!(benches);
