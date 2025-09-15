//! Performance benchmarks for agent-sync Redis operations
//!
//! This benchmark suite validates that the library meets sub-millisecond
//! performance targets for critical operations.

use agent_sync::{AgentMessage, AgentSyncClient, SerializationStrategy};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;

async fn setup_client() -> AgentSyncClient {
    AgentSyncClient::new_optimized_for_agents("redis://localhost:6379".to_string())
        .await
        .expect("Failed to create Redis client for benchmarks")
}

fn benchmark_basic_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_client());

    let mut group = c.benchmark_group("basic_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // Benchmark SET operations
    group.bench_function("set_string", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("bench:set:{}", fastrand::u64(..));
            black_box(client.set(&key, &"test_value").await.unwrap());
        });
    });

    // Benchmark GET operations
    group.bench_function("get_string", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("bench:get:{}", fastrand::u64(..));
            // Pre-populate key
            client.set(&key, &"test_value").await.unwrap();
            let result: Option<String> = black_box(client.get(&key).await.unwrap());
            assert!(result.is_some());
        });
    });

    // Benchmark INCR operations
    group.bench_function("incr", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("bench:counter:{}", fastrand::u64(..));
            black_box(client.incr(&key).await.unwrap());
        });
    });

    group.finish();
}

fn benchmark_hash_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_client());

    let mut group = c.benchmark_group("hash_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // Benchmark HSET operations
    group.bench_function("hset", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("bench:hash:{}", fastrand::u64(..));
            let field = format!("field_{}", fastrand::u32(..));
            black_box(client.hset(&key, &field, &"test_value").await.unwrap());
        });
    });

    // Benchmark HGET operations
    group.bench_function("hget", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("bench:hash:{}", fastrand::u64(..));
            let field = format!("field_{}", fastrand::u32(..));
            // Pre-populate
            client.hset(&key, &field, &"test_value").await.unwrap();
            let result: Option<String> = black_box(client.hget(&key, &field).await.unwrap());
            assert!(result.is_some());
        });
    });

    group.finish();
}

fn benchmark_list_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_client());

    let mut group = c.benchmark_group("list_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // Benchmark LPUSH operations
    group.bench_function("lpush", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("bench:list:{}", fastrand::u64(..));
            let value = format!("value_{}", fastrand::u64(..));
            black_box(client.lpush(&key, &[&value]).await.unwrap());
        });
    });

    // Benchmark RPOP operations
    group.bench_function("rpop", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("bench:list:{}", fastrand::u64(..));
            // Pre-populate list
            client.lpush(&key, &[&"test_value"]).await.unwrap();
            let result: Option<String> = black_box(client.rpop(&key).await.unwrap());
            assert!(result.is_some());
        });
    });

    group.finish();
}

fn benchmark_agent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_client());

    let mut group = c.benchmark_group("agent_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(500);

    // Benchmark agent registration
    group.bench_function("register_agent", |b| {
        b.to_async(&rt).iter(|| async {
            let agent_id = format!("bench_agent_{}", fastrand::u64(..));
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("role".to_string(), json!("test_agent"));
            metadata.insert("version".to_string(), json!("1.0"));
            black_box(client.register_agent(&agent_id, metadata).await.unwrap());
        });
    });

    // Benchmark agent heartbeat
    group.bench_function("agent_heartbeat", |b| {
        b.to_async(&rt).iter(|| async {
            let agent_id = format!("bench_agent_{}", fastrand::u64(..));
            // Pre-register agent
            client
                .register_agent(&agent_id, std::collections::HashMap::new())
                .await
                .unwrap();
            black_box(client.agent_heartbeat(&agent_id).await.unwrap());
        });
    });

    // Benchmark message sending
    group.bench_function("send_agent_message", |b| {
        b.to_async(&rt).iter(|| async {
            let agent_id = format!("bench_agent_{}", fastrand::u64(..));
            let message = AgentMessage::new(
                "sender_agent".to_string(),
                "test_message".to_string(),
                json!({"content": "benchmark message"}),
            );
            black_box(
                client
                    .send_agent_message(&agent_id, &message)
                    .await
                    .unwrap(),
            );
        });
    });

    group.finish();
}

fn benchmark_serialization_strategies(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("serialization_strategies");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    let test_data = json!({
        "agent_id": "test_agent_123",
        "timestamp": 1234567890,
        "payload": {
            "message": "This is a test message with some content",
            "priority": "high",
            "metadata": {
                "source": "benchmark",
                "batch_id": 42,
                "tags": ["test", "performance", "benchmark"]
            }
        }
    });

    for strategy in &[
        SerializationStrategy::Json,
        SerializationStrategy::MessagePack,
    ] {
        let client = rt.block_on(async {
            AgentSyncClient::with_serialization("redis://localhost:6379".to_string(), *strategy)
                .await
                .unwrap()
        });

        group.bench_with_input(
            BenchmarkId::new("set_complex_data", format!("{:?}", strategy)),
            strategy,
            |b, _strategy| {
                b.to_async(&rt).iter(|| async {
                    let key = format!("bench:complex:{}", fastrand::u64(..));
                    black_box(client.set(&key, &test_data).await.unwrap());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("get_complex_data", format!("{:?}", strategy)),
            strategy,
            |b, _strategy| {
                b.to_async(&rt).iter(|| async {
                    let key = format!("bench:complex:{}", fastrand::u64(..));
                    // Pre-populate
                    client.set(&key, &test_data).await.unwrap();
                    let result: Option<serde_json::Value> =
                        black_box(client.get(&key).await.unwrap());
                    assert!(result.is_some());
                });
            },
        );
    }

    group.finish();
}

fn benchmark_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_client());

    let mut group = c.benchmark_group("concurrent_operations");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(100);

    // Test with different levels of concurrency
    for concurrency in &[1, 10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_set", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = Vec::with_capacity(concurrency);

                    for i in 0..concurrency {
                        let client = &client;
                        let handle = tokio::spawn(async move {
                            let key = format!("bench:concurrent:{}:{}", concurrency, i);
                            client.set(&key, &i).await.unwrap();
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        black_box(handle.await.unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

fn benchmark_latency_targets(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_client());

    let mut group = c.benchmark_group("latency_targets");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(2000);

    // Set specific latency thresholds for different operations
    // These should complete in sub-millisecond times under optimal conditions

    // Critical path: Simple GET/SET operations
    group.bench_function("critical_path_set", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("bench:critical:{}", fastrand::u64(..));
            let start = std::time::Instant::now();
            client.set(&key, &"value").await.unwrap();
            let duration = start.elapsed();

            // Log if we exceed 1ms (should be rare under optimal conditions)
            if duration > Duration::from_micros(1000) {
                eprintln!("SET operation took {:?} - exceeds 1ms target", duration);
            }

            black_box(duration);
        });
    });

    group.bench_function("critical_path_get", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("bench:critical:{}", fastrand::u64(..));
            // Pre-populate
            client.set(&key, &"value").await.unwrap();

            let start = std::time::Instant::now();
            let _: Option<String> = client.get(&key).await.unwrap();
            let duration = start.elapsed();

            // Log if we exceed 1ms
            if duration > Duration::from_micros(1000) {
                eprintln!("GET operation took {:?} - exceeds 1ms target", duration);
            }

            black_box(duration);
        });
    });

    // Agent operations should complete within 2ms
    group.bench_function("agent_heartbeat_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let agent_id = format!("bench_agent_{}", fastrand::u64(..));
            client
                .register_agent(&agent_id, std::collections::HashMap::new())
                .await
                .unwrap();

            let start = std::time::Instant::now();
            client.agent_heartbeat(&agent_id).await.unwrap();
            let duration = start.elapsed();

            // Log if we exceed 2ms target
            if duration > Duration::from_micros(2000) {
                eprintln!("Heartbeat took {:?} - exceeds 2ms target", duration);
            }

            black_box(duration);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_basic_operations,
    benchmark_hash_operations,
    benchmark_list_operations,
    benchmark_agent_operations,
    benchmark_serialization_strategies,
    benchmark_concurrent_operations,
    benchmark_latency_targets
);

criterion_main!(benches);
