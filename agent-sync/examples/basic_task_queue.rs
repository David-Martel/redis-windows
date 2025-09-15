//! Basic task queue example
//!
//! This example demonstrates the basic usage of the task queue system,
//! showing how to create tasks, enqueue them, and process them.

use agent_sync::coordinator::{Coordinator, CoordinatorConfig, Task, TaskPriority, TaskResult};
use agent_sync::AgentId;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get Redis URL from environment or use default
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    println!("🚀 Starting Basic Task Queue Example");
    println!("📡 Connecting to Redis at: {}", redis_url);

    // Create coordinator
    let config = CoordinatorConfig {
        redis_url,
        max_concurrent_tasks: 10,
        task_timeout_secs: 60,
        enable_metrics: true,
        notification_buffer_size: 100,
    };

    let coordinator = Coordinator::new(config).await?;
    let task_queue = coordinator.task_queue();

    // Create producer and worker agents
    let producer_id = AgentId::new();
    let worker_id = AgentId::new();

    println!("👤 Producer Agent ID: {}", producer_id);
    println!("🔧 Worker Agent ID: {}", worker_id);

    // Create and enqueue some example tasks
    println!("\n📝 Creating and enqueueing tasks...");

    let tasks = vec![
        Task::new("process_data", b"data_chunk_1".to_vec(), producer_id)
            .with_priority(TaskPriority::High),
        Task::new("send_email", b"email_notification".to_vec(), producer_id)
            .with_priority(TaskPriority::Normal),
        Task::new("cleanup_temp_files", b"/tmp/cleanup".to_vec(), producer_id)
            .with_priority(TaskPriority::Low),
        Task::new("backup_database", b"db_backup_config".to_vec(), producer_id)
            .with_priority(TaskPriority::Critical),
    ];

    for task in &tasks {
        println!(
            "  ➕ Enqueueing task '{}' with priority {:?}",
            task.name, task.priority
        );
        task_queue.enqueue(task.clone()).await?;
    }

    // Check initial queue stats
    let stats = task_queue.get_stats().await?;
    println!("\n📊 Queue Statistics:");
    println!("  • Pending: {}", stats.pending_count);
    println!("  • In Progress: {}", stats.in_progress_count);
    println!("  • Completed: {}", stats.completed_count);

    // Process tasks (worker simulation)
    println!("\n🔧 Starting worker to process tasks...");

    let mut processed_count = 0;
    while let Some(task) = task_queue.dequeue(worker_id).await? {
        println!(
            "  📥 Dequeued task '{}' (Priority: {:?})",
            task.name, task.priority
        );

        // Simulate task processing
        let processing_time = match task.priority {
            TaskPriority::Critical => Duration::from_millis(500),
            TaskPriority::High => Duration::from_millis(1000),
            TaskPriority::Normal => Duration::from_millis(1500),
            TaskPriority::Low => Duration::from_millis(2000),
        };

        println!("    ⏳ Processing for {:?}...", processing_time);
        sleep(processing_time).await;

        // Create task result
        let mut metadata = HashMap::new();
        metadata.insert("worker_id".to_string(), worker_id.to_string());
        metadata.insert(
            "processing_time_ms".to_string(),
            processing_time.as_millis().to_string(),
        );

        let result = TaskResult {
            data: format!("Processed: {}", String::from_utf8_lossy(&task.payload)).into_bytes(),
            duration_ms: processing_time.as_millis() as u64,
            metadata,
        };

        // Complete the task
        task_queue.complete_task(task.id, result).await?;
        println!("    ✅ Task '{}' completed successfully", task.name);

        processed_count += 1;

        // Show updated stats
        let stats = task_queue.get_stats().await?;
        println!(
            "    📊 Queue: {} pending, {} in progress, {} completed",
            stats.pending_count, stats.in_progress_count, stats.completed_count
        );
    }

    println!("\n🎉 All tasks processed! Total: {}", processed_count);

    // Final statistics
    let final_stats = task_queue.get_stats().await?;
    println!("\n📈 Final Statistics:");
    println!("  • Total Tasks Processed: {}", final_stats.completed_count);
    println!("  • Remaining Pending: {}", final_stats.pending_count);
    println!("  • Still In Progress: {}", final_stats.in_progress_count);

    // Demonstrate task retrieval
    println!("\n🔍 Demonstrating task retrieval...");
    for task in &tasks {
        if let Some(completed_task) = task_queue.get_task(task.id).await? {
            println!(
                "  📋 Task '{}': Status = {:?}",
                completed_task.name,
                std::mem::discriminant(&completed_task.status)
            );
        }
    }

    // Clean up
    coordinator.shutdown().await?;
    println!("\n🛑 Task queue example completed!");

    Ok(())
}

/// Helper function to demonstrate error handling
#[allow(dead_code)]
async fn demonstrate_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚨 Demonstrating error handling...");

    // Try to connect to invalid Redis URL
    let config = CoordinatorConfig {
        redis_url: "redis://invalid:1234".to_string(),
        ..Default::default()
    };

    match Coordinator::new(config).await {
        Ok(_) => println!("  ❌ Unexpected success with invalid Redis URL"),
        Err(e) => println!("  ✅ Correctly caught connection error: {}", e),
    }

    Ok(())
}
