//! Synchronization primitives

/// Sync manager
pub struct SyncManager;

/// Sync options
#[derive(Debug, Clone)]
pub struct SyncOptions {
    /// Timeout
    pub timeout: std::time::Duration,
}
