//! High-level API example demonstrating the `AgentSync` client.
//!
//! Prerequisites: Redis running on localhost:6380 (the agent-optimized instance).
//!
//! Run with:
//!   cargo run --example high_level_api

use agent_sync::{
    coordinator::{ContextData, ContextScope, TaskResult},
    AgentCapabilities, AgentId, AgentSync, ConfigBuilder,
};
use std::collections::HashMap;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // 1. Build configuration
    let config = ConfigBuilder::new()
        .redis_url("redis://localhost:6380")
        .pool_size(4)
        .build()?;

    // 2. Connect
    let mut client = AgentSync::connect(config).await?;
    println!("Connected to Redis");

    // 3. Health check
    let healthy = client.health_check().await?;
    println!("Health check: {}", if healthy { "OK" } else { "FAIL" });

    // 4. Register this process as an agent
    let agent = client
        .register_self(
            "example-agent".to_string(),
            AgentCapabilities {
                roles: vec!["worker".to_string()],
                skills: vec!["code-review".to_string(), "testing".to_string()],
                max_concurrent_tasks: 3,
                metadata: HashMap::new(),
            },
        )
        .await?;
    println!("Registered as agent: {}", agent.id);

    // 5. Submit a task
    let task_id = client
        .submit_task("process-data", b"hello world".to_vec(), agent.id)
        .await?;
    println!("Submitted task: {}", task_id);

    // 6. Claim the task
    if let Some(task) = client.claim_task(agent.id).await? {
        println!("Claimed task: {} ({})", task.id, task.name);

        // 7. Complete the task
        client
            .complete_task(
                task.id,
                TaskResult {
                    data: b"done".to_vec(),
                    duration_ms: 42,
                    metadata: HashMap::new(),
                },
            )
            .await?;
        println!("Completed task: {}", task.id);
    }

    // 8. Context operations
    let ctx = ContextData::new(
        b"some config value".to_vec(),
        "String".to_string(),
        agent.id,
    );
    client.set_global("app:config", ctx).await?;
    println!("Stored global context");

    if let Some(data) = client.get_global("app:config").await? {
        println!(
            "Retrieved global context: {} bytes, revision {}",
            data.data.len(),
            data.revision
        );
    }

    // 9. List agents
    let agents = client.list_agents().await?;
    println!("Registered agents: {}", agents.len());

    // 10. Task stats
    let stats = client.task_stats().await?;
    println!(
        "Queue stats: {} pending, {} completed",
        stats.pending_count, stats.completed_count
    );

    // 11. Graceful shutdown
    client.shutdown().await?;
    println!("Shutdown complete");

    Ok(())
}
