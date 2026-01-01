use criterion::{black_box, criterion_group, criterion_main, Criterion};
use krcbot_kaspacom_gatewayapi::infrastructure::{KaspaComClient, RateLimiter};
use serde_json::json;

/// Benchmark ticker normalization (frequently called operation)
fn benchmark_ticker_normalization(c: &mut Criterion) {
    let mut group = c.benchmark_group("ticker_normalization");
    
    group.bench_function("normalize_uppercase", |b| {
        b.iter(|| {
            black_box(KaspaComClient::normalize_ticker("SLOW"));
        });
    });
    
    group.bench_function("normalize_lowercase", |b| {
        b.iter(|| {
            black_box(KaspaComClient::normalize_ticker("slow"));
        });
    });
    
    group.bench_function("normalize_mixed_case", |b| {
        b.iter(|| {
            black_box(KaspaComClient::normalize_ticker("SlOw"));
        });
    });
    
    group.finish();
}

/// Benchmark rate limiter operations
fn benchmark_rate_limiter(c: &mut Criterion) {
    let mut group = c.benchmark_group("rate_limiter");
    
    group.bench_function("check_rate_limit", |b| {
        let limiter = RateLimiter::new(1000);
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                black_box(limiter.check_rate_limit().await);
            });
        });
    });
    
    group.finish();
}

/// Benchmark JSON serialization/deserialization
fn benchmark_json_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_operations");
    
    let test_data = json!({
        "ticker": "SLOW",
        "price": 0.00015,
        "volume": 1000.5,
        "timestamp": 1735678800,
        "data": {
            "nested": {
                "value": 123,
                "array": [1, 2, 3, 4, 5]
            }
        }
    });
    
    group.bench_function("json_serialize", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&test_data).unwrap());
        });
    });
    
    let json_string = serde_json::to_string(&test_data).unwrap();
    group.bench_function("json_deserialize", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<serde_json::Value>(&json_string).unwrap());
        });
    });
    
    group.finish();
}

/// Benchmark string operations (common in API handlers)
fn benchmark_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");
    
    let test_string = "kaspa.com/api/v1/trade-stats?timeFrame=6h&ticker=SLOW";
    
    group.bench_function("string_clone", |b| {
        b.iter(|| {
            black_box(test_string.to_string());
        });
    });
    
    group.bench_function("string_split", |b| {
        b.iter(|| {
            black_box(test_string.split('?').collect::<Vec<_>>());
        });
    });
    
    group.bench_function("string_contains", |b| {
        b.iter(|| {
            black_box(test_string.contains("trade-stats"));
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_ticker_normalization,
    benchmark_rate_limiter,
    benchmark_json_operations,
    benchmark_string_operations
);
criterion_main!(benches);

