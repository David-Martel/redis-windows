//! Unit tests for task queue module

use super::*;
use rstest::*;
use std::time::Duration;

/// Test fixture for creating test tasks
#[fixture]
fn test_agent_id() -> AgentId {
    AgentId::new()
}

/// Test fixture for creating a test task
#[fixture]
fn test_task(test_agent_id: AgentId) -> Task {
    Task::new("test_task", b"test_payload".to_vec(), test_agent_id)
}

/// Test fixture for creating a high priority task
#[fixture]
fn high_priority_task(test_agent_id: AgentId) -> Task {
    Task::new(
        "high_priority_task",
        b"urgent_payload".to_vec(),
        test_agent_id,
    )
    .with_priority(TaskPriority::High)
}

/// Test fixture for creating a task with deadline
#[fixture]
fn task_with_deadline(test_agent_id: AgentId) -> Task {
    let deadline = SystemTime::now() + Duration::from_secs(300);
    Task::new("deadline_task", b"deadline_payload".to_vec(), test_agent_id).with_deadline(deadline)
}

#[rstest]
fn test_task_creation(test_agent_id: AgentId) {
    let task = Task::new("test_task", b"payload".to_vec(), test_agent_id);

    assert_eq!(task.name, "test_task");
    assert_eq!(task.payload, b"payload");
    assert_eq!(task.created_by, test_agent_id);
    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.priority, TaskPriority::Normal);
    assert_eq!(task.retry_count, 0);
    assert_eq!(task.max_retries, 3);
    assert_eq!(task.timeout_secs, 300);
    assert!(task.deadline.is_none());
    assert!(task.tags.is_empty());
}

#[rstest]
fn test_task_with_priority(test_agent_id: AgentId) {
    let task =
        Task::new("test", b"payload".to_vec(), test_agent_id).with_priority(TaskPriority::Critical);

    assert_eq!(task.priority, TaskPriority::Critical);
}

#[rstest]
fn test_task_with_deadline(test_agent_id: AgentId) {
    let future_time = SystemTime::now() + Duration::from_secs(60);
    let task = Task::new("test", b"payload".to_vec(), test_agent_id).with_deadline(future_time);

    assert!(task.deadline.is_some());
    assert!(!task.is_expired());
}

#[rstest]
fn test_task_with_timeout(test_agent_id: AgentId) {
    let task = Task::new("test", b"payload".to_vec(), test_agent_id)
        .with_timeout(Duration::from_secs(120));

    assert_eq!(task.timeout_secs, 120);
}

#[rstest]
fn test_task_with_tags(test_agent_id: AgentId) {
    let tags = vec!["urgent".to_string(), "processing".to_string()];
    let task = Task::new("test", b"payload".to_vec(), test_agent_id).with_tags(tags.clone());

    assert_eq!(task.tags, tags);
}

#[rstest]
fn test_task_expiry() {
    let agent_id = AgentId::new();

    // Task with past deadline should be expired
    let past_deadline = SystemTime::now() - Duration::from_secs(60);
    let expired_task =
        Task::new("expired", b"test".to_vec(), agent_id).with_deadline(past_deadline);
    assert!(expired_task.is_expired());

    // Task with future deadline should not be expired
    let future_deadline = SystemTime::now() + Duration::from_secs(60);
    let valid_task = Task::new("valid", b"test".to_vec(), agent_id).with_deadline(future_deadline);
    assert!(!valid_task.is_expired());

    // Task without deadline should never be expired
    let no_deadline_task = Task::new("no_deadline", b"test".to_vec(), agent_id);
    assert!(!no_deadline_task.is_expired());
}

#[rstest]
fn test_task_retry_logic() {
    let agent_id = AgentId::new();
    let mut task = Task::new("retry_test", b"test".to_vec(), agent_id);

    // Fresh task should be retryable
    assert!(task.can_retry());

    // After some retries, still retryable
    task.retry_count = 2;
    assert!(task.can_retry());

    // After max retries, not retryable
    task.retry_count = 3;
    assert!(!task.can_retry());

    // Beyond max retries, definitely not retryable
    task.retry_count = 5;
    assert!(!task.can_retry());
}

#[rstest]
fn test_task_priority_ordering() {
    assert!(TaskPriority::Critical > TaskPriority::High);
    assert!(TaskPriority::High > TaskPriority::Normal);
    assert!(TaskPriority::Normal > TaskPriority::Low);

    // Test numeric values
    assert_eq!(TaskPriority::Low as i32, 0);
    assert_eq!(TaskPriority::Normal as i32, 1);
    assert_eq!(TaskPriority::High as i32, 2);
    assert_eq!(TaskPriority::Critical as i32, 3);
}

#[rstest]
fn test_task_status_variants() {
    let agent_id = AgentId::new();
    let _task_id = Uuid::new_v4();

    let pending = TaskStatus::Pending;
    let in_progress = TaskStatus::InProgress { agent_id };
    let completed = TaskStatus::Completed {
        result: TaskResult {
            data: b"result".to_vec(),
            duration_ms: 1000,
            metadata: HashMap::new(),
        },
    };
    let failed = TaskStatus::Failed {
        error: "Test error".to_string(),
    };
    let cancelled = TaskStatus::Cancelled;
    let timed_out = TaskStatus::TimedOut;

    // Verify all status variants can be created
    match pending {
        TaskStatus::Pending => {}
        _ => panic!("Unexpected status"),
    }
    match in_progress {
        TaskStatus::InProgress { agent_id: _ } => {}
        _ => panic!("Unexpected status"),
    }
    match completed {
        TaskStatus::Completed { result: _ } => {}
        _ => panic!("Unexpected status"),
    }
    match failed {
        TaskStatus::Failed { error: _ } => {}
        _ => panic!("Unexpected status"),
    }
    match cancelled {
        TaskStatus::Cancelled => {}
        _ => panic!("Unexpected status"),
    }
    match timed_out {
        TaskStatus::TimedOut => {}
        _ => panic!("Unexpected status"),
    }
}

#[rstest]
fn test_task_result_creation() {
    let mut metadata = HashMap::new();
    metadata.insert("test_key".to_string(), "test_value".to_string());

    let result = TaskResult {
        data: b"test_result_data".to_vec(),
        duration_ms: 2500,
        metadata,
    };

    assert_eq!(result.data, b"test_result_data");
    assert_eq!(result.duration_ms, 2500);
    assert_eq!(
        result.metadata.get("test_key"),
        Some(&"test_value".to_string())
    );
}

#[rstest]
fn test_task_queue_stats_default() {
    let stats = TaskQueueStats::default();

    assert_eq!(stats.pending_count, 0);
    assert_eq!(stats.in_progress_count, 0);
    assert_eq!(stats.completed_count, 0);
    assert_eq!(stats.failed_count, 0);
    assert!(stats.priority_counts.is_empty());
    assert_eq!(stats.avg_processing_time_ms, 0.0);
}

#[rstest]
fn test_task_serialization(test_task: Task) {
    // Test that tasks can be serialized and deserialized
    let serialized = bincode::serialize(&test_task).expect("Failed to serialize task");
    let deserialized: Task = bincode::deserialize(&serialized).expect("Failed to deserialize task");

    assert_eq!(test_task.id, deserialized.id);
    assert_eq!(test_task.name, deserialized.name);
    assert_eq!(test_task.payload, deserialized.payload);
    assert_eq!(test_task.priority, deserialized.priority);
    assert_eq!(test_task.created_by, deserialized.created_by);
}

#[rstest]
fn test_task_status_serialization() {
    let agent_id = AgentId::new();
    let statuses = vec![
        TaskStatus::Pending,
        TaskStatus::InProgress { agent_id },
        TaskStatus::Completed {
            result: TaskResult {
                data: b"result".to_vec(),
                duration_ms: 100,
                metadata: HashMap::new(),
            },
        },
        TaskStatus::Failed {
            error: "Test error".to_string(),
        },
        TaskStatus::Cancelled,
        TaskStatus::TimedOut,
    ];

    for status in statuses {
        let serialized = bincode::serialize(&status).expect("Failed to serialize status");
        let deserialized: TaskStatus =
            bincode::deserialize(&serialized).expect("Failed to deserialize status");

        // Compare discriminants since we can't derive PartialEq for TaskStatus due to TaskResult
        assert_eq!(
            std::mem::discriminant(&status),
            std::mem::discriminant(&deserialized)
        );
    }
}

// Integration test helpers
mod integration_helpers {
    use super::*;

    pub async fn create_test_task_queue() -> Result<TaskQueue> {
        // This would use a test Redis instance in real integration tests
        TaskQueue::new("redis://localhost:6379").await
    }

    pub fn create_sample_tasks(count: usize, agent_id: AgentId) -> Vec<Task> {
        (0..count)
            .map(|i| {
                Task::new(
                    format!("task_{}", i),
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
}

// Property-based tests using proptest
#[cfg(feature = "proptest")]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_task_id_uniqueness(
            names in prop::collection::vec(any::<String>(), 1..100)
        ) {
            let agent_id = AgentId::new();
            let tasks: Vec<Task> = names.into_iter()
                .map(|name| Task::new(name, b"payload".to_vec(), agent_id))
                .collect();

            // All task IDs should be unique
            let mut ids = std::collections::HashSet::new();
            for task in tasks {
                prop_assert!(ids.insert(task.id), "Duplicate task ID found: {}", task.id);
            }
        }

        #[test]
        fn test_task_priority_consistency(
            priority in 0u8..4u8
        ) {
            let task_priority = match priority {
                0 => TaskPriority::Low,
                1 => TaskPriority::Normal,
                2 => TaskPriority::High,
                3 => TaskPriority::Critical,
                _ => unreachable!(),
            };

            prop_assert_eq!(task_priority as u8, priority);
        }

        #[test]
        fn test_task_retry_logic_property(
            retry_count in 0u32..10,
            max_retries in 0u32..10
        ) {
            let agent_id = AgentId::new();
            let mut task = Task::new("test", b"payload".to_vec(), agent_id);
            task.retry_count = retry_count;
            task.max_retries = max_retries;

            let can_retry = task.can_retry();
            prop_assert_eq!(can_retry, retry_count < max_retries);
        }
    }
}


