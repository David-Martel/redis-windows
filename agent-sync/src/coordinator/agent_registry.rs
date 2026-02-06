//! Agent registry for tracking registered agents with heartbeat-based liveness.
//!
//! Agents register themselves with capabilities and maintain liveness through
//! periodic heartbeats stored as Redis keys with TTL.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{AgentId, Error, Result};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Capabilities that an agent advertises
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Roles this agent can fulfil (e.g., "worker", "coordinator")
    pub roles: Vec<String>,
    /// Skills this agent has (e.g., "code-review", "testing")
    pub skills: Vec<String>,
    /// Maximum number of concurrent tasks this agent can handle
    pub max_concurrent_tasks: u32,
    /// Arbitrary metadata
    pub metadata: HashMap<String, String>,
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            roles: Vec::new(),
            skills: Vec::new(),
            max_concurrent_tasks: 1,
            metadata: HashMap::new(),
        }
    }
}

/// Information about a registered agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredAgent {
    /// Unique agent identifier
    pub id: AgentId,
    /// Human-readable name
    pub name: String,
    /// Agent capabilities
    pub capabilities: AgentCapabilities,
    /// Current status description
    pub status: String,
    /// Last heartbeat timestamp (unix seconds)
    pub last_heartbeat: u64,
    /// Registration timestamp (unix seconds)
    pub registered_at: u64,
}

/// Agent registry backed by Redis.
pub struct AgentRegistry {
    connection: Arc<RwLock<redis::aio::MultiplexedConnection>>,
    agent_expiry: Duration,
    _namespace: String,
    agents_hash: String,
    heartbeat_prefix: String,
}

impl AgentRegistry {
    /// Create a new agent registry.
    ///
    /// # Errors
    ///
    /// Returns error if Redis connection fails.
    pub async fn new(redis_url: &str, agent_expiry: Duration) -> Result<Self> {
        Self::new_with_namespace(
            redis_url,
            agent_expiry,
            crate::coordinator::task_queue::DEFAULT_NAMESPACE,
        )
        .await
    }

    /// Create a new agent registry with a custom namespace.
    ///
    /// # Errors
    ///
    /// Returns error if Redis connection fails.
    pub async fn new_with_namespace(
        redis_url: &str,
        agent_expiry: Duration,
        namespace: &str,
    ) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| Error::connection(format!("Failed to create Redis client: {e}")))?;
        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::connection(format!("Failed to connect to Redis: {e}")))?;

        Ok(Self {
            connection: Arc::new(RwLock::new(connection)),
            agent_expiry,
            _namespace: namespace.to_string(),
            agents_hash: format!("{namespace}:agents"),
            heartbeat_prefix: format!("{namespace}:agent"),
        })
    }

    /// Create a registry with an existing connection.
    pub fn new_with_connection(
        connection: redis::aio::MultiplexedConnection,
        agent_expiry: Duration,
    ) -> Self {
        Self::new_with_namespace_and_connection(
            connection,
            agent_expiry,
            crate::coordinator::task_queue::DEFAULT_NAMESPACE,
        )
    }

    /// Create a registry with an existing connection and custom namespace.
    pub fn new_with_namespace_and_connection(
        connection: redis::aio::MultiplexedConnection,
        agent_expiry: Duration,
        namespace: &str,
    ) -> Self {
        Self {
            connection: Arc::new(RwLock::new(connection)),
            agent_expiry,
            _namespace: namespace.to_string(),
            agents_hash: format!("{namespace}:agents"),
            heartbeat_prefix: format!("{namespace}:agent"),
        }
    }

    /// Register an agent.
    ///
    /// # Errors
    ///
    /// Returns error if the agent is already registered or Redis fails.
    ///
    /// # Panics
    ///
    /// Panics if system time is before UNIX EPOCH.
    pub async fn register(
        &self,
        id: AgentId,
        name: String,
        capabilities: AgentCapabilities,
    ) -> Result<RegisteredAgent> {
        let mut conn = self.connection.write().await;

        // Check if already registered
        let exists: bool = conn
            .hexists(&self.agents_hash, id.to_string())
            .await
            .map_err(|e| Error::other(format!("Failed to check agent existence: {e}")))?;

        if exists {
            return Err(Error::AgentAlreadyRegistered(id.to_string()));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX EPOCH")
            .as_secs();

        let agent = RegisteredAgent {
            id,
            name,
            capabilities,
            status: "active".to_string(),
            last_heartbeat: now,
            registered_at: now,
        };

        let serialized = serde_json::to_string(&agent)
            .map_err(|e| Error::serialization(format!("Failed to serialize agent: {e}")))?;

        // Store agent info in hash
        conn.hset::<_, _, _, ()>(&self.agents_hash, id.to_string(), &serialized)
            .await
            .map_err(|e| Error::other(format!("Failed to register agent: {e}")))?;

        // Set heartbeat TTL key
        let hb_key = format!("{}:{}:heartbeat", self.heartbeat_prefix, id);
        conn.set_ex::<_, _, ()>(&hb_key, "alive", self.agent_expiry.as_secs())
            .await
            .map_err(|e| Error::other(format!("Failed to set heartbeat key: {e}")))?;

        tracing::info!(%id, "Agent registered");
        Ok(agent)
    }

    /// Deregister an agent.
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn deregister(&self, id: AgentId) -> Result<bool> {
        let mut conn = self.connection.write().await;

        let removed: u32 = conn
            .hdel(&self.agents_hash, id.to_string())
            .await
            .map_err(|e| Error::other(format!("Failed to deregister agent: {e}")))?;

        let hb_key = format!("{}:{}:heartbeat", self.heartbeat_prefix, id);
        let _: () = conn
            .del(&hb_key)
            .await
            .map_err(|e| Error::other(format!("Failed to remove heartbeat key: {e}")))?;

        tracing::info!(%id, "Agent deregistered");
        Ok(removed > 0)
    }

    /// Send a heartbeat for an agent, refreshing the TTL.
    ///
    /// # Errors
    ///
    /// Returns error if the agent is not found or Redis fails.
    ///
    /// # Panics
    ///
    /// Panics if system time is before UNIX EPOCH.
    pub async fn heartbeat(&self, id: AgentId) -> Result<()> {
        let mut conn = self.connection.write().await;

        // Check agent exists
        let exists: bool = conn
            .hexists(&self.agents_hash, id.to_string())
            .await
            .map_err(|e| Error::other(format!("Failed to check agent: {e}")))?;

        if !exists {
            return Err(Error::AgentNotFound(id.to_string()));
        }

        // Refresh heartbeat TTL key
        let hb_key = format!("{}:{}:heartbeat", self.heartbeat_prefix, id);
        conn.set_ex::<_, _, ()>(&hb_key, "alive", self.agent_expiry.as_secs())
            .await
            .map_err(|e| Error::other(format!("Failed to refresh heartbeat: {e}")))?;

        // Update last_heartbeat in agent hash
        let data: Option<String> = conn
            .hget(&self.agents_hash, id.to_string())
            .await
            .map_err(|e| Error::other(format!("Failed to get agent data: {e}")))?;

        if let Some(data) = data {
            if let Ok(mut agent) = serde_json::from_str::<RegisteredAgent>(&data) {
                agent.last_heartbeat = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("System time is before UNIX EPOCH")
                    .as_secs();

                let updated = serde_json::to_string(&agent).map_err(|e| {
                    Error::serialization(format!("Failed to serialize agent: {e}"))
                })?;
                conn.hset::<_, _, _, ()>(&self.agents_hash, id.to_string(), &updated)
                    .await
                    .map_err(|e| Error::other(format!("Failed to update agent: {e}")))?;
            }
        }

        Ok(())
    }

    /// List all registered agents.
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn list(&self) -> Result<Vec<RegisteredAgent>> {
        let mut conn = self.connection.write().await;

        let entries: HashMap<String, String> = conn
            .hgetall(&self.agents_hash)
            .await
            .map_err(|e| Error::other(format!("Failed to list agents: {e}")))?;

        let mut agents = Vec::with_capacity(entries.len());
        for (_key, value) in entries {
            if let Ok(agent) = serde_json::from_str::<RegisteredAgent>(&value) {
                agents.push(agent);
            }
        }

        Ok(agents)
    }

    /// Find agents that have a specific skill.
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn find_by_skill(&self, skill: &str) -> Result<Vec<RegisteredAgent>> {
        let all = self.list().await?;
        Ok(all
            .into_iter()
            .filter(|a| a.capabilities.skills.iter().any(|s| s == skill))
            .collect())
    }

    /// Find agents that have a specific role.
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn find_by_role(&self, role: &str) -> Result<Vec<RegisteredAgent>> {
        let all = self.list().await?;
        Ok(all
            .into_iter()
            .filter(|a| a.capabilities.roles.iter().any(|r| r == role))
            .collect())
    }

    /// Start a background heartbeat task that periodically refreshes the heartbeat
    /// for the given agent. Returns a handle that can be aborted to stop it.
    pub fn start_heartbeat_task(
        &self,
        id: AgentId,
        interval: Duration,
    ) -> tokio::task::JoinHandle<()> {
        let connection = Arc::clone(&self.connection);
        let expiry = self.agent_expiry;
        let hb_key_prefix = self.heartbeat_prefix.clone();

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let mut conn = connection.write().await;
                let hb_key = format!("{hb_key_prefix}:{id}:heartbeat");
                if let Err(e) = conn
                    .set_ex::<_, _, ()>(&hb_key, "alive", expiry.as_secs())
                    .await
                {
                    tracing::warn!(%id, "Heartbeat failed: {}", e);
                }
                drop(conn);
            }
        })
    }

    /// Shutdown the agent registry (no-op, connections drop automatically).
    ///
    /// # Errors
    ///
    /// Returns error if shutdown process fails.
    pub fn shutdown(self) -> Result<()> {
        tracing::info!("Agent registry shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_capabilities() {
        let caps = AgentCapabilities::default();
        assert!(caps.roles.is_empty());
        assert!(caps.skills.is_empty());
        assert_eq!(caps.max_concurrent_tasks, 1);
    }

    #[test]
    fn test_registered_agent_serialization() {
        let agent = RegisteredAgent {
            id: AgentId::new(),
            name: "test-agent".to_string(),
            capabilities: AgentCapabilities {
                roles: vec!["worker".to_string()],
                skills: vec!["code-review".to_string()],
                max_concurrent_tasks: 5,
                metadata: HashMap::new(),
            },
            status: "active".to_string(),
            last_heartbeat: 1000,
            registered_at: 900,
        };

        let json = serde_json::to_string(&agent).expect("Failed to serialize agent");
        let deserialized: RegisteredAgent =
            serde_json::from_str(&json).expect("Failed to deserialize agent");
        assert_eq!(deserialized.name, "test-agent");
        assert_eq!(deserialized.capabilities.max_concurrent_tasks, 5);
    }
}





