//! Agent coordination primitives
//!
//! This module provides coordination mechanisms for distributed agent systems,
//! including task queues, shared context management, and notification systems.

pub mod notification_bus;
pub mod shared_context;
pub mod task_queue;

pub use notification_bus::{
    Notification, NotificationBus, NotificationFilter, NotificationPriority, NotificationStats,
    NotificationType, Subscriber,
};
pub use shared_context::{ContextData, ContextResult, ContextScope, SharedContext};
pub use task_queue::{Task, TaskPriority, TaskQueue, TaskQueueStats, TaskResult, TaskStatus};

use crate::Result;

/// Coordinator configuration
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Redis connection string
    pub redis_url: String,
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: usize,
    /// Task timeout in seconds
    pub task_timeout_secs: u64,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Buffer size for notifications
    pub notification_buffer_size: usize,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            max_concurrent_tasks: 100,
            task_timeout_secs: 300,
            enable_metrics: true,
            notification_buffer_size: 1000,
        }
    }
}

/// Main coordinator for agent synchronization
pub struct Coordinator {
    task_queue: TaskQueue,
    shared_context: SharedContext,
    notification_bus: NotificationBus,
    #[allow(dead_code)]
    config: CoordinatorConfig,
}

impl Coordinator {
    /// Create a new coordinator
    pub async fn new(config: CoordinatorConfig) -> Result<Self> {
        let task_queue = TaskQueue::new(&config.redis_url).await?;
        let shared_context = SharedContext::new(&config.redis_url).await?;
        let notification_bus = NotificationBus::new(&config.redis_url).await?;

        Ok(Self {
            task_queue,
            shared_context,
            notification_bus,
            config,
        })
    }

    /// Get task queue reference
    pub fn task_queue(&self) -> &TaskQueue {
        &self.task_queue
    }

    /// Get shared context reference
    pub fn shared_context(&self) -> &SharedContext {
        &self.shared_context
    }

    /// Get notification bus reference
    pub fn notification_bus(&self) -> &NotificationBus {
        &self.notification_bus
    }

    /// Shutdown the coordinator
    pub async fn shutdown(self) -> Result<()> {
        // Shutdown in reverse order
        self.notification_bus.shutdown().await?;
        self.shared_context.shutdown().await?;
        self.task_queue.shutdown().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_config_default() {
        let config = CoordinatorConfig::default();
        assert_eq!(config.redis_url, "redis://localhost:6379");
        assert_eq!(config.max_concurrent_tasks, 100);
        assert_eq!(config.task_timeout_secs, 300);
        assert!(config.enable_metrics);
        assert_eq!(config.notification_buffer_size, 1000);
    }
}
