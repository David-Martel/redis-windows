//! Testing utilities and mocks for agent-sync
//!
//! This module provides utilities for testing the agent-sync library,
//! including Redis mocks, test helpers, and fixtures.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mockall::mock;
use redis::{AsyncCommands, Client};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::coordinator::{
    ContextData, ContextResult, ContextScope, Notification, NotificationFilter,
    NotificationPriority, NotificationType, Task, TaskPriority, TaskResult, TaskStatus,
};
use crate::{AgentId, Error, Result, SessionId};

pub mod fixtures;
pub mod helpers;
pub mod redis_mock;

// Re-export common testing types
pub use fixtures::*;
pub use helpers::*;
pub use redis_mock::*;

/// Mock task queue for testing
mock! {
    pub TaskQueue {
        async fn enqueue(&self, task: Task) -> Result<()>;
        async fn dequeue(&self, agent_id: AgentId) -> Result<Option<Task>>;
        async fn complete_task(&self, task_id: Uuid, result: TaskResult) -> Result<()>;
        async fn fail_task(&self, task_id: Uuid, error: String) -> Result<()>;
        async fn get_task(&self, task_id: Uuid) -> Result<Option<Task>>;
        async fn get_stats(&self) -> Result<crate::coordinator::TaskQueueStats>;
        async fn shutdown(self) -> Result<()>;
    }
}

/// Mock shared context for testing
mock! {
    pub SharedContext {
        async fn set(&self, scope: ContextScope, key: &str, data: ContextData) -> Result<ContextResult>;
        async fn get(&self, scope: ContextScope, key: &str) -> Result<Option<ContextData>>;
        async fn update(&self, scope: ContextScope, key: &str, data: ContextData, expected_revision: u64) -> Result<ContextResult>;
        async fn delete(&self, scope: ContextScope, key: &str) -> Result<bool>;
        async fn exists(&self, scope: ContextScope, key: &str) -> Result<bool>;
        async fn list_keys(&self, scope: ContextScope, pattern: &str) -> Result<Vec<String>>;
        async fn set_ttl(&self, scope: ContextScope, key: &str, ttl: Duration) -> Result<bool>;
        async fn get_ttl(&self, scope: ContextScope, key: &str) -> Result<Option<Duration>>;
        async fn increment(&self, scope: ContextScope, key: &str, amount: i64) -> Result<i64>;
        async fn clear_scope(&self, scope: ContextScope) -> Result<u32>;
        async fn shutdown(self) -> Result<()>;
    }
}

/// Mock notification bus for testing
mock! {
    pub NotificationBus {
        async fn publish(&self, notification: Notification) -> Result<u64>;
        async fn subscribe(&self, agent_id: AgentId, filter: NotificationFilter) -> Result<(Uuid, mpsc::UnboundedReceiver<Notification>)>;
        async fn unsubscribe(&self, subscription_id: Uuid) -> Result<bool>;
        async fn broadcast_system(&self, component: &str, level: &str, message: &str, sender: AgentId) -> Result<u64>;
        async fn get_stats(&self) -> crate::coordinator::NotificationStats;
        async fn list_subscriptions(&self) -> Vec<(Uuid, AgentId)>;
        async fn cleanup(&self) -> Result<(u32, u32)>;
        async fn shutdown(self) -> Result<()>;
    }
}

/// Mock coordinator for testing
mock! {
    pub Coordinator {
        async fn new(config: crate::coordinator::CoordinatorConfig) -> Result<Self>;
        fn task_queue(&self) -> &MockTaskQueue;
        fn shared_context(&self) -> &MockSharedContext;
        fn notification_bus(&self) -> &MockNotificationBus;
        async fn shutdown(self) -> Result<()>;
    }
}

/// In-memory Redis-like data store for testing
#[derive(Debug, Default)]
pub struct MemoryStore {
    data: Arc<Mutex<HashMap<String, StoreEntry>>>,
    pub_sub: Arc<Mutex<HashMap<String, Vec<mpsc::UnboundedSender<Vec<u8>>>>>>,
}

#[derive(Debug, Clone)]
struct StoreEntry {
    value: Vec<u8>,
    expires_at: Option<u64>,
    created_at: u64,
}

impl StoreEntry {
    fn new(value: Vec<u8>, ttl_secs: Option<u64>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            value,
            expires_at: ttl_secs.map(|ttl| now + ttl),
            created_at: now,
        }
    }

    fn is_expired(&self) -> bool {
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
}

impl MemoryStore {
    /// Create a new memory store
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a key-value pair with optional TTL
    pub fn set(&self, key: &str, value: Vec<u8>, ttl_secs: Option<u64>) {
        let mut data = self.data.lock().unwrap();
        data.insert(key.to_string(), StoreEntry::new(value, ttl_secs));
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut data = self.data.lock().unwrap();

        if let Some(entry) = data.get(key) {
            if entry.is_expired() {
                data.remove(key);
                None
            } else {
                Some(entry.value.clone())
            }
        } else {
            None
        }
    }

    /// Delete a key
    pub fn delete(&self, key: &str) -> bool {
        let mut data = self.data.lock().unwrap();
        data.remove(key).is_some()
    }

    /// Check if key exists
    pub fn exists(&self, key: &str) -> bool {
        let mut data = self.data.lock().unwrap();

        if let Some(entry) = data.get(key) {
            if entry.is_expired() {
                data.remove(key);
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Get keys matching pattern
    pub fn keys(&self, pattern: &str) -> Vec<String> {
        let mut data = self.data.lock().unwrap();

        // Simple glob-like pattern matching
        let regex_pattern = pattern.replace("*", ".*");
        let regex = regex::Regex::new(&regex_pattern).unwrap();

        let mut keys = Vec::new();
        let mut expired_keys = Vec::new();

        for (key, entry) in data.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            } else if regex.is_match(key) {
                keys.push(key.clone());
            }
        }

        // Clean up expired keys
        for key in expired_keys {
            data.remove(&key);
        }

        keys
    }

    /// Set TTL for existing key
    pub fn expire(&self, key: &str, ttl_secs: u64) -> bool {
        let mut data = self.data.lock().unwrap();

        if let Some(entry) = data.get_mut(key) {
            if !entry.is_expired() {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                entry.expires_at = Some(now + ttl_secs);
                true
            } else {
                data.remove(key);
                false
            }
        } else {
            false
        }
    }

    /// Get TTL for key
    pub fn ttl(&self, key: &str) -> i32 {
        let mut data = self.data.lock().unwrap();

        if let Some(entry) = data.get(key) {
            if entry.is_expired() {
                data.remove(key);
                -2 // Key doesn't exist
            } else if let Some(expires_at) = entry.expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                (expires_at as i64 - now as i64).max(0) as i32
            } else {
                -1 // No expiration
            }
        } else {
            -2 // Key doesn't exist
        }
    }

    /// Increment numeric value
    pub fn incr(&self, key: &str, amount: i64) -> Result<i64> {
        let mut data = self.data.lock().unwrap();

        let current_value = if let Some(entry) = data.get(key) {
            if entry.is_expired() {
                data.remove(key);
                0
            } else {
                let value_str = String::from_utf8(entry.value.clone())
                    .map_err(|_| Error::other("Invalid UTF-8 in stored value".to_string()))?;
                value_str
                    .parse::<i64>()
                    .map_err(|_| Error::other("Stored value is not a valid integer".to_string()))?
            }
        } else {
            0
        };

        let new_value = current_value + amount;
        data.insert(
            key.to_string(),
            StoreEntry::new(new_value.to_string().into_bytes(), None),
        );

        Ok(new_value)
    }

    /// Publish message to channel
    pub fn publish(&self, channel: &str, message: Vec<u8>) -> usize {
        let pub_sub = self.pub_sub.lock().unwrap();

        if let Some(subscribers) = pub_sub.get(channel) {
            let mut delivered = 0;
            for sender in subscribers {
                if sender.send(message.clone()).is_ok() {
                    delivered += 1;
                }
            }
            delivered
        } else {
            0
        }
    }

    /// Subscribe to channel
    pub fn subscribe(&self, channel: &str) -> mpsc::UnboundedReceiver<Vec<u8>> {
        let mut pub_sub = self.pub_sub.lock().unwrap();
        let (sender, receiver) = mpsc::unbounded_channel();

        pub_sub
            .entry(channel.to_string())
            .or_insert_with(Vec::new)
            .push(sender);

        receiver
    }

    /// Clear all data
    pub fn clear(&self) {
        let mut data = self.data.lock().unwrap();
        let mut pub_sub = self.pub_sub.lock().unwrap();

        data.clear();
        pub_sub.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> MemoryStoreStats {
        let data = self.data.lock().unwrap();
        let pub_sub = self.pub_sub.lock().unwrap();

        let total_keys = data.len();
        let expired_keys = data.values().filter(|entry| entry.is_expired()).count();
        let active_keys = total_keys - expired_keys;
        let channels = pub_sub.len();
        let total_subscribers = pub_sub.values().map(|subs| subs.len()).sum();

        MemoryStoreStats {
            total_keys,
            active_keys,
            expired_keys,
            channels,
            total_subscribers,
        }
    }
}

/// Memory store statistics
#[derive(Debug, Clone)]
pub struct MemoryStoreStats {
    pub total_keys: usize,
    pub active_keys: usize,
    pub expired_keys: usize,
    pub channels: usize,
    pub total_subscribers: usize,
}

/// Test Redis server using memory store
pub struct TestRedisServer {
    store: MemoryStore,
    server_url: String,
}

impl TestRedisServer {
    /// Create a new test Redis server
    pub fn new() -> Self {
        Self {
            store: MemoryStore::new(),
            server_url: "redis://localhost:6379".to_string(),
        }
    }

    /// Get the server URL
    pub fn url(&self) -> &str {
        &self.server_url
    }

    /// Get access to the underlying memory store
    pub fn store(&self) -> &MemoryStore {
        &self.store
    }

    /// Clear all data
    pub fn clear(&self) {
        self.store.clear();
    }

    /// Get server statistics
    pub fn stats(&self) -> MemoryStoreStats {
        self.store.stats()
    }
}

impl Default for TestRedisServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock Redis connection for testing
pub struct MockRedisConnection {
    store: Arc<MemoryStore>,
}

impl MockRedisConnection {
    /// Create a new mock connection
    pub fn new(store: Arc<MemoryStore>) -> Self {
        Self { store }
    }

    /// Simulate Redis commands
    pub async fn execute_command(&self, cmd: &str, args: &[&str]) -> Result<String> {
        match cmd.to_uppercase().as_str() {
            "SET" => {
                if args.len() >= 2 {
                    self.store.set(args[0], args[1].as_bytes().to_vec(), None);
                    Ok("OK".to_string())
                } else {
                    Err(Error::other("SET requires key and value".to_string()))
                }
            }
            "GET" => {
                if args.len() >= 1 {
                    if let Some(value) = self.store.get(args[0]) {
                        Ok(String::from_utf8_lossy(&value).to_string())
                    } else {
                        Ok("(nil)".to_string())
                    }
                } else {
                    Err(Error::other("GET requires key".to_string()))
                }
            }
            "DEL" => {
                if args.len() >= 1 {
                    let deleted = self.store.delete(args[0]);
                    Ok(if deleted { "1" } else { "0" }.to_string())
                } else {
                    Err(Error::other("DEL requires key".to_string()))
                }
            }
            "EXISTS" => {
                if args.len() >= 1 {
                    let exists = self.store.exists(args[0]);
                    Ok(if exists { "1" } else { "0" }.to_string())
                } else {
                    Err(Error::other("EXISTS requires key".to_string()))
                }
            }
            "KEYS" => {
                if args.len() >= 1 {
                    let keys = self.store.keys(args[0]);
                    Ok(format!("{:?}", keys))
                } else {
                    Err(Error::other("KEYS requires pattern".to_string()))
                }
            }
            _ => Err(Error::other(format!("Unsupported command: {}", cmd))),
        }
    }
}

/// Error extension for testing
impl Error {
    /// Create a test error
    pub fn test<S: Into<String>>(message: S) -> Self {
        Error::other(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_memory_store_basic_operations() {
        let store = MemoryStore::new();

        // Test set/get
        store.set("key1", b"value1".to_vec(), None);
        assert_eq!(store.get("key1"), Some(b"value1".to_vec()));

        // Test exists
        assert!(store.exists("key1"));
        assert!(!store.exists("key2"));

        // Test delete
        assert!(store.delete("key1"));
        assert!(!store.exists("key1"));
        assert!(!store.delete("key1")); // Second delete returns false
    }

    #[test]
    fn test_memory_store_expiration() {
        let store = MemoryStore::new();

        // Set with TTL
        store.set("expiring_key", b"value".to_vec(), Some(1));
        assert!(store.exists("expiring_key"));

        // Wait for expiration
        std::thread::sleep(Duration::from_secs(2));
        assert!(!store.exists("expiring_key"));
        assert_eq!(store.get("expiring_key"), None);
    }

    #[test]
    fn test_memory_store_keys_pattern() {
        let store = MemoryStore::new();

        // Set multiple keys
        store.set("test:key1", b"value1".to_vec(), None);
        store.set("test:key2", b"value2".to_vec(), None);
        store.set("other:key", b"value3".to_vec(), None);

        // Test pattern matching
        let test_keys = store.keys("test:*");
        assert_eq!(test_keys.len(), 2);
        assert!(test_keys.contains(&"test:key1".to_string()));
        assert!(test_keys.contains(&"test:key2".to_string()));

        let all_keys = store.keys("*");
        assert_eq!(all_keys.len(), 3);
    }

    #[test]
    fn test_memory_store_increment() {
        let store = MemoryStore::new();

        // Increment non-existent key (should start at 0)
        let result = store.incr("counter", 1).unwrap();
        assert_eq!(result, 1);

        // Increment existing key
        let result = store.incr("counter", 5).unwrap();
        assert_eq!(result, 6);

        // Decrement
        let result = store.incr("counter", -2).unwrap();
        assert_eq!(result, 4);
    }

    #[tokio::test]
    async fn test_memory_store_pubsub() {
        let store = MemoryStore::new();

        // Subscribe to channel
        let mut receiver = store.subscribe("test_channel");

        // Publish message
        let delivered = store.publish("test_channel", b"test_message".to_vec());
        assert_eq!(delivered, 1);

        // Receive message
        let message = receiver.recv().await.unwrap();
        assert_eq!(message, b"test_message");
    }

    #[test]
    fn test_memory_store_ttl() {
        let store = MemoryStore::new();

        // Set key without TTL
        store.set("persistent", b"value".to_vec(), None);
        assert_eq!(store.ttl("persistent"), -1); // No expiration

        // Set TTL
        assert!(store.expire("persistent", 60));
        let ttl = store.ttl("persistent");
        assert!(ttl > 0 && ttl <= 60);

        // Non-existent key
        assert_eq!(store.ttl("non_existent"), -2);
    }

    #[test]
    fn test_memory_store_stats() {
        let store = MemoryStore::new();

        // Initially empty
        let stats = store.stats();
        assert_eq!(stats.total_keys, 0);
        assert_eq!(stats.active_keys, 0);

        // Add some keys
        store.set("key1", b"value1".to_vec(), None);
        store.set("key2", b"value2".to_vec(), Some(3600)); // 1 hour TTL

        let stats = store.stats();
        assert_eq!(stats.total_keys, 2);
        assert_eq!(stats.active_keys, 2);
        assert_eq!(stats.expired_keys, 0);
    }
}
