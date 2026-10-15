use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use protocol::message::{
    Asset, Market, Message, MessageType, Order, OrderCancelled, Side, Trade, TradeState,
    TradeStateChanged,
};
use serde_json;
use uuid::Uuid;

fn sample_message(msg_type: MessageType) -> Message {
    let payload = match msg_type {
        MessageType::OrderCreated => {
            serde_json::to_string(&Order {
                id: Uuid::new_v4(),
                trader: "trader-123".to_string(),
                market: Market {
                    base: Asset::BTC,
                    quote: Asset::XMR,
                },
                side: Side::Buy,
                price: 100 * 100_000_000,
                amount: 100_000_000,
            })
            .unwrap()
        }
        MessageType::OrderCancelled => {
            serde_json::to_string(&OrderCancelled {
                order_id: Uuid::new_v4(),
                trader: "trader-123".to_string(),
            })
            .unwrap()
        }
        MessageType::TradeCreated => {
            serde_json::to_string(&Trade {
                id: Uuid::new_v4(),
                buy_order: Uuid::new_v4(),
                sell_order: Uuid::new_v4(),
                maker: "maker".to_string(),
                taker: "taker".to_string(),
                buyer: "buyer".to_string(),
                seller: "seller".to_string(),
                market: Market {
                    base: Asset::BTC,
                    quote: Asset::XMR,
                },
                price: 100 * 100_000_000,
                amount: 100_000_000,
                fill_sequence: 0,
                state: TradeState::Created,
                created_at: 1,
                updated_at: 1,
            })
            .unwrap()
        }
        MessageType::TradeStateChanged => {
            serde_json::to_string(&TradeStateChanged {
                trade_id: Uuid::new_v4(),
                state: TradeState::Accepted,
                timestamp: 1,
            })
            .unwrap()
        }
        _ => "{}".to_string(),
    };

    Message {
        message_type: msg_type.clone(),
        sender: "peer-abc".to_string(),
        payload,
        timestamp: 1_700_000_000,
        schema_version: 1,
    }
}

fn bench_message_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_processing");

    let message_types = [
        MessageType::OrderCreated,
        MessageType::OrderCancelled,
        MessageType::TradeCreated,
        MessageType::TradeStateChanged,
    ];

    for msg_type in &message_types {
        let msg = sample_message(msg_type.clone());
        let json = serde_json::to_string(&msg).unwrap();

        group.bench_with_input(
            BenchmarkId::new("deserialize", format!("{:?}", msg_type)),
            &json,
            |b, json| {
                b.iter(|| {
                    serde_json::from_str::<Message>(json).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_processing");

    let message_types = [
        MessageType::OrderCreated,
        MessageType::OrderCancelled,
        MessageType::TradeCreated,
        MessageType::TradeStateChanged,
    ];

    for msg_type in &message_types {
        let msg = sample_message(msg_type.clone());

        group.bench_with_input(
            BenchmarkId::new("serialize", format!("{:?}", msg_type)),
            &msg,
            |b, msg| {
                b.iter(|| {
                    serde_json::to_string(msg).unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_message_deserialization, bench_message_serialization);
criterion_main!(benches);
