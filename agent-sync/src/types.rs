//! Core types for agent-sync

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

/// Agent identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct AgentId(Uuid);

impl AgentId {
    /// Generate a new random agent ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct SessionId(Uuid);

impl SessionId {
    /// Generate a new random session ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct MessageId(Uuid);

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageId {
    /// Generate a new random message ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Timestamp wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Get the current timestamp
    pub fn now() -> Self {
        Self(Utc::now())
    }
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Default)]
pub enum AgentStatus {
    /// Agent is initializing
    #[default]
    Initializing,
    /// Agent is active
    Active,
    /// Agent is idle
    Idle,
    /// Agent is busy
    Busy,
    /// Agent is in error state
    Error,
    /// Agent is shutting down
    Shutting,
    /// Agent has shut down
    Shutdown,
}


/// Agent information
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct AgentInfo {
    /// Agent identifier
    pub id: AgentId,
    /// Agent name
    pub name: String,
    /// Current status
    pub status: AgentStatus,
    /// Session ID if in a session
    pub session_id: Option<SessionId>,
    /// Registration timestamp
    pub registered_at: Timestamp,
    /// Last activity timestamp
    pub last_seen: Timestamp,
    /// Agent metadata
    pub metadata: HashMap<String, String>,
}

impl AgentInfo {
    /// Create new agent info
    pub fn new(id: AgentId, name: String) -> Self {
        let now = Timestamp::now();
        Self {
            id,
            name,
            status: AgentStatus::default(),
            session_id: None,
            registered_at: now,
            last_seen: now,
            metadata: HashMap::new(),
        }
    }
}

/// Synchronization result
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct SyncResult {
    /// Operation was successful
    pub success: bool,
    /// Operation timestamp
    pub timestamp: Timestamp,
    /// Number of agents synchronized
    pub agents_synced: u32,
    /// Operation duration in milliseconds
    pub duration_ms: u64,
}


