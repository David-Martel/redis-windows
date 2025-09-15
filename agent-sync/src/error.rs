//! Error types for agent-sync

use thiserror::Error as ThiserrorError;

/// The main result type used throughout the agent-sync library
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for agent-sync
#[derive(ThiserrorError, Debug)]
pub enum Error {
    /// Redis error
    #[error("Redis error: {0}")]
    Redis(redis::RedisError),
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    /// Other error
    #[error("Error: {0}")]
    Other(String),
}

impl Error {
    /// Create a config error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }

    /// Create a connection error
    pub fn connection<S: Into<String>>(msg: S) -> Self {
        Self::Connection(msg.into())
    }

    /// Create a serialization error
    pub fn serialization<S: Into<String>>(msg: S) -> Self {
        Self::Serialization(msg.into())
    }

    /// Create a generic error
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self::Other(msg.into())
    }
}

impl From<redis::RedisError> for Error {
    fn from(err: redis::RedisError) -> Self {
        Self::Redis(err)
    }
}
