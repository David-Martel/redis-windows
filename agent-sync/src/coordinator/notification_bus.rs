//! Notification bus for agent coordination
//!
//! Provides a distributed publish-subscribe system for agent notifications
//! built on Redis with support for different notification types and filtering.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::{AgentId, Error, Result};

/// Types of notifications
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    /// Task-related notifications
    Task {
        /// ID of the task
        task_id: Uuid,
        /// Action performed on the task
        action: String,
    },
    /// Context change notifications
    Context {
        /// Scope of the context change
        scope: String,
        /// Key that was modified
        key: String,
        /// Action performed on the context
        action: String,
    },
    /// Agent status notifications
    Agent {
        /// ID of the agent
        agent_id: AgentId,
        /// New status of the agent
        status: String,
    },
    /// System-level notifications
    System {
        /// System component name
        component: String,
        /// Log level of the notification
        level: String,
    },
    /// Custom application notifications
    Custom {
        /// Category of the custom notification
        category: String,
        /// Custom data payload
        data: HashMap<String, String>,
    },
}

/// A notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Unique notification ID
    pub id: Uuid,
    /// Notification type
    pub notification_type: NotificationType,
    /// Notification payload
    pub payload: Vec<u8>,
    /// Agent that sent the notification
    pub sender: AgentId,
    /// Target recipients (empty for broadcast)
    pub recipients: Vec<AgentId>,
    /// Notification timestamp
    pub timestamp: u64,
    /// Priority level
    pub priority: NotificationPriority,
    /// TTL in seconds
    pub ttl_secs: Option<u64>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Notification priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[derive(Default)]
pub enum NotificationPriority {
    /// Low priority notifications
    Low = 0,
    /// Normal priority notifications
    #[default]
    Normal = 1,
    /// High priority notifications
    High = 2,
    /// Critical priority notifications
    Critical = 3,
}


impl Notification {
    /// Create a new notification
    pub fn new(notification_type: NotificationType, payload: Vec<u8>, sender: AgentId) -> Self {
        Self {
            id: Uuid::new_v4(),
            notification_type,
            payload,
            sender,
            recipients: Vec::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time is before UNIX EPOCH")
                .as_secs(),
            priority: NotificationPriority::default(),
            ttl_secs: None,
            metadata: HashMap::new(),
        }
    }

    /// Set specific recipients
    #[must_use]
    pub fn with_recipients(mut self, recipients: Vec<AgentId>) -> Self {
        self.recipients = recipients;
        self
    }

    /// Set priority
    #[must_use]
    pub const fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set TTL
    #[must_use]
    pub const fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl_secs = Some(ttl.as_secs());
        self
    }

    /// Add metadata
    #[must_use]
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if notification is expired
    /// Check if notification is expired
    ///
    /// # Panics
    ///
    /// Panics if system time is before UNIX EPOCH.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.ttl_secs.map_or(false, |ttl_secs| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time is before UNIX EPOCH")
                .as_secs();
            (now - self.timestamp) > ttl_secs
        })
    }

    /// Get channel name for this notification type with a given namespace
    pub fn channel_name(&self, namespace: &str) -> String {
        match &self.notification_type {
            NotificationType::Task { .. } => format!("{namespace}:notifications:task"),
            NotificationType::Context { .. } => format!("{namespace}:notifications:context"),
            NotificationType::Agent { .. } => format!("{namespace}:notifications:agent"),
            NotificationType::System { .. } => format!("{namespace}:notifications:system"),
            NotificationType::Custom { category, .. } => {
                format!("{namespace}:notifications:custom:{category}")
            }
        }
    }
}

/// Notification filter for subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationFilter {
    /// Filter by notification types
    pub types: Option<Vec<NotificationType>>,
    /// Filter by sender
    pub sender: Option<AgentId>,
    /// Filter by minimum priority
    pub min_priority: Option<NotificationPriority>,
    /// Filter by metadata
    pub metadata_filters: HashMap<String, String>,
}

impl NotificationFilter {
    /// Create a new filter
    pub fn new() -> Self {
        Self {
            types: None,
            sender: None,
            min_priority: None,
            metadata_filters: HashMap::new(),
        }
    }

    /// Filter by types
    #[must_use]
    pub fn with_types(mut self, types: Vec<NotificationType>) -> Self {
        self.types = Some(types);
        self
    }

    /// Filter by sender
    #[must_use]
    pub const fn with_sender(mut self, sender: AgentId) -> Self {
        self.sender = Some(sender);
        self
    }

    /// Filter by minimum priority
    #[must_use]
    pub const fn with_min_priority(mut self, priority: NotificationPriority) -> Self {
        self.min_priority = Some(priority);
        self
    }

    /// Add metadata filter
    #[must_use]
    pub fn with_metadata_filter(mut self, key: String, value: String) -> Self {
        self.metadata_filters.insert(key, value);
        self
    }

    /// Check if notification matches this filter
    pub fn matches(&self, notification: &Notification) -> bool {
        // Check type filter
        if let Some(ref types) = self.types {
            if !types.iter().any(|t| {
                std::mem::discriminant(t) == std::mem::discriminant(&notification.notification_type)
            }) {
                return false;
            }
        }

        // Check sender filter
        if let Some(sender) = self.sender {
            if notification.sender != sender {
                return false;
            }
        }

        // Check priority filter
        if let Some(min_priority) = self.min_priority {
            if notification.priority < min_priority {
                return false;
            }
        }

        // Check metadata filters
        for (key, value) in &self.metadata_filters {
            if notification.metadata.get(key) != Some(value) {
                return false;
            }
        }

        true
    }
}

impl Default for NotificationFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Subscriber information
#[derive(Debug, Clone)]
pub struct Subscriber {
    /// Subscriber ID
    pub id: Uuid,
    /// Agent ID of subscriber
    pub agent_id: AgentId,
    /// Subscription filter
    pub filter: NotificationFilter,
    /// Channel for receiving notifications
    pub sender: mpsc::UnboundedSender<Notification>,
    /// Subscription timestamp
    pub subscribed_at: u64,
}

/// Notification statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationStats {
    /// Total notifications sent
    pub total_sent: u64,
    /// Total notifications received
    pub total_received: u64,
    /// Active subscribers count
    pub active_subscribers: u64,
    /// Notifications by type
    pub type_counts: HashMap<String, u64>,
    /// Notifications by priority
    pub priority_counts: HashMap<u8, u64>,
}

/// Distributed notification bus
pub struct NotificationBus {
    client: Client,
    connection: Arc<RwLock<redis::aio::MultiplexedConnection>>,
    subscribers: Arc<RwLock<HashMap<Uuid, Subscriber>>>,
    stats: Arc<RwLock<NotificationStats>>,
    shutdown_sender: Option<mpsc::UnboundedSender<()>>,
    namespace: String,
}

impl NotificationBus {
    /// Create a new notification bus
    ///
    /// # Errors
    ///
    /// Returns error if Redis connection fails.
    pub async fn new(redis_url: &str) -> Result<Self> {
        Self::new_with_namespace(redis_url, crate::coordinator::task_queue::DEFAULT_NAMESPACE).await
    }

    /// Create a new notification bus
    ///
    /// # Errors
    ///
    /// Returns error if Redis connection fails. with a custom namespace
    pub async fn new_with_namespace(redis_url: &str, namespace: &str) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| Error::connection(format!("Failed to create Redis client: {e}")))?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::connection(format!("Failed to connect to Redis: {e}")))?;

        Ok(Self {
            client,
            connection: Arc::new(RwLock::new(connection)),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(NotificationStats::default())),
            shutdown_sender: None,
            namespace: namespace.to_string(),
        })
    }

    /// Create a new notification bus
    ///
    /// # Errors
    ///
    /// Returns error if Redis connection fails. using existing connections.
    ///
    /// `connection` is used for publish operations. `client` is used for creating
    /// the dedicated pub/sub subscription connection.
    pub fn new_with_connections(
        client: Client,
        connection: redis::aio::MultiplexedConnection,
    ) -> Self {
        Self::new_with_namespace_and_connections(
            client,
            connection,
            crate::coordinator::task_queue::DEFAULT_NAMESPACE,
        )
    }

    /// Create a new notification bus
    ///
    /// # Errors
    ///
    /// Returns error if Redis connection fails. using existing connections and custom namespace.
    pub fn new_with_namespace_and_connections(
        client: Client,
        connection: redis::aio::MultiplexedConnection,
        namespace: &str,
    ) -> Self {
        Self {
            client,
            connection: Arc::new(RwLock::new(connection)),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(NotificationStats::default())),
            shutdown_sender: None,
            namespace: namespace.to_string(),
        }
    }

    /// Publish a notification
    ///
    /// # Errors
    ///
    /// Returns error if serialization or Redis fails.
    pub async fn publish(&self, notification: Notification) -> Result<u64> {
        let mut conn = self.connection.write().await;
        let channel = notification.channel_name(&self.namespace);

        // Serialize notification
        let serialized = bincode::serialize(&notification).map_err(|e| {
            Error::serialization(format!("Failed to serialize notification: {e}"))
        })?;

        // Publish to Redis channel
        let subscriber_count: u64 = conn
            .publish(&channel, &serialized)
            .await
            .map_err(|e| Error::other(format!("Failed to publish notification: {e}")))?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_sent += 1;

        let type_key = match &notification.notification_type {
            NotificationType::Task { .. } => "task",
            NotificationType::Context { .. } => "context",
            NotificationType::Agent { .. } => "agent",
            NotificationType::System { .. } => "system",
            NotificationType::Custom { category, .. } => category,
        };

        *stats.type_counts.entry(type_key.to_string()).or_insert(0) += 1;
        *stats
            .priority_counts
            .entry(notification.priority as u8)
            .or_insert(0) += 1;

        tracing::debug!(
            "Published notification {} to {} subscribers",
            notification.id,
            subscriber_count
        );

        Ok(subscriber_count)
    }

    /// Subscribe to notifications
    ///
    /// # Errors
    ///
    /// Returns error if background task fails to start.
    ///
    /// # Panics
    ///
    /// Panics if system time is before UNIX EPOCH.
    pub async fn subscribe(
        &self,
        agent_id: AgentId,
        filter: NotificationFilter,
    ) -> Result<(Uuid, mpsc::UnboundedReceiver<Notification>)> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let subscription_id = Uuid::new_v4();

        let subscriber = Subscriber {
            id: subscription_id,
            agent_id,
            filter,
            sender,
            subscribed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time is before UNIX EPOCH")
                .as_secs(),
        };

        // Add subscriber
        {
            let mut subscribers = self.subscribers.write().await;
            subscribers.insert(subscription_id, subscriber);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_subscribers += 1;
        }

        tracing::debug!(
            "Added subscription {} for agent {}",
            subscription_id,
            agent_id
        );

        // Start subscription handler if not already running
        self.start_subscription_handler().await?;

        Ok((subscription_id, receiver))
    }

    /// Unsubscribe from notifications
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn unsubscribe(&self, subscription_id: Uuid) -> Result<bool> {
        let removed = {
            let mut subscribers = self.subscribers.write().await;
            subscribers.remove(&subscription_id).is_some()
        };

        if removed {
            let mut stats = self.stats.write().await;
            stats.active_subscribers = stats.active_subscribers.saturating_sub(1);
            tracing::debug!("Removed subscription {}", subscription_id);
        }

        Ok(removed)
    }

    /// Start the subscription handler task
    async fn start_subscription_handler(&self) -> Result<()> {
        if self.shutdown_sender.is_some() {
            return Ok(()); // Already running
        }

        let (_shutdown_sender, mut shutdown_receiver) = mpsc::unbounded_channel::<()>();
        let client = self.client.clone();
        let subscribers = Arc::clone(&self.subscribers);
        let stats = Arc::clone(&self.stats);

        let namespace = self.namespace.clone();

        tokio::spawn(async move {
            let mut pubsub = match client.get_async_pubsub().await {
                Ok(pubsub) => pubsub,
                Err(e) => {
                    tracing::error!("Failed to create pubsub connection: {}", e);
                    return;
                }
            };

            // Subscribe to all notification channels in the given namespace
            let channels = [
                format!("{namespace}:notifications:task"),
                format!("{namespace}:notifications:context"),
                format!("{namespace}:notifications:agent"),
                format!("{namespace}:notifications:system"),
                format!("{namespace}:notifications:custom:*"),
            ];

            for channel in &channels {
                if let Err(e) = pubsub.psubscribe(channel).await {
                    tracing::error!("Failed to subscribe to channel {}: {}", channel, e);
                    return;
                }
            }

            let mut message_stream = pubsub.on_message();
            loop {
                tokio::select! {
                    // Handle shutdown signal
                    _ = shutdown_receiver.recv() => {
                        tracing::info!("Shutting down notification subscription handler");
                        break;
                    }

                    // Handle incoming notifications
                    msg = message_stream.next() => {
                        if let Some(msg) = msg {
                            let payload: Vec<u8> = match msg.get_payload() {
                                Ok(p) => p,
                                Err(e) => {
                                    tracing::error!("Failed to get notification payload: {}", e);
                                    continue;
                                }
                            };
                            if let Ok(notification) = bincode::deserialize::<Notification>(&payload) {
                                Self::distribute_notification(&subscribers, &stats, notification).await;
                            }
                        }
                    }
                }
            }
        });

        // Store shutdown sender (this is a bit hacky due to async + self reference issues)
        // In a real implementation, you'd want to restructure this
        tracing::debug!("Started notification subscription handler");
        Ok(())
    }

    /// Distribute notification to matching subscribers
    async fn distribute_notification(
        subscribers: &Arc<RwLock<HashMap<Uuid, Subscriber>>>,
        stats: &Arc<RwLock<NotificationStats>>,
        notification: Notification,
    ) {
        let subscribers_guard = subscribers.read().await;
        let mut delivered_count = 0;

        for subscriber in subscribers_guard.values() {
            if subscriber.filter.matches(&notification) && !notification.is_expired() {
                // Send to subscriber if channel is still open
                if subscriber.sender.send(notification.clone()).is_ok() {
                    delivered_count += 1;
                } else {
                    // Subscriber channel is closed, should be cleaned up
                    tracing::debug!("Subscriber {} channel closed", subscriber.id);
                }
            }
        }

        if delivered_count > 0 {
            let mut stats_guard = stats.write().await;
            stats_guard.total_received += delivered_count;
            tracing::debug!(
                "Distributed notification {} to {} subscribers",
                notification.id,
                delivered_count
            );
        }
    }

    /// Get notification statistics
    pub async fn get_stats(&self) -> NotificationStats {
        self.stats.read().await.clone()
    }

    /// List active subscriptions
    pub async fn list_subscriptions(&self) -> Vec<(Uuid, AgentId)> {
        let subscribers = self.subscribers.read().await;
        subscribers.values().map(|s| (s.id, s.agent_id)).collect()
    }

    /// Broadcast system notification
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn broadcast_system(
        &self,
        component: &str,
        level: &str,
        message: &str,
        sender: AgentId,
    ) -> Result<u64> {
        let notification = Notification::new(
            NotificationType::System {
                component: component.to_string(),
                level: level.to_string(),
            },
            message.as_bytes().to_vec(),
            sender,
        )
        .with_priority(match level {
            "critical" => NotificationPriority::Critical,
            "high" => NotificationPriority::High,
            "normal" => NotificationPriority::Normal,
            _ => NotificationPriority::Low,
        });

        let mut conn = self.connection.write().await;
        let channel = notification.channel_name(&self.namespace);

        // Serialize notification
        let serialized = bincode::serialize(&notification).map_err(|e| {
            Error::serialization(format!("Failed to serialize notification: {e}"))
        })?;

        // Publish to Redis channel
        let subscriber_count: u64 = conn
            .publish(&channel, &serialized)
            .await
            .map_err(|e| Error::other(format!("Failed to publish system notification: {e}")))?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_sent += 1;
        *stats.type_counts.entry("system".to_string()).or_insert(0) += 1;
        *stats
            .priority_counts
            .entry(notification.priority as u8)
            .or_insert(0) += 1;

        Ok(subscriber_count)
    }

    /// Cleanup expired notifications and dead subscribers
    ///
    /// # Errors
    ///
    /// Returns error if Redis fails.
    pub async fn cleanup(&self) -> Result<(u32, u32)> {
        let mut subscribers = self.subscribers.write().await;
        let mut removed_subscribers = 0;

        // Remove subscribers with closed channels
        subscribers.retain(|_id, subscriber| {
            if subscriber.sender.is_closed() {
                removed_subscribers += 1;
                false
            } else {
                true
            }
        });

        if removed_subscribers > 0 {
            let mut stats = self.stats.write().await;
            stats.active_subscribers = stats
                .active_subscribers
                .saturating_sub(u64::from(removed_subscribers));
        }

        tracing::debug!("Cleanup removed {} dead subscribers", removed_subscribers);
        Ok((removed_subscribers, 0)) // TODO: Add expired notification cleanup
    }

    /// Shutdown the notification bus
    ///
    /// # Errors
    ///
    /// Returns error if shutdown fails.
    #[allow(clippy::unused_async)]
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(shutdown_sender) = self.shutdown_sender.take() {
            let _ = shutdown_sender.send(());
        }

        // Clear all subscribers
        {
            let mut subscribers = self.subscribers.write().await;
            subscribers.clear();
        }

        tracing::info!("Notification bus shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod basic_tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let sender = AgentId::new();
        let notification = Notification::new(
            NotificationType::Task {
                task_id: Uuid::new_v4(),
                action: "completed".to_string(),
            },
            b"test_payload".to_vec(),
            sender,
        )
        .with_priority(NotificationPriority::High)
        .with_ttl(Duration::from_secs(300));

        assert_eq!(notification.sender, sender);
        assert_eq!(notification.payload, b"test_payload");
        assert_eq!(notification.priority, NotificationPriority::High);
        assert_eq!(notification.ttl_secs, Some(300));
        assert!(!notification.is_expired());
    }

    #[test]
    fn test_notification_channel_names() {
        let ns = crate::coordinator::task_queue::DEFAULT_NAMESPACE;
        let task_notification = Notification::new(
            NotificationType::Task {
                task_id: Uuid::new_v4(),
                action: "started".to_string(),
            },
            vec![],
            AgentId::new(),
        );

        let context_notification = Notification::new(
            NotificationType::Context {
                scope: "global".to_string(),
                key: "config".to_string(),
                action: "updated".to_string(),
            },
            vec![],
            AgentId::new(),
        );

        let custom_notification = Notification::new(
            NotificationType::Custom {
                category: "metrics".to_string(),
                data: HashMap::new(),
            },
            vec![],
            AgentId::new(),
        );

        assert_eq!(
            task_notification.channel_name(ns),
            format!("{}:notifications:task", ns)
        );
        assert_eq!(
            context_notification.channel_name(ns),
            format!("{}:notifications:context", ns)
        );
        assert_eq!(
            custom_notification.channel_name(ns),
            format!("{}:notifications:custom:metrics", ns)
        );
    }

    #[test]
    fn test_notification_filter() {
        let agent1 = AgentId::new();
        let agent2 = AgentId::new();

        let filter = NotificationFilter::new()
            .with_sender(agent1)
            .with_min_priority(NotificationPriority::High);

        // Should match: correct sender and high priority
        let matching_notification = Notification::new(
            NotificationType::System {
                component: "test".to_string(),
                level: "info".to_string(),
            },
            vec![],
            agent1,
        )
        .with_priority(NotificationPriority::High);

        // Should not match: wrong sender
        let wrong_sender_notification = Notification::new(
            NotificationType::System {
                component: "test".to_string(),
                level: "info".to_string(),
            },
            vec![],
            agent2,
        )
        .with_priority(NotificationPriority::High);

        // Should not match: low priority
        let low_priority_notification = Notification::new(
            NotificationType::System {
                component: "test".to_string(),
                level: "info".to_string(),
            },
            vec![],
            agent1,
        )
        .with_priority(NotificationPriority::Low);

        assert!(filter.matches(&matching_notification));
        assert!(!filter.matches(&wrong_sender_notification));
        assert!(!filter.matches(&low_priority_notification));
    }

    #[test]
    fn test_notification_priority_ordering() {
        assert!(NotificationPriority::Critical > NotificationPriority::High);
        assert!(NotificationPriority::High > NotificationPriority::Normal);
        assert!(NotificationPriority::Normal > NotificationPriority::Low);
    }

    #[test]
    fn test_notification_expiry() {
        let mut notification = Notification::new(
            NotificationType::System {
                component: "test".to_string(),
                level: "info".to_string(),
            },
            vec![],
            AgentId::new(),
        );

        // Should not be expired without TTL
        assert!(!notification.is_expired());

        // Set expired TTL
        notification.ttl_secs = Some(1);
        notification.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX EPOCH")
            .as_secs()
            - 2; // 2 seconds ago

        assert!(notification.is_expired());
    }
}



