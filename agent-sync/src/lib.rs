//! # Agent Sync - High-Performance Redis Agent Synchronization Library
//!
//! Agent Sync is a high-performance, cross-platform Rust library designed for synchronizing
//! LLM agents through Redis. It provides a robust foundation for agent communication,
//! state management, and coordination across Windows and WSL environments.

#![deny(missing_docs, unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc, clippy::module_name_repetitions)]
#![allow(clippy::module_name_repetitions, clippy::must_use_candidate)]

// Re-export key types from dependencies for convenience
pub use redis::{RedisError, RedisResult};
pub use uuid::Uuid;

// Core modules
pub mod connection;
pub mod coordinator;
pub mod error;
pub mod platform;
pub mod protocol;
pub mod sync;
pub mod utils;

// Configuration module
mod config;
pub use config::{Config, ConfigBuilder};

// Main library interface
mod client;
pub use client::AgentSync;

// Core types and traits
mod types;
pub use types::{AgentId, AgentInfo, AgentStatus, MessageId, SessionId, SyncResult, Timestamp};

// Re-export common result type
pub use error::{Error, Result};

// Re-export connection pool
pub use connection::{ConnectionPool, PoolStatus};

// Re-export agent registry types
pub use coordinator::{AgentCapabilities, AgentRegistry, RegisteredAgent};

// Feature-gated exports
#[cfg(feature = "serde-support")]
pub mod serde_support;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(feature = "test-utilities")]
pub mod testing;

// Version information
/// The version of the agent-sync crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The name of the agent-sync crate
pub const NAME: &str = env!("CARGO_PKG_NAME");
/// The description of the agent-sync crate
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Initialize the agent-sync library with default configuration
/// Initialize the library
///
/// # Errors
///
/// Returns error if initialization fails.
///
/// # Panics
///
/// Panics if tracing initialization fails.
pub async fn init() -> Result<()> {
    // Initialize tracing
    #[cfg(not(test))]
    {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "agent_sync=info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .try_init()
            .expect("Failed to initialize tracing subscriber");
    }

    // Initialize platform-specific components
    platform::init().await?;

    tracing::info!("Agent-sync library initialized successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constants() {
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "agent-sync");
        assert!(!DESCRIPTION.is_empty());
    }

    #[tokio::test]
    async fn test_init() {
        assert!(init().await.is_ok());
    }
}




