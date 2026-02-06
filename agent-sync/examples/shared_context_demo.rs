//! Shared context demonstration
//!
//! This example shows how multiple agents can share data through
//! the distributed context system with different scopes and access patterns.

use agent_sync::coordinator::{
    ContextData, ContextResult, ContextScope, Coordinator, CoordinatorConfig,
};
use agent_sync::{AgentId, SessionId};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get Redis URL from environment or use default
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    println!("🚀 Starting Shared Context Demo");
    println!("📡 Connecting to Redis at: {}", redis_url);

    // Create coordinator
    let config = CoordinatorConfig {
        redis_url,
        ..Default::default()
    };

    let coordinator = Coordinator::new(config).await?;
    let shared_context = coordinator.shared_context();

    // Create multiple agents and a session
    let agent1_id = AgentId::new();
    let agent2_id = AgentId::new();
    let session_id = SessionId::new();

    println!("👤 Agent 1 ID: {}", agent1_id);
    println!("👤 Agent 2 ID: {}", agent2_id);
    println!("🔗 Session ID: {}", session_id);

    // Demonstrate different context scopes
    demonstrate_global_context(shared_context, agent1_id, agent2_id).await?;
    demonstrate_session_context(shared_context, agent1_id, session_id).await?;
    demonstrate_agent_context(shared_context, agent1_id, agent2_id).await?;
    demonstrate_temporary_context(shared_context, agent1_id).await?;
    demonstrate_optimistic_locking(shared_context, agent1_id, agent2_id).await?;
    demonstrate_context_operations(shared_context, agent1_id, session_id).await?;

    // Clean up
    coordinator.shutdown().await?;
    println!("\n🛑 Shared context demo completed!");

    Ok(())
}

async fn demonstrate_global_context(
    shared_context: &agent_sync::coordinator::SharedContext,
    agent1_id: AgentId,
    _agent2_id: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🌍 === Demonstrating Global Context ===");

    // Agent 1 stores global configuration
    let config_data = ContextData::new(
        br#"{"api_endpoint": "https://api.example.com", "timeout": 30}"#.to_vec(),
        "JSON".to_string(),
        agent1_id,
    );

    let result = shared_context
        .set(ContextScope::Global, "app_config", config_data)
        .await?;

    match result {
        ContextResult::Success => println!("  ✅ Agent 1 stored global config"),
        _ => println!("  ❌ Failed to store global config"),
    }

    // Agent 2 reads the global configuration
    if let Some(retrieved_config) = shared_context
        .get(ContextScope::Global, "app_config")
        .await?
    {
        println!("  📖 Agent 2 retrieved global config:");
        println!(
            "    • Data: {}",
            String::from_utf8_lossy(&retrieved_config.data)
        );
        println!("    • Type: {}", retrieved_config.data_type);
        println!("    • Created by: {}", retrieved_config.created_by);
        println!("    • Revision: {}", retrieved_config.revision);
    } else {
        println!("  ❌ Agent 2 could not retrieve global config");
    }

    // List keys in global scope
    let keys = shared_context.list_keys(ContextScope::Global, "*").await?;
    println!("  📋 Global keys: {:?}", keys);

    Ok(())
}

async fn demonstrate_session_context(
    shared_context: &agent_sync::coordinator::SharedContext,
    agent_id: AgentId,
    session_id: SessionId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔗 === Demonstrating Session Context ===");

    let scope = ContextScope::Session(session_id);

    // Store session-specific data
    let session_data = ContextData::new(
        b"user_authenticated=true,preferences={theme:dark}".to_vec(),
        "SessionData".to_string(),
        agent_id,
    );

    shared_context
        .set(scope.clone(), "session_state", session_data)
        .await?;
    println!("  ✅ Stored session state");

    // Store multiple session variables
    let variables = vec![
        ("user_id", "12345"),
        ("language", "en-US"),
        ("timezone", "America/New_York"),
    ];

    for (key, value) in variables {
        let data = ContextData::new(value.as_bytes().to_vec(), "String".to_string(), agent_id);
        shared_context.set(scope.clone(), key, data).await?;
        println!("  📝 Set session variable '{}' = '{}'", key, value);
    }

    // List all session keys
    let session_keys = shared_context.list_keys(scope.clone(), "*").await?;
    println!("  📋 Session keys: {:?}", session_keys);

    // Retrieve and display session state
    for key in &session_keys {
        if let Some(data) = shared_context.get(scope.clone(), key).await? {
            println!("  📖 {}: {}", key, String::from_utf8_lossy(&data.data));
        }
    }

    Ok(())
}

async fn demonstrate_agent_context(
    shared_context: &agent_sync::coordinator::SharedContext,
    agent1_id: AgentId,
    agent2_id: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n👤 === Demonstrating Agent-Specific Context ===");

    // Each agent stores its own private data
    let agent1_scope = ContextScope::Agent(agent1_id);
    let agent2_scope = ContextScope::Agent(agent2_id);

    // Agent 1 stores private data
    let agent1_data = ContextData::new(
        b"secret_key=abc123,last_action=process_data".to_vec(),
        "AgentState".to_string(),
        agent1_id,
    );

    shared_context
        .set(agent1_scope.clone(), "private_state", agent1_data)
        .await?;
    println!("  🔒 Agent 1 stored private state");

    // Agent 2 stores its private data
    let agent2_data = ContextData::new(
        b"secret_key=xyz789,last_action=send_notification".to_vec(),
        "AgentState".to_string(),
        agent2_id,
    );

    shared_context
        .set(agent2_scope.clone(), "private_state", agent2_data)
        .await?;
    println!("  🔒 Agent 2 stored private state");

    // Each agent can only access its own data
    if let Some(data) = shared_context.get(agent1_scope, "private_state").await? {
        println!(
            "  📖 Agent 1 retrieved its data: {}",
            String::from_utf8_lossy(&data.data)
        );
    }

    if let Some(data) = shared_context.get(agent2_scope, "private_state").await? {
        println!(
            "  📖 Agent 2 retrieved its data: {}",
            String::from_utf8_lossy(&data.data)
        );
    }

    // Agent 1 cannot access Agent 2's data in the same key
    let agent2_scope_from_agent1 = ContextScope::Agent(agent2_id);
    if shared_context
        .get(agent2_scope_from_agent1, "private_state")
        .await?
        .is_none()
    {
        println!("  🚫 Agent 1 cannot access Agent 2's private state (as expected)");
    }

    Ok(())
}

async fn demonstrate_temporary_context(
    shared_context: &agent_sync::coordinator::SharedContext,
    agent_id: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⏰ === Demonstrating Temporary Context ===");

    // Store data with short TTL
    let temp_scope = ContextScope::Temporary { ttl_secs: 3 };
    let temp_data = ContextData::new(
        b"This data will expire in 3 seconds".to_vec(),
        "String".to_string(),
        agent_id,
    );

    shared_context
        .set(temp_scope.clone(), "temp_message", temp_data)
        .await?;
    println!("  ⏳ Stored temporary data (TTL: 3 seconds)");

    // Verify data exists immediately
    if let Some(data) = shared_context
        .get(temp_scope.clone(), "temp_message")
        .await?
    {
        println!("  ✅ Data exists: {}", String::from_utf8_lossy(&data.data));
    }

    // Wait for expiration
    println!("  ⏳ Waiting for expiration...");
    sleep(Duration::from_secs(4)).await;

    // Verify data is gone
    if shared_context
        .get(temp_scope, "temp_message")
        .await?
        .is_none()
    {
        println!("  ✅ Data expired as expected");
    } else {
        println!("  ❌ Data should have expired");
    }

    Ok(())
}

async fn demonstrate_optimistic_locking(
    shared_context: &agent_sync::coordinator::SharedContext,
    agent1_id: AgentId,
    _agent2_id: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔒 === Demonstrating Optimistic Locking ===");

    let scope = ContextScope::Global;
    let key = "counter";

    // Initialize counter
    let initial_data = ContextData::new(b"0".to_vec(), "Integer".to_string(), agent1_id);

    shared_context.set(scope.clone(), key, initial_data).await?;
    println!("  📝 Initialized counter to 0");

    // Both agents read the counter
    let agent1_view = shared_context
        .get(scope.clone(), key)
        .await?
        .expect("Counter should exist");
    let agent2_view = shared_context
        .get(scope.clone(), key)
        .await?
        .expect("Counter should exist");

    println!(
        "  👤 Agent 1 sees counter at revision {}",
        agent1_view.revision
    );
    println!(
        "  👤 Agent 2 sees counter at revision {}",
        agent2_view.revision
    );

    // Agent 1 updates first
    let mut agent1_update = agent1_view.clone();
    agent1_update.data = b"1".to_vec();

    let update_result = shared_context
        .update(scope.clone(), key, agent1_update, agent1_view.revision)
        .await?;

    match update_result {
        ContextResult::Success => println!("  ✅ Agent 1 successfully incremented counter"),
        _ => println!("  ❌ Agent 1 update failed"),
    }

    // Agent 2 tries to update with stale revision
    let mut agent2_update = agent2_view.clone();
    agent2_update.data = b"2".to_vec();

    let stale_update_result = shared_context
        .update(
            scope.clone(),
            key,
            agent2_update,
            agent2_view.revision, // This is now stale
        )
        .await?;

    match stale_update_result {
        ContextResult::ConflictDetected { current_revision } => {
            println!(
                "  🚨 Agent 2 detected conflict! Current revision: {}",
                current_revision
            );
        }
        _ => println!("  ❌ Expected conflict detection"),
    }

    // Agent 2 gets fresh data and tries again
    let fresh_view = shared_context
        .get(scope.clone(), key)
        .await?
        .expect("Counter should exist");

    let mut agent2_retry = fresh_view.clone();
    agent2_retry.data = b"2".to_vec();

    let retry_result = shared_context
        .update(scope, key, agent2_retry, fresh_view.revision)
        .await?;

    match retry_result {
        ContextResult::Success => println!("  ✅ Agent 2 successfully updated with fresh revision"),
        _ => println!("  ❌ Agent 2 retry failed"),
    }

    Ok(())
}

async fn demonstrate_context_operations(
    shared_context: &agent_sync::coordinator::SharedContext,
    agent_id: AgentId,
    session_id: SessionId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🛠️ === Demonstrating Context Operations ===");

    let scope = ContextScope::Session(session_id);

    // Test exists operation
    let exists_before = shared_context.exists(scope.clone(), "test_key").await?;
    println!("  ❓ Key exists before creation: {}", exists_before);

    // Create test data
    let test_data = ContextData::new(b"test_value".to_vec(), "String".to_string(), agent_id);

    shared_context
        .set(scope.clone(), "test_key", test_data)
        .await?;

    let exists_after = shared_context.exists(scope.clone(), "test_key").await?;
    println!("  ✅ Key exists after creation: {}", exists_after);

    // Test TTL operations
    shared_context
        .set_ttl(scope.clone(), "test_key", Duration::from_secs(60))
        .await?;
    println!("  ⏰ Set TTL to 60 seconds");

    if let Some(ttl) = shared_context.get_ttl(scope.clone(), "test_key").await? {
        println!("  📅 Current TTL: {} seconds", ttl.as_secs());
    }

    // Test increment operation for numeric values
    shared_context
        .set(
            scope.clone(),
            "numeric_counter",
            ContextData::new(b"0".to_vec(), "Integer".to_string(), agent_id),
        )
        .await?;

    let incremented = shared_context
        .increment(scope.clone(), "numeric_counter", 5)
        .await?;
    println!("  📊 Incremented counter by 5, new value: {}", incremented);

    let decremented = shared_context
        .increment(scope.clone(), "numeric_counter", -2)
        .await?;
    println!("  📊 Decremented counter by 2, new value: {}", decremented);

    // Test delete operation
    let deleted = shared_context.delete(scope.clone(), "test_key").await?;
    println!("  🗑️ Deleted test_key: {}", deleted);

    let exists_after_delete = shared_context.exists(scope.clone(), "test_key").await?;
    println!("  ❓ Key exists after deletion: {}", exists_after_delete);

    // Test bulk operations
    println!("\n📦 Testing bulk operations...");

    // Create multiple keys
    for i in 1..=5 {
        let data = ContextData::new(
            format!("value_{}", i).into_bytes(),
            "String".to_string(),
            agent_id,
        );
        shared_context
            .set(scope.clone(), &format!("bulk_key_{}", i), data)
            .await?;
    }

    // List all keys with pattern
    let bulk_keys = shared_context
        .list_keys(scope.clone(), "bulk_key_*")
        .await?;
    println!("  📋 Created bulk keys: {:?}", bulk_keys);

    // Clear entire scope
    let cleared_count = shared_context.clear_scope(scope).await?;
    println!("  🧹 Cleared {} keys from session scope", cleared_count);

    Ok(())
}
