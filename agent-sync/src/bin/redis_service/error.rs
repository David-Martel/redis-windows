//! Error types for the Redis Windows Service binary.

use thiserror::Error;

/// Service-specific error type
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Required file not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Failed to start Redis process
    #[error("Failed to start process: {0}")]
    ProcessStart(String),

    /// Windows Service Control Manager error
    #[cfg(windows)]
    #[error("Windows service error: {0}")]
    WindowsService(String),

    /// Configuration parsing error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Event log registration error
    #[error("Event log error: {0}")]
    EventLog(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
