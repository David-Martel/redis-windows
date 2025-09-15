# Agent Sync API Documentation

Complete API reference for the Agent Sync library v0.1.0.

## Table of Contents

- [Core Types](#core-types)
- [Client API](#client-api)
- [Task Queue API](#task-queue-api)
- [Shared Context API](#shared-context-api)
- [Notification Bus API](#notification-bus-api)
- [Synchronization Primitives](#synchronization-primitives)
- [Connection Management](#connection-management)
- [Error Types](#error-types)
- [Configuration](#configuration)
- [Metrics API](#metrics-api)
- [Platform-Specific APIs](#platform-specific-apis)

## Core Types

### AgentId

Unique identifier for an agent in the system.

```rust
pub struct AgentId(Uuid);

impl AgentId {
    /// Create a new random agent ID
    pub fn new() -> Self;

    /// Create from an existing UUID
    pub fn from_uuid(id: Uuid) -> Self;

    /// Parse from a string
    pub fn from_str(s: &str) -> Result<Self, Error>;

    /// Get the inner UUID
    pub fn as_uuid(&self) -> &Uuid;
}
```

### SessionId

Represents a coordination session between agents.

```rust
pub struct SessionId(String);

impl SessionId {
    /// Create a new session ID
    pub fn new() -> Self;

    /// Create from a string
    pub fn from_string(s: String) -> Self;

    /// Get as string reference
    pub fn as_str(&self) -> &str;
}
```

### Timestamp

Millisecond-precision timestamp for coordination.

```rust
pub struct Timestamp(u64);

impl Timestamp {
    /// Current timestamp
    pub fn now() -> Self;

    /// From milliseconds since epoch
    pub fn from_millis(millis: u64) -> Self;

    /// Convert to SystemTime
    pub fn to_system_time(&self) -> SystemTime;

    /// Get milliseconds value
    pub fn as_millis(&self) -> u64;
}
```

## Client API

### AgentSync

Main client for agent coordination.

```rust
pub struct AgentSync {
    // Private fields
}

impl AgentSync {
    /// Create new client with configuration
    ///
    /// # Arguments
    /// * `config` - Client configuration
    ///
    /// # Returns
    /// * `Result<Self>` - Client instance or error
    ///
    /// # Example
    /// ```rust
    /// let config = Config::default().with_redis_url("redis://localhost:6379");
    /// let client = AgentSync::new(config).await?;
    /// ```
    pub async fn new(config: Config) -> Result<Self>;

    /// Create new client with default configuration
    pub async fn default() -> Result<Self>;

    /// Submit a task for processing
    ///
    /// # Arguments
    /// * `name` - Task name/type
    /// * `payload` - Task data
    /// * `options` - Task options
    ///
    /// # Returns
    /// * `Result<Uuid>` - Task ID
    pub async fn submit_task(
        &self,
        name: &str,
        payload: &[u8],
        options: TaskOptions
    ) -> Result<Uuid>;

    /// Wait for task completion
    ///
    /// # Arguments
    /// * `task_id` - ID of task to wait for
    ///
    /// # Returns
    /// * `Result<TaskResult>` - Task result or error
    pub async fn await_task_result(&self, task_id: Uuid) -> Result<TaskResult>;

    /// Get task status without waiting
    pub async fn get_task_status(&self, task_id: Uuid) -> Result<TaskStatus>;

    /// Cancel a pending task
    pub async fn cancel_task(&self, task_id: Uuid) -> Result<()>;

    /// Store data in shared context
    pub async fn store_context(
        &self,
        key: &str,
        value: serde_json::Value
    ) -> Result<()>;

    /// Retrieve data from shared context
    pub async fn get_context(&self, key: &str) -> Result<Option<serde_json::Value>>;

    /// Publish notification
    pub async fn publish(
        &self,
        topic: &str,
        message: serde_json::Value
    ) -> Result<()>;

    /// Subscribe to notifications
    pub async fn subscribe(&self, topics: Vec<String>) -> Result<Subscriber>;

    /// Get client metrics
    pub fn metrics(&self) -> &Metrics;

    /// Health check
    pub async fn health_check(&self) -> Result<HealthStatus>;

    /// Graceful shutdown
    pub async fn shutdown(self) -> Result<()>;
}
```

### TaskOptions

Options for task submission.

```rust
#[derive(Default, Clone)]
pub struct TaskOptions {
    pub priority: TaskPriority,
    pub timeout_secs: Option<u64>,
    pub max_retries: Option<u32>,
    pub deadline: Option<Timestamp>,
    pub tags: Vec<String>,
}

impl TaskOptions {
    /// Create with default options
    pub fn new() -> Self;

    /// Set task priority
    pub fn with_priority(mut self, priority: TaskPriority) -> Self;

    /// Set timeout in seconds
    pub fn with_timeout(mut self, secs: u64) -> Self;

    /// Set maximum retry attempts
    pub fn with_max_retries(mut self, retries: u32) -> Self;

    /// Set deadline timestamp
    pub fn with_deadline(mut self, deadline: Timestamp) -> Self;

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self;
}
```

## Task Queue API

### TaskQueue

Distributed task queue implementation.

```rust
pub struct TaskQueue {
    // Private fields
}

impl TaskQueue {
    /// Create new task queue
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL
    ///
    /// # Example
    /// ```rust
    /// let queue = TaskQueue::new("redis://localhost:6379").await?;
    /// ```
    pub async fn new(redis_url: &str) -> Result<Self>;

    /// Create with custom configuration
    pub async fn with_config(config: TaskQueueConfig) -> Result<Self>;

    /// Enqueue a task
    ///
    /// # Arguments
    /// * `queue_name` - Name of the queue
    /// * `task` - Task to enqueue
    ///
    /// # Returns
    /// * `Result<String>` - Entry ID in the stream
    pub async fn enqueue(&self, queue_name: &str, task: Task) -> Result<String>;

    /// Enqueue multiple tasks atomically
    pub async fn enqueue_batch(
        &self,
        queue_name: &str,
        tasks: Vec<Task>
    ) -> Result<Vec<String>>;

    /// Create a consumer for processing tasks
    pub async fn create_consumer(
        &self,
        consumer_name: &str
    ) -> Result<TaskConsumer>;

    /// Get queue statistics
    pub async fn get_stats(&self, queue_name: &str) -> Result<QueueStats>;

    /// Get pending tasks count
    pub async fn pending_count(&self, queue_name: &str) -> Result<usize>;

    /// Clear all tasks from queue
    pub async fn clear(&self, queue_name: &str) -> Result<()>;

    /// Delete a queue
    pub async fn delete_queue(&self, queue_name: &str) -> Result<()>;
}
```

### Task

Represents a task in the queue.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub payload: Vec<u8>,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub created_by: AgentId,
    pub created_at: u64,
    pub deadline: Option<u64>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub timeout_secs: u64,
    pub tags: Vec<String>,
}

impl Task {
    /// Create a new task
    pub fn new(name: String, payload: Vec<u8>) -> Self;

    /// Create a task builder
    pub fn builder() -> TaskBuilder;

    /// Check if task has expired
    pub fn is_expired(&self) -> bool;

    /// Check if task can be retried
    pub fn can_retry(&self) -> bool;

    /// Increment retry count
    pub fn increment_retry(&mut self);

    /// Update task status
    pub fn set_status(&mut self, status: TaskStatus);
}
```

### TaskBuilder

Builder pattern for task creation.

```rust
pub struct TaskBuilder {
    // Private fields
}

impl TaskBuilder {
    /// Set task name
    pub fn name(mut self, name: impl Into<String>) -> Self;

    /// Set task payload
    pub fn payload(mut self, payload: impl Into<Vec<u8>>) -> Self;

    /// Set priority
    pub fn priority(mut self, priority: TaskPriority) -> Self;

    /// Set timeout
    pub fn timeout_secs(mut self, secs: u64) -> Self;

    /// Set max retries
    pub fn max_retries(mut self, retries: u32) -> Self;

    /// Set deadline
    pub fn deadline(mut self, deadline: Timestamp) -> Self;

    /// Add tag
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self;

    /// Build the task
    pub fn build(self) -> Task;
}
```

### TaskConsumer

Consumer for processing tasks from queue.

```rust
pub struct TaskConsumer {
    // Private fields
}

impl TaskConsumer {
    /// Get next task from queue
    ///
    /// # Returns
    /// * `Result<Option<(String, Task)>>` - Entry ID and task, or None if queue empty
    pub async fn next_task(&mut self) -> Result<Option<(String, Task)>>;

    /// Get multiple tasks
    pub async fn next_batch(&mut self, count: usize) -> Result<Vec<(String, Task)>>;

    /// Mark task as completed
    pub async fn complete_task(
        &self,
        entry_id: &str,
        result: TaskResult
    ) -> Result<()>;

    /// Mark task as failed
    pub async fn fail_task(
        &self,
        entry_id: &str,
        error: &str
    ) -> Result<()>;

    /// Acknowledge task without result
    pub async fn ack_task(&self, entry_id: &str) -> Result<()>;

    /// Return task to queue for retry
    pub async fn retry_task(&self, entry_id: &str) -> Result<()>;

    /// Claim abandoned tasks
    pub async fn claim_abandoned(
        &self,
        idle_time_ms: u64
    ) -> Result<Vec<(String, Task)>>;
}
```

### TaskPriority

Task priority levels.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}
```

### TaskStatus

Task execution status.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,

    /// Task is being processed
    InProgress { agent_id: AgentId },

    /// Task completed successfully
    Completed { result: TaskResult },

    /// Task failed
    Failed { error: String },

    /// Task was cancelled
    Cancelled,

    /// Task timed out
    TimedOut,
}
```

### TaskResult

Result of task execution.

```rust
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub data: Vec<u8>,
    pub duration_ms: u64,
    pub metadata: HashMap<String, String>,
}

impl TaskResult {
    /// Create new result
    pub fn new(data: Vec<u8>) -> Self;

    /// Set duration
    pub fn with_duration(mut self, ms: u64) -> Self;

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self;
}
```

## Shared Context API

### SharedContext

Distributed state management.

```rust
pub struct SharedContext {
    // Private fields
}

impl SharedContext {
    /// Create new shared context
    pub async fn new(redis_url: &str) -> Result<Self>;

    /// Store context entry
    ///
    /// # Arguments
    /// * `namespace` - Context namespace
    /// * `entry` - Entry to store
    pub async fn store(
        &self,
        namespace: &str,
        entry: ContextEntry
    ) -> Result<()>;

    /// Retrieve context entry
    pub async fn retrieve(
        &self,
        namespace: &str,
        key: &str
    ) -> Result<Option<ContextEntry>>;

    /// Update entry with optimistic locking
    pub async fn update(
        &self,
        namespace: &str,
        key: &str,
        value: serde_json::Value,
        expected_version: u64
    ) -> Result<ContextEntry>;

    /// Update only if version matches
    pub async fn update_if_version(
        &self,
        namespace: &str,
        key: &str,
        value: serde_json::Value,
        version: u64
    ) -> Result<ContextEntry>;

    /// Delete entry
    pub async fn delete(
        &self,
        namespace: &str,
        key: &str
    ) -> Result<()>;

    /// List keys in namespace
    pub async fn list_keys(
        &self,
        namespace: &str
    ) -> Result<Vec<String>>;

    /// Batch store operations
    pub async fn batch_store(
        &self,
        namespace: &str,
        entries: Vec<ContextEntry>
    ) -> Result<()>;

    /// Batch retrieve operations
    pub async fn batch_retrieve(
        &self,
        namespace: &str,
        keys: &[String]
    ) -> Result<HashMap<String, ContextEntry>>;

    /// Clear namespace
    pub async fn clear_namespace(&self, namespace: &str) -> Result<()>;

    /// Set TTL for entry
    pub async fn set_ttl(
        &self,
        namespace: &str,
        key: &str,
        ttl_secs: u64
    ) -> Result<()>;
}
```

### ContextEntry

Entry in shared context.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub version: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub created_by: AgentId,
    pub tags: HashMap<String, String>,
}

impl ContextEntry {
    /// Create new entry
    pub fn new(key: impl Into<String>, value: serde_json::Value) -> Result<Self>;

    /// Add tag
    pub fn add_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self;

    /// Set TTL
    pub fn with_ttl(mut self, ttl_secs: u64) -> Self;

    /// Check if expired
    pub fn is_expired(&self) -> bool;
}
```

## Notification Bus API

### NotificationBus

Real-time message broadcasting.

```rust
pub struct NotificationBus {
    // Private fields
}

impl NotificationBus {
    /// Create new notification bus
    pub async fn new(redis_url: &str) -> Result<Self>;

    /// Publish message to topic
    ///
    /// # Arguments
    /// * `topic` - Topic name
    /// * `message` - Message to publish
    pub async fn publish(
        &self,
        topic: &str,
        message: NotificationMessage
    ) -> Result<()>;

    /// Subscribe to topics
    ///
    /// # Arguments
    /// * `topics` - List of topics to subscribe to
    ///
    /// # Returns
    /// * `Result<NotificationStream>` - Stream of notifications
    pub async fn subscribe(
        &self,
        topics: Vec<String>
    ) -> Result<NotificationStream>;

    /// Subscribe with pattern
    pub async fn psubscribe(
        &self,
        pattern: &str
    ) -> Result<NotificationStream>;

    /// Unsubscribe from topics
    pub async fn unsubscribe(&self, topics: &[String]) -> Result<()>;

    /// Get subscriber count for topic
    pub async fn subscriber_count(&self, topic: &str) -> Result<usize>;
}
```

### NotificationMessage

Message sent through notification bus.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMessage {
    pub id: Uuid,
    pub topic: String,
    pub payload: serde_json::Value,
    pub sender: AgentId,
    pub timestamp: u64,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
}

impl NotificationMessage {
    /// Create new message
    pub fn new(topic: impl Into<String>, payload: serde_json::Value) -> Result<Self>;

    /// Set correlation ID
    pub fn with_correlation_id(mut self, id: String) -> Self;

    /// Set reply-to topic
    pub fn with_reply_to(mut self, topic: String) -> Self;
}
```

### NotificationStream

Stream of notifications from subscription.

```rust
pub struct NotificationStream {
    // Private fields
}

impl Stream for NotificationStream {
    type Item = Result<NotificationMessage>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>>;
}

impl NotificationStream {
    /// Get next notification
    pub async fn next(&mut self) -> Option<Result<NotificationMessage>>;

    /// Try to get next notification without blocking
    pub fn try_next(&mut self) -> Option<Result<NotificationMessage>>;

    /// Close the stream
    pub async fn close(self) -> Result<()>;
}
```

## Synchronization Primitives

### DistributedLock

Distributed mutex implementation.

```rust
pub struct DistributedLock {
    // Private fields
}

impl DistributedLock {
    /// Create new distributed lock
    pub fn new(redis: &Redis, key: impl Into<String>) -> Self;

    /// Acquire lock
    pub async fn acquire(&self) -> Result<LockGuard>;

    /// Try to acquire lock
    pub async fn try_acquire(&self) -> Result<Option<LockGuard>>;

    /// Acquire with timeout
    pub async fn acquire_timeout(
        &self,
        timeout: Duration
    ) -> Result<LockGuard>;

    /// Check if locked
    pub async fn is_locked(&self) -> Result<bool>;

    /// Force unlock (dangerous!)
    pub async fn force_unlock(&self) -> Result<()>;
}
```

### LockGuard

RAII guard for distributed lock.

```rust
pub struct LockGuard {
    // Private fields
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Automatically releases lock
    }
}

impl LockGuard {
    /// Extend lock TTL
    pub async fn extend(&self, ttl: Duration) -> Result<()>;

    /// Get remaining TTL
    pub async fn ttl(&self) -> Result<Duration>;

    /// Manually release lock
    pub async fn unlock(self) -> Result<()>;
}
```

### Barrier

Distributed barrier for synchronization.

```rust
pub struct Barrier {
    // Private fields
}

impl Barrier {
    /// Create new barrier
    pub fn new(redis: &Redis, key: impl Into<String>, count: u32) -> Self;

    /// Wait at barrier
    pub async fn wait(&self) -> Result<()>;

    /// Wait with timeout
    pub async fn wait_timeout(&self, timeout: Duration) -> Result<bool>;

    /// Get current wait count
    pub async fn waiting_count(&self) -> Result<u32>;

    /// Reset barrier
    pub async fn reset(&self) -> Result<()>;
}
```

### Semaphore

Distributed semaphore for resource limiting.

```rust
pub struct Semaphore {
    // Private fields
}

impl Semaphore {
    /// Create new semaphore
    pub fn new(redis: &Redis, key: impl Into<String>, permits: u32) -> Self;

    /// Acquire permit
    pub async fn acquire(&self) -> Result<Permit>;

    /// Try to acquire permit
    pub async fn try_acquire(&self) -> Result<Option<Permit>>;

    /// Acquire multiple permits
    pub async fn acquire_many(&self, n: u32) -> Result<Permits>;

    /// Get available permits
    pub async fn available_permits(&self) -> Result<u32>;

    /// Add permits
    pub async fn add_permits(&self, n: u32) -> Result<()>;
}
```

### Permit

RAII guard for semaphore permit.

```rust
pub struct Permit {
    // Private fields
}

impl Drop for Permit {
    fn drop(&mut self) {
        // Automatically releases permit
    }
}

impl Permit {
    /// Manually release permit
    pub async fn release(self) -> Result<()>;
}
```

## Connection Management

### ConnectionPool

Redis connection pool management.

```rust
pub struct ConnectionPool {
    // Private fields
}

impl ConnectionPool {
    /// Create new pool
    pub async fn new(config: PoolConfig) -> Result<Self>;

    /// Get connection from pool
    pub async fn get(&self) -> Result<PooledConnection>;

    /// Get connection with timeout
    pub async fn get_timeout(
        &self,
        timeout: Duration
    ) -> Result<PooledConnection>;

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats;

    /// Check pool health
    pub async fn health_check(&self) -> Result<()>;

    /// Resize pool
    pub async fn resize(&self, size: u32) -> Result<()>;

    /// Clear idle connections
    pub async fn clear_idle(&self) -> Result<()>;

    /// Shutdown pool
    pub async fn shutdown(self) -> Result<()>;
}
```

### PoolConfig

Connection pool configuration.

```rust
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub url: String,
    pub max_size: u32,
    pub min_idle: Option<u32>,
    pub max_lifetime: Option<Duration>,
    pub idle_timeout: Option<Duration>,
    pub connection_timeout: Duration,
    pub test_on_check_out: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            max_size: 20,
            min_idle: Some(5),
            max_lifetime: Some(Duration::from_secs(1800)),
            idle_timeout: Some(Duration::from_secs(600)),
            connection_timeout: Duration::from_secs(5),
            test_on_check_out: true,
        }
    }
}
```

### PooledConnection

Pooled Redis connection.

```rust
pub struct PooledConnection {
    // Private fields
}

impl Deref for PooledConnection {
    type Target = redis::aio::Connection;

    fn deref(&self) -> &Self::Target;
}

impl PooledConnection {
    /// Test connection
    pub async fn ping(&mut self) -> Result<()>;

    /// Get connection ID
    pub fn id(&self) -> u64;

    /// Check if connection is valid
    pub fn is_valid(&self) -> bool;
}
```

## Error Types

### Error

Main error type for the library.

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Connection pool error: {0}")]
    Pool(String),

    #[error("Task validation error: {0}")]
    TaskValidation(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("Notification error: {0}")]
    Notification(String),

    #[error("Lock error: {0}")]
    Lock(String),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl Error {
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool;

    /// Get error code
    pub fn code(&self) -> ErrorCode;

    /// Get retry advice
    pub fn retry_after(&self) -> Option<Duration>;
}
```

### ErrorCode

Error classification codes.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ConnectionFailed = 1001,
    PoolExhausted = 1002,
    TaskNotFound = 2001,
    TaskExpired = 2002,
    ContextVersionMismatch = 3001,
    LockAcquisitionFailed = 4001,
    Timeout = 5001,
    InvalidConfiguration = 6001,
    PlatformNotSupported = 7001,
}
```

## Configuration

### Config

Main configuration structure.

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub redis: RedisConfig,
    pub pool: PoolConfig,
    pub task_queue: TaskQueueConfig,
    pub context: ContextConfig,
    pub notification: NotificationConfig,
    pub metrics: MetricsConfig,
    pub security: SecurityConfig,
}

impl Default for Config {
    fn default() -> Self;
}

impl Config {
    /// Load from file
    pub fn from_file(path: &Path) -> Result<Self>;

    /// Load from environment
    pub fn from_env() -> Result<Self>;

    /// Validate configuration
    pub fn validate(&self) -> Result<()>;

    /// Builder pattern
    pub fn builder() -> ConfigBuilder;
}
```

### ConfigBuilder

Configuration builder.

```rust
pub struct ConfigBuilder {
    // Private fields
}

impl ConfigBuilder {
    /// Set Redis URL
    pub fn redis_url(mut self, url: impl Into<String>) -> Self;

    /// Set pool size
    pub fn pool_size(mut self, size: u32) -> Self;

    /// Set connection timeout
    pub fn connection_timeout(mut self, timeout: Duration) -> Self;

    /// Enable metrics
    pub fn enable_metrics(mut self) -> Self;

    /// Enable TLS
    pub fn enable_tls(mut self, tls_config: TlsConfig) -> Self;

    /// Enable compression
    pub fn enable_compression(mut self, threshold: usize) -> Self;

    /// Build configuration
    pub fn build(self) -> Result<Config>;
}
```

## Metrics API

### Metrics

Performance metrics collection.

```rust
pub struct Metrics {
    // Private fields
}

impl Metrics {
    /// Get counter value
    pub fn counter(&self, name: &str) -> u64;

    /// Get gauge value
    pub fn gauge(&self, name: &str) -> f64;

    /// Get histogram
    pub fn histogram(&self, name: &str) -> Option<Histogram>;

    /// Export Prometheus format
    pub fn export_prometheus(&self) -> String;

    /// Export JSON format
    pub fn export_json(&self) -> serde_json::Value;

    /// Reset all metrics
    pub fn reset(&self);
}
```

### Available Metrics

```rust
// Task Queue Metrics
agent_sync_tasks_enqueued_total
agent_sync_tasks_dequeued_total
agent_sync_tasks_completed_total
agent_sync_tasks_failed_total
agent_sync_task_queue_depth
agent_sync_task_processing_duration_seconds

// Context Metrics
agent_sync_context_stores_total
agent_sync_context_retrievals_total
agent_sync_context_updates_total
agent_sync_context_deletes_total
agent_sync_context_version_conflicts_total

// Notification Metrics
agent_sync_notifications_published_total
agent_sync_notifications_received_total
agent_sync_notification_subscribers

// Connection Metrics
agent_sync_redis_connections_active
agent_sync_redis_connections_idle
agent_sync_redis_connection_errors_total
agent_sync_redis_command_duration_seconds

// Lock Metrics
agent_sync_locks_acquired_total
agent_sync_locks_released_total
agent_sync_lock_wait_duration_seconds
agent_sync_lock_held_duration_seconds
```

## Platform-Specific APIs

### Windows Platform

```rust
#[cfg(target_os = "windows")]
pub mod windows {
    /// Windows-specific optimizations
    pub struct WindowsPlatform;

    impl WindowsPlatform {
        /// Initialize Windows-specific features
        pub async fn init() -> Result<()>;

        /// Use Windows named pipes for IPC
        pub fn enable_named_pipes() -> Result<()>;

        /// Integrate with Windows Event Log
        pub fn enable_event_log() -> Result<()>;
    }
}
```

### Linux Platform

```rust
#[cfg(target_os = "linux")]
pub mod linux {
    /// Linux-specific optimizations
    pub struct LinuxPlatform;

    impl LinuxPlatform {
        /// Initialize Linux-specific features
        pub async fn init() -> Result<()>;

        /// Use io_uring for async I/O
        pub fn enable_io_uring() -> Result<()>;

        /// Enable hugepages
        pub fn enable_hugepages() -> Result<()>;
    }
}
```

### WSL Platform

```rust
#[cfg(all(target_os = "linux", feature = "wsl"))]
pub mod wsl {
    /// WSL-specific optimizations
    pub struct WslPlatform;

    impl WslPlatform {
        /// Initialize WSL-specific features
        pub async fn init() -> Result<()>;

        /// Enable Windows interop
        pub fn enable_windows_interop() -> Result<()>;

        /// Share Redis with Windows host
        pub fn share_windows_redis() -> Result<()>;
    }
}
```

## Feature Flags

### Available Features

```toml
[features]
default = ["tokio-runtime", "serde-support"]
tokio-runtime = ["tokio", "tokio-util"]
async-std-runtime = ["async-std", "async-trait"]
serde-support = ["serde", "serde_json", "bincode", "rmp-serde"]
metrics = ["prometheus"]
tls = ["tokio-native-tls", "native-tls"]
cluster = ["crc16"]
test-utilities = []
wsl-optimized = []
```

### Feature-Gated APIs

```rust
// Only available with "metrics" feature
#[cfg(feature = "metrics")]
pub fn enable_prometheus_endpoint(port: u16) -> Result<()>;

// Only available with "tls" feature
#[cfg(feature = "tls")]
pub fn configure_tls(config: TlsConfig) -> Result<()>;

// Only available with "cluster" feature
#[cfg(feature = "cluster")]
pub fn enable_cluster_mode(nodes: Vec<String>) -> Result<()>;

// Only available with "test-utilities" feature
#[cfg(feature = "test-utilities")]
pub mod testing {
    pub fn create_test_redis() -> TestRedis;
    pub fn create_mock_task() -> Task;
}
```

## Usage Examples

### Complete Example

```rust
use agent_sync::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize library
    agent_sync::init().await?;

    // Configure client
    let config = Config::builder()
        .redis_url("redis://localhost:6379")
        .pool_size(50)
        .enable_metrics()
        .build()?;

    // Create client
    let client = AgentSync::new(config).await?;

    // Submit task
    let task_id = client.submit_task(
        "process_document",
        b"document content",
        TaskOptions::default()
            .with_priority(TaskPriority::High)
            .with_timeout(60)
    ).await?;

    // Store context
    client.store_context(
        "processing_state",
        json!({
            "task_id": task_id,
            "status": "processing"
        })
    ).await?;

    // Publish notification
    client.publish(
        "task.started",
        json!({
            "task_id": task_id,
            "timestamp": Timestamp::now().as_millis()
        })
    ).await?;

    // Wait for result
    let result = client.await_task_result(task_id).await?;
    println!("Task completed: {:?}", result);

    // Shutdown
    client.shutdown().await?;

    Ok(())
}
```

## Thread Safety

All types in this library are thread-safe and can be shared across threads:

- `AgentSync` - `Send + Sync`
- `TaskQueue` - `Send + Sync`
- `SharedContext` - `Send + Sync`
- `NotificationBus` - `Send + Sync`
- `ConnectionPool` - `Send + Sync`

## Performance Considerations

1. **Connection Pooling**: Always use connection pooling for production
2. **Batching**: Use batch operations when processing multiple items
3. **Compression**: Enable compression for large payloads (>1KB)
4. **Pipelining**: Pipeline Redis commands when possible
5. **Async/Await**: All operations are async - use appropriate executors

## Versioning

This library follows [Semantic Versioning 2.0.0](https://semver.org/).

- MAJOR version for incompatible API changes
- MINOR version for backwards-compatible functionality
- PATCH version for backwards-compatible bug fixes

Current version: 0.1.0