//! High-level client interface for agent-sync.
//!
//! `AgentSync` is the primary entry point for applications. It wraps a connection
//! pool, coordinator, and agent registry into a single ergonomic API.

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::connection::ConnectionPool;
use crate::coordinator::{
    AgentCapabilities, AgentRegistry, ContextData, ContextResult, ContextScope, Coordinator,
    CoordinatorConfig, Notification, NotificationFilter, RegisteredAgent, Task, TaskPriority,
    TaskQueueStats, TaskResult,
};
use crate::error::Result;
use crate::{AgentId, Config};

/// Main client for agent synchronization.
///
/// Provides a unified API over connection pooling, task coordination,
/// shared context, notifications, and agent registration.
pub struct AgentSync {
    pool: ConnectionPool,
    coordinator: Coordinator,
    registry: AgentRegistry,
    config: Config,
    self_agent_id: Option<AgentId>,
    heartbeat_handle: Option<tokio::task::JoinHandle<()>>,
}

impl AgentSync {
    /// Connect to Redis and initialize all subsystems.
    /// Connect to Redis
    ///
    /// # Errors
    ///
    /// Returns error if connection fails.
pub async fn connect(config: Config) -> Result<Self> {
        config.validate()?;

        let pool = ConnectionPool::new(&config).await?;

        let coord_config = CoordinatorConfig {
            redis_url: config.redis_url.clone(),
            ..CoordinatorConfig::default()
        };
        let coordinator = Coordinator::new_with_pool(coord_config, &pool).await?;

        let registry_conn = pool.get_multiplexed().await?;
        let registry = AgentRegistry::new_with_connection(registry_conn, config.agent_expiry);

        Ok(Self {
            pool,
            coordinator,
            registry,
            config,
            self_agent_id: None,
            heartbeat_handle: None,
        })
    }

    // ── Task Methods ────────────────────────────────────────────────

    /// Submit a task to the distributed queue.
    pub async fn submit_task(
        &self,
        name: impl Into<String>,
        payload: Vec<u8>,
        created_by: AgentId,
    ) -> Result<Uuid> {
        let task = Task::new(name, payload, created_by);
        let id = task.id;
        self.coordinator.task_queue().enqueue(task).await?;
        Ok(id)
    }

    /// Submit a task with priority.
    pub async fn submit_task_with_priority(
        &self,
        name: impl Into<String>,
        payload: Vec<u8>,
        created_by: AgentId,
        priority: TaskPriority,
    ) -> Result<Uuid> {
        let task = Task::new(name, payload, created_by).with_priority(priority);
        let id = task.id;
        self.coordinator.task_queue().enqueue(task).await?;
        Ok(id)
    }

    /// Claim (dequeue) the next highest-priority task for an agent.
    pub async fn claim_task(&self, agent_id: AgentId) -> Result<Option<Task>> {
        self.coordinator.task_queue().dequeue(agent_id).await
    }

    /// Mark a task as completed.
    pub async fn complete_task(&self, task_id: Uuid, result: TaskResult) -> Result<()> {
        self.coordinator
            .task_queue()
            .complete_task(task_id, result)
            .await
    }

    /// Mark a task as failed. Will re-enqueue if retries remain.
    pub async fn fail_task(&self, task_id: Uuid, error: String) -> Result<()> {
        self.coordinator
            .task_queue()
            .fail_task(task_id, error)
            .await
    }

    /// Look up a task by ID across all queues.
    pub async fn get_task(&self, task_id: Uuid) -> Result<Option<Task>> {
        self.coordinator.task_queue().get_task(task_id).await
    }

    /// Get task queue statistics.
    pub async fn task_stats(&self) -> Result<TaskQueueStats> {
        self.coordinator.task_queue().get_stats().await
    }

    // ── Context Methods ─────────────────────────────────────────────

    /// Store context data under a specific scope and key.
    pub async fn set_context(
        &self,
        scope: ContextScope,
        key: &str,
        data: ContextData,
    ) -> Result<ContextResult> {
        self.coordinator
            .shared_context()
            .set(scope, key, data)
            .await
    }

    /// Retrieve context data.
    pub async fn get_context(&self, scope: ContextScope, key: &str) -> Result<Option<ContextData>> {
        self.coordinator.shared_context().get(scope, key).await
    }

    /// Update context data with optimistic locking.
    pub async fn update_context(
        &self,
        scope: ContextScope,
        key: &str,
        data: ContextData,
        expected_revision: u64,
    ) -> Result<ContextResult> {
        self.coordinator
            .shared_context()
            .update(scope, key, data, expected_revision)
            .await
    }

    /// Delete context data.
    pub async fn delete_context(&self, scope: ContextScope, key: &str) -> Result<bool> {
        self.coordinator.shared_context().delete(scope, key).await
    }

    /// Convenience: set a global context key.
    pub async fn set_global(&self, key: &str, data: ContextData) -> Result<ContextResult> {
        self.set_context(ContextScope::Global, key, data).await
    }

    /// Convenience: get a global context key.
    pub async fn get_global(&self, key: &str) -> Result<Option<ContextData>> {
        self.get_context(ContextScope::Global, key).await
    }

    // ── Notification Methods ────────────────────────────────────────

    /// Subscribe to notifications matching a filter.
    pub async fn subscribe(
        &self,
        agent_id: AgentId,
        filter: NotificationFilter,
    ) -> Result<(Uuid, mpsc::UnboundedReceiver<Notification>)> {
        self.coordinator
            .notification_bus()
            .subscribe(agent_id, filter)
            .await
    }

    /// Publish a notification.
    pub async fn publish(&self, notification: Notification) -> Result<u64> {
        self.coordinator
            .notification_bus()
            .publish(notification)
            .await
    }

    /// Unsubscribe from notifications.
    pub async fn unsubscribe(&self, subscription_id: Uuid) -> Result<bool> {
        self.coordinator
            .notification_bus()
            .unsubscribe(subscription_id)
            .await
    }

    /// Broadcast a system-level notification.
    pub async fn broadcast_system(
        &self,
        component: &str,
        level: &str,
        message: &str,
        sender: AgentId,
    ) -> Result<u64> {
        self.coordinator
            .notification_bus()
            .broadcast_system(component, level, message, sender)
            .await
    }

    // ── Agent Registration Methods ──────────────────────────────────

    /// Register this process as an agent and start a background heartbeat.
    pub async fn register_self(
        &mut self,
        name: String,
        capabilities: AgentCapabilities,
    ) -> Result<RegisteredAgent> {
        let id = AgentId::new();
        let agent = self.registry.register(id, name, capabilities).await?;

        // Start background heartbeat
        let handle = self
            .registry
            .start_heartbeat_task(id, self.config.agent_heartbeat_interval);
        self.heartbeat_handle = Some(handle);
        self.self_agent_id = Some(id);

        Ok(agent)
    }

    /// Register an external agent (no heartbeat started).
    pub async fn register_agent(
        &self,
        id: AgentId,
        name: String,
        capabilities: AgentCapabilities,
    ) -> Result<RegisteredAgent> {
        self.registry.register(id, name, capabilities).await
    }

    /// Deregister an agent.
    pub async fn deregister_agent(&self, id: AgentId) -> Result<bool> {
        self.registry.deregister(id).await
    }

    /// List all registered agents.
    pub async fn list_agents(&self) -> Result<Vec<RegisteredAgent>> {
        self.registry.list().await
    }

    /// Find agents with a specific skill.
    pub async fn find_agents_by_skill(&self, skill: &str) -> Result<Vec<RegisteredAgent>> {
        self.registry.find_by_skill(skill).await
    }

    // ── Lifecycle Methods ───────────────────────────────────────────

    /// Run a health check on the connection pool.
    pub async fn health_check(&self) -> Result<bool> {
        self.pool.health_check().await
    }

    /// Gracefully shut down: stop heartbeat, deregister self, release connections.
    #[allow(clippy::unused_async)]
    pub async fn shutdown(mut self) -> Result<()> {
        // Stop heartbeat
        if let Some(handle) = self.heartbeat_handle.take() {
            handle.abort();
        }

        // Deregister self
        if let Some(id) = self.self_agent_id.take() {
            let _ = self.registry.deregister(id).await;
        }

        tracing::info!("AgentSync shutdown complete");
        Ok(())
    }
}



