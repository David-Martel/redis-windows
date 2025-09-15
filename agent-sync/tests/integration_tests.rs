//! Integration tests for agent-sync using testcontainers
//!
//! These tests use a real Redis instance in a Docker container to test
//! the full functionality of the coordination modules.

use agent_sync::coordinator::{
    ContextData, ContextResult, ContextScope, Coordinator, CoordinatorConfig, Notification,
    NotificationFilter, NotificationPriority, NotificationType, Task, TaskPriority, TaskResult,
    TaskStatus,
};
use agent_sync::{AgentId, Error, Result, SessionId};
use rstest::*;
use serial_test::serial;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use testcontainers::{clients, Container, Docker};
use testcontainers_modules::redis::Redis;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

/// Redis container wrapper for tests
pub struct RedisContainer<'a> {
    container: Container<'a, Redis>,
    redis_url: String,
}

impl<'a> RedisContainer<'a> {
    pub fn new(docker: &'a clients::Cli) -> Self {
        let container = docker.run(Redis::default());
        let host_port = container.get_host_port_ipv4(6379);
        let redis_url = format!("redis://127.0.0.1:{}", host_port);

        Self {
            container,
            redis_url,
        }
    }

    pub fn url(&self) -> &str {
        &self.redis_url
    }
}

/// Fixture for Docker client
#[fixture]
fn docker() -> clients::Cli {
    clients::Cli::default()
}

/// Fixture for Redis container
#[fixture]
fn redis_container(docker: &clients::Cli) -> RedisContainer<'_> {
    RedisContainer::new(docker)
}

/// Fixture for coordinator configuration
#[fixture]
fn coordinator_config(redis_container: &RedisContainer<'_>) -> CoordinatorConfig {
    CoordinatorConfig {
        redis_url: redis_container.url().to_string(),
        max_concurrent_tasks: 50,
        task_timeout_secs: 60,
        enable_metrics: true,
        notification_buffer_size: 100,
    }
}

/// Fixture for test agent ID
#[fixture]
fn agent_id() -> AgentId {
    AgentId::new()
}

/// Fixture for another test agent ID
#[fixture]
fn other_agent_id() -> AgentId {
    AgentId::new()
}

/// Fixture for test session ID
#[fixture]
fn session_id() -> SessionId {
    SessionId::new()
}

// Task Queue Integration Tests

#[rstest]
#[tokio::test]
#[serial]
async fn test_task_queue_basic_operations(
    coordinator_config: CoordinatorConfig,
    agent_id: AgentId,
    other_agent_id: AgentId,
) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let task_queue = coordinator.task_queue();

    // Create and enqueue a task
    let task = Task::new("integration_test_task", b"test_payload".to_vec(), agent_id)
        .with_priority(TaskPriority::High);

    let task_id = task.id;
    task_queue
        .enqueue(task)
        .await
        .expect("Failed to enqueue task");

    // Verify task queue stats
    let stats = task_queue.get_stats().await.expect("Failed to get stats");
    assert_eq!(stats.pending_count, 1);
    assert_eq!(stats.in_progress_count, 0);

    // Dequeue the task
    let dequeued_task = task_queue
        .dequeue(other_agent_id)
        .await
        .expect("Failed to dequeue task")
        .expect("No task available");

    assert_eq!(dequeued_task.id, task_id);
    assert_eq!(dequeued_task.name, "integration_test_task");
    assert_eq!(dequeued_task.payload, b"test_payload");
    assert_eq!(dequeued_task.priority, TaskPriority::High);

    // Verify task is in progress
    match dequeued_task.status {
        TaskStatus::InProgress {
            agent_id: assigned_agent,
        } => {
            assert_eq!(assigned_agent, other_agent_id);
        }
        _ => panic!("Task should be in progress"),
    }

    // Complete the task
    let result = TaskResult {
        data: b"task_completed".to_vec(),
        duration_ms: 1500,
        metadata: HashMap::new(),
    };

    task_queue
        .complete_task(task_id, result)
        .await
        .expect("Failed to complete task");

    // Verify task completion
    let completed_task = task_queue
        .get_task(task_id)
        .await
        .expect("Failed to get task")
        .expect("Task not found");

    match completed_task.status {
        TaskStatus::Completed { result } => {
            assert_eq!(result.data, b"task_completed");
            assert_eq!(result.duration_ms, 1500);
        }
        _ => panic!("Task should be completed"),
    }

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_task_queue_priority_ordering(
    coordinator_config: CoordinatorConfig,
    agent_id: AgentId,
    other_agent_id: AgentId,
) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let task_queue = coordinator.task_queue();

    // Enqueue tasks with different priorities
    let low_task =
        Task::new("low_priority", b"low".to_vec(), agent_id).with_priority(TaskPriority::Low);

    let high_task =
        Task::new("high_priority", b"high".to_vec(), agent_id).with_priority(TaskPriority::High);

    let normal_task = Task::new("normal_priority", b"normal".to_vec(), agent_id)
        .with_priority(TaskPriority::Normal);

    let critical_task = Task::new("critical_priority", b"critical".to_vec(), agent_id)
        .with_priority(TaskPriority::Critical);

    // Enqueue in random order
    task_queue
        .enqueue(low_task)
        .await
        .expect("Failed to enqueue low task");
    task_queue
        .enqueue(high_task)
        .await
        .expect("Failed to enqueue high task");
    task_queue
        .enqueue(normal_task)
        .await
        .expect("Failed to enqueue normal task");
    task_queue
        .enqueue(critical_task)
        .await
        .expect("Failed to enqueue critical task");

    // Dequeue and verify priority order
    let priorities = vec![
        TaskPriority::Critical,
        TaskPriority::High,
        TaskPriority::Normal,
        TaskPriority::Low,
    ];

    for expected_priority in priorities {
        let task = task_queue
            .dequeue(other_agent_id)
            .await
            .expect("Failed to dequeue")
            .expect("No task available");

        assert_eq!(task.priority, expected_priority);
    }

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_task_queue_retry_logic(
    coordinator_config: CoordinatorConfig,
    agent_id: AgentId,
    other_agent_id: AgentId,
) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let task_queue = coordinator.task_queue();

    // Create a task with limited retries
    let task = Task::new("retry_test", b"payload".to_vec(), agent_id);
    let task_id = task.id;

    task_queue
        .enqueue(task)
        .await
        .expect("Failed to enqueue task");

    // Fail the task multiple times
    for attempt in 0..3 {
        let dequeued_task = task_queue
            .dequeue(other_agent_id)
            .await
            .expect("Failed to dequeue")
            .expect("No task available");

        assert_eq!(dequeued_task.id, task_id);
        assert_eq!(dequeued_task.retry_count, attempt);

        // Fail the task
        task_queue
            .fail_task(task_id, format!("Attempt {} failed", attempt + 1))
            .await
            .expect("Failed to fail task");
    }

    // Fourth attempt should fail permanently
    let dequeued_task = task_queue
        .dequeue(other_agent_id)
        .await
        .expect("Failed to dequeue")
        .expect("No task available");

    assert_eq!(dequeued_task.retry_count, 3);

    // This failure should be permanent
    task_queue
        .fail_task(task_id, "Final failure".to_string())
        .await
        .expect("Failed to fail task");

    // Task should now be in failed state
    let failed_task = task_queue
        .get_task(task_id)
        .await
        .expect("Failed to get task")
        .expect("Task not found");

    match failed_task.status {
        TaskStatus::Failed { error } => {
            assert_eq!(error, "Final failure");
        }
        _ => panic!("Task should be failed"),
    }

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

// Shared Context Integration Tests

#[rstest]
#[tokio::test]
#[serial]
async fn test_shared_context_basic_operations(
    coordinator_config: CoordinatorConfig,
    agent_id: AgentId,
    session_id: SessionId,
) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let shared_context = coordinator.shared_context();

    // Test different scopes
    let scopes = vec![
        ContextScope::Global,
        ContextScope::Session(session_id),
        ContextScope::Agent(agent_id),
        ContextScope::Temporary { ttl_secs: 60 },
    ];

    for scope in scopes {
        let key = "test_key";
        let data = ContextData::new(b"test_data".to_vec(), "String".to_string(), agent_id);

        // Set data
        let result = shared_context
            .set(scope.clone(), key, data.clone())
            .await
            .expect("Failed to set context data");

        match result {
            ContextResult::Success => {}
            _ => panic!("Set should succeed"),
        }

        // Get data
        let retrieved = shared_context
            .get(scope.clone(), key)
            .await
            .expect("Failed to get context data")
            .expect("Data not found");

        assert_eq!(retrieved.data, data.data);
        assert_eq!(retrieved.data_type, data.data_type);
        assert_eq!(retrieved.created_by, data.created_by);

        // Check existence
        let exists = shared_context
            .exists(scope.clone(), key)
            .await
            .expect("Failed to check existence");
        assert!(exists);

        // Delete data
        let deleted = shared_context
            .delete(scope, key)
            .await
            .expect("Failed to delete data");
        assert!(deleted);
    }

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_shared_context_optimistic_locking(
    coordinator_config: CoordinatorConfig,
    agent_id: AgentId,
) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let shared_context = coordinator.shared_context();

    let scope = ContextScope::Global;
    let key = "lock_test";

    // Set initial data
    let initial_data = ContextData::new(b"initial".to_vec(), "String".to_string(), agent_id);

    shared_context
        .set(scope.clone(), key, initial_data)
        .await
        .expect("Failed to set initial data");

    // Get current data
    let current_data = shared_context
        .get(scope.clone(), key)
        .await
        .expect("Failed to get data")
        .expect("Data not found");

    let current_revision = current_data.revision;

    // Update with correct revision
    let mut updated_data = current_data.clone();
    updated_data.data = b"updated".to_vec();

    let result = shared_context
        .update(scope.clone(), key, updated_data, current_revision)
        .await
        .expect("Failed to update data");

    match result {
        ContextResult::Success => {}
        _ => panic!("Update should succeed with correct revision"),
    }

    // Try to update with stale revision
    let mut stale_update = current_data;
    stale_update.data = b"stale".to_vec();

    let result = shared_context
        .update(scope, key, stale_update, current_revision)
        .await
        .expect("Failed to attempt stale update");

    match result {
        ContextResult::ConflictDetected {
            current_revision: detected_revision,
        } => {
            assert!(detected_revision > current_revision);
        }
        _ => panic!("Should detect conflict with stale revision"),
    }

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_shared_context_expiration(coordinator_config: CoordinatorConfig, agent_id: AgentId) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let shared_context = coordinator.shared_context();

    let scope = ContextScope::Temporary { ttl_secs: 1 };
    let key = "expiring_key";

    let data = ContextData::new(b"expiring_data".to_vec(), "String".to_string(), agent_id);

    // Set data with short TTL
    shared_context
        .set(scope.clone(), key, data)
        .await
        .expect("Failed to set expiring data");

    // Data should be available immediately
    let retrieved = shared_context
        .get(scope.clone(), key)
        .await
        .expect("Failed to get data");
    assert!(retrieved.is_some());

    // Wait for expiration
    sleep(Duration::from_secs(2)).await;

    // Data should be expired and cleaned up
    let expired = shared_context
        .get(scope, key)
        .await
        .expect("Failed to get expired data");
    assert!(expired.is_none());

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

// Notification Bus Integration Tests

#[rstest]
#[tokio::test]
#[serial]
async fn test_notification_bus_basic_operations(
    coordinator_config: CoordinatorConfig,
    agent_id: AgentId,
    other_agent_id: AgentId,
) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let notification_bus = coordinator.notification_bus();

    // Subscribe to notifications
    let (subscription_id, mut receiver) = notification_bus
        .subscribe(other_agent_id, NotificationFilter::default())
        .await
        .expect("Failed to subscribe");

    // Create and publish a notification
    let notification = Notification::new(
        NotificationType::Task {
            task_id: Uuid::new_v4(),
            action: "completed".to_string(),
        },
        b"task_completed".to_vec(),
        agent_id,
    )
    .with_priority(NotificationPriority::High);

    let notification_id = notification.id;

    let subscriber_count = notification_bus
        .publish(notification)
        .await
        .expect("Failed to publish notification");

    assert!(subscriber_count > 0);

    // Receive the notification
    let received = timeout(Duration::from_secs(5), receiver.recv())
        .await
        .expect("Timeout waiting for notification")
        .expect("Channel closed");

    assert_eq!(received.id, notification_id);
    assert_eq!(received.payload, b"task_completed");
    assert_eq!(received.sender, agent_id);
    assert_eq!(received.priority, NotificationPriority::High);

    // Check statistics
    let stats = notification_bus.get_stats().await;
    assert_eq!(stats.total_sent, 1);
    assert!(stats.active_subscribers > 0);

    // Unsubscribe
    let unsubscribed = notification_bus
        .unsubscribe(subscription_id)
        .await
        .expect("Failed to unsubscribe");
    assert!(unsubscribed);

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_notification_bus_filtering(
    coordinator_config: CoordinatorConfig,
    agent_id: AgentId,
    other_agent_id: AgentId,
) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let notification_bus = coordinator.notification_bus();

    // Subscribe with a filter for high priority notifications from agent_id
    let filter = NotificationFilter::new()
        .with_sender(agent_id)
        .with_min_priority(NotificationPriority::High);

    let (subscription_id, mut receiver) = notification_bus
        .subscribe(other_agent_id, filter)
        .await
        .expect("Failed to subscribe with filter");

    // Publish a matching notification
    let matching_notification = Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "error".to_string(),
        },
        b"high_priority".to_vec(),
        agent_id,
    )
    .with_priority(NotificationPriority::High);

    let matching_id = matching_notification.id;

    notification_bus
        .publish(matching_notification)
        .await
        .expect("Failed to publish matching notification");

    // Publish a non-matching notification (wrong sender)
    let wrong_sender_notification = Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "error".to_string(),
        },
        b"wrong_sender".to_vec(),
        other_agent_id, // Different sender
    )
    .with_priority(NotificationPriority::High);

    notification_bus
        .publish(wrong_sender_notification)
        .await
        .expect("Failed to publish non-matching notification");

    // Publish a non-matching notification (low priority)
    let low_priority_notification = Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "info".to_string(),
        },
        b"low_priority".to_vec(),
        agent_id,
    )
    .with_priority(NotificationPriority::Low);

    notification_bus
        .publish(low_priority_notification)
        .await
        .expect("Failed to publish low priority notification");

    // Should only receive the matching notification
    let received = timeout(Duration::from_secs(2), receiver.recv())
        .await
        .expect("Timeout waiting for notification")
        .expect("Channel closed");

    assert_eq!(received.id, matching_id);
    assert_eq!(received.payload, b"high_priority");

    // Should not receive any more notifications
    let no_more = timeout(Duration::from_millis(500), receiver.recv()).await;
    assert!(
        no_more.is_err(),
        "Should not receive filtered notifications"
    );

    notification_bus
        .unsubscribe(subscription_id)
        .await
        .expect("Failed to unsubscribe");

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_notification_bus_multiple_subscribers(
    coordinator_config: CoordinatorConfig,
    agent_id: AgentId,
) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let notification_bus = coordinator.notification_bus();

    // Create multiple subscribers
    let mut subscribers = Vec::new();
    for i in 0..3 {
        let subscriber_id = AgentId::new();
        let (subscription_id, receiver) = notification_bus
            .subscribe(subscriber_id, NotificationFilter::default())
            .await
            .expect("Failed to subscribe");

        subscribers.push((subscription_id, receiver));
    }

    // Publish a notification
    let notification = Notification::new(
        NotificationType::Custom {
            category: "broadcast".to_string(),
            data: HashMap::new(),
        },
        b"broadcast_message".to_vec(),
        agent_id,
    );

    let notification_id = notification.id;

    let subscriber_count = notification_bus
        .publish(notification)
        .await
        .expect("Failed to publish notification");

    assert_eq!(subscriber_count, 3);

    // All subscribers should receive the notification
    for (_, mut receiver) in subscribers {
        let received = timeout(Duration::from_secs(2), receiver.recv())
            .await
            .expect("Timeout waiting for notification")
            .expect("Channel closed");

        assert_eq!(received.id, notification_id);
        assert_eq!(received.payload, b"broadcast_message");
    }

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

// Full Coordinator Integration Tests

#[rstest]
#[tokio::test]
#[serial]
async fn test_coordinator_full_workflow(
    coordinator_config: CoordinatorConfig,
    agent_id: AgentId,
    other_agent_id: AgentId,
    session_id: SessionId,
) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    // 1. Store workflow configuration in shared context
    let workflow_config = ContextData::new(
        b"workflow_v1".to_vec(),
        "WorkflowConfig".to_string(),
        agent_id,
    );

    let context_result = coordinator
        .shared_context()
        .set(
            ContextScope::Session(session_id),
            "workflow_config",
            workflow_config,
        )
        .await
        .expect("Failed to set workflow config");

    match context_result {
        ContextResult::Success => {}
        _ => panic!("Context set should succeed"),
    }

    // 2. Subscribe to task completion notifications
    let task_filter = NotificationFilter::new().with_types(vec![NotificationType::Task {
        task_id: Uuid::new_v4(), // Will be ignored in discriminant matching
        action: "".to_string(),
    }]);

    let (subscription_id, mut receiver) = coordinator
        .notification_bus()
        .subscribe(agent_id, task_filter)
        .await
        .expect("Failed to subscribe to task notifications");

    // 3. Create and enqueue a task
    let task = Task::new("workflow_task", b"workflow_data".to_vec(), agent_id)
        .with_priority(TaskPriority::Normal);

    let task_id = task.id;

    coordinator
        .task_queue()
        .enqueue(task)
        .await
        .expect("Failed to enqueue task");

    // 4. Worker agent dequeues and processes the task
    let dequeued_task = coordinator
        .task_queue()
        .dequeue(other_agent_id)
        .await
        .expect("Failed to dequeue task")
        .expect("No task available");

    assert_eq!(dequeued_task.id, task_id);

    // 5. Complete the task
    let task_result = TaskResult {
        data: b"workflow_completed".to_vec(),
        duration_ms: 2000,
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("processed_by".to_string(), other_agent_id.to_string());
            meta
        },
    };

    coordinator
        .task_queue()
        .complete_task(task_id, task_result.clone())
        .await
        .expect("Failed to complete task");

    // 6. Publish task completion notification
    let completion_notification = Notification::new(
        NotificationType::Task {
            task_id,
            action: "completed".to_string(),
        },
        bincode::serialize(&task_result).expect("Failed to serialize result"),
        other_agent_id,
    );

    coordinator
        .notification_bus()
        .publish(completion_notification)
        .await
        .expect("Failed to publish completion notification");

    // 7. Verify workflow completion
    let received_notification = timeout(Duration::from_secs(5), receiver.recv())
        .await
        .expect("Timeout waiting for completion notification")
        .expect("Notification channel closed");

    match received_notification.notification_type {
        NotificationType::Task {
            task_id: notif_task_id,
            action,
        } => {
            assert_eq!(notif_task_id, task_id);
            assert_eq!(action, "completed");
        }
        _ => panic!("Expected task notification"),
    }

    // 8. Store workflow result in context
    let result_data = ContextData::new(
        received_notification.payload,
        "TaskResult".to_string(),
        agent_id,
    );

    coordinator
        .shared_context()
        .set(
            ContextScope::Session(session_id),
            "workflow_result",
            result_data,
        )
        .await
        .expect("Failed to store workflow result");

    // 9. Cleanup
    coordinator
        .notification_bus()
        .unsubscribe(subscription_id)
        .await
        .expect("Failed to unsubscribe");

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

// Stress Tests

#[rstest]
#[tokio::test]
#[serial]
#[ignore] // Remove ignore to run stress tests
async fn test_coordinator_stress_tasks(coordinator_config: CoordinatorConfig, agent_id: AgentId) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let task_count = 100;
    let worker_count = 5;

    // Enqueue many tasks
    for i in 0..task_count {
        let task = Task::new(
            format!("stress_task_{}", i),
            format!("payload_{}", i).into_bytes(),
            agent_id,
        )
        .with_priority(match i % 4 {
            0 => TaskPriority::Low,
            1 => TaskPriority::Normal,
            2 => TaskPriority::High,
            3 => TaskPriority::Critical,
            _ => unreachable!(),
        });

        coordinator
            .task_queue()
            .enqueue(task)
            .await
            .expect("Failed to enqueue stress task");
    }

    // Create worker tasks
    let mut handles = Vec::new();
    let task_queue = std::sync::Arc::new(coordinator.task_queue());

    for worker_id in 0..worker_count {
        let worker_agent = AgentId::new();
        let queue_ref = task_queue.clone();

        let handle = tokio::spawn(async move {
            let mut processed = 0;
            while let Ok(Some(task)) = queue_ref.dequeue(worker_agent).await {
                // Simulate processing
                sleep(Duration::from_millis(10)).await;

                let result = TaskResult {
                    data: format!("processed_by_worker_{}", worker_id).into_bytes(),
                    duration_ms: 10,
                    metadata: HashMap::new(),
                };

                queue_ref
                    .complete_task(task.id, result)
                    .await
                    .expect("Failed to complete task");

                processed += 1;
            }
            processed
        });

        handles.push(handle);
    }

    // Wait for all workers to finish
    let mut total_processed = 0;
    for handle in handles {
        let processed = handle.await.expect("Worker task failed");
        total_processed += processed;
    }

    assert_eq!(total_processed, task_count);

    // Verify final stats
    let stats = coordinator
        .task_queue()
        .get_stats()
        .await
        .expect("Failed to get final stats");

    assert_eq!(stats.pending_count, 0);
    assert_eq!(stats.in_progress_count, 0);
    assert_eq!(stats.completed_count, task_count as u64);

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

// Error handling tests

#[rstest]
#[tokio::test]
#[serial]
async fn test_coordinator_invalid_redis_url() {
    let config = CoordinatorConfig {
        redis_url: "redis://invalid:1234".to_string(),
        ..Default::default()
    };

    let result = Coordinator::new(config).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        Error::Connection(_) => {} // Expected
        other => panic!("Expected connection error, got: {:?}", other),
    }
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_task_queue_nonexistent_task_operations(coordinator_config: CoordinatorConfig) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let nonexistent_task_id = Uuid::new_v4();

    // Try to get nonexistent task
    let task = coordinator
        .task_queue()
        .get_task(nonexistent_task_id)
        .await
        .expect("Failed to query task");
    assert!(task.is_none());

    // Try to complete nonexistent task
    let result = TaskResult {
        data: vec![],
        duration_ms: 0,
        metadata: HashMap::new(),
    };

    let complete_result = coordinator
        .task_queue()
        .complete_task(nonexistent_task_id, result)
        .await;
    assert!(complete_result.is_err());

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}

// Performance tests

#[rstest]
#[tokio::test]
#[serial]
#[ignore] // Remove ignore to run performance tests
async fn test_coordinator_performance_baseline(
    coordinator_config: CoordinatorConfig,
    agent_id: AgentId,
) {
    let coordinator = Coordinator::new(coordinator_config)
        .await
        .expect("Failed to create coordinator");

    let start = std::time::Instant::now();

    // Enqueue 1000 tasks
    for i in 0..1000 {
        let task = Task::new(format!("perf_task_{}", i), vec![], agent_id);
        coordinator
            .task_queue()
            .enqueue(task)
            .await
            .expect("Failed to enqueue task");
    }

    let enqueue_duration = start.elapsed();
    println!("Enqueued 1000 tasks in {:?}", enqueue_duration);

    let start = std::time::Instant::now();

    // Dequeue all tasks
    let mut dequeue_count = 0;
    while coordinator
        .task_queue()
        .dequeue(agent_id)
        .await
        .expect("Failed to dequeue")
        .is_some()
    {
        dequeue_count += 1;
    }

    let dequeue_duration = start.elapsed();
    println!("Dequeued {} tasks in {:?}", dequeue_count, dequeue_duration);

    assert_eq!(dequeue_count, 1000);

    coordinator
        .shutdown()
        .await
        .expect("Failed to shutdown coordinator");
}
