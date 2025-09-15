//! Redis mocking utilities for testing
//!
//! This module provides comprehensive Redis mocking capabilities
//! for testing the agent-sync library without requiring a real Redis instance.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mockall::mock;
use redis::{AsyncCommands, RedisResult, Value};
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::{Error, Result};

/// Mock Redis client for testing
pub struct MockRedisClient {
    store: Arc<MockRedisStore>,
}

impl MockRedisClient {
    /// Create a new mock Redis client
    pub fn new() -> Self {
        Self {
            store: Arc::new(MockRedisStore::new()),
        }
    }

    /// Create a mock client with shared store
    pub fn with_store(store: Arc<MockRedisStore>) -> Self {
        Self { store }
    }

    /// Get reference to the underlying store
    pub fn store(&self) -> &Arc<MockRedisStore> {
        &self.store
    }

    /// Reset the mock client state
    pub fn reset(&self) {
        self.store.clear();
    }
}

impl Default for MockRedisClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock Redis store with comprehensive Redis command support
pub struct MockRedisStore {
    /// Key-value storage
    data: RwLock<HashMap<String, MockRedisValue>>,
    /// Pub/sub channels
    pubsub: RwLock<HashMap<String, broadcast::Sender<Vec<u8>>>>,
    /// Lists storage
    lists: RwLock<HashMap<String, VecDeque<Vec<u8>>>>,
    /// Sets storage
    sets: RwLock<HashMap<String, std::collections::HashSet<Vec<u8>>>>,
    /// Hash storage
    hashes: RwLock<HashMap<String, HashMap<String, Vec<u8>>>>,
    /// Sorted sets storage
    zsets: RwLock<HashMap<String, std::collections::BTreeMap<OrderedFloat, Vec<u8>>>>,
    /// Command history for debugging
    command_history: Mutex<Vec<String>>,
    /// Configuration
    config: Mutex<MockRedisConfig>,
}

/// Wrapper for f64 to make it orderable for sorted sets
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl From<f64> for OrderedFloat {
    fn from(value: f64) -> Self {
        OrderedFloat(value)
    }
}

/// Redis value with metadata
#[derive(Debug, Clone)]
struct MockRedisValue {
    data: Vec<u8>,
    expires_at: Option<u64>,
    created_at: u64,
}

impl MockRedisValue {
    fn new(data: Vec<u8>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            data,
            expires_at: None,
            created_at: now,
        }
    }

    fn with_expiry(data: Vec<u8>, ttl_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            data,
            expires_at: Some(now + ttl_secs),
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

/// Mock Redis configuration
#[derive(Debug, Clone)]
struct MockRedisConfig {
    pub simulate_network_delay: bool,
    pub network_delay_ms: u64,
    pub simulate_failures: bool,
    pub failure_rate: f64,
    pub max_memory: Option<usize>,
}

impl Default for MockRedisConfig {
    fn default() -> Self {
        Self {
            simulate_network_delay: false,
            network_delay_ms: 1,
            simulate_failures: false,
            failure_rate: 0.0,
            max_memory: None,
        }
    }
}

impl MockRedisStore {
    /// Create a new mock Redis store
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            pubsub: RwLock::new(HashMap::new()),
            lists: RwLock::new(HashMap::new()),
            sets: RwLock::new(HashMap::new()),
            hashes: RwLock::new(HashMap::new()),
            zsets: RwLock::new(HashMap::new()),
            command_history: Mutex::new(Vec::new()),
            config: Mutex::new(MockRedisConfig::default()),
        }
    }

    /// Configure the mock store
    pub fn configure(&self, config: MockRedisConfig) {
        *self.config.lock().unwrap() = config;
    }

    /// Enable network delay simulation
    pub fn enable_network_delay(&self, delay_ms: u64) {
        let mut config = self.config.lock().unwrap();
        config.simulate_network_delay = true;
        config.network_delay_ms = delay_ms;
    }

    /// Enable failure simulation
    pub fn enable_failure_simulation(&self, failure_rate: f64) {
        let mut config = self.config.lock().unwrap();
        config.simulate_failures = true;
        config.failure_rate = failure_rate;
    }

    /// Record command execution
    fn record_command(&self, command: &str) {
        let mut history = self.command_history.lock().unwrap();
        history.push(command.to_string());
    }

    /// Get command history
    pub fn get_command_history(&self) -> Vec<String> {
        self.command_history.lock().unwrap().clone()
    }

    /// Clear command history
    pub fn clear_command_history(&self) {
        self.command_history.lock().unwrap().clear();
    }

    /// Simulate network delay if configured
    async fn simulate_delay(&self) {
        let config = self.config.lock().unwrap().clone();
        if config.simulate_network_delay {
            tokio::time::sleep(Duration::from_millis(config.network_delay_ms)).await;
        }
    }

    /// Check if operation should fail
    fn should_fail(&self) -> bool {
        let config = self.config.lock().unwrap();
        if config.simulate_failures {
            rand::random::<f64>() < config.failure_rate
        } else {
            false
        }
    }

    /// Clean expired keys
    async fn cleanup_expired(&self) {
        let mut data = self.data.write().await;
        data.retain(|_, value| !value.is_expired());
    }

    // String Commands

    /// SET command implementation
    pub async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.record_command(&format!(
            "SET {} {:?}",
            key,
            String::from_utf8_lossy(&value)
        ));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut data = self.data.write().await;
        data.insert(key.to_string(), MockRedisValue::new(value));
        Ok(())
    }

    /// SETEX command implementation
    pub async fn setex(&self, key: &str, value: Vec<u8>, ttl_secs: u64) -> Result<()> {
        self.record_command(&format!(
            "SETEX {} {} {:?}",
            key,
            ttl_secs,
            String::from_utf8_lossy(&value)
        ));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut data = self.data.write().await;
        data.insert(
            key.to_string(),
            MockRedisValue::with_expiry(value, ttl_secs),
        );
        Ok(())
    }

    /// GET command implementation
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.record_command(&format!("GET {}", key));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut data = self.data.write().await;
        if let Some(value) = data.get(key) {
            if value.is_expired() {
                data.remove(key);
                Ok(None)
            } else {
                Ok(Some(value.data.clone()))
            }
        } else {
            Ok(None)
        }
    }

    /// DEL command implementation
    pub async fn del(&self, key: &str) -> Result<u32> {
        self.record_command(&format!("DEL {}", key));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut data = self.data.write().await;
        if data.remove(key).is_some() {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    /// EXISTS command implementation
    pub async fn exists(&self, key: &str) -> Result<bool> {
        self.record_command(&format!("EXISTS {}", key));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut data = self.data.write().await;
        if let Some(value) = data.get(key) {
            if value.is_expired() {
                data.remove(key);
                Ok(false)
            } else {
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    /// KEYS command implementation
    pub async fn keys(&self, pattern: &str) -> Result<Vec<String>> {
        self.record_command(&format!("KEYS {}", pattern));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        self.cleanup_expired().await;

        let data = self.data.read().await;
        let regex_pattern = pattern.replace("*", ".*");
        let regex = regex::Regex::new(&regex_pattern).unwrap();

        let keys: Vec<String> = data
            .keys()
            .filter(|key| regex.is_match(key))
            .cloned()
            .collect();

        Ok(keys)
    }

    /// EXPIRE command implementation
    pub async fn expire(&self, key: &str, ttl_secs: u64) -> Result<bool> {
        self.record_command(&format!("EXPIRE {} {}", key, ttl_secs));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut data = self.data.write().await;
        if let Some(value) = data.get_mut(key) {
            if !value.is_expired() {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                value.expires_at = Some(now + ttl_secs);
                Ok(true)
            } else {
                data.remove(key);
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// TTL command implementation
    pub async fn ttl(&self, key: &str) -> Result<i32> {
        self.record_command(&format!("TTL {}", key));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut data = self.data.write().await;
        if let Some(value) = data.get(key) {
            if value.is_expired() {
                data.remove(key);
                Ok(-2) // Key doesn't exist
            } else if let Some(expires_at) = value.expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                Ok((expires_at as i64 - now as i64).max(0) as i32)
            } else {
                Ok(-1) // No expiration
            }
        } else {
            Ok(-2) // Key doesn't exist
        }
    }

    /// INCR command implementation
    pub async fn incr(&self, key: &str, amount: i64) -> Result<i64> {
        self.record_command(&format!("INCR {} {}", key, amount));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut data = self.data.write().await;
        let current_value = if let Some(value) = data.get(key) {
            if value.is_expired() {
                data.remove(key);
                0
            } else {
                let value_str = String::from_utf8(value.data.clone())
                    .map_err(|_| Error::Test("Invalid UTF-8 in stored value".to_string()))?;
                value_str
                    .parse::<i64>()
                    .map_err(|_| Error::Test("Stored value is not a valid integer".to_string()))?
            }
        } else {
            0
        };

        let new_value = current_value + amount;
        data.insert(
            key.to_string(),
            MockRedisValue::new(new_value.to_string().into_bytes()),
        );

        Ok(new_value)
    }

    // Hash Commands

    /// HSET command implementation
    pub async fn hset(&self, key: &str, field: &str, value: Vec<u8>) -> Result<()> {
        self.record_command(&format!(
            "HSET {} {} {:?}",
            key,
            field,
            String::from_utf8_lossy(&value)
        ));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut hashes = self.hashes.write().await;
        let hash = hashes.entry(key.to_string()).or_insert_with(HashMap::new);
        hash.insert(field.to_string(), value);

        Ok(())
    }

    /// HGET command implementation
    pub async fn hget(&self, key: &str, field: &str) -> Result<Option<Vec<u8>>> {
        self.record_command(&format!("HGET {} {}", key, field));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let hashes = self.hashes.read().await;
        if let Some(hash) = hashes.get(key) {
            Ok(hash.get(field).cloned())
        } else {
            Ok(None)
        }
    }

    /// HDEL command implementation
    pub async fn hdel(&self, key: &str, field: &str) -> Result<u32> {
        self.record_command(&format!("HDEL {} {}", key, field));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut hashes = self.hashes.write().await;
        if let Some(hash) = hashes.get_mut(key) {
            if hash.remove(field).is_some() {
                if hash.is_empty() {
                    hashes.remove(key);
                }
                Ok(1)
            } else {
                Ok(0)
            }
        } else {
            Ok(0)
        }
    }

    /// HLEN command implementation
    pub async fn hlen(&self, key: &str) -> Result<u64> {
        self.record_command(&format!("HLEN {}", key));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let hashes = self.hashes.read().await;
        if let Some(hash) = hashes.get(key) {
            Ok(hash.len() as u64)
        } else {
            Ok(0)
        }
    }

    // Sorted Set Commands

    /// ZADD command implementation
    pub async fn zadd(&self, key: &str, value: Vec<u8>, score: f64) -> Result<u32> {
        self.record_command(&format!(
            "ZADD {} {} {:?}",
            key,
            score,
            String::from_utf8_lossy(&value)
        ));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut zsets = self.zsets.write().await;
        let zset = zsets
            .entry(key.to_string())
            .or_insert_with(std::collections::BTreeMap::new);
        let old_value = zset.insert(OrderedFloat(score), value);

        Ok(if old_value.is_some() { 0 } else { 1 })
    }

    /// ZCARD command implementation
    pub async fn zcard(&self, key: &str) -> Result<u64> {
        self.record_command(&format!("ZCARD {}", key));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let zsets = self.zsets.read().await;
        if let Some(zset) = zsets.get(key) {
            Ok(zset.len() as u64)
        } else {
            Ok(0)
        }
    }

    /// ZPOPMAX command implementation
    pub async fn zpopmax(&self, key: &str, count: usize) -> Result<Vec<(Vec<u8>, f64)>> {
        self.record_command(&format!("ZPOPMAX {} {}", key, count));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let mut zsets = self.zsets.write().await;
        let mut results = Vec::new();

        if let Some(zset) = zsets.get_mut(key) {
            for _ in 0..count {
                if let Some((score, value)) = zset.pop_last() {
                    results.push((value, score.0));
                } else {
                    break;
                }
            }

            if zset.is_empty() {
                zsets.remove(key);
            }
        }

        Ok(results)
    }

    // Pub/Sub Commands

    /// PUBLISH command implementation
    pub async fn publish(&self, channel: &str, message: Vec<u8>) -> Result<u64> {
        self.record_command(&format!(
            "PUBLISH {} {:?}",
            channel,
            String::from_utf8_lossy(&message)
        ));
        self.simulate_delay().await;

        if self.should_fail() {
            return Err(Error::test("Simulated Redis failure".to_string()));
        }

        let pubsub = self.pubsub.read().await;
        if let Some(sender) = pubsub.get(channel) {
            match sender.send(message) {
                Ok(subscriber_count) => Ok(subscriber_count as u64),
                Err(_) => Ok(0), // No receivers
            }
        } else {
            Ok(0)
        }
    }

    /// Subscribe to a channel
    pub async fn subscribe(&self, channel: &str) -> broadcast::Receiver<Vec<u8>> {
        self.record_command(&format!("SUBSCRIBE {}", channel));

        let mut pubsub = self.pubsub.write().await;
        let sender = pubsub
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(1000).0);

        sender.subscribe()
    }

    /// Clear all data
    pub fn clear(&self) {
        // Use blocking operations since this is called from synchronous context
        tokio::runtime::Handle::current().block_on(async {
            let mut data = self.data.write().await;
            let mut pubsub = self.pubsub.write().await;
            let mut lists = self.lists.write().await;
            let mut sets = self.sets.write().await;
            let mut hashes = self.hashes.write().await;
            let mut zsets = self.zsets.write().await;

            data.clear();
            pubsub.clear();
            lists.clear();
            sets.clear();
            hashes.clear();
            zsets.clear();
        });

        self.clear_command_history();
    }

    /// Get statistics about the store
    pub async fn stats(&self) -> MockRedisStats {
        let data = self.data.read().await;
        let pubsub = self.pubsub.read().await;
        let lists = self.lists.read().await;
        let sets = self.sets.read().await;
        let hashes = self.hashes.read().await;
        let zsets = self.zsets.read().await;

        let total_keys = data.len();
        let expired_keys = data.values().filter(|v| v.is_expired()).count();

        MockRedisStats {
            total_keys,
            active_keys: total_keys - expired_keys,
            expired_keys,
            pubsub_channels: pubsub.len(),
            list_keys: lists.len(),
            set_keys: sets.len(),
            hash_keys: hashes.len(),
            zset_keys: zsets.len(),
        }
    }
}

impl Default for MockRedisStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for mock Redis store
#[derive(Debug, Clone)]
pub struct MockRedisStats {
    pub total_keys: usize,
    pub active_keys: usize,
    pub expired_keys: usize,
    pub pubsub_channels: usize,
    pub list_keys: usize,
    pub set_keys: usize,
    pub hash_keys: usize,
    pub zset_keys: usize,
}

/// Builder for creating configured mock Redis clients
pub struct MockRedisClientBuilder {
    network_delay_ms: Option<u64>,
    failure_rate: Option<f64>,
    max_memory: Option<usize>,
}

impl MockRedisClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            network_delay_ms: None,
            failure_rate: None,
            max_memory: None,
        }
    }

    /// Enable network delay simulation
    pub fn with_network_delay(mut self, delay_ms: u64) -> Self {
        self.network_delay_ms = Some(delay_ms);
        self
    }

    /// Enable failure simulation
    pub fn with_failure_rate(mut self, failure_rate: f64) -> Self {
        self.failure_rate = Some(failure_rate);
        self
    }

    /// Set maximum memory limit
    pub fn with_max_memory(mut self, max_memory: usize) -> Self {
        self.max_memory = Some(max_memory);
        self
    }

    /// Build the mock client
    pub fn build(self) -> MockRedisClient {
        let client = MockRedisClient::new();

        if let Some(delay_ms) = self.network_delay_ms {
            client.store.enable_network_delay(delay_ms);
        }

        if let Some(failure_rate) = self.failure_rate {
            client.store.enable_failure_simulation(failure_rate);
        }

        client
    }
}

impl Default for MockRedisClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_redis_basic_operations() {
        let store = MockRedisStore::new();

        // Test SET/GET
        store.set("key1", b"value1".to_vec()).await.unwrap();
        let value = store.get("key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // Test EXISTS
        assert!(store.exists("key1").await.unwrap());
        assert!(!store.exists("key2").await.unwrap());

        // Test DEL
        assert_eq!(store.del("key1").await.unwrap(), 1);
        assert!(!store.exists("key1").await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_redis_expiration() {
        let store = MockRedisStore::new();

        // Set with TTL
        store.setex("expiring", b"value".to_vec(), 1).await.unwrap();
        assert!(store.exists("expiring").await.unwrap());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(!store.exists("expiring").await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_redis_hash_operations() {
        let store = MockRedisStore::new();

        // Test HSET/HGET
        store
            .hset("hash1", "field1", b"value1".to_vec())
            .await
            .unwrap();
        let value = store.hget("hash1", "field1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // Test HLEN
        store
            .hset("hash1", "field2", b"value2".to_vec())
            .await
            .unwrap();
        assert_eq!(store.hlen("hash1").await.unwrap(), 2);

        // Test HDEL
        assert_eq!(store.hdel("hash1", "field1").await.unwrap(), 1);
        assert_eq!(store.hlen("hash1").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_mock_redis_sorted_set_operations() {
        let store = MockRedisStore::new();

        // Test ZADD
        store.zadd("zset1", b"member1".to_vec(), 1.0).await.unwrap();
        store.zadd("zset1", b"member2".to_vec(), 2.0).await.unwrap();

        // Test ZCARD
        assert_eq!(store.zcard("zset1").await.unwrap(), 2);

        // Test ZPOPMAX
        let popped = store.zpopmax("zset1", 1).await.unwrap();
        assert_eq!(popped.len(), 1);
        assert_eq!(popped[0].0, b"member2");
        assert_eq!(popped[0].1, 2.0);
    }

    #[tokio::test]
    async fn test_mock_redis_pubsub() {
        let store = MockRedisStore::new();

        // Subscribe to channel
        let mut receiver = store.subscribe("test_channel").await;

        // Publish message
        let subscriber_count = store
            .publish("test_channel", b"test_message".to_vec())
            .await
            .unwrap();
        assert_eq!(subscriber_count, 1);

        // Receive message
        let message = receiver.recv().await.unwrap();
        assert_eq!(message, b"test_message");
    }

    #[test]
    fn test_mock_redis_builder() {
        let client = MockRedisClientBuilder::new()
            .with_network_delay(10)
            .with_failure_rate(0.1)
            .build();

        // Verify client was created (stats() returns a Future, so we need to await it)
        let stats = client.store.stats().await;
        assert!(stats.commands_executed >= 0);
    }

    #[tokio::test]
    async fn test_mock_redis_command_history() {
        let store = MockRedisStore::new();

        store.set("key1", b"value1".to_vec()).await.unwrap();
        store.get("key1").await.unwrap();
        store.del("key1").await.unwrap();

        let history = store.get_command_history();
        assert_eq!(history.len(), 3);
        assert!(history[0].starts_with("SET"));
        assert!(history[1].starts_with("GET"));
        assert!(history[2].starts_with("DEL"));
    }

    #[tokio::test]
    async fn test_mock_redis_stats() {
        let store = MockRedisStore::new();

        // Add some data
        store.set("key1", b"value1".to_vec()).await.unwrap();
        store
            .hset("hash1", "field1", b"value1".to_vec())
            .await
            .unwrap();
        store.zadd("zset1", b"member1".to_vec(), 1.0).await.unwrap();

        let stats = store.stats().await;
        assert_eq!(stats.total_keys, 1); // Only string keys count here
        assert_eq!(stats.hash_keys, 1);
        assert_eq!(stats.zset_keys, 1);
    }
}
