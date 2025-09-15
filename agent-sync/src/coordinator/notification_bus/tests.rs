//! Unit tests for notification bus module

use super::*;
use crate::testing::{MockRedisConnection, TestRedisServer};
use rstest::*;
use std::time::Duration;
use tokio::time::timeout;

#[fixture]
fn test_agent_id() -> AgentId {
    AgentId::new()
}

#[fixture]
fn test_notification(test_agent_id: AgentId) -> Notification {
    Notification::new(
        NotificationType::Task {
            task_id: Uuid::new_v4(),
            action: "started".to_string(),
        },
        b"test_payload".to_vec(),
        test_agent_id,
    )
}

#[fixture]
fn test_high_priority_notification(test_agent_id: AgentId) -> Notification {
    Notification::new(
        NotificationType::System {
            component: "test_component".to_string(),
            level: "critical".to_string(),
        },
        b"critical_payload".to_vec(),
        test_agent_id,
    )
    .with_priority(NotificationPriority::Critical)
}

#[fixture]
fn test_notification_filter() -> NotificationFilter {
    NotificationFilter::new().with_min_priority(NotificationPriority::Normal)
}

#[rstest]
fn test_notification_priority_ordering() {
    assert!(NotificationPriority::Critical > NotificationPriority::High);
    assert!(NotificationPriority::High > NotificationPriority::Normal);
    assert!(NotificationPriority::Normal > NotificationPriority::Low);

    // Test numeric values
    assert_eq!(NotificationPriority::Low as u8, 0);
    assert_eq!(NotificationPriority::Normal as u8, 1);
    assert_eq!(NotificationPriority::High as u8, 2);
    assert_eq!(NotificationPriority::Critical as u8, 3);
}

#[rstest]
fn test_notification_priority_default() {
    let default_priority = NotificationPriority::default();
    assert_eq!(default_priority, NotificationPriority::Normal);
}

#[rstest]
fn test_notification_creation(test_agent_id: AgentId) {
    let task_id = Uuid::new_v4();
    let notification = Notification::new(
        NotificationType::Task {
            task_id,
            action: "completed".to_string(),
        },
        b"test_payload".to_vec(),
        test_agent_id,
    );

    assert_eq!(notification.sender, test_agent_id);
    assert_eq!(notification.payload, b"test_payload");
    assert_eq!(notification.priority, NotificationPriority::Normal);
    assert!(notification.recipients.is_empty());
    assert!(notification.ttl_secs.is_none());
    assert!(notification.metadata.is_empty());
    assert!(!notification.is_expired());

    // Check notification type
    match notification.notification_type {
        NotificationType::Task {
            task_id: id,
            action,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(action, "completed");
        }
        _ => panic!("Expected Task notification type"),
    }
}

#[rstest]
fn test_notification_with_recipients(test_agent_id: AgentId) {
    let recipient1 = AgentId::new();
    let recipient2 = AgentId::new();
    let recipients = vec![recipient1, recipient2];

    let notification = Notification::new(
        NotificationType::Agent {
            agent_id: test_agent_id,
            status: "online".to_string(),
        },
        b"status_update".to_vec(),
        test_agent_id,
    )
    .with_recipients(recipients.clone());

    assert_eq!(notification.recipients, recipients);
}

#[rstest]
fn test_notification_with_priority(test_agent_id: AgentId) {
    let notification = Notification::new(
        NotificationType::System {
            component: "auth".to_string(),
            level: "error".to_string(),
        },
        b"auth_error".to_vec(),
        test_agent_id,
    )
    .with_priority(NotificationPriority::High);

    assert_eq!(notification.priority, NotificationPriority::High);
}

#[rstest]
fn test_notification_with_ttl(test_agent_id: AgentId) {
    let ttl = Duration::from_secs(300);
    let notification = Notification::new(
        NotificationType::Custom {
            category: "temp".to_string(),
            data: HashMap::new(),
        },
        b"temporary_data".to_vec(),
        test_agent_id,
    )
    .with_ttl(ttl);

    assert_eq!(notification.ttl_secs, Some(300));
}

#[rstest]
fn test_notification_with_metadata(test_agent_id: AgentId) {
    let notification = Notification::new(
        NotificationType::Context {
            scope: "global".to_string(),
            key: "config".to_string(),
            action: "updated".to_string(),
        },
        b"config_data".to_vec(),
        test_agent_id,
    )
    .with_metadata("version".to_string(), "1.2.3".to_string())
    .with_metadata("environment".to_string(), "production".to_string());

    assert_eq!(notification.metadata.len(), 2);
    assert_eq!(
        notification.metadata.get("version"),
        Some(&"1.2.3".to_string())
    );
    assert_eq!(
        notification.metadata.get("environment"),
        Some(&"production".to_string())
    );
}

#[rstest]
fn test_notification_expiry(test_agent_id: AgentId) {
    // Create a notification with a TTL that should not be expired
    let future_notification = Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "info".to_string(),
        },
        b"future_data".to_vec(),
        test_agent_id,
    )
    .with_ttl(Duration::from_secs(300));

    assert!(!future_notification.is_expired());

    // Create a notification that should be expired
    let mut expired_notification = Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "info".to_string(),
        },
        b"expired_data".to_vec(),
        test_agent_id,
    );

    // Set timestamp to 2 minutes ago and TTL to 1 minute
    expired_notification.timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 120; // 2 minutes ago
    expired_notification.ttl_secs = Some(60); // 1 minute TTL

    assert!(expired_notification.is_expired());

    // Notification without TTL should never be expired
    let no_ttl_notification = Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "info".to_string(),
        },
        b"no_ttl_data".to_vec(),
        test_agent_id,
    );

    assert!(!no_ttl_notification.is_expired());
}

#[rstest]
fn test_notification_channel_names(test_agent_id: AgentId) {
    let notifications = vec![
        (
            Notification::new(
                NotificationType::Task {
                    task_id: Uuid::new_v4(),
                    action: "started".to_string(),
                },
                vec![],
                test_agent_id,
            ),
            "agent_sync:notifications:task",
        ),
        (
            Notification::new(
                NotificationType::Context {
                    scope: "global".to_string(),
                    key: "config".to_string(),
                    action: "updated".to_string(),
                },
                vec![],
                test_agent_id,
            ),
            "agent_sync:notifications:context",
        ),
        (
            Notification::new(
                NotificationType::Agent {
                    agent_id: test_agent_id,
                    status: "online".to_string(),
                },
                vec![],
                test_agent_id,
            ),
            "agent_sync:notifications:agent",
        ),
        (
            Notification::new(
                NotificationType::System {
                    component: "auth".to_string(),
                    level: "info".to_string(),
                },
                vec![],
                test_agent_id,
            ),
            "agent_sync:notifications:system",
        ),
        (
            Notification::new(
                NotificationType::Custom {
                    category: "metrics".to_string(),
                    data: HashMap::new(),
                },
                vec![],
                test_agent_id,
            ),
            "agent_sync:notifications:custom:metrics",
        ),
    ];

    for (notification, expected_channel) in notifications {
        assert_eq!(notification.channel_name(), expected_channel);
    }
}

#[rstest]
fn test_notification_type_variants(test_agent_id: AgentId) {
    let task_id = Uuid::new_v4();

    // Test all NotificationType variants
    let task_notification = NotificationType::Task {
        task_id,
        action: "completed".to_string(),
    };

    let context_notification = NotificationType::Context {
        scope: "session".to_string(),
        key: "data".to_string(),
        action: "created".to_string(),
    };

    let agent_notification = NotificationType::Agent {
        agent_id: test_agent_id,
        status: "idle".to_string(),
    };

    let system_notification = NotificationType::System {
        component: "database".to_string(),
        level: "warning".to_string(),
    };

    let mut custom_data = HashMap::new();
    custom_data.insert("metric".to_string(), "cpu_usage".to_string());
    let custom_notification = NotificationType::Custom {
        category: "performance".to_string(),
        data: custom_data.clone(),
    };

    // Verify pattern matching works for all variants
    match task_notification {
        NotificationType::Task {
            task_id: id,
            action,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(action, "completed");
        }
        _ => panic!("Expected Task notification"),
    }

    match context_notification {
        NotificationType::Context { scope, key, action } => {
            assert_eq!(scope, "session");
            assert_eq!(key, "data");
            assert_eq!(action, "created");
        }
        _ => panic!("Expected Context notification"),
    }

    match agent_notification {
        NotificationType::Agent { agent_id, status } => {
            assert_eq!(agent_id, test_agent_id);
            assert_eq!(status, "idle");
        }
        _ => panic!("Expected Agent notification"),
    }

    match system_notification {
        NotificationType::System { component, level } => {
            assert_eq!(component, "database");
            assert_eq!(level, "warning");
        }
        _ => panic!("Expected System notification"),
    }

    match custom_notification {
        NotificationType::Custom { category, data } => {
            assert_eq!(category, "performance");
            assert_eq!(data, custom_data);
        }
        _ => panic!("Expected Custom notification"),
    }
}

#[rstest]
fn test_notification_filter_creation() {
    let sender = AgentId::new();
    let filter = NotificationFilter::new()
        .with_sender(sender)
        .with_min_priority(NotificationPriority::High)
        .with_metadata_filter("env".to_string(), "prod".to_string());

    assert_eq!(filter.sender, Some(sender));
    assert_eq!(filter.min_priority, Some(NotificationPriority::High));
    assert_eq!(filter.metadata_filters.len(), 1);
    assert_eq!(
        filter.metadata_filters.get("env"),
        Some(&"prod".to_string())
    );
}

#[rstest]
fn test_notification_filter_default() {
    let filter = NotificationFilter::default();

    assert!(filter.types.is_none());
    assert!(filter.sender.is_none());
    assert!(filter.min_priority.is_none());
    assert!(filter.metadata_filters.is_empty());
}

#[rstest]
fn test_notification_filter_matching(test_agent_id: AgentId) {
    let other_agent = AgentId::new();

    // Create a filter for high priority notifications from test_agent_id
    let filter = NotificationFilter::new()
        .with_sender(test_agent_id)
        .with_min_priority(NotificationPriority::High)
        .with_metadata_filter("category".to_string(), "important".to_string());

    // Notification that should match
    let matching_notification = Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "error".to_string(),
        },
        vec![],
        test_agent_id,
    )
    .with_priority(NotificationPriority::High)
    .with_metadata("category".to_string(), "important".to_string());

    assert!(filter.matches(&matching_notification));

    // Notification with wrong sender
    let wrong_sender_notification = Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "error".to_string(),
        },
        vec![],
        other_agent,
    )
    .with_priority(NotificationPriority::High)
    .with_metadata("category".to_string(), "important".to_string());

    assert!(!filter.matches(&wrong_sender_notification));

    // Notification with low priority
    let low_priority_notification = Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "info".to_string(),
        },
        vec![],
        test_agent_id,
    )
    .with_priority(NotificationPriority::Low)
    .with_metadata("category".to_string(), "important".to_string());

    assert!(!filter.matches(&low_priority_notification));

    // Notification with wrong metadata
    let wrong_metadata_notification = Notification::new(
        NotificationType::System {
            component: "test".to_string(),
            level: "error".to_string(),
        },
        vec![],
        test_agent_id,
    )
    .with_priority(NotificationPriority::High)
    .with_metadata("category".to_string(), "normal".to_string());

    assert!(!filter.matches(&wrong_metadata_notification));
}

#[rstest]
fn test_notification_filter_type_matching(test_agent_id: AgentId) {
    let task_type = NotificationType::Task {
        task_id: Uuid::new_v4(),
        action: "started".to_string(),
    };

    let system_type = NotificationType::System {
        component: "test".to_string(),
        level: "info".to_string(),
    };

    let filter = NotificationFilter::new().with_types(vec![task_type.clone()]);

    // Create notifications of different types
    let task_notification = Notification::new(
        NotificationType::Task {
            task_id: Uuid::new_v4(),
            action: "completed".to_string(),
        },
        vec![],
        test_agent_id,
    );

    let system_notification = Notification::new(system_type, vec![], test_agent_id);

    // Task notification should match
    assert!(filter.matches(&task_notification));

    // System notification should not match
    assert!(!filter.matches(&system_notification));
}

#[rstest]
fn test_subscriber_creation() {
    let agent_id = AgentId::new();
    let filter = NotificationFilter::new();
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

    let subscriber = Subscriber {
        id: Uuid::new_v4(),
        agent_id,
        filter: filter.clone(),
        sender,
        subscribed_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    assert_eq!(subscriber.agent_id, agent_id);
    // Note: We can't easily test filter equality due to the sender field
    assert!(subscriber.subscribed_at > 0);
}

#[rstest]
fn test_notification_stats_default() {
    let stats = NotificationStats::default();

    assert_eq!(stats.total_sent, 0);
    assert_eq!(stats.total_received, 0);
    assert_eq!(stats.active_subscribers, 0);
    assert!(stats.type_counts.is_empty());
    assert!(stats.priority_counts.is_empty());
}

#[rstest]
fn test_notification_serialization(test_notification: Notification) {
    let serialized =
        bincode::serialize(&test_notification).expect("Failed to serialize notification");

    let deserialized: Notification =
        bincode::deserialize(&serialized).expect("Failed to deserialize notification");

    assert_eq!(test_notification.id, deserialized.id);
    assert_eq!(test_notification.payload, deserialized.payload);
    assert_eq!(test_notification.sender, deserialized.sender);
    assert_eq!(test_notification.recipients, deserialized.recipients);
    assert_eq!(test_notification.timestamp, deserialized.timestamp);
    assert_eq!(test_notification.priority, deserialized.priority);
    assert_eq!(test_notification.ttl_secs, deserialized.ttl_secs);
    assert_eq!(test_notification.metadata, deserialized.metadata);

    // Check notification type discriminant (can't check full equality due to complex enum)
    assert_eq!(
        std::mem::discriminant(&test_notification.notification_type),
        std::mem::discriminant(&deserialized.notification_type)
    );
}

#[rstest]
fn test_notification_filter_serialization(test_notification_filter: NotificationFilter) {
    let serialized = bincode::serialize(&test_notification_filter)
        .expect("Failed to serialize notification filter");

    let deserialized: NotificationFilter =
        bincode::deserialize(&serialized).expect("Failed to deserialize notification filter");

    assert_eq!(test_notification_filter.sender, deserialized.sender);
    assert_eq!(
        test_notification_filter.min_priority,
        deserialized.min_priority
    );
    assert_eq!(
        test_notification_filter.metadata_filters,
        deserialized.metadata_filters
    );
}

#[rstest]
fn test_notification_priority_serialization() {
    let priorities = vec![
        NotificationPriority::Low,
        NotificationPriority::Normal,
        NotificationPriority::High,
        NotificationPriority::Critical,
    ];

    for priority in priorities {
        let serialized = bincode::serialize(&priority).expect("Failed to serialize priority");

        let deserialized: NotificationPriority =
            bincode::deserialize(&serialized).expect("Failed to deserialize priority");

        assert_eq!(priority, deserialized);
    }
}

// Property-based tests
#[cfg(feature = "proptest")]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_notification_id_uniqueness(
            notifications_count in 1..100usize
        ) {
            let agent_id = AgentId::new();
            let notifications: Vec<Notification> = (0..notifications_count)
                .map(|_| Notification::new(
                    NotificationType::System {
                        component: "test".to_string(),
                        level: "info".to_string(),
                    },
                    vec![],
                    agent_id,
                ))
                .collect();

            let mut ids = std::collections::HashSet::new();
            for notification in notifications {
                prop_assert!(ids.insert(notification.id));
            }
        }

        #[test]
        fn test_notification_priority_numeric_consistency(
            priority in 0u8..4u8
        ) {
            let notification_priority = match priority {
                0 => NotificationPriority::Low,
                1 => NotificationPriority::Normal,
                2 => NotificationPriority::High,
                3 => NotificationPriority::Critical,
                _ => unreachable!(),
            };

            prop_assert_eq!(notification_priority as u8, priority);
        }

        #[test]
        fn test_notification_ttl_expiry_logic(
            ttl_secs in 1u64..3600u64,
            age_secs in 0u64..7200u64
        ) {
            let agent_id = AgentId::new();
            let mut notification = Notification::new(
                NotificationType::System {
                    component: "test".to_string(),
                    level: "info".to_string(),
                },
                vec![],
                agent_id,
            );

            notification.ttl_secs = Some(ttl_secs);
            notification.timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - age_secs;

            let should_be_expired = age_secs > ttl_secs;
            prop_assert_eq!(notification.is_expired(), should_be_expired);
        }
    }
}

// Integration test helpers
mod integration_helpers {
    use super::*;

    pub async fn create_test_notification_bus() -> Result<NotificationBus> {
        NotificationBus::new("redis://localhost:6379").await
    }

    pub fn create_test_notifications(count: usize, sender: AgentId) -> Vec<Notification> {
        (0..count)
            .map(|i| {
                let priority = match i % 4 {
                    0 => NotificationPriority::Low,
                    1 => NotificationPriority::Normal,
                    2 => NotificationPriority::High,
                    3 => NotificationPriority::Critical,
                    _ => unreachable!(),
                };

                Notification::new(
                    NotificationType::Custom {
                        category: format!("test_{}", i),
                        data: HashMap::new(),
                    },
                    format!("payload_{}", i).into_bytes(),
                    sender,
                )
                .with_priority(priority)
            })
            .collect()
    }

    pub async fn test_notification_roundtrip(
        bus: &NotificationBus,
        notification: Notification,
        agent_id: AgentId,
    ) -> Result<()> {
        // Subscribe to notifications
        let (subscription_id, mut receiver) = bus
            .subscribe(agent_id, NotificationFilter::default())
            .await?;

        // Publish notification
        let subscriber_count = bus.publish(notification.clone()).await?;
        assert!(subscriber_count > 0);

        // Receive notification with timeout
        let received = timeout(Duration::from_secs(5), receiver.recv())
            .await
            .map_err(|_| Error::other("Timeout waiting for notification".to_string()))?
            .ok_or_else(|| Error::other("Channel closed".to_string()))?;

        // Verify notification
        assert_eq!(received.id, notification.id);
        assert_eq!(received.payload, notification.payload);

        // Cleanup
        bus.unsubscribe(subscription_id).await?;

        Ok(())
    }
}

// Mock tests
#[cfg(feature = "test-utilities")]
mod mock_tests {
    use super::*;
    use crate::testing::MockNotificationBus;

    #[rstest]
    #[tokio::test]
    async fn test_mock_notification_bus_publish() {
        let mut mock_bus = MockNotificationBus::new();
        let agent_id = AgentId::new();
        let notification = Notification::new(
            NotificationType::System {
                component: "test".to_string(),
                level: "info".to_string(),
            },
            b"test_payload".to_vec(),
            agent_id,
        );

        mock_bus
            .expect_publish()
            .with(mockall::predicate::always())
            .returning(|_| Box::pin(async { Ok(1) }));

        let result = mock_bus.publish(notification).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_mock_notification_bus_subscribe() {
        let mut mock_bus = MockNotificationBus::new();
        let agent_id = AgentId::new();
        let filter = NotificationFilter::default();
        let subscription_id = Uuid::new_v4();

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        mock_bus
            .expect_subscribe()
            .with(
                mockall::predicate::eq(agent_id),
                mockall::predicate::always(),
            )
            .returning(move |_, _| {
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                Box::pin(async move { Ok((subscription_id, rx)) })
            });

        let result = mock_bus.subscribe(agent_id, filter).await;
        assert!(result.is_ok());
        let (returned_id, _receiver) = result.unwrap();
        assert_eq!(returned_id, subscription_id);
    }
}
