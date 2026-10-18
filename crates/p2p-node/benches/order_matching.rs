use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use p2p_node::orderbook::OrderBook;
use protocol::message::{Asset, Market, Order, Side};
use std::path::PathBuf;
use uuid::Uuid;

fn create_order(id: Uuid, side: Side, price: u64, amount: u64) -> Order {
    Order {
        id,
        trader: format!("trader-{}", id.hyphenated()),
        market: Market {
            base: Asset::BTC,
            quote: Asset::XMR,
        },
        side,
        price,
        amount,
    }
}

fn bench_order_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("order_matching");

    for &order_count in &[10, 100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("upsert_order", order_count),
            &order_count,
            |b, &n| {
                let tmp_dir = std::env::temp_dir()
                    .join(format!("opendex-bench-{}", Uuid::new_v4().hyphenated()));
                let mut book = OrderBook::new(tmp_dir.clone());

                // Pre-populate with resting sell orders
                for i in 0..n {
                    let order = create_order(
                        Uuid::new_v4(),
                        Side::Sell,
                        (100 + i) * 100_000_000,
                        100_000_000,
                    );
                    book.upsert_order(order);
                }

                b.iter(|| {
                    // Insert a buy order that may match against resting sells
                    let buy_order =
                        create_order(Uuid::new_v4(), Side::Buy, 100 * 100_000_000, 100_000_000);
                    book.upsert_order(buy_order)
                });

                // Cleanup temp directory
                let _ = std::fs::remove_dir_all(tmp_dir);
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_order_matching);
criterion_main!(benches);
