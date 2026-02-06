# Comprehensive Testing Strategy for agent-sync Rust Library

## Overview

This document outlines a complete testing strategy for the `agent-sync` Rust library that provides async Redis operations for LLM agents. The strategy covers unit testing, integration testing, mocking, performance benchmarking, and cross-platform compatibility.

## Table of Contents

1. [Testing Architecture](#testing-architecture)
2. [Unit Testing Strategy](#unit-testing-strategy)
3. [Integration Testing](#integration-testing)
4. [Mock Strategies](#mock-strategies)
5. [Performance Benchmarking](#performance-benchmarking)
6. [Cross-Platform Testing](#cross-platform-testing)
7. [CI/CD Pipeline](#cicd-pipeline)
8. [Test Utilities](#test-utilities)

## Testing Architecture

### Test Organization Structure

```
agent-sync/
├── src/
│   ├── lib.rs
│   ├── redis_client.rs
│   ├── memory_store.rs
│   └── agent_sync.rs
├── tests/
│   ├── unit/
│   │   ├── mod.rs
│   │   ├── redis_client_tests.rs
│   │   ├── memory_store_tests.rs
│   │   └── agent_sync_tests.rs
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── redis_integration.rs
│   │   ├── memory_persistence.rs
│   │   └── cross_platform.rs
│   ├── benchmarks/
│   │   ├── mod.rs
│   │   ├── redis_ops_bench.rs
│   │   └── memory_bench.rs
│   └── utils/
│       ├── mod.rs
│       ├── test_redis.rs
│       ├── mock_redis.rs
│       └── fixtures.rs
├── benches/
│   ├── redis_operations.rs
│   └── memory_operations.rs
└── Cargo.toml
```

### Dependencies

```toml
[dev-dependencies]
# Testing framework
tokio-test = "0.4"
rstest = "0.18"
wiremock = "0.5"
testcontainers = "0.15"
testcontainers-modules = { version = "0.3", features = ["redis"] }

# Async runtime for tests
tokio = { version = "1.0", features = ["test-util", "macros", "rt-multi-thread"] }

# Mocking and fixtures
mockall = "0.11"
fake = "2.9"
uuid = { version = "1.0", features = ["v4"] }

# Benchmarking
criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }
divan = "0.1"

# Property-based testing
proptest = "1.4"

# Cross-platform testing utilities
serial_test = "3.0"
```

## Unit Testing Strategy

### 1. Redis Client Unit Tests

```rust
// tests/unit/redis_client_tests.rs
use agent_sync::{RedisClient, RedisConfig, RedisError};
use mockall::{mock, predicate::*};
use rstest::*;
use tokio_test;

// Mock Redis connection for unit tests
mock! {
    RedisConnection {}

    #[async_trait]
    impl redis::aio::ConnectionLike for RedisConnection {
        async fn req_packed_command(&mut self, cmd: &redis::Cmd) -> redis::RedisResult<redis::Value>;
        async fn get_db(&self) -> i64;
    }
}

#[fixture]
fn redis_config() -> RedisConfig {
    RedisConfig {
        url: "redis://localhost:6379".to_string(),
        pool_size: 10,
        timeout_ms: 5000,
        retry_attempts: 3,
    }
}

#[fixture]
async fn mock_redis_client() -> RedisClient<MockRedisConnection> {
    let mut mock_conn = MockRedisConnection::new();
    mock_conn
        .expect_req_packed_command()
        .returning(|_| Ok(redis::Value::Okay));

    RedisClient::new_with_connection(mock_conn)
}

#[rstest]
#[tokio::test]
async fn test_set_memory_success(#[from(mock_redis_client)] client: RedisClient<MockRedisConnection>) {
    // Arrange
    let key = "agent:memory:test";
    let value = serde_json::json!({"data": "test_memory"});

    // Act
    let result = client.set_memory(key, &value, None).await;

    // Assert
    assert!(result.is_ok());
}

#[rstest]
#[tokio::test]
async fn test_get_memory_not_found(#[from(mock_redis_client)] mut client: RedisClient<MockRedisConnection>) {
    // Arrange
    client.connection
        .expect_req_packed_command()
        .returning(|_| Ok(redis::Value::Nil));

    let key = "agent:memory:nonexistent";

    // Act
    let result = client.get_memory::<serde_json::Value>(key).await;

    // Assert
    assert!(matches!(result, Err(RedisError::KeyNotFound(_))));
}

#[rstest]
#[case(1, 100)]
#[case(10, 1000)]
#[case(100, 10000)]
#[tokio::test]
async fn test_memory_operations_performance(
    #[case] batch_size: usize,
    #[case] expected_max_ms: u64,
    #[from(mock_redis_client)] client: RedisClient<MockRedisConnection>
) {
    use std::time::Instant;

    let start = Instant::now();

    // Generate test data
    let operations: Vec<_> = (0..batch_size)
        .map(|i| format!("test:key:{}", i))
        .collect();

    // Execute batch operations
    for key in operations {
        let _ = client.set_memory(&key, &"test_value", None).await;
    }

    let duration = start.elapsed();
    assert!(duration.as_millis() < expected_max_ms as u128);
}

// Property-based testing
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_key_validation_properties(key in "[a-zA-Z0-9:_-]{1,100}") {
        let result = RedisClient::validate_key(&key);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_key_properties(key in "[^a-zA-Z0-9:_-]+") {
        let result = RedisClient::validate_key(&key);
        prop_assert!(result.is_err());
    }
}
```

### 2. Memory Store Unit Tests

```rust
// tests/unit/memory_store_tests.rs
use agent_sync::{MemoryStore, MemoryEntry, MemoryMetadata};
use fake::{Fake, Faker};
use rstest::*;
use uuid::Uuid;

#[fixture]
fn sample_memory_entry() -> MemoryEntry {
    MemoryEntry {
        id: Uuid::new_v4().to_string(),
        content: "Sample memory content".to_string(),
        metadata: MemoryMetadata {
            timestamp: chrono::Utc::now(),
            agent_id: "test-agent".to_string(),
            session_id: Some("test-session".to_string()),
            tags: vec!["test".to_string(), "memory".to_string()],
            embedding: None,
        },
    }
}

#[fixture]
async fn memory_store() -> MemoryStore {
    let redis_client = create_mock_redis_client().await;
    MemoryStore::new(redis_client)
}

#[rstest]
#[tokio::test]
async fn test_store_memory_entry(
    #[from(memory_store)] store: MemoryStore,
    #[from(sample_memory_entry)] entry: MemoryEntry,
) {
    // Act
    let result = store.store_memory(&entry).await;

    // Assert
    assert!(result.is_ok());
    let stored_id = result.unwrap();
    assert_eq!(stored_id, entry.id);
}

#[rstest]
#[tokio::test]
async fn test_retrieve_memory_by_id(
    #[from(memory_store)] store: MemoryStore,
    #[from(sample_memory_entry)] entry: MemoryEntry,
) {
    // Arrange
    store.store_memory(&entry).await.unwrap();

    // Act
    let result = store.get_memory(&entry.id).await;

    // Assert
    assert!(result.is_ok());
    let retrieved = result.unwrap();
    assert_eq!(retrieved.id, entry.id);
    assert_eq!(retrieved.content, entry.content);
}

#[rstest]
#[tokio::test]
async fn test_search_memories_by_tag(#[from(memory_store)] store: MemoryStore) {
    // Arrange
    let mut entries = vec![];
    for i in 0..5 {
        let entry = MemoryEntry {
            id: Uuid::new_v4().to_string(),
            content: format!("Memory entry {}", i),
            metadata: MemoryMetadata {
                timestamp: chrono::Utc::now(),
                agent_id: "test-agent".to_string(),
                session_id: None,
                tags: if i % 2 == 0 {
                    vec!["even".to_string()]
                } else {
                    vec!["odd".to_string()]
                },
                embedding: None,
            },
        };
        store.store_memory(&entry).await.unwrap();
        entries.push(entry);
    }

    // Act
    let even_results = store.search_by_tags(&["even"]).await.unwrap();
    let odd_results = store.search_by_tags(&["odd"]).await.unwrap();

    // Assert
    assert_eq!(even_results.len(), 3); // 0, 2, 4
    assert_eq!(odd_results.len(), 2);  // 1, 3
}

// Fuzzing tests for robustness
#[rstest]
#[tokio::test]
async fn test_memory_store_with_random_data(#[from(memory_store)] store: MemoryStore) {
    use fake::faker::lorem::en::*;

    for _ in 0..100 {
        let entry = MemoryEntry {
            id: Uuid::new_v4().to_string(),
            content: Sentence(1..10).fake::<String>(),
            metadata: MemoryMetadata {
                timestamp: chrono::Utc::now(),
                agent_id: format!("agent-{}", (1..100).fake::<u32>()),
                session_id: Some(Uuid::new_v4().to_string()),
                tags: Words(1..5).fake::<Vec<String>>(),
                embedding: None,
            },
        };

        let result = store.store_memory(&entry).await;
        assert!(result.is_ok(), "Failed to store entry: {:?}", entry);
    }
}
```

## Integration Testing

### 1. Redis Integration Tests with Test Containers

```rust
// tests/integration/redis_integration.rs
use agent_sync::{RedisClient, RedisConfig, MemoryStore};
use testcontainers::{clients, images, Container};
use testcontainers_modules::redis::Redis;
use tokio_test;
use serial_test::serial;

struct TestRedisContainer {
    container: Container<'static, Redis>,
    redis_client: RedisClient,
}

impl TestRedisContainer {
    async fn new() -> Self {
        let docker = clients::Cli::default();
        let redis_image = images::redis::Redis::default();
        let container = docker.run(redis_image);

        let port = container.get_host_port_ipv4(6379);
        let redis_url = format!("redis://127.0.0.1:{}", port);

        let config = RedisConfig {
            url: redis_url,
            pool_size: 5,
            timeout_ms: 3000,
            retry_attempts: 2,
        };

        let redis_client = RedisClient::new(config).await.unwrap();

        Self {
            container,
            redis_client,
        }
    }
}

#[tokio::test]
#[serial]
async fn test_redis_connection_integration() {
    let test_container = TestRedisContainer::new().await;

    // Test basic connectivity
    let ping_result = test_container.redis_client.ping().await;
    assert!(ping_result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_memory_persistence_integration() {
    let test_container = TestRedisContainer::new().await;
    let memory_store = MemoryStore::new(test_container.redis_client);

    // Create test memory entries
    let entries = create_test_memory_entries(10);

    // Store all entries
    for entry in &entries {
        let result = memory_store.store_memory(entry).await;
        assert!(result.is_ok(), "Failed to store memory: {:?}", entry);
    }

    // Verify all entries can be retrieved
    for entry in &entries {
        let retrieved = memory_store.get_memory(&entry.id).await;
        assert!(retrieved.is_ok(), "Failed to retrieve memory: {}", entry.id);

        let retrieved_entry = retrieved.unwrap();
        assert_eq!(retrieved_entry.content, entry.content);
        assert_eq!(retrieved_entry.metadata.agent_id, entry.metadata.agent_id);
    }
}

#[tokio::test]
#[serial]
async fn test_concurrent_redis_operations() {
    let test_container = TestRedisContainer::new().await;
    let memory_store = MemoryStore::new(test_container.redis_client);

    use tokio::task::JoinSet;

    let mut join_set = JoinSet::new();

    // Spawn concurrent operations
    for i in 0..50 {
        let store_clone = memory_store.clone();
        join_set.spawn(async move {
            let entry = create_test_memory_entry(&format!("concurrent-test-{}", i));
            store_clone.store_memory(&entry).await
        });
    }

    // Wait for all operations to complete
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.unwrap());
    }

    // Verify all operations succeeded
    assert_eq!(results.len(), 50);
    assert!(results.iter().all(|r| r.is_ok()));
}

#[tokio::test]
#[serial]
async fn test_redis_failure_recovery() {
    let mut test_container = TestRedisContainer::new().await;
    let memory_store = MemoryStore::new(test_container.redis_client.clone());

    // Store initial data
    let entry = create_test_memory_entry("recovery-test");
    memory_store.store_memory(&entry).await.unwrap();

    // Simulate Redis failure by stopping container
    test_container.container.stop();

    // Operations should fail gracefully
    let failed_result = memory_store.get_memory(&entry.id).await;
    assert!(failed_result.is_err());

    // Restart Redis container
    test_container = TestRedisContainer::new().await;
    let new_memory_store = MemoryStore::new(test_container.redis_client);

    // Data should be gone (container restart), but operations should work
    let new_entry = create_test_memory_entry("recovery-test-new");
    let result = new_memory_store.store_memory(&new_entry).await;
    assert!(result.is_ok());
}
```

### 2. Cross-Platform Integration Tests

```rust
// tests/integration/cross_platform.rs
use agent_sync::{RedisClient, RedisConfig, MemoryStore};
use std::process::Command;
use tokio_test;
use serial_test::serial;

// Test helper to detect platform
fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

fn is_wsl() -> bool {
    if cfg!(target_os = "linux") {
        std::path::Path::new("/proc/version")
            .exists()
            .then(|| {
                std::fs::read_to_string("/proc/version")
                    .map(|content| content.to_lowercase().contains("microsoft"))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    } else {
        false
    }
}

async fn get_platform_redis_config() -> RedisConfig {
    if is_windows() && !is_wsl() {
        // Windows native Redis
        RedisConfig {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_size: 10,
            timeout_ms: 5000,
            retry_attempts: 3,
        }
    } else if is_wsl() {
        // WSL Redis - might need different networking
        RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            timeout_ms: 8000, // Higher timeout for WSL
            retry_attempts: 5,
        }
    } else {
        // Linux native
        RedisConfig {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_size: 10,
            timeout_ms: 3000,
            retry_attempts: 3,
        }
    }
}

#[tokio::test]
#[serial]
async fn test_platform_specific_redis_connection() {
    let config = get_platform_redis_config().await;

    // This test requires Redis to be running on the platform
    let client_result = RedisClient::new(config).await;

    if client_result.is_err() {
        println!(
            "Skipping platform test - Redis not available on platform: {}",
            if is_windows() { "Windows" }
            else if is_wsl() { "WSL" }
            else { "Linux" }
        );
        return;
    }

    let client = client_result.unwrap();
    let ping_result = client.ping().await;
    assert!(ping_result.is_ok());
}

#[tokio::test]
#[serial]
#[cfg(target_os = "windows")]
async fn test_windows_redis_service_integration() {
    // Test connecting to Windows Redis service
    let config = RedisConfig {
        url: "redis://127.0.0.1:6379".to_string(),
        pool_size: 5,
        timeout_ms: 5000,
        retry_attempts: 3,
    };

    let client = match RedisClient::new(config).await {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping Windows Redis service test - service not running");
            return;
        }
    };

    // Test Windows-specific configurations
    let memory_store = MemoryStore::new(client);
    let entry = create_test_memory_entry("windows-test");

    let result = memory_store.store_memory(&entry).await;
    assert!(result.is_ok());

    let retrieved = memory_store.get_memory(&entry.id).await;
    assert!(retrieved.is_ok());
}

#[tokio::test]
#[serial]
async fn test_wsl_redis_bridge() {
    if !is_wsl() {
        println!("Skipping WSL test - not running in WSL environment");
        return;
    }

    // Test WSL to Windows Redis bridge
    let configs = vec![
        // Direct localhost
        "redis://localhost:6379",
        // WSL bridge IP
        "redis://172.17.0.1:6379",
        // Windows host IP (if available)
        "redis://host.docker.internal:6379",
    ];

    let mut successful_connections = 0;

    for redis_url in configs {
        let config = RedisConfig {
            url: redis_url.to_string(),
            pool_size: 3,
            timeout_ms: 3000,
            retry_attempts: 1,
        };

        if let Ok(client) = RedisClient::new(config).await {
            if client.ping().await.is_ok() {
                successful_connections += 1;
                println!("Successfully connected to Redis via: {}", redis_url);
            }
        }
    }

    // At least one connection method should work in WSL
    assert!(successful_connections > 0, "No Redis connections available in WSL");
}

// Helper function for platform-specific performance testing
#[tokio::test]
#[serial]
async fn test_cross_platform_performance() {
    let config = get_platform_redis_config().await;

    let client = match RedisClient::new(config).await {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping performance test - Redis not available");
            return;
        }
    };

    let memory_store = MemoryStore::new(client);

    use std::time::Instant;
    let start = Instant::now();

    // Perform 100 operations
    for i in 0..100 {
        let entry = create_test_memory_entry(&format!("perf-test-{}", i));
        memory_store.store_memory(&entry).await.unwrap();
    }

    let duration = start.elapsed();

    // Platform-specific performance expectations
    let expected_max_ms = if is_windows() && !is_wsl() {
        5000 // Windows native
    } else if is_wsl() {
        8000 // WSL has higher latency
    } else {
        3000 // Linux native
    };

    assert!(
        duration.as_millis() < expected_max_ms,
        "Performance test failed on platform: {}ms > {}ms expected",
        duration.as_millis(),
        expected_max_ms
    );

    println!(
        "Platform performance test passed: {}ms (limit: {}ms) on {}",
        duration.as_millis(),
        expected_max_ms,
        if is_windows() && !is_wsl() { "Windows" }
        else if is_wsl() { "WSL" }
        else { "Linux" }
    );
}
```

## Mock Strategies

### 1. Redis Connection Mocks

```rust
// tests/utils/mock_redis.rs
use async_trait::async_trait;
use mockall::mock;
use redis::{RedisResult, Value as RedisValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

// In-memory Redis mock for fast unit tests
#[derive(Debug, Clone)]
pub struct MockRedisStore {
    data: Arc<RwLock<HashMap<String, RedisValue>>>,
    call_count: Arc<Mutex<HashMap<String, usize>>>,
    latency_ms: u64,
}

impl MockRedisStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            call_count: Arc::new(Mutex::new(HashMap::new())),
            latency_ms: 0,
        }
    }

    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    pub fn get_call_count(&self, command: &str) -> usize {
        self.call_count
            .lock()
            .unwrap()
            .get(command)
            .copied()
            .unwrap_or(0)
    }

    pub async fn preset_data(&self, key: &str, value: RedisValue) {
        self.data.write().await.insert(key.to_string(), value);
    }

    async fn increment_call_count(&self, command: &str) {
        let mut counts = self.call_count.lock().unwrap();
        *counts.entry(command.to_string()).or_insert(0) += 1;
    }

    async fn simulate_latency(&self) {
        if self.latency_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.latency_ms)).await;
        }
    }
}

#[async_trait]
impl redis::aio::ConnectionLike for MockRedisStore {
    async fn req_packed_command(&mut self, cmd: &redis::Cmd) -> RedisResult<RedisValue> {
        use redis::cmd;

        self.simulate_latency().await;

        let cmd_str = format!("{:?}", cmd);
        let command_name = if cmd_str.contains("SET") {
            "SET"
        } else if cmd_str.contains("GET") {
            "GET"
        } else if cmd_str.contains("DEL") {
            "DEL"
        } else if cmd_str.contains("EXISTS") {
            "EXISTS"
        } else if cmd_str.contains("PING") {
            "PING"
        } else {
            "OTHER"
        };

        self.increment_call_count(command_name).await;

        // Simple command parsing and execution
        if command_name == "PING" {
            Ok(RedisValue::Status("PONG".to_string()))
        } else if command_name == "SET" {
            // Extract key and value (simplified)
            let parts: Vec<&str> = cmd_str.split_whitespace().collect();
            if parts.len() >= 3 {
                let key = parts[1].trim_matches('"');
                let value = RedisValue::Data(parts[2].as_bytes().to_vec());
                self.data.write().await.insert(key.to_string(), value);
                Ok(RedisValue::Okay)
            } else {
                Ok(RedisValue::Nil)
            }
        } else if command_name == "GET" {
            let parts: Vec<&str> = cmd_str.split_whitespace().collect();
            if parts.len() >= 2 {
                let key = parts[1].trim_matches('"');
                let data = self.data.read().await;
                Ok(data.get(key).cloned().unwrap_or(RedisValue::Nil))
            } else {
                Ok(RedisValue::Nil)
            }
        } else {
            Ok(RedisValue::Okay)
        }
    }

    async fn get_db(&self) -> i64 {
        0
    }
}

// Behavior-driven mock for specific scenarios
pub struct ScenarioMockRedis {
    scenarios: Vec<Box<dyn Fn(&str) -> RedisResult<RedisValue> + Send + Sync>>,
}

impl ScenarioMockRedis {
    pub fn new() -> Self {
        Self {
            scenarios: Vec::new(),
        }
    }

    pub fn when_command_matches<F>(mut self, pattern: &str, response: F) -> Self
    where
        F: Fn(&str) -> RedisResult<RedisValue> + Send + Sync + 'static,
    {
        let pattern = pattern.to_string();
        self.scenarios.push(Box::new(move |cmd| {
            if cmd.contains(&pattern) {
                response(cmd)
            } else {
                Ok(RedisValue::Nil)
            }
        }));
        self
    }

    pub fn simulate_connection_failure(self) -> Self {
        self.when_command_matches("", |_| {
            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Connection refused"
            )))
        })
    }

    pub fn simulate_timeout(self) -> Self {
        self.when_command_matches("", |_| {
            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Timeout"
            )))
        })
    }
}

// Mock builder for complex test scenarios
pub struct MockRedisBuilder {
    store: MockRedisStore,
    failure_probability: f64,
    command_delays: HashMap<String, u64>,
}

impl MockRedisBuilder {
    pub fn new() -> Self {
        Self {
            store: MockRedisStore::new(),
            failure_probability: 0.0,
            command_delays: HashMap::new(),
        }
    }

    pub fn with_failure_probability(mut self, prob: f64) -> Self {
        self.failure_probability = prob;
        self
    }

    pub fn with_command_delay(mut self, command: &str, delay_ms: u64) -> Self {
        self.command_delays.insert(command.to_string(), delay_ms);
        self
    }

    pub async fn with_preset_memory(self, key: &str, content: &str) -> Self {
        self.store.preset_data(key, RedisValue::Data(content.as_bytes().to_vec())).await;
        self
    }

    pub fn build(self) -> MockRedisStore {
        self.store
    }
}
```

### 2. Agent Sync Mocks

```rust
// tests/utils/mock_agent_sync.rs
use agent_sync::{AgentSync, SyncResult, AgentState, AgentMessage};
use async_trait::async_trait;
use mockall::mock;
use std::collections::VecDeque;
use tokio::sync::Mutex;

mock! {
    AgentSyncService {}

    #[async_trait]
    impl AgentSync for AgentSyncService {
        async fn sync_agent_state(&self, agent_id: &str, state: &AgentState) -> SyncResult<()>;
        async fn get_agent_state(&self, agent_id: &str) -> SyncResult<AgentState>;
        async fn send_message(&self, from: &str, to: &str, message: &AgentMessage) -> SyncResult<()>;
        async fn receive_messages(&self, agent_id: &str) -> SyncResult<Vec<AgentMessage>>;
        async fn broadcast_message(&self, from: &str, message: &AgentMessage) -> SyncResult<()>;
    }
}

// Deterministic mock for testing message ordering
pub struct DeterministicAgentSync {
    message_queue: Mutex<VecDeque<AgentMessage>>,
    state_store: Mutex<std::collections::HashMap<String, AgentState>>,
    message_history: Mutex<Vec<(String, String, AgentMessage)>>, // from, to, message
}

impl DeterministicAgentSync {
    pub fn new() -> Self {
        Self {
            message_queue: Mutex::new(VecDeque::new()),
            state_store: Mutex::new(std::collections::HashMap::new()),
            message_history: Mutex::new(Vec::new()),
        }
    }

    pub async fn get_message_history(&self) -> Vec<(String, String, AgentMessage)> {
        self.message_history.lock().await.clone()
    }

    pub async fn clear_message_queue(&self) {
        self.message_queue.lock().await.clear();
    }
}

#[async_trait]
impl AgentSync for DeterministicAgentSync {
    async fn sync_agent_state(&self, agent_id: &str, state: &AgentState) -> SyncResult<()> {
        self.state_store.lock().await.insert(agent_id.to_string(), state.clone());
        Ok(())
    }

    async fn get_agent_state(&self, agent_id: &str) -> SyncResult<AgentState> {
        self.state_store
            .lock()
            .await
            .get(agent_id)
            .cloned()
            .ok_or_else(|| agent_sync::Error::AgentNotFound(agent_id.to_string()))
    }

    async fn send_message(&self, from: &str, to: &str, message: &AgentMessage) -> SyncResult<()> {
        self.message_queue.lock().await.push_back(message.clone());
        self.message_history.lock().await.push((from.to_string(), to.to_string(), message.clone()));
        Ok(())
    }

    async fn receive_messages(&self, _agent_id: &str) -> SyncResult<Vec<AgentMessage>> {
        let mut queue = self.message_queue.lock().await;
        let messages: Vec<_> = queue.drain(..).collect();
        Ok(messages)
    }

    async fn broadcast_message(&self, from: &str, message: &AgentMessage) -> SyncResult<()> {
        // Simulate broadcast to all known agents
        let agents: Vec<String> = self.state_store.lock().await.keys().cloned().collect();

        for agent_id in agents {
            if agent_id != from {
                self.send_message(from, &agent_id, message).await?;
            }
        }
        Ok(())
    }
}
```

## Performance Benchmarking

### 1. Redis Operations Benchmarks

```rust
// benches/redis_operations.rs
use agent_sync::{RedisClient, RedisConfig, MemoryStore, MemoryEntry};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;

fn create_test_entries(count: usize) -> Vec<MemoryEntry> {
    (0..count)
        .map(|i| create_test_memory_entry(&format!("bench-entry-{}", i)))
        .collect()
}

async fn setup_redis_client() -> RedisClient {
    let config = RedisConfig {
        url: "redis://localhost:6379".to_string(),
        pool_size: 20,
        timeout_ms: 5000,
        retry_attempts: 2,
    };
    RedisClient::new(config).await.unwrap()
}

fn bench_redis_set_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let redis_client = rt.block_on(setup_redis_client());
    let memory_store = MemoryStore::new(redis_client);

    let mut group = c.benchmark_group("redis_set_operations");

    for size in [1, 10, 100, 1000].iter() {
        let entries = create_test_entries(*size);

        group.bench_with_input(
            BenchmarkId::new("sequential_sets", size),
            &entries,
            |b, entries| {
                b.to_async(&rt).iter(|| async {
                    for entry in entries {
                        black_box(memory_store.store_memory(entry).await.unwrap());
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("parallel_sets", size),
            &entries,
            |b, entries| {
                b.to_async(&rt).iter(|| async {
                    use futures::future::try_join_all;
                    let futures = entries.iter().map(|entry| {
                        memory_store.store_memory(entry)
                    });
                    black_box(try_join_all(futures).await.unwrap());
                });
            },
        );
    }

    group.finish();
}

fn bench_redis_get_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let redis_client = rt.block_on(setup_redis_client());
    let memory_store = MemoryStore::new(redis_client);

    // Pre-populate with test data
    let entries = create_test_entries(1000);
    rt.block_on(async {
        for entry in &entries {
            memory_store.store_memory(entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("redis_get_operations");

    for batch_size in [1, 10, 100].iter() {
        let entry_ids: Vec<String> = entries.iter()
            .take(*batch_size)
            .map(|e| e.id.clone())
            .collect();

        group.bench_with_input(
            BenchmarkId::new("sequential_gets", batch_size),
            &entry_ids,
            |b, ids| {
                b.to_async(&rt).iter(|| async {
                    for id in ids {
                        black_box(memory_store.get_memory(id).await.unwrap());
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("parallel_gets", batch_size),
            &entry_ids,
            |b, ids| {
                b.to_async(&rt).iter(|| async {
                    use futures::future::try_join_all;
                    let futures = ids.iter().map(|id| {
                        memory_store.get_memory(id)
                    });
                    black_box(try_join_all(futures).await.unwrap());
                });
            },
        );
    }

    group.finish();
}

fn bench_memory_search_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let redis_client = rt.block_on(setup_redis_client());
    let memory_store = MemoryStore::new(redis_client);

    // Pre-populate with tagged test data
    rt.block_on(async {
        for i in 0..1000 {
            let mut entry = create_test_memory_entry(&format!("search-entry-{}", i));
            entry.metadata.tags = vec![
                format!("category-{}", i % 10),
                format!("type-{}", i % 5),
                "searchable".to_string(),
            ];
            memory_store.store_memory(&entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("memory_search_operations");

    let search_tags = vec![
        vec!["category-0"],
        vec!["type-1"],
        vec!["searchable"],
        vec!["category-0", "type-1"],
    ];

    for tags in search_tags {
        group.bench_with_input(
            BenchmarkId::new("tag_search", tags.len()),
            &tags,
            |b, tags| {
                b.to_async(&rt).iter(|| async {
                    black_box(memory_store.search_by_tags(tags).await.unwrap());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_redis_set_operations,
    bench_redis_get_operations,
    bench_memory_search_operations
);
criterion_main!(benches);
```

### 2. Memory Operations Benchmarks

```rust
// benches/memory_operations.rs
use agent_sync::{MemoryStore, MemoryEntry, MemoryMetadata};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use divan::Bencher;

#[divan::bench(args = [10, 100, 1000, 10000])]
fn bench_memory_serialization(bencher: Bencher, size: usize) {
    let entries = create_test_entries(size);

    bencher.bench(|| {
        for entry in &entries {
            black_box(serde_json::to_string(entry).unwrap());
        }
    });
}

#[divan::bench(args = [10, 100, 1000, 10000])]
fn bench_memory_deserialization(bencher: Bencher, size: usize) {
    let entries = create_test_entries(size);
    let serialized: Vec<String> = entries.iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect();

    bencher.bench(|| {
        for json_str in &serialized {
            black_box(serde_json::from_str::<MemoryEntry>(json_str).unwrap());
        }
    });
}

#[divan::bench(args = [100, 1000, 10000])]
async fn bench_memory_compression(bencher: Bencher, size: usize) {
    let entries = create_test_entries(size);
    let data = serde_json::to_string(&entries).unwrap();

    bencher.bench(|| async {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data.as_bytes()).unwrap();
        black_box(encoder.finish().unwrap());
    });
}

// Load testing scenarios
#[divan::bench(
    name = "concurrent_memory_operations",
    args = [1, 5, 10, 20, 50]
)]
async fn bench_concurrent_operations(bencher: Bencher, concurrency: usize) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let redis_client = rt.block_on(setup_redis_client());
    let memory_store = MemoryStore::new(redis_client);

    bencher
        .with_inputs(|| create_test_entries(100))
        .bench_values(|entries| async {
            use futures::stream::{FuturesUnordered, StreamExt};

            let mut futures = FuturesUnordered::new();

            for chunk in entries.chunks(entries.len() / concurrency) {
                let store_clone = memory_store.clone();
                let chunk = chunk.to_vec();

                futures.push(async move {
                    for entry in chunk {
                        store_clone.store_memory(&entry).await.unwrap();
                    }
                });
            }

            while let Some(_) = futures.next().await {}
        });
}

fn main() {
    divan::main();
}
```

## Cross-Platform Testing

### 1. Platform-Specific Test Configuration

```rust
// tests/utils/platform_config.rs
use agent_sync::RedisConfig;
use std::env;

pub struct PlatformTestConfig {
    pub redis_config: RedisConfig,
    pub expected_latency_ms: u64,
    pub max_concurrent_connections: usize,
    pub platform_name: &'static str,
}

impl PlatformTestConfig {
    pub fn for_current_platform() -> Self {
        if cfg!(target_os = "windows") {
            Self::windows_config()
        } else if Self::is_wsl() {
            Self::wsl_config()
        } else if cfg!(target_os = "linux") {
            Self::linux_config()
        } else if cfg!(target_os = "macos") {
            Self::macos_config()
        } else {
            Self::default_config()
        }
    }

    fn windows_config() -> Self {
        Self {
            redis_config: RedisConfig {
                url: env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
                pool_size: 15,
                timeout_ms: 5000,
                retry_attempts: 3,
            },
            expected_latency_ms: 10,
            max_concurrent_connections: 50,
            platform_name: "Windows",
        }
    }

    fn wsl_config() -> Self {
        Self {
            redis_config: RedisConfig {
                url: env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                pool_size: 10,
                timeout_ms: 8000, // Higher timeout for WSL networking
                retry_attempts: 5,
            },
            expected_latency_ms: 25, // Higher latency expected
            max_concurrent_connections: 30,
            platform_name: "WSL",
        }
    }

    fn linux_config() -> Self {
        Self {
            redis_config: RedisConfig {
                url: env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
                pool_size: 20,
                timeout_ms: 3000,
                retry_attempts: 2,
            },
            expected_latency_ms: 5,
            max_concurrent_connections: 100,
            platform_name: "Linux",
        }
    }

    fn macos_config() -> Self {
        Self {
            redis_config: RedisConfig {
                url: env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
                pool_size: 15,
                timeout_ms: 4000,
                retry_attempts: 3,
            },
            expected_latency_ms: 8,
            max_concurrent_connections: 75,
            platform_name: "macOS",
        }
    }

    fn default_config() -> Self {
        Self {
            redis_config: RedisConfig {
                url: "redis://127.0.0.1:6379".to_string(),
                pool_size: 10,
                timeout_ms: 5000,
                retry_attempts: 3,
            },
            expected_latency_ms: 20,
            max_concurrent_connections: 25,
            platform_name: "Unknown",
        }
    }

    fn is_wsl() -> bool {
        cfg!(target_os = "linux") &&
        std::path::Path::new("/proc/version").exists() &&
        std::fs::read_to_string("/proc/version")
            .map(|content| content.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
    }
}

// Platform-specific test runner
pub struct CrossPlatformTestRunner {
    config: PlatformTestConfig,
}

impl CrossPlatformTestRunner {
    pub fn new() -> Self {
        Self {
            config: PlatformTestConfig::for_current_platform(),
        }
    }

    pub async fn run_platform_tests(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Running tests on platform: {}", self.config.platform_name);

        // Test Redis connectivity
        self.test_redis_connectivity().await?;

        // Test platform-specific performance
        self.test_platform_performance().await?;

        // Test concurrent connections limit
        self.test_concurrent_connections().await?;

        println!("All platform tests passed for: {}", self.config.platform_name);
        Ok(())
    }

    async fn test_redis_connectivity(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = agent_sync::RedisClient::new(self.config.redis_config.clone()).await?;
        client.ping().await?;
        println!("✓ Redis connectivity test passed");
        Ok(())
    }

    async fn test_platform_performance(&self) -> Result<(), Box<dyn std::error::Error>> {
        use std::time::Instant;

        let client = agent_sync::RedisClient::new(self.config.redis_config.clone()).await?;
        let memory_store = agent_sync::MemoryStore::new(client);

        let start = Instant::now();
        let entry = create_test_memory_entry("performance-test");
        memory_store.store_memory(&entry).await?;
        let _ = memory_store.get_memory(&entry.id).await?;
        let duration = start.elapsed();

        assert!(
            duration.as_millis() <= self.config.expected_latency_ms as u128,
            "Platform performance test failed: {}ms > {}ms expected on {}",
            duration.as_millis(),
            self.config.expected_latency_ms,
            self.config.platform_name
        );

        println!("✓ Platform performance test passed: {}ms", duration.as_millis());
        Ok(())
    }

    async fn test_concurrent_connections(&self) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::task::JoinSet;

        let mut join_set = JoinSet::new();
        let connection_count = std::cmp::min(self.config.max_concurrent_connections, 20);

        for i in 0..connection_count {
            let config = self.config.redis_config.clone();
            join_set.spawn(async move {
                let client = agent_sync::RedisClient::new(config).await?;
                client.ping().await?;
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            });
        }

        let mut success_count = 0;
        while let Some(result) = join_set.join_next().await {
            if result??.is_ok() {
                success_count += 1;
            }
        }

        assert!(
            success_count >= connection_count * 8 / 10, // Allow 20% failure rate
            "Concurrent connection test failed: {}/{} connections succeeded on {}",
            success_count,
            connection_count,
            self.config.platform_name
        );

        println!("✓ Concurrent connection test passed: {}/{} connections", success_count, connection_count);
        Ok(())
    }
}
```

## CI/CD Pipeline

### 1. GitHub Actions Workflow

```yaml
# .github/workflows/test.yml
name: Cross-Platform Testing

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  test-matrix:
    name: Test on ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable, beta]
        redis-version: [6.2, 7.0]
        include:
          # WSL testing on Windows
          - os: windows-latest
            rust: stable
            redis-version: 7.0
            wsl-test: true

    services:
      redis:
        image: redis:${{ matrix.redis-version }}
        ports:
          - 6379:6379
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Install Rust toolchain
      uses: dtolnay/rust-toolchain@master
      with:
        toolchain: ${{ matrix.rust }}
        components: rustfmt, clippy

    - name: Cache Rust dependencies
      uses: Swatinem/rust-cache@v2
      with:
        key: ${{ matrix.os }}-${{ matrix.rust }}-${{ matrix.redis-version }}

    - name: Install system dependencies (Ubuntu)
      if: matrix.os == 'ubuntu-latest'
      run: |
        sudo apt-get update
        sudo apt-get install -y build-essential pkg-config

    - name: Install system dependencies (macOS)
      if: matrix.os == 'macos-latest'
      run: |
        brew install pkg-config

    - name: Setup Windows development environment
      if: matrix.os == 'windows-latest'
      run: |
        # Install Redis for Windows (if needed for native tests)
        choco install redis-64 -y
        # Configure Redis service
        redis-server --service-install
        redis-server --service-start

    - name: Setup WSL environment (Windows)
      if: matrix.wsl-test == true
      uses: Vampire/setup-wsl@v2
      with:
        distribution: Ubuntu-20.04
        additional-packages: redis-server build-essential

    - name: Verify Redis connectivity
      run: |
        cargo run --example redis_health_check
      env:
        REDIS_URL: redis://localhost:6379

    - name: Run unit tests
      run: |
        cargo test --lib --verbose
      env:
        RUST_LOG: debug

    - name: Run integration tests
      run: |
        cargo test --test '*' --verbose
      env:
        REDIS_URL: redis://localhost:6379
        RUST_LOG: info

    - name: Run WSL integration tests (Windows only)
      if: matrix.wsl-test == true
      run: |
        wsl bash -c "
          cd /mnt/c/projects/agent-sync &&
          curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y &&
          source ~/.cargo/env &&
          cargo test --test cross_platform --verbose
        "
      env:
        REDIS_URL: redis://localhost:6379

    - name: Run benchmarks
      run: |
        cargo bench --bench redis_operations -- --output-format verbose
        cargo bench --bench memory_operations -- --output-format verbose
      env:
        REDIS_URL: redis://localhost:6379

    - name: Run cross-platform compatibility tests
      run: |
        cargo test --test cross_platform --verbose
      env:
        REDIS_URL: redis://localhost:6379

    - name: Check code formatting
      run: cargo fmt --all -- --check

    - name: Run clippy
      run: cargo clippy --all-targets --all-features -- -D warnings

    - name: Generate test coverage
      if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'
      run: |
        cargo install cargo-tarpaulin
        cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out xml

    - name: Upload coverage to Codecov
      if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'
      uses: codecov/codecov-action@v3
      with:
        file: ./cobertura.xml
        fail_ci_if_error: true

  security-audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Run security audit
      run: |
        cargo install cargo-audit
        cargo audit

  performance-regression:
    name: Performance Regression Tests
    runs-on: ubuntu-latest
    services:
      redis:
        image: redis:7.0
        ports:
          - 6379:6379
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable

    - name: Install criterion
      run: cargo install cargo-criterion

    - name: Run baseline benchmarks
      run: |
        git checkout HEAD~1 2>/dev/null || echo "Skipping baseline for first commit"
        cargo criterion --message-format=json > baseline.json 2>/dev/null || echo "{}" > baseline.json

    - name: Checkout current code
      run: git checkout -

    - name: Run current benchmarks
      run: |
        cargo criterion --message-format=json > current.json

    - name: Compare benchmark results
      run: |
        # Simple regression detection (in real scenario, use more sophisticated tooling)
        python3 scripts/compare_benchmarks.py baseline.json current.json
```

### 2. Test Automation Scripts

```bash
#!/bin/bash
# scripts/run_cross_platform_tests.sh

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🚀 Starting cross-platform test suite..."

# Configuration
export RUST_BACKTRACE=1
export RUST_LOG=info
REDIS_URL="${REDIS_URL:-redis://localhost:6379}"

# Function to run tests with timeout
run_with_timeout() {
    local timeout_duration=$1
    local test_command=$2
    local description=$3

    echo "⏱️  Running $description (timeout: ${timeout_duration}s)..."

    if timeout "$timeout_duration" bash -c "$test_command"; then
        echo "✅ $description passed"
        return 0
    else
        echo "❌ $description failed or timed out"
        return 1
    fi
}

# Detect platform
detect_platform() {
    case "$(uname -s)" in
        Linux*)
            if grep -q Microsoft /proc/version 2>/dev/null; then
                echo "WSL"
            else
                echo "Linux"
            fi
            ;;
        Darwin*)  echo "macOS" ;;
        MINGW*|CYGWIN*|MSYS*) echo "Windows" ;;
        *) echo "Unknown" ;;
    esac
}

PLATFORM=$(detect_platform)
echo "🖥️  Detected platform: $PLATFORM"

# Platform-specific Redis setup
setup_redis() {
    case "$PLATFORM" in
        "Linux"|"WSL")
            if ! pgrep -x "redis-server" > /dev/null; then
                echo "🔧 Starting Redis server..."
                redis-server --daemonize yes --port 6379
                sleep 2
            fi
            ;;
        "macOS")
            if ! brew services list | grep redis | grep started > /dev/null; then
                echo "🔧 Starting Redis via Homebrew..."
                brew services start redis
                sleep 2
            fi
            ;;
        "Windows")
            echo "🔧 Assuming Redis is managed by Windows service..."
            ;;
    esac
}

# Test Redis connectivity
test_redis_connection() {
    echo "🔌 Testing Redis connectivity..."
    if redis-cli -u "$REDIS_URL" ping | grep -q PONG; then
        echo "✅ Redis connection successful"
        return 0
    else
        echo "❌ Redis connection failed"
        return 1
    fi
}

# Run test suite
run_test_suite() {
    local test_type=$1
    local timeout=${2:-300}  # Default 5 minutes

    case "$test_type" in
        "unit")
            run_with_timeout "$timeout" \
                "cargo test --lib --verbose" \
                "Unit tests"
            ;;
        "integration")
            run_with_timeout "$timeout" \
                "cargo test --test '*' --verbose" \
                "Integration tests"
            ;;
        "benchmarks")
            run_with_timeout "$timeout" \
                "cargo bench --verbose" \
                "Performance benchmarks"
            ;;
        "cross-platform")
            run_with_timeout "$timeout" \
                "cargo test --test cross_platform --verbose" \
                "Cross-platform compatibility tests"
            ;;
        "all")
            run_test_suite "unit" && \
            run_test_suite "integration" && \
            run_test_suite "cross-platform"
            ;;
    esac
}

# Cleanup function
cleanup() {
    echo "🧹 Cleaning up..."

    case "$PLATFORM" in
        "Linux"|"WSL")
            pkill redis-server 2>/dev/null || true
            ;;
        "macOS")
            brew services stop redis 2>/dev/null || true
            ;;
    esac
}

# Trap to ensure cleanup on exit
trap cleanup EXIT

# Main execution flow
main() {
    cd "$PROJECT_ROOT"

    # Setup
    setup_redis
    test_redis_connection || exit 1

    # Run tests based on command line argument
    TEST_TYPE="${1:-all}"

    echo "🧪 Running $TEST_TYPE tests on $PLATFORM..."

    if run_test_suite "$TEST_TYPE"; then
        echo "🎉 All tests passed successfully on $PLATFORM!"
        exit 0
    else
        echo "💥 Some tests failed on $PLATFORM"
        exit 1
    fi
}

# Run main function with all arguments
main "$@"
```

### 3. Test Result Analysis

```python
# scripts/analyze_test_results.py
#!/usr/bin/env python3

import json
import sys
import argparse
from pathlib import Path
from typing import Dict, List, Any
from dataclasses import dataclass
from datetime import datetime

@dataclass
class TestResult:
    name: str
    status: str  # "passed", "failed", "skipped"
    duration_ms: float
    platform: str
    error_message: str = None

@dataclass
class BenchmarkResult:
    name: str
    mean_time_ns: float
    std_dev_ns: float
    platform: str
    throughput_ops_per_sec: float = None

class TestAnalyzer:
    def __init__(self, results_dir: Path):
        self.results_dir = Path(results_dir)
        self.test_results: List[TestResult] = []
        self.benchmark_results: List[BenchmarkResult] = []

    def load_test_results(self, platform: str):
        """Load test results from JSON files."""
        test_file = self.results_dir / f"test_results_{platform}.json"
        bench_file = self.results_dir / f"benchmark_results_{platform}.json"

        # Load test results
        if test_file.exists():
            with open(test_file) as f:
                data = json.load(f)
                for test in data.get("tests", []):
                    self.test_results.append(TestResult(
                        name=test["name"],
                        status=test["status"],
                        duration_ms=test["duration_ms"],
                        platform=platform,
                        error_message=test.get("error")
                    ))

        # Load benchmark results
        if bench_file.exists():
            with open(bench_file) as f:
                data = json.load(f)
                for bench in data.get("benchmarks", []):
                    self.benchmark_results.append(BenchmarkResult(
                        name=bench["name"],
                        mean_time_ns=bench["mean_time_ns"],
                        std_dev_ns=bench["std_dev_ns"],
                        platform=platform,
                        throughput_ops_per_sec=bench.get("throughput_ops_per_sec")
                    ))

    def analyze_cross_platform_performance(self) -> Dict[str, Any]:
        """Analyze performance differences across platforms."""
        platform_perf = {}

        # Group benchmarks by name
        bench_by_name = {}
        for bench in self.benchmark_results:
            if bench.name not in bench_by_name:
                bench_by_name[bench.name] = {}
            bench_by_name[bench.name][bench.platform] = bench

        # Compare performance across platforms
        for bench_name, platforms in bench_by_name.items():
            if len(platforms) > 1:
                times = {platform: bench.mean_time_ns for platform, bench in platforms.items()}
                fastest = min(times.items(), key=lambda x: x[1])
                slowest = max(times.items(), key=lambda x: x[1])

                performance_ratio = slowest[1] / fastest[1] if fastest[1] > 0 else float('inf')

                platform_perf[bench_name] = {
                    "fastest": {"platform": fastest[0], "time_ns": fastest[1]},
                    "slowest": {"platform": slowest[0], "time_ns": slowest[1]},
                    "performance_ratio": performance_ratio,
                    "all_platforms": times
                }

        return platform_perf

    def check_performance_regressions(self, baseline_file: Path, threshold_percent: float = 10.0) -> List[Dict[str, Any]]:
        """Check for performance regressions against baseline."""
        regressions = []

        if not baseline_file.exists():
            print(f"Warning: Baseline file {baseline_file} not found")
            return regressions

        with open(baseline_file) as f:
            baseline_data = json.load(f)

        baseline_benchmarks = {b["name"]: b for b in baseline_data.get("benchmarks", [])}

        for bench in self.benchmark_results:
            baseline_bench = baseline_benchmarks.get(bench.name)
            if baseline_bench:
                baseline_time = baseline_bench["mean_time_ns"]
                current_time = bench.mean_time_ns

                if baseline_time > 0:
                    change_percent = ((current_time - baseline_time) / baseline_time) * 100

                    if change_percent > threshold_percent:
                        regressions.append({
                            "benchmark": bench.name,
                            "platform": bench.platform,
                            "baseline_time_ns": baseline_time,
                            "current_time_ns": current_time,
                            "regression_percent": change_percent
                        })

        return regressions

    def generate_test_report(self) -> str:
        """Generate a comprehensive test report."""
        report = []
        report.append("# Agent-Sync Test Results Report")
        report.append(f"Generated: {datetime.now().isoformat()}")
        report.append("")

        # Test Results Summary
        total_tests = len(self.test_results)
        passed_tests = len([t for t in self.test_results if t.status == "passed"])
        failed_tests = len([t for t in self.test_results if t.status == "failed"])
        skipped_tests = len([t for t in self.test_results if t.status == "skipped"])

        report.append("## Test Results Summary")
        report.append(f"- Total Tests: {total_tests}")
        report.append(f"- Passed: {passed_tests} ({passed_tests/total_tests*100:.1f}%)")
        report.append(f"- Failed: {failed_tests} ({failed_tests/total_tests*100:.1f}%)")
        report.append(f"- Skipped: {skipped_tests} ({skipped_tests/total_tests*100:.1f}%)")
        report.append("")

        # Platform Breakdown
        platforms = set(t.platform for t in self.test_results)
        report.append("## Platform Test Results")
        for platform in sorted(platforms):
            platform_tests = [t for t in self.test_results if t.platform == platform]
            platform_passed = len([t for t in platform_tests if t.status == "passed"])
            platform_total = len(platform_tests)
            success_rate = (platform_passed / platform_total * 100) if platform_total > 0 else 0

            report.append(f"### {platform}")
            report.append(f"- Success Rate: {success_rate:.1f}% ({platform_passed}/{platform_total})")

            # List failed tests
            failed_platform_tests = [t for t in platform_tests if t.status == "failed"]
            if failed_platform_tests:
                report.append("- Failed Tests:")
                for test in failed_platform_tests:
                    error_msg = f" ({test.error_message})" if test.error_message else ""
                    report.append(f"  - {test.name}{error_msg}")
            report.append("")

        # Performance Analysis
        if self.benchmark_results:
            perf_analysis = self.analyze_cross_platform_performance()
            report.append("## Cross-Platform Performance Analysis")

            for bench_name, data in perf_analysis.items():
                fastest_platform = data["fastest"]["platform"]
                slowest_platform = data["slowest"]["platform"]
                ratio = data["performance_ratio"]

                report.append(f"### {bench_name}")
                report.append(f"- Fastest: {fastest_platform} ({data['fastest']['time_ns']/1e6:.2f}ms)")
                report.append(f"- Slowest: {slowest_platform} ({data['slowest']['time_ns']/1e6:.2f}ms)")
                report.append(f"- Performance Ratio: {ratio:.2f}x")

                if ratio > 2.0:  # Flag significant differences
                    report.append(f"  ⚠️  Significant performance difference detected!")
                report.append("")

        return "\n".join(report)

    def export_results(self, output_file: Path):
        """Export results to JSON for further analysis."""
        data = {
            "generated_at": datetime.now().isoformat(),
            "test_results": [
                {
                    "name": t.name,
                    "status": t.status,
                    "duration_ms": t.duration_ms,
                    "platform": t.platform,
                    "error_message": t.error_message
                } for t in self.test_results
            ],
            "benchmark_results": [
                {
                    "name": b.name,
                    "mean_time_ns": b.mean_time_ns,
                    "std_dev_ns": b.std_dev_ns,
                    "platform": b.platform,
                    "throughput_ops_per_sec": b.throughput_ops_per_sec
                } for b in self.benchmark_results
            ]
        }

        with open(output_file, 'w') as f:
            json.dump(data, f, indent=2)

def main():
    parser = argparse.ArgumentParser(description="Analyze agent-sync test results")
    parser.add_argument("results_dir", help="Directory containing test results")
    parser.add_argument("--platforms", nargs="+", default=["linux", "windows", "macos", "wsl"],
                       help="Platforms to analyze")
    parser.add_argument("--baseline", help="Baseline benchmark file for regression detection")
    parser.add_argument("--output", help="Output file for analysis results")
    parser.add_argument("--regression-threshold", type=float, default=10.0,
                       help="Performance regression threshold percentage")

    args = parser.parse_args()

    analyzer = TestAnalyzer(args.results_dir)

    # Load results for all platforms
    for platform in args.platforms:
        try:
            analyzer.load_test_results(platform)
            print(f"Loaded test results for {platform}")
        except Exception as e:
            print(f"Warning: Could not load results for {platform}: {e}")

    # Generate report
    report = analyzer.generate_test_report()
    print(report)

    # Check for regressions if baseline provided
    if args.baseline:
        regressions = analyzer.check_performance_regressions(
            Path(args.baseline),
            args.regression_threshold
        )

        if regressions:
            print("\n❌ Performance Regressions Detected:")
            for reg in regressions:
                print(f"- {reg['benchmark']} on {reg['platform']}: "
                      f"{reg['regression_percent']:.1f}% slower")
            sys.exit(1)
        else:
            print("\n✅ No performance regressions detected")

    # Export results if requested
    if args.output:
        analyzer.export_results(Path(args.output))
        print(f"Results exported to {args.output}")

if __name__ == "__main__":
    main()
```

## Test Utilities

### 1. Test Data Factories

```rust
// tests/utils/fixtures.rs
use agent_sync::{MemoryEntry, MemoryMetadata, AgentMessage, AgentState};
use fake::{Fake, Faker};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct MemoryEntryBuilder {
    id: Option<String>,
    content: Option<String>,
    agent_id: Option<String>,
    session_id: Option<String>,
    tags: Vec<String>,
    timestamp: Option<DateTime<Utc>>,
    embedding: Option<Vec<f32>>,
}

impl Default for MemoryEntryBuilder {
    fn default() -> Self {
        Self {
            id: None,
            content: None,
            agent_id: None,
            session_id: None,
            tags: Vec::new(),
            timestamp: None,
            embedding: None,
        }
    }
}

impl MemoryEntryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn with_content<S: Into<String>>(mut self, content: S) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn with_agent_id<S: Into<String>>(mut self, agent_id: S) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_session_id<S: Into<String>>(mut self, session_id: S) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    pub fn build(self) -> MemoryEntry {
        use fake::faker::lorem::en::*;

        MemoryEntry {
            id: self.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            content: self.content.unwrap_or_else(|| Sentence(3..10).fake()),
            metadata: MemoryMetadata {
                timestamp: self.timestamp.unwrap_or_else(Utc::now),
                agent_id: self.agent_id.unwrap_or_else(|| format!("agent-{}", (1..1000).fake::<u32>())),
                session_id: self.session_id.map(|s| s),
                tags: if self.tags.is_empty() {
                    (1..4).map(|_| Word().fake()).collect()
                } else {
                    self.tags
                },
                embedding: self.embedding,
            },
        }
    }
}

// Factory for creating test memory entries
pub fn create_test_memory_entry(id_suffix: &str) -> MemoryEntry {
    MemoryEntryBuilder::new()
        .with_id(format!("test-memory-{}", id_suffix))
        .with_content(format!("Test memory content for {}", id_suffix))
        .with_agent_id("test-agent")
        .with_tag("test")
        .build()
}

pub fn create_test_memory_entries(count: usize) -> Vec<MemoryEntry> {
    (0..count)
        .map(|i| create_test_memory_entry(&i.to_string()))
        .collect()
}

pub fn create_memory_entries_with_tags(tags: &[&str], count_per_tag: usize) -> Vec<MemoryEntry> {
    let mut entries = Vec::new();

    for tag in tags {
        for i in 0..count_per_tag {
            let entry = MemoryEntryBuilder::new()
                .with_content(format!("Content for tag {} entry {}", tag, i))
                .with_tag(tag.to_string())
                .build();
            entries.push(entry);
        }
    }

    entries
}

// Agent state factory
pub fn create_test_agent_state(agent_id: &str) -> AgentState {
    AgentState {
        id: agent_id.to_string(),
        status: "active".to_string(),
        last_seen: Utc::now(),
        metadata: {
            let mut map = HashMap::new();
            map.insert("test_mode".to_string(), "true".to_string());
            map.insert("platform".to_string(), std::env::consts::OS.to_string());
            map
        },
        memory_size: (100..10000).fake(),
        message_queue_size: (0..50).fake(),
    }
}

// Message factory
pub fn create_test_agent_message(from: &str, to: &str, content: &str) -> AgentMessage {
    use fake::faker::chrono::en::*;

    AgentMessage {
        id: Uuid::new_v4().to_string(),
        from: from.to_string(),
        to: to.to_string(),
        content: content.to_string(),
        message_type: "test".to_string(),
        timestamp: DateTimeBetween(
            Utc::now() - chrono::Duration::hours(1),
            Utc::now()
        ).fake(),
        priority: (1..10).fake(),
        metadata: HashMap::new(),
    }
}

// Test data scenarios
pub struct TestScenario {
    pub name: String,
    pub memory_entries: Vec<MemoryEntry>,
    pub agent_states: Vec<AgentState>,
    pub messages: Vec<AgentMessage>,
}

impl TestScenario {
    pub fn simple_scenario() -> Self {
        Self {
            name: "Simple Test Scenario".to_string(),
            memory_entries: create_test_memory_entries(5),
            agent_states: vec![
                create_test_agent_state("agent-1"),
                create_test_agent_state("agent-2"),
            ],
            messages: vec![
                create_test_agent_message("agent-1", "agent-2", "Hello from agent 1"),
                create_test_agent_message("agent-2", "agent-1", "Hello back from agent 2"),
            ],
        }
    }

    pub fn large_dataset_scenario() -> Self {
        Self {
            name: "Large Dataset Scenario".to_string(),
            memory_entries: create_test_memory_entries(1000),
            agent_states: (0..20)
                .map(|i| create_test_agent_state(&format!("agent-{}", i)))
                .collect(),
            messages: (0..100)
                .map(|i| create_test_agent_message(
                    &format!("agent-{}", i % 20),
                    &format!("agent-{}", (i + 1) % 20),
                    &format!("Message {}", i)
                ))
                .collect(),
        }
    }

    pub fn tagged_memory_scenario() -> Self {
        let tags = ["work", "personal", "research", "notes", "ideas"];
        Self {
            name: "Tagged Memory Scenario".to_string(),
            memory_entries: create_memory_entries_with_tags(&tags, 20),
            agent_states: vec![create_test_agent_state("research-agent")],
            messages: Vec::new(),
        }
    }
}

// Performance testing utilities
pub fn create_large_memory_entry(size_kb: usize) -> MemoryEntry {
    let content_size = size_kb * 1024;
    let content = "x".repeat(content_size);

    MemoryEntryBuilder::new()
        .with_content(content)
        .with_tag("large-content")
        .build()
}

pub fn create_memory_entries_batch(batch_size: usize, total_batches: usize) -> Vec<Vec<MemoryEntry>> {
    (0..total_batches)
        .map(|batch_id| {
            (0..batch_size)
                .map(|entry_id| {
                    MemoryEntryBuilder::new()
                        .with_id(format!("batch-{}-entry-{}", batch_id, entry_id))
                        .with_content(format!("Batch {} Entry {} content", batch_id, entry_id))
                        .with_tag(format!("batch-{}", batch_id))
                        .build()
                })
                .collect()
        })
        .collect()
}

// Test assertion helpers
pub fn assert_memory_entries_equal(actual: &MemoryEntry, expected: &MemoryEntry) {
    assert_eq!(actual.id, expected.id, "Memory entry IDs don't match");
    assert_eq!(actual.content, expected.content, "Memory entry content doesn't match");
    assert_eq!(actual.metadata.agent_id, expected.metadata.agent_id, "Agent IDs don't match");
    assert_eq!(actual.metadata.tags, expected.metadata.tags, "Tags don't match");
}

pub fn assert_entries_have_tags(entries: &[MemoryEntry], expected_tags: &[String]) {
    for entry in entries {
        assert!(!entry.metadata.tags.is_empty(), "Entry {} has no tags", entry.id);
        assert!(
            entry.metadata.tags.iter().any(|tag| expected_tags.contains(tag)),
            "Entry {} doesn't have any expected tags. Expected: {:?}, Actual: {:?}",
            entry.id,
            expected_tags,
            entry.metadata.tags
        );
    }
}
```

This comprehensive testing strategy covers all aspects of testing the agent-sync Rust library with Redis operations. The strategy includes:

1. **Complete test organization** with unit, integration, and benchmark tests
2. **Sophisticated mocking strategies** for Redis connections and operations
3. **Cross-platform testing** with Windows, WSL, Linux, and macOS support
4. **Performance benchmarking** with regression detection
5. **CI/CD pipeline** with matrix testing across platforms and Redis versions
6. **Test utilities and factories** for generating test data
7. **Automated analysis and reporting** of test results

The key aspects that make this strategy comprehensive:

- **Test pyramid approach**: Many unit tests, fewer integration tests, minimal end-to-end tests
- **Platform-specific configurations**: Different settings for Windows vs WSL vs Linux
- **Mock variations**: Simple mocks, behavioral mocks, and scenario-based mocks
- **Performance focus**: Benchmarking with regression detection and cross-platform comparison
- **Automation**: Fully automated CI/CD with comprehensive reporting
- **Maintainability**: Clear test organization and reusable test utilities

This strategy ensures the agent-sync library is thoroughly tested across all supported platforms while maintaining fast feedback loops and comprehensive coverage.