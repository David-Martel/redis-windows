//! Shared context management for agent coordination
//!
//! Provides distributed shared state management across multiple agents using Redis
//! as the backing store with support for different data scopes and access patterns.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{AgentId, Error, Result, SessionId};

/// Context data scope determines visibility and lifecycle
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextScope {
    /// Global context visible to all agents
    Global,
    /// Session-specific context
    Session(SessionId),
    /// Agent-specific context
    Agent(AgentId),
    /// Temporary context with TTL
    Temporary {
        /// Time-to-live in seconds
        ttl_secs: u64,
    },
}

impl ContextScope {
    /// Generate Redis key for this scope
    pub fn redis_key(&self, key: &str) -> String {
        match self {
            ContextScope::Global => format!("agent_sync:context:global:{}", key),
            ContextScope::Session(session_id) => {
                format!("agent_sync:context:session:{}:{}", session_id, key)
            }
            ContextScope::Agent(agent_id) => {
                format!("agent_sync:context:agent:{}:{}", agent_id, key)
            }
            ContextScope::Temporary { .. } => {
                format!("agent_sync:context:temp:{}", key)
            }
        }
    }
}

/// Context data wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextData {
    /// The actual data payload
    pub data: Vec<u8>,
    /// Data type hint for deserialization
    pub data_type: String,
    /// Who created this context
    pub created_by: AgentId,
    /// When was this created
    pub created_at: u64,
    /// When was this last updated
    pub updated_at: u64,
    /// Optional expiration timestamp
    pub expires_at: Option<u64>,
    /// Revision counter for optimistic locking
    pub revision: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ContextData {
    /// Create new context data
    pub fn new(data: Vec<u8>, data_type: String, created_by: AgentId) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            data,
            data_type,
            created_by,
            created_at: now,
            updated_at: now,
            expires_at: None,
            revision: 1,
            metadata: HashMap::new(),
        }
    }

    /// Create context data with expiration
    pub fn with_expiration(mut self, expires_in: Duration) -> Self {
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + expires_in.as_secs();
        self.expires_at = Some(expires_at);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if data is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now > expires_at
        } else {
            false
        }
    }

    /// Update the data payload
    pub fn update_data(&mut self, data: Vec<u8>, updated_by: AgentId) {
        self.data = data;
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.revision += 1;
        self.metadata
            .insert("updated_by".to_string(), updated_by.to_string());
    }
}

/// Context operation result
#[derive(Debug, Clone)]
pub enum ContextResult {
    /// Operation succeeded
    Success,
    /// Key not found
    NotFound,
    /// Version conflict (optimistic locking)
    ConflictDetected {
        /// Current revision number in the store
        current_revision: u64,
    },
    /// Permission denied
    PermissionDenied,
    /// Data expired
    Expired,
}

/// Watch notification for context changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWatch {
    /// Watch ID
    pub id: Uuid,
    /// Key pattern being watched
    pub pattern: String,
    /// Scope being watched
    pub scope: ContextScope,
    /// Agent requesting the watch
    pub agent_id: AgentId,
    /// Callback channel or webhook
    pub callback: String,
}

/// Shared context manager
pub struct SharedContext {
    #[allow(dead_code)]
    client: Client,
    connection: Arc<RwLock<redis::aio::MultiplexedConnection>>,
    #[allow(dead_code)]
    watches: Arc<RwLock<HashMap<Uuid, ContextWatch>>>,
}

impl SharedContext {
    /// Create a new shared context manager
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
            watches: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Store context data
    pub async fn set(
        &self,
        scope: ContextScope,
        key: &str,
        data: ContextData,
    ) -> Result<ContextResult> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key);

        // Serialize the context data
        let serialized = bincode::serialize(&data).map_err(|e| {
            Error::serialization(format!("Failed to serialize context data: {}", e))
        })?;

        match &scope {
            ContextScope::Temporary { ttl_secs } => {
                // Set with TTL
                conn.set_ex::<_, _, ()>(&redis_key, &serialized, *ttl_secs as u64)
                    .await
                    .map_err(|e| Error::other(format!("Failed to set temporary context: {}", e)))?;
            }
            _ => {
                // Regular set operation
                if let Some(expires_at) = data.expires_at {
                    let ttl = expires_at
                        - SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();

                    conn.set_ex::<_, _, ()>(&redis_key, &serialized, ttl as u64)
                        .await
                        .map_err(|e| {
                            Error::other(format!("Failed to set context with expiration: {}", e))
                        })?;
                } else {
                    conn.set::<_, _, ()>(&redis_key, &serialized)
                        .await
                        .map_err(|e| Error::other(format!("Failed to set context: {}", e)))?;
                }
            }
        }

        tracing::debug!("Set context data for key: {}", redis_key);
        Ok(ContextResult::Success)
    }

    /// Get context data
    pub async fn get(&self, scope: ContextScope, key: &str) -> Result<Option<ContextData>> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key);

        let data: Option<Vec<u8>> = conn
            .get(&redis_key)
            .await
            .map_err(|e| Error::other(format!("Failed to get context: {}", e)))?;

        if let Some(data) = data {
            let context_data: ContextData = bincode::deserialize(&data).map_err(|e| {
                Error::serialization(format!("Failed to deserialize context data: {}", e))
            })?;

            // Check if expired
            if context_data.is_expired() {
                // Clean up expired data
                let _: () = conn.del(&redis_key).await.map_err(|e| {
                    Error::other(format!("Failed to delete expired context: {}", e))
                })?;
                return Ok(None);
            }

            Ok(Some(context_data))
        } else {
            Ok(None)
        }
    }

    /// Update context data with optimistic locking
    pub async fn update(
        &self,
        scope: ContextScope,
        key: &str,
        mut data: ContextData,
        expected_revision: u64,
    ) -> Result<ContextResult> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key);

        // Get current data to check revision
        if let Some(current) = self.get(scope.clone(), key).await? {
            if current.revision != expected_revision {
                return Ok(ContextResult::ConflictDetected {
                    current_revision: current.revision,
                });
            }
        } else {
            return Ok(ContextResult::NotFound);
        }

        // Update revision
        data.revision = expected_revision + 1;
        data.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Store updated data
        let serialized = bincode::serialize(&data).map_err(|e| {
            Error::serialization(format!("Failed to serialize updated context: {}", e))
        })?;

        conn.set::<_, _, ()>(&redis_key, &serialized)
            .await
            .map_err(|e| Error::other(format!("Failed to update context: {}", e)))?;

        tracing::debug!("Updated context data for key: {}", redis_key);
        Ok(ContextResult::Success)
    }

    /// Delete context data
    pub async fn delete(&self, scope: ContextScope, key: &str) -> Result<bool> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key);

        let deleted: u32 = conn
            .del(&redis_key)
            .await
            .map_err(|e| Error::other(format!("Failed to delete context: {}", e)))?;

        tracing::debug!("Deleted context data for key: {}", redis_key);
        Ok(deleted > 0)
    }

    /// List keys matching a pattern
    pub async fn list_keys(&self, scope: ContextScope, pattern: &str) -> Result<Vec<String>> {
        let mut conn = self.connection.write().await;
        let redis_pattern = scope.redis_key(pattern);

        let keys: Vec<String> = conn
            .keys(&redis_pattern)
            .await
            .map_err(|e| Error::other(format!("Failed to list keys: {}", e)))?;

        // Strip the scope prefix to return clean keys
        let prefix = match &scope {
            ContextScope::Global => "agent_sync:context:global:".to_string(),
            ContextScope::Session(session_id) => {
                format!("agent_sync:context:session:{}:", session_id)
            }
            ContextScope::Agent(agent_id) => {
                format!("agent_sync:context:agent:{}:", agent_id)
            }
            ContextScope::Temporary { .. } => "agent_sync:context:temp:".to_string(),
        };

        let clean_keys = keys
            .into_iter()
            .filter_map(|key| key.strip_prefix(&prefix).map(String::from))
            .collect();

        Ok(clean_keys)
    }

    /// Check if key exists
    pub async fn exists(&self, scope: ContextScope, key: &str) -> Result<bool> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key);

        let exists: bool = conn
            .exists(&redis_key)
            .await
            .map_err(|e| Error::other(format!("Failed to check key existence: {}", e)))?;

        Ok(exists)
    }

    /// Set TTL for existing key
    pub async fn set_ttl(&self, scope: ContextScope, key: &str, ttl: Duration) -> Result<bool> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key);

        let result: bool = conn
            .expire(&redis_key, ttl.as_secs() as i64)
            .await
            .map_err(|e| Error::other(format!("Failed to set TTL: {}", e)))?;

        Ok(result)
    }

    /// Get TTL for a key
    pub async fn get_ttl(&self, scope: ContextScope, key: &str) -> Result<Option<Duration>> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key);

        let ttl_secs: i32 = conn
            .ttl(&redis_key)
            .await
            .map_err(|e| Error::other(format!("Failed to get TTL: {}", e)))?;

        match ttl_secs {
            -1 => Ok(None), // Key exists but has no expiration
            -2 => Ok(None), // Key doesn't exist
            secs => Ok(Some(Duration::from_secs(secs as u64))),
        }
    }

    /// Increment a numeric value atomically
    pub async fn increment(&self, scope: ContextScope, key: &str, amount: i64) -> Result<i64> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key);

        let result: i64 = conn
            .incr(&redis_key, amount)
            .await
            .map_err(|e| Error::other(format!("Failed to increment: {}", e)))?;

        Ok(result)
    }

    /// Clear all data for a scope
    pub async fn clear_scope(&self, scope: ContextScope) -> Result<u32> {
        let mut conn = self.connection.write().await;
        let pattern = scope.redis_key("*");

        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .map_err(|e| Error::other(format!("Failed to get keys for clearing: {}", e)))?;

        if keys.is_empty() {
            return Ok(0);
        }

        let deleted: u32 = conn
            .del(&keys)
            .await
            .map_err(|e| Error::other(format!("Failed to clear scope: {}", e)))?;

        tracing::debug!("Cleared {} keys for scope", deleted);
        Ok(deleted)
    }

    /// Shutdown the shared context manager
    pub async fn shutdown(self) -> Result<()> {
        // Connection will be dropped automatically
        tracing::info!("Shared context manager shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod basic_tests {
    use super::*;

    #[test]
    fn test_context_scope_redis_key() {
        let global_scope = ContextScope::Global;
        let session_scope = ContextScope::Session(SessionId::new());
        let agent_id = AgentId::new();
        let agent_scope = ContextScope::Agent(agent_id);
        let temp_scope = ContextScope::Temporary { ttl_secs: 300 };

        assert_eq!(
            global_scope.redis_key("test"),
            "agent_sync:context:global:test"
        );
        assert!(session_scope
            .redis_key("test")
            .starts_with("agent_sync:context:session:"));
        assert!(agent_scope
            .redis_key("test")
            .contains(&agent_id.to_string()));
        assert_eq!(temp_scope.redis_key("test"), "agent_sync:context:temp:test");
    }

    #[test]
    fn test_context_data_creation() {
        let agent_id = AgentId::new();
        let data = ContextData::new(b"test_data".to_vec(), "String".to_string(), agent_id);

        assert_eq!(data.data, b"test_data");
        assert_eq!(data.data_type, "String");
        assert_eq!(data.created_by, agent_id);
        assert_eq!(data.revision, 1);
        assert!(data.expires_at.is_none());
        assert!(!data.is_expired());
    }

    #[test]
    fn test_context_data_with_expiration() {
        let agent_id = AgentId::new();
        let data = ContextData::new(b"test_data".to_vec(), "String".to_string(), agent_id)
            .with_expiration(Duration::from_secs(60));

        assert!(data.expires_at.is_some());
        assert!(!data.is_expired());
    }

    #[test]
    fn test_context_data_update() {
        let agent_id = AgentId::new();
        let updater_id = AgentId::new();
        let mut data = ContextData::new(b"original".to_vec(), "String".to_string(), agent_id);

        let original_revision = data.revision;
        data.update_data(b"updated".to_vec(), updater_id);

        assert_eq!(data.data, b"updated");
        assert_eq!(data.revision, original_revision + 1);
        assert!(data.metadata.contains_key("updated_by"));
    }

    #[test]
    fn test_expired_data_detection() {
        let agent_id = AgentId::new();

        // Create data that expired 1 second ago
        let mut data = ContextData::new(b"expired".to_vec(), "String".to_string(), agent_id);

        data.expires_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 1,
        );

        assert!(data.is_expired());
    }
}
