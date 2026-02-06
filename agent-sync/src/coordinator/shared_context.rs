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
    /// Generate Redis key for this scope with a given namespace
    pub fn redis_key(&self, key: &str, namespace: &str) -> String {
        match self {
            Self::Global => format!("{namespace}:context:global:{key}"),
            Self::Session(session_id) => {
                format!("{namespace}:context:session:{session_id}:{key}")
            }
            Self::Agent(agent_id) => {
                format!("{namespace}:context:agent:{agent_id}:{key}")
            }
            Self::Temporary { .. } => {
                format!("{namespace}:context:temp:{key}")
            }
        }
    }

    /// Get the prefix for a scope in a given namespace
    pub fn prefix(&self, namespace: &str) -> String {
        match self {
            Self::Global => format!("{namespace}:context:global:"),
            Self::Session(session_id) => {
                format!("{namespace}:context:session:{session_id}:")
            }
            Self::Agent(agent_id) => {
                format!("{namespace}:context:agent:{agent_id}:")
            }
            Self::Temporary { .. } => format!("{namespace}:context:temp:"),
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
            .expect("System time is before UNIX EPOCH")
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
    #[must_use]
    pub fn with_expiration(mut self, expires_in: Duration) -> Self {
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX EPOCH")
            .as_secs()
            + expires_in.as_secs();
        self.expires_at = Some(expires_at);
        self
    }

    /// Add metadata
    #[must_use]
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if data is expired
    ///
    /// # Panics
    ///
    /// Panics if system time is before UNIX EPOCH.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|expires_at| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time is before UNIX EPOCH")
                .as_secs();
            now > expires_at
        })
    }

    /// Update the data payload
    pub fn update_data(&mut self, data: Vec<u8>, updated_by: AgentId) {
        self.data = data;
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX EPOCH")
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
    namespace: String,
}

impl SharedContext {
    /// Create a new shared context manager
    ///
    /// # Errors
    ///
    /// Returns error if Redis connection fails.
    pub async fn new(redis_url: &str) -> Result<Self> {
        Self::new_with_namespace(redis_url, crate::coordinator::task_queue::DEFAULT_NAMESPACE).await
    }

    /// Create a new shared context manager
    ///
    /// # Errors
    ///
    /// Returns error if Redis connection fails. with a custom namespace
    pub async fn new_with_namespace(redis_url: &str, namespace: &str) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| Error::connection(format!("Failed to create Redis client: {e}")))?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::connection(format!("Failed to connect to Redis: {e}")))?;

        Ok(Self {
            client,
            connection: Arc::new(RwLock::new(connection)),
            watches: Arc::new(RwLock::new(HashMap::new())),
            namespace: namespace.to_string(),
        })
    }

    /// Create a new shared context manager
    ///
    /// # Errors
    ///
    /// Returns error if Redis connection fails. using an existing multiplexed connection.
    pub fn new_with_connection(
        client: Client,
        connection: redis::aio::MultiplexedConnection,
    ) -> Self {
        Self::new_with_namespace_and_connection(
            client,
            connection,
            crate::coordinator::task_queue::DEFAULT_NAMESPACE,
        )
    }

    /// Create a new shared context manager
    ///
    /// # Errors
    ///
    /// Returns error if Redis connection fails. using an existing connection and custom namespace.
    pub fn new_with_namespace_and_connection(
        client: Client,
        connection: redis::aio::MultiplexedConnection,
        namespace: &str,
    ) -> Self {
        Self {
            client,
            connection: Arc::new(RwLock::new(connection)),
            watches: Arc::new(RwLock::new(HashMap::new())),
            namespace: namespace.to_string(),
        }
    }

    /// Store context data
    ///
    /// # Errors
    ///
    /// Returns error if serialization or Redis fails.
    ///
    /// # Panics
    ///
    /// Panics if system time is before UNIX EPOCH.
    pub async fn set(
        &self,
        scope: ContextScope,
        key: &str,
        data: ContextData,
    ) -> Result<ContextResult> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key, &self.namespace);

        // Serialize the context data
        let serialized = bincode::serialize(&data).map_err(|e| {
            Error::serialization(format!("Failed to serialize context data: {e}"))
        })?;

        match &scope {
            ContextScope::Temporary { ttl_secs } => {
                // Set with TTL
                conn.set_ex::<_, _, ()>(&redis_key, &serialized, *ttl_secs)
                    .await
                    .map_err(|e| Error::other(format!("Failed to set temporary context: {e}")))?;
            }
            _ => {
                // Regular set operation
                if let Some(expires_at) = data.expires_at {
                    let ttl = expires_at
                        - SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("System time is before UNIX EPOCH")
                            .as_secs();

                    conn.set_ex::<_, _, ()>(&redis_key, &serialized, ttl)
                        .await
                        .map_err(|e| {
                            Error::other(format!("Failed to set context with expiration: {e}"))
                        })?;
                } else {
                    conn.set::<_, _, ()>(&redis_key, &serialized)
                        .await
                        .map_err(|e| Error::other(format!("Failed to set context: {e}")))?;
                }
            }
        }

        drop(conn);
        tracing::debug!("Set context data for key: {}", redis_key);
        Ok(ContextResult::Success)
    }

    /// Get context data
    ///
    /// # Errors
    ///
    /// Returns error if deserialization or Redis fails.
    pub async fn get(&self, scope: ContextScope, key: &str) -> Result<Option<ContextData>> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key, &self.namespace);

        let data: Option<Vec<u8>> = conn
            .get(&redis_key)
            .await
            .map_err(|e| Error::other(format!("Failed to get context: {e}")))?;

        if let Some(data) = data {
            let context_data: ContextData = bincode::deserialize(&data).map_err(|e| {
                Error::serialization(format!("Failed to deserialize context data: {e}"))
            })?;

            // Check if expired
            if context_data.is_expired() {
                // Clean up expired data
                let _: () = conn.del(&redis_key).await.map_err(|e| {
                    Error::other(format!("Failed to delete expired context: {e}"))
                })?;
                drop(conn);
                return Ok(None);
            }

            drop(conn);
            Ok(Some(context_data))
        } else {
            drop(conn);
            Ok(None)
        }
    }

    /// Update context data with optimistic locking
    ///
    /// # Errors
    ///
    /// Returns error if deserialization or Redis fails.
    ///
    /// # Panics
    ///
    /// Panics if system time is before UNIX EPOCH.
    pub async fn update(
        &self,
        scope: ContextScope,
        key: &str,
        mut data: ContextData,
        expected_revision: u64,
    ) -> Result<ContextResult> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key, &self.namespace);

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
            .expect("System time is before UNIX EPOCH")
            .as_secs();

        // Store updated data
        let serialized = bincode::serialize(&data).map_err(|e| {
            Error::serialization(format!("Failed to serialize updated context: {e}"))
        })?;

        conn.set::<_, _, ()>(&redis_key, &serialized)
            .await
            .map_err(|e| Error::other(format!("Failed to update context: {e}")))?;

        drop(conn);
        tracing::debug!("Updated context data for key: {}", redis_key);
        Ok(ContextResult::Success)
    }

    /// Delete context data
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn delete(&self, scope: ContextScope, key: &str) -> Result<bool> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key, &self.namespace);

        let deleted: u32 = conn
            .del(&redis_key)
            .await
            .map_err(|e| Error::other(format!("Failed to delete context: {e}")))?;

        drop(conn);
        tracing::debug!("Deleted context data for key: {}", redis_key);
        Ok(deleted > 0)
    }

    /// List keys matching a pattern
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn list_keys(&self, scope: ContextScope, pattern: &str) -> Result<Vec<String>> {
        let mut conn = self.connection.write().await;
        let redis_pattern = scope.redis_key(pattern, &self.namespace);

        let keys: Vec<String> = conn
            .keys(&redis_pattern)
            .await
            .map_err(|e| Error::other(format!("Failed to list keys: {e}")))?;

        drop(conn);
        // Strip the scope prefix to return clean keys
        let prefix = scope.prefix(&self.namespace);

        let clean_keys = keys
            .into_iter()
            .filter_map(|key| key.strip_prefix(&prefix).map(String::from))
            .collect();

        Ok(clean_keys)
    }

    /// Check if key exists
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn exists(&self, scope: ContextScope, key: &str) -> Result<bool> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key, &self.namespace);

        let exists: bool = conn
            .exists(&redis_key)
            .await
            .map_err(|e| Error::other(format!("Failed to check key existence: {e}")))?;

        drop(conn);
        Ok(exists)
    }

    /// Set TTL for existing key
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn set_ttl(&self, scope: ContextScope, key: &str, ttl: Duration) -> Result<bool> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key, &self.namespace);

        let result: bool = conn
            .expire(&redis_key, i64::try_from(ttl.as_secs()).unwrap_or(i64::MAX))
            .await
            .map_err(|e| Error::other(format!("Failed to set TTL: {e}")))?;

        drop(conn);
        Ok(result)
    }

    /// Get TTL for a key
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn get_ttl(&self, scope: ContextScope, key: &str) -> Result<Option<Duration>> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key, &self.namespace);

        let ttl_secs: i32 = conn
            .ttl(&redis_key)
            .await
            .map_err(|e| Error::other(format!("Failed to get TTL: {e}")))?;

        drop(conn);
        match ttl_secs {
            -1 | -2 => Ok(None), // Key has no expiration or doesn't exist
            secs if secs >= 0 => Ok(Some(Duration::from_secs(u64::try_from(secs).unwrap_or(0)))),
            _ => Ok(None),
        }
    }

    /// Increment a numeric value atomically
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn increment(&self, scope: ContextScope, key: &str, amount: i64) -> Result<i64> {
        let mut conn = self.connection.write().await;
        let redis_key = scope.redis_key(key, &self.namespace);

        let result: i64 = conn
            .incr(&redis_key, amount)
            .await
            .map_err(|e| Error::other(format!("Failed to increment: {e}")))?;

        drop(conn);
        Ok(result)
    }

    /// Clear all data for a scope
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn clear_scope(&self, scope: ContextScope) -> Result<u32> {
        let mut conn = self.connection.write().await;
        let pattern = scope.redis_key("*", &self.namespace);

        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .map_err(|e| Error::other(format!("Failed to get keys for clearing: {e}")))?;

        if keys.is_empty() {
            return Ok(0);
        }

        let deleted: u32 = conn
            .del(&keys)
            .await
            .map_err(|e| Error::other(format!("Failed to clear scope: {e}")))?;

        drop(conn);
        tracing::debug!("Cleared {} keys for scope", deleted);
        Ok(deleted)
    }

    /// Shutdown the shared context manager
    ///
    /// # Errors
    ///
    /// Returns error if shutdown process fails.
    #[allow(clippy::unused_async)]
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
        let ns = crate::coordinator::task_queue::DEFAULT_NAMESPACE;
        let global_scope = ContextScope::Global;
        let session_scope = ContextScope::Session(SessionId::new());
        let agent_id = AgentId::new();
        let agent_scope = ContextScope::Agent(agent_id);
        let temp_scope = ContextScope::Temporary { ttl_secs: 300 };

        assert_eq!(
            global_scope.redis_key("test", ns),
            format!("{ns}:context:global:test")
        );
        assert!(session_scope
            .redis_key("test", ns)
            .starts_with(&format!("{ns}:context:session:")));
        assert!(agent_scope
            .redis_key("test", ns)
            .contains(&agent_id.to_string()));
        assert_eq!(temp_scope.redis_key("test", ns), format!("{ns}:context:temp:test"));
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



