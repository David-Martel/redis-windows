//! Task queue implementation for agent coordination
//!
//! Provides a distributed task queue built on Redis for managing work distribution
//! across multiple agents with priority support and reliable delivery.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{AgentId, Error, Result};

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    /// Low priority tasks
    Low = 0,
    /// Normal priority tasks
    Normal = 1,
    /// High priority tasks
    High = 2,
    /// Critical priority tasks
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Task execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,
    /// Task is currently being executed
    InProgress {
        /// ID of the agent processing the task
        agent_id: AgentId,
    },
    /// Task completed successfully
    Completed {
        /// Result of the completed task
        result: TaskResult,
    },
    /// Task failed with error
    Failed {
        /// Error message describing the failure
        error: String,
    },
    /// Task was cancelled
    Cancelled,
    /// Task timed out
    TimedOut,
}

/// Task execution result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskResult {
    /// Result data
    pub data: Vec<u8>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// A task in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: Uuid,
    /// Task name/type
    pub name: String,
    /// Task payload data
    pub payload: Vec<u8>,
    /// Task priority
    pub priority: TaskPriority,
    /// Current status
    pub status: TaskStatus,
    /// Agent that created the task
    pub created_by: AgentId,
    /// Task creation timestamp
    pub created_at: u64,
    /// Task deadline timestamp (optional)
    pub deadline: Option<u64>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Task timeout in seconds
    pub timeout_secs: u64,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl Task {
    /// Create a new task
    pub fn new(name: impl Into<String>, payload: Vec<u8>, created_by: AgentId) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            payload,
            priority: TaskPriority::default(),
            status: TaskStatus::Pending,
            created_by,
            created_at: now,
            deadline: None,
            retry_count: 0,
            max_retries: 3,
            timeout_secs: 300,
            tags: Vec::new(),
        }
    }

    /// Set task priority
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set task deadline
    pub fn with_deadline(mut self, deadline: SystemTime) -> Self {
        self.deadline = Some(deadline.duration_since(UNIX_EPOCH).unwrap().as_secs());
        self
    }

    /// Set task timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_secs = timeout.as_secs();
        self
    }

    /// Add tags to task
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Check if task is expired
    pub fn is_expired(&self) -> bool {
        if let Some(deadline) = self.deadline {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now > deadline
        } else {
            false
        }
    }

    /// Check if task can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }
}

/// Statistics for task queue monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskQueueStats {
    /// Total pending tasks
    pub pending_count: u64,
    /// Total in-progress tasks
    pub in_progress_count: u64,
    /// Total completed tasks
    pub completed_count: u64,
    /// Total failed tasks
    pub failed_count: u64,
    /// Tasks by priority
    pub priority_counts: HashMap<u8, u64>,
    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,
}

/// Distributed task queue implementation
pub struct TaskQueue {
    #[allow(dead_code)]
    client: Client,
    connection: Arc<RwLock<redis::aio::MultiplexedConnection>>,
    queue_key: String,
    processing_key: String,
    completed_key: String,
    failed_key: String,
}

impl TaskQueue {
    /// Redis key prefixes
    const QUEUE_PREFIX: &'static str = "agent_sync:task_queue";
    const PROCESSING_PREFIX: &'static str = "agent_sync:task_processing";
    const COMPLETED_PREFIX: &'static str = "agent_sync:task_completed";
    const FAILED_PREFIX: &'static str = "agent_sync:task_failed";

    /// Create a new task queue
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| Error::connection(format!("Failed to create Redis client: {}", e)))?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::connection(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            client,
            connection: Arc::new(RwLock::new(connection)),
            queue_key: format!("{}:pending", Self::QUEUE_PREFIX),
            processing_key: format!("{}:processing", Self::PROCESSING_PREFIX),
            completed_key: format!("{}:completed", Self::COMPLETED_PREFIX),
            failed_key: format!("{}:failed", Self::FAILED_PREFIX),
        })
    }

    /// Enqueue a task
    pub async fn enqueue(&self, task: Task) -> Result<()> {
        let mut conn = self.connection.write().await;

        // Serialize task
        let task_data = bincode::serialize(&task)
            .map_err(|e| Error::serialization(format!("Failed to serialize task: {}", e)))?;

        // Use priority as score for sorted set
        let score = task.priority as i64;

        // Add to priority queue
        conn.zadd::<_, _, _, ()>(&self.queue_key, &task_data, score)
            .await
            .map_err(|e| Error::other(format!("Failed to enqueue task: {}", e)))?;

        tracing::debug!(
            "Enqueued task {} with priority {:?}",
            task.id,
            task.priority
        );
        Ok(())
    }

    /// Dequeue the highest priority task
    pub async fn dequeue(&self, agent_id: AgentId) -> Result<Option<Task>> {
        let mut conn = self.connection.write().await;

        // Pop highest priority task atomically
        let result: Option<(Vec<u8>, f64)> = conn
            .zpopmax(&self.queue_key, 1)
            .await
            .map_err(|e| Error::other(format!("Failed to dequeue task: {}", e)))?;

        if let Some((task_data, _score)) = result {
            let mut task: Task = bincode::deserialize(&task_data)
                .map_err(|e| Error::serialization(format!("Failed to deserialize task: {}", e)))?;

            // Update task status
            task.status = TaskStatus::InProgress { agent_id };

            // Move to processing queue
            let processing_data = bincode::serialize(&task).map_err(|e| {
                Error::serialization(format!("Failed to serialize processing task: {}", e))
            })?;

            conn.hset::<_, _, _, ()>(&self.processing_key, task.id.to_string(), &processing_data)
                .await
                .map_err(|e| {
                    Error::other(format!("Failed to add task to processing queue: {}", e))
                })?;

            tracing::debug!("Dequeued task {} for agent {}", task.id, agent_id);
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// Complete a task
    pub async fn complete_task(&self, task_id: Uuid, result: TaskResult) -> Result<()> {
        let mut conn = self.connection.write().await;

        // Get task from processing queue
        let task_data: Vec<u8> = conn
            .hget(&self.processing_key, task_id.to_string())
            .await
            .map_err(|e| Error::other(format!("Failed to get processing task: {}", e)))?;

        let mut task: Task = bincode::deserialize(&task_data)
            .map_err(|e| Error::serialization(format!("Failed to deserialize task: {}", e)))?;

        // Update task status
        task.status = TaskStatus::Completed { result };

        // Serialize updated task
        let completed_data = bincode::serialize(&task).map_err(|e| {
            Error::serialization(format!("Failed to serialize completed task: {}", e))
        })?;

        // Move to completed queue and remove from processing
        let _: () = redis::pipe()
            .hset(&self.completed_key, task_id.to_string(), &completed_data)
            .hdel(&self.processing_key, task_id.to_string())
            .query_async(&mut *conn)
            .await
            .map_err(|e| Error::other(format!("Failed to complete task: {}", e)))?;

        tracing::debug!("Completed task {}", task_id);
        Ok(())
    }

    /// Fail a task
    pub async fn fail_task(&self, task_id: Uuid, error: String) -> Result<()> {
        let mut conn = self.connection.write().await;

        // Get task from processing queue
        let task_data: Vec<u8> = conn
            .hget(&self.processing_key, task_id.to_string())
            .await
            .map_err(|e| Error::other(format!("Failed to get processing task: {}", e)))?;

        let mut task: Task = bincode::deserialize(&task_data)
            .map_err(|e| Error::serialization(format!("Failed to deserialize task: {}", e)))?;

        // Check if task can be retried
        if task.can_retry() {
            task.retry_count += 1;
            task.status = TaskStatus::Pending;

            // Re-enqueue for retry
            let retry_data = bincode::serialize(&task).map_err(|e| {
                Error::serialization(format!("Failed to serialize retry task: {}", e))
            })?;

            let score = task.priority as i64;

            let _: () = redis::pipe()
                .zadd(&self.queue_key, &retry_data, score)
                .hdel(&self.processing_key, task_id.to_string())
                .query_async(&mut *conn)
                .await
                .map_err(|e| Error::other(format!("Failed to retry task: {}", e)))?;

            tracing::debug!("Retrying task {} (attempt {})", task_id, task.retry_count);
        } else {
            // Mark as failed permanently
            task.status = TaskStatus::Failed { error };

            let failed_data = bincode::serialize(&task).map_err(|e| {
                Error::serialization(format!("Failed to serialize failed task: {}", e))
            })?;

            let _: () = redis::pipe()
                .hset(&self.failed_key, task_id.to_string(), &failed_data)
                .hdel(&self.processing_key, task_id.to_string())
                .query_async(&mut *conn)
                .await
                .map_err(|e| Error::other(format!("Failed to fail task: {}", e)))?;

            tracing::debug!("Failed task {} permanently", task_id);
        }

        Ok(())
    }

    /// Get task queue statistics
    pub async fn get_stats(&self) -> Result<TaskQueueStats> {
        let mut conn = self.connection.write().await;

        let pending_count: u64 = conn
            .zcard(&self.queue_key)
            .await
            .map_err(|e| Error::other(format!("Failed to get pending count: {}", e)))?;

        let in_progress_count: u64 = conn
            .hlen(&self.processing_key)
            .await
            .map_err(|e| Error::other(format!("Failed to get in-progress count: {}", e)))?;

        let completed_count: u64 = conn
            .hlen(&self.completed_key)
            .await
            .map_err(|e| Error::other(format!("Failed to get completed count: {}", e)))?;

        let failed_count: u64 = conn
            .hlen(&self.failed_key)
            .await
            .map_err(|e| Error::other(format!("Failed to get failed count: {}", e)))?;

        Ok(TaskQueueStats {
            pending_count,
            in_progress_count,
            completed_count,
            failed_count,
            priority_counts: HashMap::new(), // TODO: Implement priority counts
            avg_processing_time_ms: 0.0,     // TODO: Implement average processing time
        })
    }

    /// Get task by ID from any queue
    pub async fn get_task(&self, task_id: Uuid) -> Result<Option<Task>> {
        let mut conn = self.connection.write().await;

        // Check processing queue first
        if let Ok(task_data) = conn
            .hget::<_, _, Vec<u8>>(&self.processing_key, task_id.to_string())
            .await
        {
            let task: Task = bincode::deserialize(&task_data)
                .map_err(|e| Error::serialization(format!("Failed to deserialize task: {}", e)))?;
            return Ok(Some(task));
        }

        // Check completed queue
        if let Ok(task_data) = conn
            .hget::<_, _, Vec<u8>>(&self.completed_key, task_id.to_string())
            .await
        {
            let task: Task = bincode::deserialize(&task_data)
                .map_err(|e| Error::serialization(format!("Failed to deserialize task: {}", e)))?;
            return Ok(Some(task));
        }

        // Check failed queue
        if let Ok(task_data) = conn
            .hget::<_, _, Vec<u8>>(&self.failed_key, task_id.to_string())
            .await
        {
            let task: Task = bincode::deserialize(&task_data)
                .map_err(|e| Error::serialization(format!("Failed to deserialize task: {}", e)))?;
            return Ok(Some(task));
        }

        Ok(None)
    }

    /// Shutdown the task queue
    pub async fn shutdown(self) -> Result<()> {
        // Connection will be dropped automatically
        tracing::info!("Task queue shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod basic_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_task_creation() {
        let agent_id = AgentId::new();
        let task = Task::new("test_task", b"payload".to_vec(), agent_id)
            .with_priority(TaskPriority::High)
            .with_timeout(Duration::from_secs(60))
            .with_tags(vec!["test".to_string(), "unit".to_string()]);

        assert_eq!(task.name, "test_task");
        assert_eq!(task.payload, b"payload");
        assert_eq!(task.created_by, agent_id);
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.timeout_secs, 60);
        assert_eq!(task.tags, vec!["test", "unit"]);
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, 3);
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[test]
    fn test_task_expiry() {
        let agent_id = AgentId::new();
        let past_deadline = SystemTime::now() - Duration::from_secs(60);
        let future_deadline = SystemTime::now() + Duration::from_secs(60);

        let expired_task =
            Task::new("expired", b"test".to_vec(), agent_id).with_deadline(past_deadline);

        let valid_task =
            Task::new("valid", b"test".to_vec(), agent_id).with_deadline(future_deadline);

        assert!(expired_task.is_expired());
        assert!(!valid_task.is_expired());
    }

    #[test]
    fn test_task_retry_logic() {
        let agent_id = AgentId::new();
        let mut task = Task::new("retry_test", b"test".to_vec(), agent_id);

        assert!(task.can_retry());

        task.retry_count = 2;
        assert!(task.can_retry());

        task.retry_count = 3;
        assert!(!task.can_retry());
    }
}
