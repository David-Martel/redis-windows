//! Criterion benchmarks for agent-sync coordinator modules
//!
//! These benchmarks measure the performance of task queues, shared context,
//! and notification systems under various load conditions.

use agent_sync::coordinator::{
    ContextData, ContextScope, Coordinator, CoordinatorConfig, Notification,
    NotificationFilter, NotificationPriority, NotificationType, Task, TaskPriority, TaskResult,
};
use agent_sync::{AgentId, SessionId};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use tokio::runtime::Runtime;

/// Benchmark configuration
const REDIS_URL: &str = "redis://127.0.0.1:6379";
const SMALL_BATCH: usize = 10;
const MEDIUM_BATCH: usize = 100;
const LARGE_BATCH: usize = 1000;

/// Helper function to create a test coordinator
async fn create_test_coordinator() -> Coordinator {
    let config = CoordinatorConfig {
        redis_url: REDIS_URL.to_string(),
        max_concurrent_tasks: 1000,
        task_timeout_secs: 300,
        enable_metrics: false, // Disable for benchmarks
        notification_buffer_size: 1000,
        namespace: "benchmark".to_string(),
    };

    Coordinator::new(config)
        .await
        .expect("Failed to create coordinator")
}

/// Helper function to create test tasks
fn create_test_tasks(count: usize, agent_id: AgentId) -> Vec<Task> {
    (0..count)
        .map(|i| {
            Task::new(
                format!("benchmark_task_{}", i),
                format!("payload_{}", i).into_bytes(),
                agent_id,
            )
            .with_priority(match i % 4 {
                0 => TaskPriority::Low,
                1 => TaskPriority::Normal,
                2 => TaskPriority::High,
                3 => TaskPriority::Critical,
                _ => unreachable!(),
            })
        })
        .collect()
}

/// Helper function to create test context data
fn create_test_context_data(count: usize, agent_id: AgentId) -> Vec<(String, ContextData)> {
    (0..count)
        .map(|i| {
            let key = format!("benchmark_key_{}", i);
            let data = ContextData::new(
                format!("benchmark_data_{}", i).into_bytes(),
                "String".to_string(),
                agent_id,
            );
            (key, data)
        })
        .collect()
}

/// Helper function to create test notifications
fn create_test_notifications(count: usize, agent_id: AgentId) -> Vec<Notification> {
    (0..count)
        .map(|i| {
            Notification::new(
                NotificationType::Custom {
                    category: "benchmark".to_string(),
                    data: {
                        let mut data = HashMap::new();
                        data.insert("index".to_string(), i.to_string());
                        data
                    },
                },
                format!("notification_payload_{}", i).into_bytes(),
                agent_id,
            )
            .with_priority(match i % 4 {
                0 => NotificationPriority::Low,
                1 => NotificationPriority::Normal,
                2 => NotificationPriority::High,
                3 => NotificationPriority::Critical,
                _ => unreachable!(),
            })
        })
        .collect()
}



// Task Queue Benchmarks

fn bench_task_queue_enqueue(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();

    let mut group = c.benchmark_group("task_queue_enqueue");
    group.throughput(Throughput::Elements(1));

    for &size in &[SMALL_BATCH, MEDIUM_BATCH, LARGE_BATCH] {
        group.bench_with_input(BenchmarkId::new("single_task", size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let coordinator = create_test_coordinator().await;
                let tasks = create_test_tasks(size, agent_id);

                let start = std::time::Instant::now();
                for task in tasks {
                    coordinator
                        .task_queue()
                        .enqueue(black_box(task))
                        .await
                        .expect("Failed to enqueue task");
                }
                let duration = start.elapsed();

                coordinator.shutdown().await.expect("Failed to shutdown");
                duration
            });
        });
    }

    group.finish();
}

fn bench_task_queue_dequeue(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();
    let worker_id = AgentId::new();

    let mut group = c.benchmark_group("task_queue_dequeue");
    group.throughput(Throughput::Elements(1));

    for &size in &[SMALL_BATCH, MEDIUM_BATCH, LARGE_BATCH] {
        group.bench_with_input(BenchmarkId::new("single_task", size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let coordinator = create_test_coordinator().await;
                let tasks = create_test_tasks(size, agent_id);

                // Pre-populate queue
                for task in tasks {
                    coordinator
                        .task_queue()
                        .enqueue(task)
                        .await
                        .expect("Failed to enqueue task");
                }

                let start = std::time::Instant::now();
                for _ in 0..size {
                    let task = coordinator
                        .task_queue()
                        .dequeue(black_box(worker_id))
                        .await
                        .expect("Failed to dequeue task")
                        .expect("No task available");
                    black_box(task);
                }
                let duration = start.elapsed();

                coordinator.shutdown().await.expect("Failed to shutdown");
                duration
            });
        });
    }

    group.finish();
}

fn bench_task_queue_complete_task(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();
    let worker_id = AgentId::new();

    let mut group = c.benchmark_group("task_queue_complete");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single_completion", |b| {
        b.to_async(&rt).iter(|| async {
            let coordinator = create_test_coordinator().await;
            let task = Task::new("benchmark_task", b"payload".to_vec(), agent_id);
            let task_id = task.id;

            coordinator
                .task_queue()
                .enqueue(task)
                .await
                .expect("Failed to enqueue task");

            let _dequeued = coordinator
                .task_queue()
                .dequeue(worker_id)
                .await
                .expect("Failed to dequeue task")
                .expect("No task available");

            let result = TaskResult {
                data: b"completed".to_vec(),
                duration_ms: 100,
                metadata: HashMap::new(),
            };

            let start = std::time::Instant::now();
            coordinator
                .task_queue()
                .complete_task(black_box(task_id), black_box(result))
                .await
                .expect("Failed to complete task");
            let duration = start.elapsed();

            coordinator.shutdown().await.expect("Failed to shutdown");
            duration
        });
    });

    group.finish();
}

fn bench_task_queue_priority_ordering(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();
    let worker_id = AgentId::new();

    let mut group = c.benchmark_group("task_queue_priority");
    group.throughput(Throughput::Elements(1));

    group.bench_function("mixed_priorities", |b| {
        b.to_async(&rt).iter(|| async {
            let coordinator = create_test_coordinator().await;
            let tasks = create_test_tasks(100, agent_id); // Mixed priorities

            // Enqueue all tasks
            for task in tasks {
                coordinator
                    .task_queue()
                    .enqueue(task)
                    .await
                    .expect("Failed to enqueue task");
            }

            let start = std::time::Instant::now();
            let mut dequeued_priorities = Vec::new();
            while let Ok(Some(task)) = coordinator.task_queue().dequeue(black_box(worker_id)).await
            {
                dequeued_priorities.push(task.priority);
            }
            let duration = start.elapsed();

            // Verify priority ordering
            let mut last_priority = TaskPriority::Critical;
            for priority in dequeued_priorities {
                assert!(priority <= last_priority, "Priority ordering violated");
                last_priority = priority;
            }

            coordinator.shutdown().await.expect("Failed to shutdown");
            duration
        });
    });

    group.finish();
}

// Shared Context Benchmarks

fn bench_shared_context_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();

    let mut group = c.benchmark_group("shared_context_set");
    group.throughput(Throughput::Elements(1));

    for &size in &[SMALL_BATCH, MEDIUM_BATCH, LARGE_BATCH] {
        group.bench_with_input(BenchmarkId::new("global_scope", size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let coordinator = create_test_coordinator().await;
                let context_data = create_test_context_data(size, agent_id);
                let scope = ContextScope::Global;

                let start = std::time::Instant::now();
                for (key, data) in context_data {
                    coordinator
                        .shared_context()
                        .set(black_box(scope.clone()), black_box(&key), black_box(data))
                        .await
                        .expect("Failed to set context data");
                }
                let duration = start.elapsed();

                coordinator.shutdown().await.expect("Failed to shutdown");
                duration
            });
        });
    }

    group.finish();
}

fn bench_shared_context_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();

    let mut group = c.benchmark_group("shared_context_get");
    group.throughput(Throughput::Elements(1));

    for &size in &[SMALL_BATCH, MEDIUM_BATCH, LARGE_BATCH] {
        group.bench_with_input(BenchmarkId::new("global_scope", size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let coordinator = create_test_coordinator().await;
                let context_data = create_test_context_data(size, agent_id);
                let scope = ContextScope::Global;

                // Pre-populate context
                for (key, data) in &context_data {
                    coordinator
                        .shared_context()
                        .set(scope.clone(), key, data.clone())
                        .await
                        .expect("Failed to set context data");
                }

                let start = std::time::Instant::now();
                for (key, _) in &context_data {
                    let retrieved = coordinator
                        .shared_context()
                        .get(black_box(scope.clone()), black_box(key))
                        .await
                        .expect("Failed to get context data");
                    black_box(retrieved);
                }
                let duration = start.elapsed();

                coordinator.shutdown().await.expect("Failed to shutdown");
                duration
            });
        });
    }

    group.finish();
}

fn bench_shared_context_different_scopes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();
    let session_id = SessionId::new();

    let mut group = c.benchmark_group("shared_context_scopes");
    group.throughput(Throughput::Elements(1));

    let scopes = vec![
        ("global", ContextScope::Global),
        ("session", ContextScope::Session(session_id)),
        ("agent", ContextScope::Agent(agent_id)),
        ("temporary", ContextScope::Temporary { ttl_secs: 300 }),
    ];

    for (scope_name, scope) in scopes {
        group.bench_function(scope_name, |b| {
            b.to_async(&rt).iter(|| async {
                let coordinator = create_test_coordinator().await;
                let data =
                    ContextData::new(b"benchmark_data".to_vec(), "String".to_string(), agent_id);

                let start = std::time::Instant::now();
                coordinator
                    .shared_context()
                    .set(
                        black_box(scope.clone()),
                        black_box("benchmark_key"),
                        black_box(data),
                    )
                    .await
                    .expect("Failed to set context data");
                let duration = start.elapsed();

                coordinator.shutdown().await.expect("Failed to shutdown");
                duration
            });
        });
    }

    group.finish();
}

fn bench_shared_context_optimistic_locking(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();

    let mut group = c.benchmark_group("shared_context_locking");
    group.throughput(Throughput::Elements(1));

    group.bench_function("update_with_correct_revision", |b| {
        b.to_async(&rt).iter(|| async {
            let coordinator = create_test_coordinator().await;
            let scope = ContextScope::Global;
            let key = "lock_test";

            // Set initial data
            let initial_data =
                ContextData::new(b"initial".to_vec(), "String".to_string(), agent_id);

            coordinator
                .shared_context()
                .set(scope.clone(), key, initial_data)
                .await
                .expect("Failed to set initial data");

            // Get current revision
            let current_data = coordinator
                .shared_context()
                .get(scope.clone(), key)
                .await
                .expect("Failed to get data")
                .expect("Data not found");

            let mut updated_data = current_data.clone();
            updated_data.data = b"updated".to_vec();

            let start = std::time::Instant::now();
            coordinator
                .shared_context()
                .update(
                    black_box(scope),
                    black_box(key),
                    black_box(updated_data),
                    black_box(current_data.revision),
                )
                .await
                .expect("Failed to update data");
            let duration = start.elapsed();

            coordinator.shutdown().await.expect("Failed to shutdown");
            duration
        });
    });

    group.finish();
}

// Notification Bus Benchmarks

fn bench_notification_bus_publish(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();

    let mut group = c.benchmark_group("notification_bus_publish");
    group.throughput(Throughput::Elements(1));

    for &size in &[SMALL_BATCH, MEDIUM_BATCH, LARGE_BATCH] {
        group.bench_with_input(
            BenchmarkId::new("no_subscribers", size),
            &size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let coordinator = create_test_coordinator().await;
                    let notifications = create_test_notifications(size, agent_id);

                    let start = std::time::Instant::now();
                    for notification in notifications {
                        coordinator
                            .notification_bus()
                            .publish(black_box(notification))
                            .await
                            .expect("Failed to publish notification");
                    }
                    let duration = start.elapsed();

                    coordinator.shutdown().await.expect("Failed to shutdown");
                    duration
                });
            },
        );
    }

    group.finish();
}

fn bench_notification_bus_with_subscribers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();

    let mut group = c.benchmark_group("notification_bus_with_subscribers");
    group.throughput(Throughput::Elements(1));

    let subscriber_counts = [1, 5, 10, 20];

    for &subscriber_count in &subscriber_counts {
        group.bench_with_input(
            BenchmarkId::new("publish_to_subscribers", subscriber_count),
            &subscriber_count,
            |b, &subscriber_count| {
                b.to_async(&rt).iter(|| async {
                    let coordinator = create_test_coordinator().await;

                    // Create subscribers
                    let mut subscriptions = Vec::new();
                    for _ in 0..subscriber_count {
                        let subscriber_id = AgentId::new();
                        let (subscription_id, _receiver) = coordinator
                            .notification_bus()
                            .subscribe(subscriber_id, NotificationFilter::default())
                            .await
                            .expect("Failed to subscribe");
                        subscriptions.push(subscription_id);
                    }

                    let notification = Notification::new(
                        NotificationType::Custom {
                            category: "benchmark".to_string(),
                            data: HashMap::new(),
                        },
                        b"benchmark_payload".to_vec(),
                        agent_id,
                    );

                    let start = std::time::Instant::now();
                    let subscriber_count = coordinator
                        .notification_bus()
                        .publish(black_box(notification))
                        .await
                        .expect("Failed to publish notification");
                    let duration = start.elapsed();

                    assert!(subscriber_count > 0);

                    coordinator.shutdown().await.expect("Failed to shutdown");
                    duration
                });
            },
        );
    }

    group.finish();
}

fn bench_notification_filtering(c: &mut Criterion) {
    let _rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();

    let mut group = c.benchmark_group("notification_filtering");
    group.throughput(Throughput::Elements(1));

    group.bench_function("complex_filter_matching", |b| {
        b.iter(|| {
            let filter = NotificationFilter::new()
                .with_sender(agent_id)
                .with_min_priority(NotificationPriority::High)
                .with_metadata_filter("env".to_string(), "prod".to_string());

            let notification = Notification::new(
                NotificationType::System {
                    component: "test".to_string(),
                    level: "error".to_string(),
                },
                b"test_payload".to_vec(),
                agent_id,
            )
            .with_priority(NotificationPriority::High)
            .with_metadata("env".to_string(), "prod".to_string());

            black_box(filter.matches(black_box(&notification)))
        });
    });

    group.finish();
}

// Combined Workflow Benchmarks

fn bench_full_coordinator_workflow(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent_id = AgentId::new();
    let worker_id = AgentId::new();

    let mut group = c.benchmark_group("full_workflow");
    group.throughput(Throughput::Elements(1));

    group.bench_function("complete_task_workflow", |b| {
        b.to_async(&rt).iter(|| async {
            let coordinator = create_test_coordinator().await;

            let start = std::time::Instant::now();

            // 1. Create and enqueue task
            let task = Task::new("workflow_task", b"payload".to_vec(), agent_id);
            let task_id = task.id;
            coordinator
                .task_queue()
                .enqueue(black_box(task))
                .await
                .expect("Failed to enqueue task");

            // 2. Dequeue task
            let dequeued_task = coordinator
                .task_queue()
                .dequeue(black_box(worker_id))
                .await
                .expect("Failed to dequeue task")
                .expect("No task available");
            black_box(dequeued_task);

            // 3. Store intermediate result in context
            let intermediate_data = ContextData::new(
                b"intermediate_result".to_vec(),
                "String".to_string(),
                worker_id,
            );
            coordinator
                .shared_context()
                .set(
                    black_box(ContextScope::Global),
                    black_box("intermediate_result"),
                    black_box(intermediate_data),
                )
                .await
                .expect("Failed to set context");

            // 4. Complete task
            let result = TaskResult {
                data: b"completed".to_vec(),
                duration_ms: 100,
                metadata: HashMap::new(),
            };
            coordinator
                .task_queue()
                .complete_task(black_box(task_id), black_box(result.clone()))
                .await
                .expect("Failed to complete task");

            // 5. Publish completion notification
            let notification = Notification::new(
                NotificationType::Task {
                    task_id,
                    action: "completed".to_string(),
                },
                bincode::serialize(&result).expect("Failed to serialize result"),
                worker_id,
            );
            coordinator
                .notification_bus()
                .publish(black_box(notification))
                .await
                .expect("Failed to publish notification");

            let duration = start.elapsed();
            coordinator.shutdown().await.expect("Failed to shutdown");
            duration
        });
    });

    group.finish();
}

// Serialization Benchmarks

fn bench_serialization(c: &mut Criterion) {
    let agent_id = AgentId::new();

    let mut group = c.benchmark_group("serialization");
    group.throughput(Throughput::Bytes(1024)); // Approximate payload size

    let task = Task::new("serialization_test", vec![0u8; 1024], agent_id);
    let context_data = ContextData::new(vec![0u8; 1024], "TestData".to_string(), agent_id);
    let notification = Notification::new(
        NotificationType::Custom {
            category: "test".to_string(),
            data: HashMap::new(),
        },
        vec![0u8; 1024],
        agent_id,
    );

    group.bench_function("task_serialize", |b| {
        b.iter(|| black_box(bincode::serialize(&task).expect("Failed to serialize task")));
    });

    group.bench_function("context_data_serialize", |b| {
        b.iter(|| {
            black_box(bincode::serialize(&context_data).expect("Failed to serialize context data"))
        });
    });

    group.bench_function("notification_serialize", |b| {
        b.iter(|| {
            black_box(bincode::serialize(&notification).expect("Failed to serialize notification"))
        });
    });

    let serialized_task = bincode::serialize(&task).unwrap();
    let serialized_context = bincode::serialize(&context_data).unwrap();
    let serialized_notification = bincode::serialize(&notification).unwrap();

    group.bench_function("task_deserialize", |b| {
        b.iter(|| {
            black_box(
                bincode::deserialize::<Task>(&serialized_task).expect("Failed to deserialize task"),
            )
        });
    });

    group.bench_function("context_data_deserialize", |b| {
        b.iter(|| {
            black_box(
                bincode::deserialize::<ContextData>(&serialized_context)
                    .expect("Failed to deserialize context data"),
            )
        });
    });

    group.bench_function("notification_deserialize", |b| {
        b.iter(|| {
            black_box(
                bincode::deserialize::<Notification>(&serialized_notification)
                    .expect("Failed to deserialize notification"),
            )
        });
    });

    group.finish();
}

// Criterion configuration
criterion_group!(
    benches,
    bench_task_queue_enqueue,
    bench_task_queue_dequeue,
    bench_task_queue_complete_task,
    bench_task_queue_priority_ordering,
    bench_shared_context_set,
    bench_shared_context_get,
    bench_shared_context_different_scopes,
    bench_shared_context_optimistic_locking,
    bench_notification_bus_publish,
    bench_notification_bus_with_subscribers,
    bench_notification_filtering,
    bench_full_coordinator_workflow,
    bench_serialization,
);

criterion_main!(benches);

