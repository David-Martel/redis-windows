//! Configuration for agent-sync

use crate::error::Result;
use crate::Error;
use std::time::Duration;

/// Configuration for agent-sync
#[derive(Debug, Clone)]
pub struct Config {
    /// Redis URL
    pub redis_url: String,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Connection pool size
    pub pool_size: u32,
    /// Optional namespace prefix for all Redis keys
    pub namespace: Option<String>,
    /// Agent heartbeat interval
    pub agent_heartbeat_interval: Duration,
    /// Agent expiry duration (agent considered dead after this)
    pub agent_expiry: Duration,
    /// Maximum number of retries for operations
    pub max_retries: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6380".to_string(),
            connect_timeout: Duration::from_secs(5),
            pool_size: 4,
            namespace: None,
            agent_heartbeat_interval: Duration::from_secs(30),
            agent_expiry: Duration::from_secs(90),
            max_retries: 3,
        }
    }
}

impl Config {
    /// Create configuration from environment variables
    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns error if environment variables are invalid.
pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        if let Ok(url) = std::env::var("REDIS_URL") {
            config.redis_url = url;
        }
        if let Ok(pool) = std::env::var("AGENT_SYNC_POOL_SIZE") {
            if let Ok(size) = pool.parse() {
                config.pool_size = size;
            }
        }
        if let Ok(ns) = std::env::var("AGENT_SYNC_NAMESPACE") {
            config.namespace = Some(ns);
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid.
pub fn validate(&self) -> Result<()> {
        if self.redis_url.is_empty() {
            return Err(Error::config("redis_url must not be empty"));
        }
        if !self.redis_url.starts_with("redis://") && !self.redis_url.starts_with("rediss://") {
            return Err(Error::config(
                "redis_url must start with redis:// or rediss://",
            ));
        }
        if self.pool_size == 0 {
            return Err(Error::config("pool_size must be at least 1"));
        }
        if self.agent_expiry <= self.agent_heartbeat_interval {
            return Err(Error::config(
                "agent_expiry must be greater than agent_heartbeat_interval",
            ));
        }
        Ok(())
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
    #[must_use]
    pub fn redis_url<S: Into<String>>(mut self, url: S) -> Self {
        self.config.redis_url = url.into();
        self
    }

    /// Set connection timeout
    #[must_use]
    pub const fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Set connection pool size
    #[must_use]
    pub const fn pool_size(mut self, size: u32) -> Self {
        self.config.pool_size = size;
        self
    }

    /// Set namespace prefix
    #[must_use]
    pub fn namespace<S: Into<String>>(mut self, ns: S) -> Self {
        self.config.namespace = Some(ns.into());
        self
    }

    /// Set agent heartbeat interval
    #[must_use]
    pub const fn agent_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.config.agent_heartbeat_interval = interval;
        self
    }

    /// Set agent expiry duration
    #[must_use]
    pub const fn agent_expiry(mut self, expiry: Duration) -> Self {
        self.config.agent_expiry = expiry;
        self
    }

    /// Set max retries
    #[must_use]
    pub const fn max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = retries;
        self
    }

    /// Build the configuration, validating all fields
    /// Build the configuration
    ///
    /// # Errors
    ///
    /// Returns error if validation fails.
pub fn build(self) -> Result<Config> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.redis_url, "redis://localhost:6380");
        assert_eq!(config.pool_size, 4);
        assert_eq!(config.agent_heartbeat_interval, Duration::from_secs(30));
        assert_eq!(config.agent_expiry, Duration::from_secs(90));
        assert_eq!(config.max_retries, 3);
        assert!(config.namespace.is_none());
    }

    #[test]
    fn test_config_validation_ok() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_url() {
        let config = Config {
            redis_url: String::new(),
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_bad_scheme() {
        let config = Config {
            redis_url: "http://localhost:6380".to_string(),
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_pool() {
        let config = Config {
            pool_size: 0,
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_expiry_less_than_heartbeat() {
        let config = Config {
            agent_heartbeat_interval: Duration::from_secs(90),
            agent_expiry: Duration::from_secs(30),
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_builder() {
        let config = ConfigBuilder::new()
            .redis_url("redis://myhost:6380")
            .pool_size(8)
            .namespace("test")
            .max_retries(5)
            .build()
            .unwrap();
        assert_eq!(config.redis_url, "redis://myhost:6380");
        assert_eq!(config.pool_size, 8);
        assert_eq!(config.namespace, Some("test".to_string()));
        assert_eq!(config.max_retries, 5);
    }
}



