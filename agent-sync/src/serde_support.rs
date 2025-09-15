//! Serde support for agent-sync types

use crate::{AgentInfo, Result};

/// Serialize an `AgentInfo` to JSON
pub fn serialize_agent_info(info: &AgentInfo) -> Result<String> {
    serde_json::to_string(info)
        .map_err(|e| crate::Error::other(format!("Serialization error: {e}")))
}
