//! Configuration for agent-sync

use crate::error::Result;
use std::time::Duration;

/// Configuration for agent-sync
#[derive(Debug, Clone)]
pub struct Config {
    /// Redis URL
    pub redis_url: String,
    /// Connection timeout
    pub connect_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            connect_timeout: Duration::from_secs(5),
        }
    }
}

impl Config {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        if let Ok(url) = std::env::var("REDIS_URL") {
            config.redis_url = url;
        }

        Ok(config)
    }
}

/// Configuration builder
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    /// Set Redis URL
    pub fn redis_url<S: Into<String>>(mut self, url: S) -> Self {
        self.config.redis_url = url.into();
        self
    }

    /// Build the configuration
    pub fn build(self) -> Config {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
