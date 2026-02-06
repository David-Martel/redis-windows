//! Notification system example
//!
//! This example demonstrates the pub-sub notification system with multiple
//! subscribers, filtering, and different notification types.

use agent_sync::coordinator::{
    Coordinator, CoordinatorConfig, Notification, NotificationFilter, NotificationPriority,
    NotificationType,
};
use agent_sync::AgentId;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get Redis URL from environment or use default
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    println!("🚀 Starting Notification System Example");
    println!("📡 Connecting to Redis at: {}", redis_url);

    // Create coordinator
    let config = CoordinatorConfig {
        redis_url,
        ..Default::default()
    };

    let coordinator = Coordinator::new(config).await?;
    let notification_bus = coordinator.notification_bus();

    // Create multiple agents
    let publisher_id = AgentId::new();
    let subscriber1_id = AgentId::new();
    let subscriber2_id = AgentId::new();
    let subscriber3_id = AgentId::new();

    println!("👤 Publisher Agent ID: {}", publisher_id);
    println!("👤 Subscriber 1 Agent ID: {}", subscriber1_id);
    println!("👤 Subscriber 2 Agent ID: {}", subscriber2_id);
    println!("👤 Subscriber 3 Agent ID: {}", subscriber3_id);

    // Demonstrate basic pub-sub
    basic_pubsub_demo(&notification_bus, publisher_id, subscriber1_id).await?;

    // Demonstrate notification filtering
    filtering_demo(
        &notification_bus,
        publisher_id,
        subscriber1_id,
        subscriber2_id,
    )
    .await?;

    // Demonstrate multiple notification types
    notification_types_demo(&notification_bus, publisher_id, subscriber1_id).await?;

    // Demonstrate notification priorities
    priority_demo(&notification_bus, publisher_id, subscriber1_id).await?;

    // Demonstrate broadcast to multiple subscribers
    broadcast_demo(
        &notification_bus,
        publisher_id,
        vec![subscriber1_id, subscriber2_id, subscriber3_id],
    )
    .await?;

    // Demonstrate system notifications
    system_notifications_demo(&notification_bus, publisher_id, subscriber1_id).await?;

    // Show final statistics
    show_statistics(&notification_bus).await?;

    // Clean up
    coordinator.shutdown().await?;
    println!("\n🛑 Notification system example completed!");

    Ok(())
}

async fn basic_pubsub_demo(
    notification_bus: &agent_sync::coordinator::NotificationBus,
    publisher_id: AgentId,
    subscriber_id: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📢 === Basic Pub-Sub Demo ===");

    // Subscribe to all notifications
    let (subscription_id, mut receiver) = notification_bus
        .subscribe(subscriber_id, NotificationFilter::default())
        .await?;

    println!("  📝 Subscriber registered with ID: {}", subscription_id);

    // Publish a simple notification
    let notification = Notification::new(
        NotificationType::Custom {
            category: "demo".to_string(),
            data: {
                let mut data = HashMap::new();
                data.insert("message".to_string(), "Hello, World!".to_string());
                data
            },
        },
        b"This is a basic notification".to_vec(),
        publisher_id,
    );

    let notification_id = notification.id;
    println!("  📤 Publishing notification: {}", notification_id);

    let subscriber_count = notification_bus.publish(notification).await?;
    println!("  📊 Delivered to {} subscribers", subscriber_count);

    // Receive the notification
    let received = timeout(Duration::from_secs(5), receiver.recv())
        .await?
        .expect("Channel closed");

    println!("  📥 Received notification: {}", received.id);
    println!(
        "    • Payload: {}",
        String::from_utf8_lossy(&received.payload)
    );
    println!("    • Sender: {}", received.sender);

    // Unsubscribe
    notification_bus.unsubscribe(subscription_id).await?;
    println!("  📤 Unsubscribed");

    Ok(())
}

async fn filtering_demo(
    notification_bus: &agent_sync::coordinator::NotificationBus,
    publisher_id: AgentId,
    subscriber1_id: AgentId,
    subscriber2_id: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 === Notification Filtering Demo ===");

    // Subscriber 1: Only high priority notifications from publisher
    let filter1 = NotificationFilter::new()
        .with_sender(publisher_id)
        .with_min_priority(NotificationPriority::High);

    let (sub1_id, mut receiver1) = notification_bus.subscribe(subscriber1_id, filter1).await?;

    println!("  📝 Subscriber 1: High priority from publisher only");

    // Subscriber 2: All system notifications
    let filter2 = NotificationFilter::new().with_types(vec![NotificationType::System {
        component: "".to_string(),
        level: "".to_string(),
    }]);

    let (sub2_id, mut receiver2) = notification_bus.subscribe(subscriber2_id, filter2).await?;

    println!("  📝 Subscriber 2: System notifications only");

    // Publish different types of notifications
    let notifications = vec![
        (
            "High priority custom (should reach Subscriber 1)",
            Notification::new(
                NotificationType::Custom {
                    category: "important".to_string(),
                    data: HashMap::new(),
                },
                b"High priority message".to_vec(),
                publisher_id,
            )
            .with_priority(NotificationPriority::High),
        ),
        (
            "Low priority custom (should not reach Subscriber 1)",
            Notification::new(
                NotificationType::Custom {
                    category: "routine".to_string(),
                    data: HashMap::new(),
                },
                b"Low priority message".to_vec(),
                publisher_id,
            )
            .with_priority(NotificationPriority::Low),
        ),
        (
            "System notification (should reach Subscriber 2)",
            Notification::new(
                NotificationType::System {
                    component: "auth".to_string(),
                    level: "info".to_string(),
                },
                b"System status update".to_vec(),
                publisher_id,
            )
            .with_priority(NotificationPriority::Normal),
        ),
    ];

    for (description, notification) in notifications {
        println!("  📤 Publishing: {}", description);
        notification_bus.publish(notification).await?;
    }

    // Check what each subscriber received
    println!("\n  📥 Checking subscriber 1 (should get 1 notification):");
    let mut sub1_count = 0;
    while let Ok(Some(received)) = timeout(Duration::from_millis(500), receiver1.recv()).await {
        sub1_count += 1;
        println!(
            "    • Received: {}",
            String::from_utf8_lossy(&received.payload)
        );
    }
    println!("    Total: {} notifications", sub1_count);

    println!("\n  📥 Checking subscriber 2 (should get 1 notification):");
    let mut sub2_count = 0;
    while let Ok(Some(received)) = timeout(Duration::from_millis(500), receiver2.recv()).await {
        sub2_count += 1;
        println!(
            "    • Received: {}",
            String::from_utf8_lossy(&received.payload)
        );
    }
    println!("    Total: {} notifications", sub2_count);

    // Clean up
    notification_bus.unsubscribe(sub1_id).await?;
    notification_bus.unsubscribe(sub2_id).await?;

    Ok(())
}

async fn notification_types_demo(
    notification_bus: &agent_sync::coordinator::NotificationBus,
    publisher_id: AgentId,
    subscriber_id: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🏷️ === Notification Types Demo ===");

    // Subscribe to all notifications
    let (subscription_id, mut receiver) = notification_bus
        .subscribe(subscriber_id, NotificationFilter::default())
        .await?;

    // Demonstrate different notification types
    let task_id = Uuid::new_v4();

    let notification_types = vec![
        (
            "Task Notification",
            NotificationType::Task {
                task_id,
                action: "completed".to_string(),
            },
            b"Task processing completed successfully".to_vec(),
        ),
        (
            "Context Notification",
            NotificationType::Context {
                scope: "global".to_string(),
                key: "config".to_string(),
                action: "updated".to_string(),
            },
            br#"{"timeout": 60, "retries": 3}"#.to_vec(),
        ),
        (
            "Agent Status Notification",
            NotificationType::Agent {
                agent_id: publisher_id,
                status: "online".to_string(),
            },
            b"Agent came online".to_vec(),
        ),
        (
            "System Notification",
            NotificationType::System {
                component: "database".to_string(),
                level: "warning".to_string(),
            },
            b"High connection count detected".to_vec(),
        ),
        (
            "Custom Notification",
            NotificationType::Custom {
                category: "metrics".to_string(),
                data: {
                    let mut data = HashMap::new();
                    data.insert("cpu_usage".to_string(), "85%".to_string());
                    data.insert("memory_usage".to_string(), "72%".to_string());
                    data
                },
            },
            b"Performance metrics update".to_vec(),
        ),
    ];

    for (name, notification_type, payload) in notification_types {
        println!("  📤 Publishing {}", name);

        let notification = Notification::new(notification_type, payload, publisher_id)
            .with_metadata("demo".to_string(), "notification_types".to_string());

        notification_bus.publish(notification).await?;

        // Receive and display
        let received = timeout(Duration::from_secs(2), receiver.recv())
            .await?
            .expect("Channel closed");

        println!(
            "    📥 Received: {}",
            String::from_utf8_lossy(&received.payload)
        );
        println!("    📍 Channel: {}", received.channel_name("example"));
    }

    notification_bus.unsubscribe(subscription_id).await?;

    Ok(())
}

async fn priority_demo(
    notification_bus: &agent_sync::coordinator::NotificationBus,
    publisher_id: AgentId,
    subscriber_id: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⭐ === Priority Handling Demo ===");

    // Subscribe to all notifications
    let (subscription_id, mut receiver) = notification_bus
        .subscribe(subscriber_id, NotificationFilter::default())
        .await?;

    // Create notifications with different priorities
    let priorities = vec![
        (NotificationPriority::Critical, "System is down!"),
        (NotificationPriority::High, "High CPU usage detected"),
        (NotificationPriority::Normal, "Regular status update"),
        (NotificationPriority::Low, "Debug information"),
    ];

    println!("  📤 Publishing notifications with different priorities:");

    for (priority, message) in priorities {
        let notification = Notification::new(
            NotificationType::System {
                component: "monitor".to_string(),
                level: format!("{:?}", priority).to_lowercase(),
            },
            message.as_bytes().to_vec(),
            publisher_id,
        )
        .with_priority(priority);

        println!("    • {:?}: {}", priority, message);
        notification_bus.publish(notification).await?;
    }

    println!("\n  📥 Receiving notifications:");

    for _ in 0..4 {
        let received = timeout(Duration::from_secs(2), receiver.recv())
            .await?
            .expect("Channel closed");

        println!(
            "    • {:?}: {}",
            received.priority,
            String::from_utf8_lossy(&received.payload)
        );
    }

    notification_bus.unsubscribe(subscription_id).await?;

    Ok(())
}

async fn broadcast_demo(
    notification_bus: &agent_sync::coordinator::NotificationBus,
    publisher_id: AgentId,
    subscriber_ids: Vec<AgentId>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📡 === Broadcast Demo ===");

    // Subscribe all agents
    let mut subscriptions = Vec::new();
    let mut receivers = Vec::new();

    for (i, subscriber_id) in subscriber_ids.iter().enumerate() {
        let (subscription_id, receiver) = notification_bus
            .subscribe(*subscriber_id, NotificationFilter::default())
            .await?;

        subscriptions.push(subscription_id);
        receivers.push((i + 1, receiver));
        println!("  📝 Subscriber {} registered", i + 1);
    }

    // Broadcast a message
    let broadcast_message = Notification::new(
        NotificationType::System {
            component: "broadcast".to_string(),
            level: "info".to_string(),
        },
        b"This is a broadcast message to all subscribers".to_vec(),
        publisher_id,
    )
    .with_priority(NotificationPriority::High);

    println!("  📤 Broadcasting message...");
    let subscriber_count = notification_bus.publish(broadcast_message).await?;
    println!("  📊 Delivered to {} subscribers", subscriber_count);

    // Verify all subscribers received the message
    println!("  📥 Checking all subscribers:");
    for (index, mut receiver) in receivers {
        let received = timeout(Duration::from_secs(2), receiver.recv())
            .await?
            .expect("Channel closed");

        println!(
            "    • Subscriber {}: {}",
            index,
            String::from_utf8_lossy(&received.payload)
        );
    }

    // Clean up all subscriptions
    for subscription_id in subscriptions {
        notification_bus.unsubscribe(subscription_id).await?;
    }

    Ok(())
}

async fn system_notifications_demo(
    notification_bus: &agent_sync::coordinator::NotificationBus,
    publisher_id: AgentId,
    subscriber_id: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🖥️ === System Notifications Demo ===");

    // Subscribe to system notifications only
    let filter = NotificationFilter::new().with_types(vec![NotificationType::System {
        component: "".to_string(),
        level: "".to_string(),
    }]);

    let (subscription_id, mut receiver) = notification_bus.subscribe(subscriber_id, filter).await?;

    // Use the convenience method for system broadcasts
    let system_events = vec![
        ("auth", "info", "User authentication successful"),
        ("database", "warning", "Connection pool at 80% capacity"),
        ("cache", "error", "Redis connection lost"),
        ("api", "critical", "Service unavailable"),
    ];

    println!("  📤 Broadcasting system notifications:");

    for (component, level, message) in system_events {
        let subscriber_count = notification_bus
            .broadcast_system(component, level, message, publisher_id)
            .await?;

        println!(
            "    • {}/{}: {} (delivered to {} subscribers)",
            component, level, message, subscriber_count
        );

        // Receive the notification
        let received = timeout(Duration::from_secs(2), receiver.recv())
            .await?
            .expect("Channel closed");

        println!("      📥 Received priority: {:?}", received.priority);
    }

    notification_bus.unsubscribe(subscription_id).await?;

    Ok(())
}

async fn show_statistics(
    notification_bus: &agent_sync::coordinator::NotificationBus,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 === Final Statistics ===");

    let stats = notification_bus.get_stats().await;

    println!("  📈 Total Statistics:");
    println!("    • Notifications Sent: {}", stats.total_sent);
    println!("    • Notifications Received: {}", stats.total_received);
    println!("    • Active Subscribers: {}", stats.active_subscribers);

    if !stats.type_counts.is_empty() {
        println!("  📋 Notifications by Type:");
        for (notification_type, count) in stats.type_counts {
            println!("    • {}: {}", notification_type, count);
        }
    }

    if !stats.priority_counts.is_empty() {
        println!("  ⭐ Notifications by Priority:");
        for (priority_num, count) in stats.priority_counts {
            let priority_name = match priority_num {
                0 => "Low",
                1 => "Normal",
                2 => "High",
                3 => "Critical",
                _ => "Unknown",
            };
            println!("    • {}: {}", priority_name, count);
        }
    }

    // Test cleanup
    let (cleaned_subscribers, _cleaned_notifications) = notification_bus.cleanup().await?;
    if cleaned_subscribers > 0 {
        println!("  🧹 Cleaned up {} dead subscribers", cleaned_subscribers);
    }

    Ok(())
}

