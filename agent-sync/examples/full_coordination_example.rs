//! Full coordination example
//!
//! This example demonstrates a complete workflow using all coordinator
//! components: task queues, shared context, and notifications working together.

use agent_sync::coordinator::{
    ContextData, ContextResult, ContextScope, Coordinator, CoordinatorConfig, Notification,
    NotificationFilter, NotificationPriority, NotificationType, Task, TaskPriority, TaskResult,
    TaskStatus,
};
use agent_sync::{AgentId, SessionId};
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::join;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

/// Represents different agent roles in our workflow
#[derive(Debug, Clone, Copy)]
enum AgentRole {
    Coordinator,
    Worker,
    Monitor,
}

/// Workflow state stored in shared context
#[derive(Debug, Clone)]
struct WorkflowState {
    workflow_id: Uuid,
    total_tasks: usize,
    completed_tasks: usize,
    failed_tasks: usize,
    status: String,
}

impl WorkflowState {
    fn new(workflow_id: Uuid, total_tasks: usize) -> Self {
        Self {
            workflow_id,
            total_tasks,
            completed_tasks: 0,
            failed_tasks: 0,
            status: "running".to_string(),
        }
    }

    fn to_json(&self) -> String {
        serde_json::json!({
            "workflow_id": self.workflow_id,
            "total_tasks": self.total_tasks,
            "completed_tasks": self.completed_tasks,
            "failed_tasks": self.failed_tasks,
            "status": self.status
        })
        .to_string()
    }

    fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        Ok(Self {
            workflow_id: Uuid::parse_str(value["workflow_id"].as_str().unwrap_or("")).unwrap(),
            total_tasks: value["total_tasks"].as_u64().unwrap_or(0) as usize,
            completed_tasks: value["completed_tasks"].as_u64().unwrap_or(0) as usize,
            failed_tasks: value["failed_tasks"].as_u64().unwrap_or(0) as usize,
            status: value["status"].as_str().unwrap_or("unknown").to_string(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get Redis URL from environment or use default
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    println!("🚀 Starting Full Coordination Example");
    println!("📡 Connecting to Redis at: {}", redis_url);

    // Create coordinator
    let config = CoordinatorConfig {
        redis_url,
        max_concurrent_tasks: 20,
        task_timeout_secs: 300,
        enable_metrics: true,
        notification_buffer_size: 200,
    };

    let coordinator = Coordinator::new(config).await?;

    // Create agents with different roles
    let coordinator_agent = AgentId::new();
    let worker1_agent = AgentId::new();
    let worker2_agent = AgentId::new();
    let monitor_agent = AgentId::new();
    let session_id = SessionId::new();

    println!("👤 Coordinator Agent: {}", coordinator_agent);
    println!("🔧 Worker 1 Agent: {}", worker1_agent);
    println!("🔧 Worker 2 Agent: {}", worker2_agent);
    println!("📊 Monitor Agent: {}", monitor_agent);
    println!("🔗 Session ID: {}", session_id);

    // Run the complete workflow
    run_data_processing_workflow(
        &coordinator,
        coordinator_agent,
        vec![worker1_agent, worker2_agent],
        monitor_agent,
        session_id,
    )
    .await?;

    // Clean up
    coordinator.shutdown().await?;
    println!("\n🛑 Full coordination example completed!");

    Ok(())
}

async fn run_data_processing_workflow(
    coordinator: &Coordinator,
    coordinator_agent: AgentId,
    worker_agents: Vec<AgentId>,
    monitor_agent: AgentId,
    session_id: SessionId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 === Starting Data Processing Workflow ===");

    let workflow_id = Uuid::new_v4();
    println!("🆔 Workflow ID: {}", workflow_id);

    // 1. Initialize workflow state in shared context
    let task_count = 10;
    let workflow_state = WorkflowState::new(workflow_id, task_count);

    let workflow_data = ContextData::new(
        workflow_state.to_json().into_bytes(),
        "WorkflowState".to_string(),
        coordinator_agent,
    );

    coordinator
        .shared_context()
        .set(
            ContextScope::Session(session_id),
            "workflow_state",
            workflow_data,
        )
        .await?;

    println!("✅ Workflow state initialized");

    // 2. Start monitoring task
    let monitor_handle =
        start_workflow_monitor(coordinator, monitor_agent, session_id, workflow_id);

    // 3. Create and enqueue data processing tasks
    println!("\n📝 Creating {} data processing tasks...", task_count);

    for i in 0..task_count {
        let task_data = serde_json::json!({
            "workflow_id": workflow_id,
            "task_index": i,
            "data_chunk": format!("chunk_{:03}", i),
            "processing_type": match i % 3 {
                0 => "transform",
                1 => "validate",
                2 => "aggregate",
                _ => unreachable!(),
            }
        });

        let task = Task::new(
            format!("process_data_{}", i),
            task_data.to_string().into_bytes(),
            coordinator_agent,
        )
        .with_priority(match i {
            0 => TaskPriority::Critical, // First task is critical
            x if x < 3 => TaskPriority::High,
            x if x < 7 => TaskPriority::Normal,
            _ => TaskPriority::Low,
        })
        .with_metadata("workflow_id".to_string(), workflow_id.to_string());

        coordinator.task_queue().enqueue(task).await?;
        println!("  ➕ Enqueued task {}: {}", i, task.name);
    }

    // 4. Start worker tasks
    println!("\n🔧 Starting {} worker agents...", worker_agents.len());

    let worker_handles: Vec<_> = worker_agents
        .into_iter()
        .enumerate()
        .map(|(index, worker_agent)| {
            start_worker_agent(
                coordinator,
                worker_agent,
                index + 1,
                session_id,
                workflow_id,
            )
        })
        .collect();

    // 5. Wait for all tasks to complete
    println!("\n⏳ Waiting for workflow completion...");

    // Wait for all workers to finish
    let worker_results = join!(
        worker_handles.get(0).unwrap(),
        worker_handles.get(1).unwrap(),
    );

    println!("🔧 Worker 1 processed {} tasks", worker_results.0?);
    println!("🔧 Worker 2 processed {} tasks", worker_results.1?);

    // 6. Wait for monitoring to complete
    let monitoring_result = monitor_handle.await?;
    println!("📊 Monitoring completed: {}", monitoring_result);

    // 7. Generate workflow summary
    generate_workflow_summary(coordinator, session_id, workflow_id).await?;

    Ok(())
}

async fn start_worker_agent(
    coordinator: &Coordinator,
    worker_agent: AgentId,
    worker_number: usize,
    session_id: SessionId,
    workflow_id: Uuid,
) -> tokio::task::JoinHandle<Result<usize, Box<dyn std::error::Error + Send + Sync>>> {
    let coordinator = coordinator.clone(); // Assume Clone is implemented or use Arc

    tokio::spawn(async move {
        println!("🔧 Worker {} starting...", worker_number);

        let mut processed_count = 0;
        let task_queue = coordinator.task_queue();
        let shared_context = coordinator.shared_context();
        let notification_bus = coordinator.notification_bus();

        loop {
            // Try to dequeue a task
            match task_queue.dequeue(worker_agent).await {
                Ok(Some(task)) => {
                    println!(
                        "  🔧 Worker {} processing task: {}",
                        worker_number, task.name
                    );

                    // Parse task data
                    let task_data: serde_json::Value = serde_json::from_slice(&task.payload)?;
                    let task_workflow_id =
                        Uuid::parse_str(task_data["workflow_id"].as_str().unwrap_or(""))?;

                    // Only process tasks for our workflow
                    if task_workflow_id != workflow_id {
                        continue;
                    }

                    // Simulate processing time based on task type
                    let processing_type =
                        task_data["processing_type"].as_str().unwrap_or("unknown");
                    let processing_time = match processing_type {
                        "transform" => Duration::from_millis(800),
                        "validate" => Duration::from_millis(500),
                        "aggregate" => Duration::from_millis(1200),
                        _ => Duration::from_millis(1000),
                    };

                    println!(
                        "    ⏳ Processing {} for {:?}",
                        processing_type, processing_time
                    );
                    sleep(processing_time).await;

                    // Simulate occasional failures
                    let task_index = task_data["task_index"].as_u64().unwrap_or(0);
                    let should_fail = task_index == 7; // Task 7 will fail for demonstration

                    if should_fail {
                        // Fail the task
                        task_queue
                            .fail_task(task.id, "Simulated processing error".to_string())
                            .await?;

                        println!(
                            "    ❌ Worker {} failed task {} (simulated)",
                            worker_number, task.name
                        );

                        // Notify about failure
                        let failure_notification = Notification::new(
                            NotificationType::Task {
                                task_id: task.id,
                                action: "failed".to_string(),
                            },
                            format!("Task {} failed", task.name).into_bytes(),
                            worker_agent,
                        )
                        .with_priority(NotificationPriority::High);

                        notification_bus.publish(failure_notification).await?;

                        // Update workflow state
                        update_workflow_state(shared_context, session_id, |state| {
                            state.failed_tasks += 1;
                        })
                        .await?;
                    } else {
                        // Complete the task successfully
                        let mut result_metadata = HashMap::new();
                        result_metadata.insert("worker_id".to_string(), worker_agent.to_string());
                        result_metadata
                            .insert("processing_type".to_string(), processing_type.to_string());
                        result_metadata
                            .insert("worker_number".to_string(), worker_number.to_string());

                        let result = TaskResult {
                            data: format!("Processed: {}", processing_type).into_bytes(),
                            duration_ms: processing_time.as_millis() as u64,
                            metadata: result_metadata,
                        };

                        task_queue.complete_task(task.id, result).await?;
                        processed_count += 1;

                        println!(
                            "    ✅ Worker {} completed task {}",
                            worker_number, task.name
                        );

                        // Notify about completion
                        let completion_notification = Notification::new(
                            NotificationType::Task {
                                task_id: task.id,
                                action: "completed".to_string(),
                            },
                            format!("Task {} completed by worker {}", task.name, worker_number)
                                .into_bytes(),
                            worker_agent,
                        )
                        .with_priority(NotificationPriority::Normal);

                        notification_bus.publish(completion_notification).await?;

                        // Update workflow state
                        update_workflow_state(shared_context, session_id, |state| {
                            state.completed_tasks += 1;
                        })
                        .await?;
                    }
                }
                Ok(None) => {
                    // No tasks available, check if workflow is complete
                    let current_state = get_workflow_state(shared_context, session_id).await?;
                    if current_state.completed_tasks + current_state.failed_tasks
                        >= current_state.total_tasks
                    {
                        break;
                    }

                    // Wait a bit before checking again
                    sleep(Duration::from_millis(500)).await;
                }
                Err(e) => {
                    println!("    ❌ Worker {} error: {}", worker_number, e);
                    break;
                }
            }
        }

        println!(
            "🔧 Worker {} finished, processed {} tasks",
            worker_number, processed_count
        );
        Ok(processed_count)
    })
}

async fn start_workflow_monitor(
    coordinator: &Coordinator,
    monitor_agent: AgentId,
    session_id: SessionId,
    workflow_id: Uuid,
) -> tokio::task::JoinHandle<Result<String, Box<dyn std::error::Error + Send + Sync>>> {
    let shared_context = coordinator.shared_context().clone(); // Assume Clone or use Arc
    let notification_bus = coordinator.notification_bus().clone(); // Assume Clone or use Arc

    tokio::spawn(async move {
        println!("📊 Monitor starting...");

        // Subscribe to task notifications
        let task_filter = NotificationFilter::new().with_types(vec![NotificationType::Task {
            task_id: Uuid::new_v4(), // Will be ignored in discriminant matching
            action: "".to_string(),
        }]);

        let (subscription_id, mut receiver) = notification_bus
            .subscribe(monitor_agent, task_filter)
            .await?;

        let mut last_update = std::time::Instant::now();

        loop {
            tokio::select! {
                // Handle task notifications
                notification_result = receiver.recv() => {
                    if let Some(notification) = notification_result {
                        match &notification.notification_type {
                            NotificationType::Task { task_id, action } => {
                                println!("📊 Monitor: Task {} {}", task_id, action);
                            }
                            _ => {}
                        }
                    }
                }

                // Periodic status updates
                _ = sleep(Duration::from_secs(2)) => {
                    let current_state = get_workflow_state(&shared_context, session_id).await?;

                    println!("📊 Workflow Progress: {}/{} completed, {} failed",
                            current_state.completed_tasks,
                            current_state.total_tasks,
                            current_state.failed_tasks);

                    // Check if workflow is complete
                    if current_state.completed_tasks + current_state.failed_tasks >= current_state.total_tasks {
                        // Update final status
                        update_workflow_state(
                            &shared_context,
                            session_id,
                            |state| {
                                state.status = if state.failed_tasks == 0 {
                                    "completed_successfully".to_string()
                                } else {
                                    "completed_with_failures".to_string()
                                };
                            }
                        ).await?;

                        // Send completion notification
                        let completion_notification = Notification::new(
                            NotificationType::System {
                                component: "workflow".to_string(),
                                level: "info".to_string(),
                            },
                            format!("Workflow {} completed", workflow_id).into_bytes(),
                            monitor_agent,
                        ).with_priority(NotificationPriority::High);

                        notification_bus.publish(completion_notification).await?;

                        break;
                    }

                    last_update = std::time::Instant::now();
                }
            }
        }

        notification_bus.unsubscribe(subscription_id).await?;

        let final_state = get_workflow_state(&shared_context, session_id).await?;
        Ok(format!(
            "Workflow monitoring completed: {}",
            final_state.status
        ))
    })
}

async fn get_workflow_state(
    shared_context: &agent_sync::coordinator::SharedContext,
    session_id: SessionId,
) -> Result<WorkflowState, Box<dyn std::error::Error + Send + Sync>> {
    let data = shared_context
        .get(ContextScope::Session(session_id), "workflow_state")
        .await?
        .ok_or("Workflow state not found")?;

    let json_str = String::from_utf8(data.data)?;
    let state = WorkflowState::from_json(&json_str)?;
    Ok(state)
}

async fn update_workflow_state<F>(
    shared_context: &agent_sync::coordinator::SharedContext,
    session_id: SessionId,
    updater: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce(&mut WorkflowState),
{
    let current_data = shared_context
        .get(ContextScope::Session(session_id), "workflow_state")
        .await?
        .ok_or("Workflow state not found")?;

    let json_str = String::from_utf8(current_data.data)?;
    let mut state = WorkflowState::from_json(&json_str)?;

    updater(&mut state);

    let updated_data = ContextData::new(
        state.to_json().into_bytes(),
        "WorkflowState".to_string(),
        current_data.created_by, // Keep original creator
    );

    shared_context
        .update(
            ContextScope::Session(session_id),
            "workflow_state",
            updated_data,
            current_data.revision,
        )
        .await?;

    Ok(())
}

async fn generate_workflow_summary(
    coordinator: &Coordinator,
    session_id: SessionId,
    workflow_id: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 === Workflow Summary ===");

    // Get final workflow state
    let final_state = get_workflow_state(coordinator.shared_context(), session_id).await?;

    println!("🆔 Workflow ID: {}", workflow_id);
    println!("📊 Total Tasks: {}", final_state.total_tasks);
    println!("✅ Completed: {}", final_state.completed_tasks);
    println!("❌ Failed: {}", final_state.failed_tasks);
    println!("🏁 Final Status: {}", final_state.status);

    // Get task queue statistics
    let queue_stats = coordinator.task_queue().get_stats().await?;
    println!("\n📈 Queue Statistics:");
    println!("  • Pending: {}", queue_stats.pending_count);
    println!("  • In Progress: {}", queue_stats.in_progress_count);
    println!("  • Completed: {}", queue_stats.completed_count);
    println!("  • Failed: {}", queue_stats.failed_count);

    // Get notification statistics
    let notification_stats = coordinator.notification_bus().get_stats().await;
    println!("\n📢 Notification Statistics:");
    println!("  • Total Sent: {}", notification_stats.total_sent);
    println!("  • Total Received: {}", notification_stats.total_received);

    // Store final summary in context
    let summary_data = serde_json::json!({
        "workflow_id": workflow_id,
        "summary": final_state.to_json(),
        "queue_stats": {
            "completed": queue_stats.completed_count,
            "failed": queue_stats.failed_count
        },
        "notification_stats": {
            "sent": notification_stats.total_sent,
            "received": notification_stats.total_received
        }
    });

    let summary_context = ContextData::new(
        summary_data.to_string().into_bytes(),
        "WorkflowSummary".to_string(),
        AgentId::new(), // System generated
    );

    coordinator
        .shared_context()
        .set(
            ContextScope::Session(session_id),
            "workflow_summary",
            summary_context,
        )
        .await?;

    println!("📄 Workflow summary saved to context");

    Ok(())
}
