# CLAUDE.md - Agent-Sync Project Standards

This file provides coding standards and requirements for Claude Code when working on the agent-sync Rust project.

## Core Development Principles

### 1. Performance First
- **Sub-millisecond latency** is non-negotiable for agent coordination
- Use `Arc<T>` over `Rc<T>` for thread-safe reference counting
- Use `Cow<'_, str>` for string operations where ownership is conditional
- Profile before optimizing - use `criterion` benchmarks as evidence

### 2. Error Handling Excellence
```rust
// ALWAYS use custom error types with context
#[derive(Debug, thiserror::Error)]
pub enum AgentSyncError {
    #[error("Redis connection failed: {0}")]
    Connection(#[from] redis::RedisError),

    #[error("Task {task_id} timed out after {duration:?}")]
    TaskTimeout { task_id: TaskId, duration: Duration },
}

// NEVER use .unwrap() in production code
// Use .expect() only in tests with descriptive messages
```

### 3. Async Best Practices
- Use `tokio::select!` for concurrent operations
- Implement graceful shutdown with `tokio::signal`
- Use `tokio::sync::RwLock` for async-aware locking
- Prefer `futures::stream::StreamExt` for stream processing
- Always set timeouts on async operations

### 4. Type Safety Requirements
- No `unsafe` code without `// SAFETY:` comments and review
- Use newtype pattern for domain identifiers (AgentId, TaskId, etc.)
- Implement `From` and `TryFrom` for type conversions
- Use builder pattern for complex struct construction
- Leverage const generics where applicable

## Testing Standards

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("valid_input", expected_output)]
    #[tokio::test]
    async fn test_function_name(#[case] input: &str, #[case] expected: &str) {
        // Arrange
        let sut = SystemUnderTest::new();

        // Act
        let result = sut.process(input).await;

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Integration Tests
- Use `testcontainers` for Redis instances
- Clean up resources in test teardown
- Use unique namespaces per test to avoid conflicts
- Test both happy path and error scenarios

## Documentation Requirements

### Public API Documentation
```rust
/// Submits a task to the distributed queue for processing by agents.
///
/// # Arguments
/// * `task` - The task to submit, containing payload and metadata
///
/// # Returns
/// * `Ok(TaskId)` - Unique identifier for tracking the task
/// * `Err(AgentSyncError)` - If submission fails
///
/// # Example
/// ```rust
/// # use agent_sync::*;
/// # async fn example() -> Result<(), AgentSyncError> {
/// let task = Task::new("process_data", payload);
/// let task_id = queue.submit_task(task).await?;
/// # Ok(())
/// # }
/// ```
pub async fn submit_task<T>(&self, task: Task<T>) -> Result<TaskId, AgentSyncError>
```

## Code Organization

### Module Structure
- One primary type per file
- Tests in submodule at bottom of file
- Benchmarks in separate `benches/` directory
- Examples in `examples/` directory with descriptive names

### Import Organization
```rust
// Standard library
use std::sync::Arc;
use std::time::Duration;

// External crates
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

// Internal modules
use crate::coordinator::TaskQueue;
use crate::types::TaskId;

// Re-exports at module level
pub use self::task_queue::RedisTaskQueue;
```

## Platform-Specific Code

### Windows Considerations
```rust
#[cfg(target_os = "windows")]
mod windows {
    use winapi::um::sysinfoapi;

    pub fn get_system_info() -> SystemInfo {
        // Windows-specific implementation
    }
}

#[cfg(not(target_os = "windows"))]
mod unix {
    pub fn get_system_info() -> SystemInfo {
        // Unix-like implementation
    }
}
```

### Path Handling
```rust
use std::path::PathBuf;

// ALWAYS use PathBuf for cross-platform paths
let config_path = PathBuf::from("config")
    .join("redis.conf");

// Use dunce crate for Windows UNC paths
#[cfg(target_os = "windows")]
let canonical_path = dunce::canonicalize(&path)?;
```

## Performance Guidelines

### Memory Management
- Use `Vec::with_capacity()` when size is known
- Implement `Drop` for resources requiring cleanup
- Use `Box::pin()` for self-referential async structs
- Prefer stack allocation for small fixed-size data

### Concurrency Patterns
```rust
// Use channels for inter-task communication
let (tx, mut rx) = tokio::sync::mpsc::channel(100);

// Use Arc<Mutex<T>> sparingly, prefer message passing
let shared_state = Arc::new(RwLock::new(State::new()));

// Use rayon for CPU-bound parallel processing
use rayon::prelude::*;
let results: Vec<_> = items.par_iter()
    .map(|item| process(item))
    .collect();
```

## Security Considerations

### Input Validation
- Validate all external inputs at boundaries
- Use `validator` crate for complex validation
- Sanitize data before Redis operations
- Implement rate limiting for API endpoints

### Secrets Management
```rust
// NEVER hardcode secrets
// Use environment variables or secure vaults
let redis_password = std::env::var("REDIS_PASSWORD")
    .unwrap_or_else(|_| String::new());
```

## CI/CD Requirements

### Pre-commit Checks
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo audit
```

### Coverage Requirements
- Minimum 80% code coverage
- 100% coverage for error handling paths
- Integration tests for all public APIs

## Dependency Management

### Approved Core Dependencies
```toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }
redis = { version = "0.32", features = ["tokio-comp", "connection-manager", "aio"] }
serde = { version = "1.0", features = ["derive"] }
thiserror = "2.0"
tracing = "0.1"
bb8 = "0.9"
bb8-redis = "0.24"

[dev-dependencies]
criterion = "0.5"
rstest = "0.18"
testcontainers = "0.15"
mockall = "0.12"
```

### Version Policy
- Use exact versions for direct dependencies
- Allow patch updates for dev-dependencies
- Audit dependencies weekly with `cargo-audit`

## Logging and Tracing

### Structured Logging
```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(self))]
pub async fn process_task(&self, task_id: TaskId) -> Result<(), Error> {
    info!(%task_id, "Processing task");

    // Use structured fields
    info!(
        task.id = %task_id,
        task.priority = ?priority,
        "Task processing started"
    );
}
```

## Local Development Setup

### Required Tools
```bash
# Install Rust toolchain
rustup default stable
rustup component add rustfmt clippy

# Install development tools
cargo install cargo-watch cargo-audit cargo-tarpaulin

# Install Redis (Windows)
# Use the existing Redis Windows service from parent project
```

### Development Workflow
```bash
# Watch mode for development
cargo watch -x test -x 'clippy -- -D warnings'

# Run benchmarks
cargo bench --bench task_queue_bench

# Generate documentation
cargo doc --open --no-deps

# Check cross-platform compatibility
cargo check --target x86_64-pc-windows-msvc
cargo check --target x86_64-unknown-linux-gnu
```

## Code Review Checklist

Before submitting code:
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation is complete
- [ ] Benchmarks show no regression
- [ ] Error handling is comprehensive
- [ ] No `.unwrap()` in production code
- [ ] Cross-platform compatibility verified

## Performance Targets

### Latency Requirements
- Task submission: < 1ms
- Context retrieval: < 500μs
- Notification broadcast: < 100μs
- Agent registration: < 5ms

### Throughput Requirements
- 100,000 tasks/second
- 1M context operations/second
- 10,000 concurrent agents
- 50,000 notifications/second

## Integration with Parent Project

### Redis Configuration
- Connect to port 6380 (agent-optimized config)
- Use existing `redis-agent.conf` settings
- Respect 2GB memory limit with LRU eviction
- Compatible with Windows Service wrapper

### File Locations
```
T:\projects\redis-windows\
├── agent-sync\           # This Rust project
├── redis-agent.conf      # Redis configuration
├── RedisService.exe      # Windows service wrapper
└── .claude\              # Project standards
```

---

These standards ensure high-quality, performant, and maintainable code for the agent-sync project.