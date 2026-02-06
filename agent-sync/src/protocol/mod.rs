//! Protocol definitions

/// Message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Register message
    Register,
    /// Unregister message
    Unregister,
    /// Heartbeat message
    Heartbeat,
}


