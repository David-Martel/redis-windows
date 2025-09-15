//! Test fixtures for agent-sync testing
//!
//! This module provides reusable test fixtures and data for testing
//! the agent-sync library components.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::coordinator::{
    ContextData, ContextScope, CoordinatorConfig, Notification, NotificationFilter,
    NotificationPriority, NotificationType, Task, TaskPriority, TaskResult, TaskStatus,
};
use crate::{AgentId, SessionId};

/// Standard test configuration for coordinator
pub fn test_coordinator_config() -> CoordinatorConfig {
    CoordinatorConfig {
        redis_url: "redis://localhost:6379".to_string(),
        max_concurrent_tasks: 10,
        task_timeout_secs: 60,
        enable_metrics: false, // Disable for tests
        notification_buffer_size: 100,
    }
}

/// Test configuration with custom Redis URL
pub fn test_coordinator_config_with_url(redis_url: String) -> CoordinatorConfig {
    CoordinatorConfig {
        redis_url,
        ..test_coordinator_config()
    }
}

/// Create a test agent ID
pub fn test_agent_id() -> AgentId {
    AgentId::new()
}

/// Create multiple test agent IDs
pub fn test_agent_ids(count: usize) -> Vec<AgentId> {
    (0..count).map(|_| AgentId::new()).collect()
}

/// Create a test session ID
pub fn test_session_id() -> SessionId {
    SessionId::new()
}

/// Create a basic test task
pub fn test_task(agent_id: AgentId) -> Task {
    Task::new("test_task", b"test_payload".to_vec(), agent_id)
}

/// Create a test task with custom properties
pub fn test_task_with_props(
    name: &str,
    payload: Vec<u8>,
    agent_id: AgentId,
    priority: TaskPriority,
) -> Task {
    Task::new(name, payload, agent_id).with_priority(priority)
}

/// Create a high priority test task
pub fn test_high_priority_task(agent_id: AgentId) -> Task {
    Task::new("high_priority_task", b"urgent_payload".to_vec(), agent_id)
        .with_priority(TaskPriority::High)
}

/// Create a task with deadline
pub fn test_task_with_deadline(agent_id: AgentId, deadline_secs: u64) -> Task {
    let deadline = SystemTime::now() + Duration::from_secs(deadline_secs);
    Task::new("deadline_task", b"deadline_payload".to_vec(), agent_id).with_deadline(deadline)
}

/// Create a task with tags
pub fn test_task_with_tags(agent_id: AgentId, tags: Vec<String>) -> Task {
    Task::new("tagged_task", b"tagged_payload".to_vec(), agent_id).with_tags(tags)
}

/// Create multiple test tasks with different priorities
pub fn test_tasks_mixed_priority(agent_id: AgentId, count: usize) -> Vec<Task> {
    (0..count)
        .map(|i| {
            let priority = match i % 4 {
                0 => TaskPriority::Low,
                1 => TaskPriority::Normal,
                2 => TaskPriority::High,
                3 => TaskPriority::Critical,
                _ => unreachable!(),
            };

            Task::new(
                format!("task_{}", i),
                format!("payload_{}", i).into_bytes(),
                agent_id,
            )
            .with_priority(priority)
        })
        .collect()
}

/// Create a test task result
pub fn test_task_result() -> TaskResult {
    let mut metadata = HashMap::new();
    metadata.insert("test_key".to_string(), "test_value".to_string());

    TaskResult {
        data: b"test_result_data".to_vec(),
        duration_ms: 1000,
        metadata,
    }
}

/// Create a test task result with custom data
pub fn test_task_result_with_data(data: Vec<u8>, duration_ms: u64) -> TaskResult {
    TaskResult {
        data,
        duration_ms,
        metadata: HashMap::new(),
    }
}

/// Create test context data
pub fn test_context_data(agent_id: AgentId) -> ContextData {
    ContextData::new(
        b"test_context_data".to_vec(),
        "String".to_string(),
        agent_id,
    )
}

/// Create test context data with expiration
pub fn test_context_data_with_expiration(agent_id: AgentId, ttl: Duration) -> ContextData {
    ContextData::new(b"expiring_data".to_vec(), "String".to_string(), agent_id).with_expiration(ttl)
}

/// Create test context data with metadata
pub fn test_context_data_with_metadata(
    agent_id: AgentId,
    metadata: Vec<(String, String)>,
) -> ContextData {
    let mut data = ContextData::new(
        b"metadata_test_data".to_vec(),
        "String".to_string(),
        agent_id,
    );

    for (key, value) in metadata {
        data = data.with_metadata(key, value);
    }

    data
}

/// Create different context scopes for testing
pub fn test_context_scopes(agent_id: AgentId, session_id: SessionId) -> Vec<ContextScope> {
    vec![
        ContextScope::Global,
        ContextScope::Session(session_id),
        ContextScope::Agent(agent_id),
        ContextScope::Temporary { ttl_secs: 300 },
    ]
}

/// Create a test notification
pub fn test_notification(agent_id: AgentId) -> Notification {
    Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "info".to_string(),
        },
        b"test_notification_payload".to_vec(),
        agent_id,
    )
}

/// Create a test task notification
pub fn test_task_notification(agent_id: AgentId, task_id: Uuid) -> Notification {
    Notification::new(
        NotificationType::Task {
            task_id,
            action: "completed".to_string(),
        },
        b"task_completed".to_vec(),
        agent_id,
    )
}

/// Create a test custom notification
pub fn test_custom_notification(agent_id: AgentId, category: &str) -> Notification {
    let mut data = HashMap::new();
    data.insert("category".to_string(), category.to_string());
    data.insert("timestamp".to_string(), "2024-01-01T00:00:00Z".to_string());

    Notification::new(
        NotificationType::Custom {
            category: category.to_string(),
            data,
        },
        format!("Custom notification: {}", category).into_bytes(),
        agent_id,
    )
}

/// Create notifications with different priorities
pub fn test_notifications_mixed_priority(agent_id: AgentId, count: usize) -> Vec<Notification> {
    (0..count)
        .map(|i| {
            let priority = match i % 4 {
                0 => NotificationPriority::Low,
                1 => NotificationPriority::Normal,
                2 => NotificationPriority::High,
                3 => NotificationPriority::Critical,
                _ => unreachable!(),
            };

            Notification::new(
                NotificationType::Custom {
                    category: "test".to_string(),
                    data: {
                        let mut data = HashMap::new();
                        data.insert("index".to_string(), i.to_string());
                        data
                    },
                },
                format!("notification_{}", i).into_bytes(),
                agent_id,
            )
            .with_priority(priority)
        })
        .collect()
}

/// Create a notification with recipients
pub fn test_notification_with_recipients(
    sender: AgentId,
    recipients: Vec<AgentId>,
) -> Notification {
    Notification::new(
        NotificationType::Agent {
            agent_id: sender,
            status: "online".to_string(),
        },
        b"agent_status_update".to_vec(),
        sender,
    )
    .with_recipients(recipients)
}

/// Create a notification with TTL
pub fn test_notification_with_ttl(agent_id: AgentId, ttl: Duration) -> Notification {
    Notification::new(
        NotificationType::System {
            component: "cache".to_string(),
            level: "warning".to_string(),
        },
        b"temporary_warning".to_vec(),
        agent_id,
    )
    .with_ttl(ttl)
}

/// Create different notification filters for testing
pub fn test_notification_filters(agent_id: AgentId) -> Vec<NotificationFilter> {
    vec![
        // Default filter (matches all)
        NotificationFilter::default(),
        // Filter by sender
        NotificationFilter::new().with_sender(agent_id),
        // Filter by priority
        NotificationFilter::new().with_min_priority(NotificationPriority::High),
        // Filter by type
        NotificationFilter::new().with_types(vec![NotificationType::System {
            component: "".to_string(),
            level: "".to_string(),
        }]),
        // Complex filter
        NotificationFilter::new()
            .with_sender(agent_id)
            .with_min_priority(NotificationPriority::Normal)
            .with_metadata_filter("env".to_string(), "test".to_string()),
    ]
}

/// Test data for serialization/deserialization tests
pub struct SerializationTestData {
    pub task: Task,
    pub task_result: TaskResult,
    pub context_data: ContextData,
    pub notification: Notification,
    pub context_scopes: Vec<ContextScope>,
}

impl SerializationTestData {
    pub fn new(agent_id: AgentId, session_id: SessionId) -> Self {
        Self {
            task: test_task(agent_id),
            task_result: test_task_result(),
            context_data: test_context_data(agent_id),
            notification: test_notification(agent_id),
            context_scopes: test_context_scopes(agent_id, session_id),
        }
    }
}

/// Performance test data generator
pub struct PerformanceTestData;

impl PerformanceTestData {
    /// Generate tasks for performance testing
    pub fn tasks(count: usize, agent_id: AgentId) -> Vec<Task> {
        (0..count)
            .map(|i| {
                Task::new(
                    format!("perf_task_{:06}", i),
                    format!("performance_payload_{}", i).into_bytes(),
                    agent_id,
                )
                .with_priority(match i % 10 {
                    0..=2 => TaskPriority::Low,
                    3..=6 => TaskPriority::Normal,
                    7..=8 => TaskPriority::High,
                    9 => TaskPriority::Critical,
                    _ => unreachable!(),
                })
            })
            .collect()
    }

    /// Generate context data for performance testing
    pub fn context_data(count: usize, agent_id: AgentId) -> Vec<(String, ContextData)> {
        (0..count)
            .map(|i| {
                let key = format!("perf_key_{:06}", i);
                let data = ContextData::new(
                    format!("performance_data_{}", i).into_bytes(),
                    "String".to_string(),
                    agent_id,
                );
                (key, data)
            })
            .collect()
    }

    /// Generate notifications for performance testing
    pub fn notifications(count: usize, agent_id: AgentId) -> Vec<Notification> {
        (0..count)
            .map(|i| {
                Notification::new(
                    NotificationType::Custom {
                        category: "performance".to_string(),
                        data: {
                            let mut data = HashMap::new();
                            data.insert("batch".to_string(), (i / 100).to_string());
                            data.insert("index".to_string(), i.to_string());
                            data
                        },
                    },
                    format!("performance_notification_{}", i).into_bytes(),
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
}

/// Error cases for testing error handling
pub struct ErrorTestCases;

impl ErrorTestCases {
    /// Get invalid Redis URLs for connection testing
    pub fn invalid_redis_urls() -> Vec<String> {
        vec![
            "redis://invalid:1234".to_string(),
            "invalid://localhost:6379".to_string(),
            "redis://nonexistent.host:6379".to_string(),
            "".to_string(),
            "not-a-url".to_string(),
        ]
    }

    /// Get tasks that should cause serialization errors
    pub fn problematic_tasks(agent_id: AgentId) -> Vec<Task> {
        vec![
            // Task with very large payload
            Task::new(
                "large_task",
                vec![0u8; 10_000_000], // 10MB payload
                agent_id,
            ),
            // Task with invalid UTF-8 in name (won't actually be invalid due to String type)
            Task::new(
                "test_task_with_unicode_🚀",
                b"unicode_payload".to_vec(),
                agent_id,
            ),
        ]
    }

    /// Get invalid context keys
    pub fn invalid_context_keys() -> Vec<String> {
        vec![
            "".to_string(),
            " ".to_string(),
            "\n".to_string(),
            "\t".to_string(),
            "key with spaces".to_string(),
            "key:with:colons".to_string(),
            "key*with*wildcards".to_string(),
        ]
    }
}

/// Timing utilities for tests
pub struct TestTiming;

impl TestTiming {
    /// Get current timestamp
    pub fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Get timestamp in the past
    pub fn past(seconds_ago: u64) -> u64 {
        Self::now().saturating_sub(seconds_ago)
    }

    /// Get timestamp in the future
    pub fn future(seconds_ahead: u64) -> u64 {
        Self::now() + seconds_ahead
    }

    /// Create a duration from seconds
    pub fn duration_secs(secs: u64) -> Duration {
        Duration::from_secs(secs)
    }

    /// Create a duration from milliseconds
    pub fn duration_millis(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_creation() {
        let agent_id = test_agent_id();
        let session_id = test_session_id();

        // Test task fixtures
        let task = test_task(agent_id);
        assert_eq!(task.name, "test_task");
        assert_eq!(task.created_by, agent_id);

        let high_priority_task = test_high_priority_task(agent_id);
        assert_eq!(high_priority_task.priority, TaskPriority::High);

        // Test context data fixtures
        let context_data = test_context_data(agent_id);
        assert_eq!(context_data.created_by, agent_id);
        assert_eq!(context_data.data_type, "String");

        // Test notification fixtures
        let notification = test_notification(agent_id);
        assert_eq!(notification.sender, agent_id);
        assert_eq!(notification.priority, NotificationPriority::Normal);

        // Test scopes
        let scopes = test_context_scopes(agent_id, session_id);
        assert_eq!(scopes.len(), 4);
    }

    #[test]
    fn test_mixed_priority_generation() {
        let agent_id = test_agent_id();

        let tasks = test_tasks_mixed_priority(agent_id, 8);
        assert_eq!(tasks.len(), 8);

        // Verify priority distribution
        let priorities: Vec<_> = tasks.iter().map(|t| t.priority).collect();
        assert_eq!(priorities[0], TaskPriority::Low);
        assert_eq!(priorities[1], TaskPriority::Normal);
        assert_eq!(priorities[2], TaskPriority::High);
        assert_eq!(priorities[3], TaskPriority::Critical);
        assert_eq!(priorities[4], TaskPriority::Low); // Pattern repeats
    }

    #[test]
    fn test_performance_data_generation() {
        let agent_id = test_agent_id();

        let tasks = PerformanceTestData::tasks(100, agent_id);
        assert_eq!(tasks.len(), 100);

        let context_data = PerformanceTestData::context_data(50, agent_id);
        assert_eq!(context_data.len(), 50);

        let notifications = PerformanceTestData::notifications(25, agent_id);
        assert_eq!(notifications.len(), 25);
    }

    #[test]
    fn test_timing_utilities() {
        let now = TestTiming::now();
        let past = TestTiming::past(60);
        let future = TestTiming::future(60);

        assert!(past < now);
        assert!(future > now);
        assert_eq!(future - now, 60);
        assert_eq!(now - past, 60);
    }

    #[test]
    fn test_error_test_cases() {
        let invalid_urls = ErrorTestCases::invalid_redis_urls();
        assert!(!invalid_urls.is_empty());

        let invalid_keys = ErrorTestCases::invalid_context_keys();
        assert!(!invalid_keys.is_empty());

        let agent_id = test_agent_id();
        let problematic_tasks = ErrorTestCases::problematic_tasks(agent_id);
        assert!(!problematic_tasks.is_empty());
    }

    #[test]
    fn test_serialization_test_data() {
        let agent_id = test_agent_id();
        let session_id = test_session_id();

        let test_data = SerializationTestData::new(agent_id, session_id);

        // Verify all components can be created
        assert_eq!(test_data.task.created_by, agent_id);
        assert_eq!(test_data.context_data.created_by, agent_id);
        assert_eq!(test_data.notification.sender, agent_id);
        assert_eq!(test_data.context_scopes.len(), 4);
    }
}
