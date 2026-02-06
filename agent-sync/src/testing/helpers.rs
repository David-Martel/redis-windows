#![allow(missing_docs)]
//! Test helper functions and utilities
//!
//! This module provides utility functions to make testing easier
//! and more consistent across the agent-sync test suite.

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::time::{sleep, timeout};
use uuid::Uuid;

use crate::coordinator::{
    ContextData, ContextScope, Coordinator, CoordinatorConfig, Notification, NotificationFilter,
    Task, TaskResult, TaskStatus,
};
use crate::{AgentId, Error, Result};

/// Utility functions for async testing
pub struct AsyncTestHelpers;

impl AsyncTestHelpers {
    /// Wait for a condition to become true with timeout
    pub async fn wait_for<F, Fut>(
        mut condition: F,
        timeout_duration: Duration,
        check_interval: Duration,
    ) -> Result<()>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = bool>,
    {
        let start = Instant::now();

        loop {
            if condition().await {
                return Ok(());
            }

            if start.elapsed() > timeout_duration {
                return Err(Error::other("Timeout waiting for condition".to_string()));
            }

            sleep(check_interval).await;
        }
    }

    /// Wait for a condition with a custom error message
    pub async fn wait_for_with_message<F, Fut>(
        condition: F,
        timeout_duration: Duration,
        check_interval: Duration,
        error_message: &str,
    ) -> Result<()>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = bool>,
    {
        Self::wait_for(condition, timeout_duration, check_interval)
            .await
            .map_err(|_| Error::other(error_message.to_string()))
    }

    /// Run a test with timeout
    pub async fn with_timeout<F, Fut, T>(future: F, timeout_duration: Duration) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        timeout(timeout_duration, future())
            .await
            .map_err(|_| Error::other("Test timed out".to_string()))?
    }

    /// Retry a test operation
    pub async fn retry<F, Fut, T>(
        mut operation: F,
        max_attempts: usize,
        delay_between_attempts: Duration,
    ) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut last_error = Error::other("No attempts made".to_string());

        for attempt in 1..=max_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = e;
                    if attempt < max_attempts {
                        sleep(delay_between_attempts).await;
                    }
                }
            }
        }

        Err(last_error)
    }
}

/// Utilities for testing task queue operations
pub struct TaskQueueTestHelpers;

impl TaskQueueTestHelpers {
    /// Create and enqueue multiple tasks
    pub async fn enqueue_tasks(
        task_queue: &crate::coordinator::TaskQueue,
        tasks: Vec<Task>,
    ) -> Result<Vec<Uuid>> {
        let mut task_ids = Vec::new();

        for task in tasks {
            let task_id = task.id;
            task_queue.enqueue(task).await?;
            task_ids.push(task_id);
        }

        Ok(task_ids)
    }

    /// Dequeue all available tasks
    pub async fn dequeue_all_tasks(
        task_queue: &crate::coordinator::TaskQueue,
        worker_agent: AgentId,
    ) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();

        while let Some(task) = task_queue.dequeue(worker_agent).await? {
            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Complete multiple tasks with default results
    pub async fn complete_tasks(
        task_queue: &crate::coordinator::TaskQueue,
        task_ids: Vec<Uuid>,
    ) -> Result<()> {
        for task_id in task_ids {
            let result = TaskResult {
                data: b"completed".to_vec(),
                duration_ms: 100,
                metadata: std::collections::HashMap::new(),
            };
            task_queue.complete_task(task_id, result).await?;
        }

        Ok(())
    }

    /// Wait for queue to reach expected stats
    pub async fn wait_for_queue_stats(
        task_queue: &crate::coordinator::TaskQueue,
        expected_pending: u64,
        expected_completed: u64,
        timeout_duration: Duration,
    ) -> Result<()> {
        AsyncTestHelpers::wait_for(
            || async {
                if let Ok(stats) = task_queue.get_stats().await {
                    stats.pending_count == expected_pending
                        && stats.completed_count == expected_completed
                } else {
                    false
                }
            },
            timeout_duration,
            Duration::from_millis(100),
        )
        .await
    }

    /// Verify task status
    pub async fn verify_task_status(
        task_queue: &crate::coordinator::TaskQueue,
        task_id: Uuid,
        expected_status: TaskStatus,
    ) -> Result<()> {
        let task = task_queue
            .get_task(task_id)
            .await?
            .ok_or_else(|| Error::other("Task not found".to_string()))?;

        // Compare discriminants since TaskStatus can't derive PartialEq
        let actual_discriminant = std::mem::discriminant(&task.status);
        let expected_discriminant = std::mem::discriminant(&expected_status);

        if actual_discriminant != expected_discriminant {
            return Err(Error::other(format!(
                "Task status mismatch. Expected: {expected_discriminant:?}, Actual: {actual_discriminant:?}"
            )));
        }

        Ok(())
    }
}

/// Utilities for testing shared context operations
pub struct SharedContextTestHelpers;

impl SharedContextTestHelpers {
    /// Set multiple context entries
    pub async fn set_multiple(
        shared_context: &crate::coordinator::SharedContext,
        scope: ContextScope,
        entries: Vec<(&str, ContextData)>,
    ) -> Result<()> {
        for (key, data) in entries {
            shared_context.set(scope.clone(), key, data).await?;
        }

        Ok(())
    }

    /// Get multiple context entries
    pub async fn get_multiple(
        shared_context: &crate::coordinator::SharedContext,
        scope: ContextScope,
        keys: Vec<&str>,
    ) -> Result<Vec<Option<ContextData>>> {
        let mut results = Vec::new();

        for key in keys {
            let data = shared_context.get(scope.clone(), key).await?;
            results.push(data);
        }

        Ok(results)
    }

    /// Verify context data equality
    pub fn verify_context_data(actual: &ContextData, expected: &ContextData) -> Result<()> {
        if actual.data != expected.data {
            return Err(Error::other("Context data mismatch".to_string()));
        }
        if actual.data_type != expected.data_type {
            return Err(Error::other("Context data type mismatch".to_string()));
        }
        if actual.created_by != expected.created_by {
            return Err(Error::other("Context created_by mismatch".to_string()));
        }

        Ok(())
    }

    /// Wait for context key to exist
    pub async fn wait_for_key_exists(
        shared_context: &crate::coordinator::SharedContext,
        scope: ContextScope,
        key: &str,
        timeout_duration: Duration,
    ) -> Result<()> {
        AsyncTestHelpers::wait_for(
            || async {
                shared_context
                    .exists(scope.clone(), key)
                    .await
                    .expect("Failed to check key existence in wait_for_key_exists")
            },
            timeout_duration,
            Duration::from_millis(50),
        )
        .await
    }

    /// Wait for context key to be deleted
    pub async fn wait_for_key_deleted(
        shared_context: &crate::coordinator::SharedContext,
        scope: ContextScope,
        key: &str,
        timeout_duration: Duration,
    ) -> Result<()> {
        AsyncTestHelpers::wait_for(
            || async {
                !shared_context
                    .exists(scope.clone(), key)
                    .await
                    .expect("Failed to check key existence in wait_for_key_deleted")
            },
            timeout_duration,
            Duration::from_millis(50),
        )
        .await
    }
}

/// Utilities for testing notification bus operations
pub struct NotificationTestHelpers;

impl NotificationTestHelpers {
    /// Publish multiple notifications
    pub async fn publish_multiple(
        notification_bus: &crate::coordinator::NotificationBus,
        notifications: Vec<Notification>,
    ) -> Result<Vec<u64>> {
        let mut results = Vec::new();

        for notification in notifications {
            let count = notification_bus.publish(notification).await?;
            results.push(count);
        }

        Ok(results)
    }

    /// Collect notifications from receiver with timeout
    pub async fn collect_notifications(
        receiver: &mut tokio::sync::mpsc::UnboundedReceiver<Notification>,
        expected_count: usize,
        timeout_duration: Duration,
    ) -> Result<Vec<Notification>> {
        let mut notifications = Vec::new();

        let deadline = Instant::now() + timeout_duration;

        while notifications.len() < expected_count {
            let remaining_time = deadline.saturating_duration_since(Instant::now());
            if remaining_time.is_zero() {
                break;
            }

            match timeout(remaining_time, receiver.recv()).await {
                Ok(Some(notification)) => notifications.push(notification),
                Ok(None) => break, // Channel closed
                Err(_) => break,   // Timeout
            }
        }

        if notifications.len() != expected_count {
            return Err(Error::other(format!(
                "Expected {} notifications, got {}",
                expected_count,
                notifications.len()
            )));
        }

        Ok(notifications)
    }

    /// Verify notification matches filter
    pub fn verify_notification_matches_filter(
        notification: &Notification,
        filter: &NotificationFilter,
    ) -> Result<()> {
        if !filter.matches(notification) {
            return Err(Error::other(
                "Notification does not match filter".to_string(),
            ));
        }

        Ok(())
    }

    /// Wait for notification stats to reach expected values
    pub async fn wait_for_notification_stats(
        notification_bus: &crate::coordinator::NotificationBus,
        expected_sent: u64,
        expected_received: u64,
        timeout_duration: Duration,
    ) -> Result<()> {
        AsyncTestHelpers::wait_for(
            || async {
                let stats = notification_bus.get_stats().await;
                stats.total_sent == expected_sent && stats.total_received == expected_received
            },
            timeout_duration,
            Duration::from_millis(100),
        )
        .await
    }
}

/// Utilities for coordinator integration testing
pub struct CoordinatorTestHelpers;

impl CoordinatorTestHelpers {
    /// Create a test coordinator with retry logic
    pub async fn create_test_coordinator(config: CoordinatorConfig) -> Result<Coordinator> {
        AsyncTestHelpers::retry(
            || async { Coordinator::new(config.clone()).await },
            3,
            Duration::from_millis(500),
        )
        .await
    }

    /// Run a complete task workflow for testing
    pub async fn run_task_workflow(
        coordinator: &Coordinator,
        _producer_agent: AgentId,
        worker_agent: AgentId,
        tasks: Vec<Task>,
    ) -> Result<Vec<TaskResult>> {
        let mut results = Vec::new();

        // Enqueue all tasks
        for task in tasks {
            coordinator.task_queue().enqueue(task).await?;
        }

        // Process all tasks
        while let Some(task) = coordinator.task_queue().dequeue(worker_agent).await? {
            // Simulate processing
            sleep(Duration::from_millis(10)).await;

            let result = TaskResult {
                data: format!("Processed: {}", task.name).into_bytes(),
                duration_ms: 10,
                metadata: std::collections::HashMap::new(),
            };

            coordinator
                .task_queue()
                .complete_task(task.id, result.clone())
                .await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Test complete pub-sub workflow
    pub async fn test_pubsub_workflow(
        coordinator: &Coordinator,
        _publisher: AgentId,
        subscribers: Vec<AgentId>,
        notifications: Vec<Notification>,
    ) -> Result<Vec<Vec<Notification>>> {
        let mut subscription_results = Vec::new();

        // Subscribe all agents
        let mut subscriptions = Vec::new();
        for subscriber in &subscribers {
            let (subscription_id, receiver) = coordinator
                .notification_bus()
                .subscribe(*subscriber, NotificationFilter::default())
                .await?;
            subscriptions.push((subscription_id, receiver));
        }

        // Publish all notifications
        for notification in notifications.clone() {
            coordinator.notification_bus().publish(notification).await?;
        }

        // Collect results from all subscribers
        for (_, mut receiver) in subscriptions {
            let received = NotificationTestHelpers::collect_notifications(
                &mut receiver,
                notifications.len(),
                Duration::from_secs(2),
            )
            .await?;
            subscription_results.push(received);
        }

        Ok(subscription_results)
    }
}

/// Utilities for performance testing
pub struct PerformanceTestHelpers;

impl PerformanceTestHelpers {
    /// Measure execution time of an async operation
    pub async fn measure_time<F, Fut, T>(operation: F) -> (T, Duration)
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        let start = Instant::now();
        let result = operation().await;
        let elapsed = start.elapsed();
        (result, elapsed)
    }

    /// Run performance test with statistics
    pub async fn run_performance_test<F, Fut, T>(
        operation: F,
        iterations: usize,
    ) -> PerformanceStats<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut results = Vec::new();
        let mut durations = Vec::new();
        let mut errors = Vec::new();

        for _ in 0..iterations {
            let (result, duration) = Self::measure_time(&operation).await;

            durations.push(duration);

            match result {
                Ok(value) => results.push(value),
                Err(error) => errors.push(error),
            }
        }

        PerformanceStats {
            successful_results: results,
            errors,
            durations,
        }
    }

    /// Calculate throughput (operations per second)
    pub fn calculate_throughput(operations: usize, duration: Duration) -> f64 {
        operations as f64 / duration.as_secs_f64()
    }
}

/// Performance test statistics
pub struct PerformanceStats<T> {
    pub successful_results: Vec<T>,
    pub errors: Vec<Error>,
    pub durations: Vec<Duration>,
}

impl<T> PerformanceStats<T> {
    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.successful_results.len() + self.errors.len();
        if total == 0 {
            0.0
        } else {
            self.successful_results.len() as f64 / total as f64
        }
    }

    /// Get average duration
    pub fn average_duration(&self) -> Duration {
        if self.durations.is_empty() {
            Duration::from_secs(0)
        } else {
            let total_nanos: u64 = self.durations.iter().map(|d| d.as_nanos() as u64).sum();
            Duration::from_nanos(total_nanos / self.durations.len() as u64)
        }
    }

    /// Get median duration
    pub fn median_duration(&self) -> Duration {
        if self.durations.is_empty() {
            return Duration::from_secs(0);
        }

        let mut sorted_durations = self.durations.clone();
        sorted_durations.sort();

        let mid = sorted_durations.len() / 2;
        if sorted_durations.len() % 2 == 0 {
            // Even number of elements, average the two middle ones
            let sum_nanos = sorted_durations[mid - 1].as_nanos() + sorted_durations[mid].as_nanos();
            Duration::from_nanos((sum_nanos / 2) as u64)
        } else {
            // Odd number of elements, take the middle one
            sorted_durations[mid]
        }
    }

    /// Get 95th percentile duration
    pub fn percentile_95_duration(&self) -> Duration {
        if self.durations.is_empty() {
            return Duration::from_secs(0);
        }

        let mut sorted_durations = self.durations.clone();
        sorted_durations.sort();

        let index =
            ((sorted_durations.len() as f64 * 0.95) as usize).min(sorted_durations.len() - 1);
        sorted_durations[index]
    }
}

/// Assertion helpers for tests
pub struct AssertionHelpers;

impl AssertionHelpers {
    /// Assert that two durations are approximately equal
    pub fn assert_duration_approx_eq(
        actual: Duration,
        expected: Duration,
        tolerance: Duration,
    ) -> Result<()> {
        let diff = if actual > expected {
            actual.checked_sub(expected).unwrap()
        } else {
            expected.checked_sub(actual).unwrap()
        };

        if diff > tolerance {
            return Err(Error::other(format!(
                "Duration assertion failed: actual {actual:?}, expected {expected:?}, tolerance {tolerance:?}"
            )));
        }

        Ok(())
    }

    /// Assert that a value is within a range
    pub fn assert_in_range<T>(actual: T, min: T, max: T) -> Result<()>
    where
        T: PartialOrd + std::fmt::Debug,
    {
        if actual < min || actual > max {
            return Err(Error::other(format!(
                "Value {actual:?} is not in range [{min:?}, {max:?}]"
            )));
        }

        Ok(())
    }

    /// Assert that a collection contains expected items
    pub fn assert_contains_all<T>(collection: &[T], expected: &[T]) -> Result<()>
    where
        T: PartialEq + std::fmt::Debug,
    {
        for item in expected {
            if !collection.contains(item) {
                return Err(Error::other(format!(
                    "Collection does not contain expected item: {item:?}"
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_wait_for() {
        let counter = Arc::new(std::sync::Mutex::new(0));

        let result = AsyncTestHelpers::wait_for(
            {
                let counter = counter.clone();
                move || {
                    let counter = counter.clone();
                    let fut = async move {
                        let mut val = counter.lock().expect("Failed to lock counter");
                        *val += 1;
                        *val >= 3
                    };
                    Box::pin(fut)
                }
            },
            Duration::from_secs(1),
            Duration::from_millis(10),
        )
        .await;

        assert!(result.is_ok());
        assert!(*counter.lock().expect("Failed to lock counter") >= 3);
    }

    #[tokio::test]
    async fn test_async_retry() {
        let attempt_count = Arc::new(std::sync::Mutex::new(0));

        let result = AsyncTestHelpers::retry(
            {
                let attempt_count = attempt_count.clone();
                move || {
                    let attempt_count = attempt_count.clone();
                    Box::pin(async move {
                        let mut count = attempt_count.lock().expect("Failed to lock attempt_count");
                        *count += 1;
                        if *count < 3 {
                            Err(Error::other("Not ready yet".to_string()))
                        } else {
                            Ok("success")
                        }
                    })
                }
            },
            5,
            Duration::from_millis(10),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.expect("Failed to get result"), "success");
        assert_eq!(*attempt_count.lock().expect("Failed to lock attempt_count"), 3);
    }

    #[tokio::test]
    async fn test_performance_measurement() {
        let (result, duration) = PerformanceTestHelpers::measure_time(|| async {
            sleep(Duration::from_millis(10)).await;
            "completed"
        })
        .await;

        assert_eq!(result, "completed");
        assert!(duration >= Duration::from_millis(10));
        assert!(duration <= Duration::from_millis(50)); // Allow some tolerance
    }

    #[test]
    fn test_performance_stats() {
        let durations = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
        ];

        let stats = PerformanceStats {
            successful_results: vec![1, 2, 3, 4, 5],
            errors: vec![],
            durations,
        };

        assert_eq!(stats.success_rate(), 1.0);
        assert_eq!(stats.average_duration(), Duration::from_millis(30));
        assert_eq!(stats.median_duration(), Duration::from_millis(30));
    }

    #[test]
    fn test_assertion_helpers() {
        // Test duration approximation
        let result = AssertionHelpers::assert_duration_approx_eq(
            Duration::from_millis(105),
            Duration::from_millis(100),
            Duration::from_millis(10),
        );
        assert!(result.is_ok());

        // Test range assertion
        let result = AssertionHelpers::assert_in_range(5, 1, 10);
        assert!(result.is_ok());

        // Test contains assertion
        let collection = vec![1, 2, 3, 4, 5];
        let expected = vec![2, 4];
        let result = AssertionHelpers::assert_contains_all(&collection, &expected);
        assert!(result.is_ok());
    }
}











