//! Connection management with bb8 connection pooling.

use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::Client;

use crate::{Config, Error, Result};

/// Connection pool status information
#[derive(Debug, Clone)]
pub struct PoolStatus {
    /// Total connections in the pool (idle + active)
    pub size: u32,
    /// Number of idle connections
    pub idle: u32,
    /// Configured maximum pool size
    pub max_size: u32,
}

/// A Redis connection pool backed by bb8.
pub struct ConnectionPool {
    pool: Pool<RedisConnectionManager>,
    client: Client,
    config: Config,
}

impl ConnectionPool {
    /// Create a new connection pool from configuration.
    pub async fn new(config: &Config) -> Result<Self> {
        let manager = RedisConnectionManager::new(config.redis_url.as_str())
            .map_err(|e| Error::Pool(format!("Failed to create connection manager: {e}")))?;

        let pool = Pool::builder()
            .max_size(config.pool_size)
            .connection_timeout(config.connect_timeout)
            .build(manager)
            .await
            .map_err(|e| Error::Pool(format!("Failed to build connection pool: {e}")))?;

        let client = Client::open(config.redis_url.as_str())
            .map_err(|e| Error::connection(format!("Failed to create Redis client: {e}")))?;

        Ok(Self {
            pool,
            client,
            config: config.clone(),
        })
    }

    /// Get a pooled connection.
    pub async fn get(&self) -> Result<bb8::PooledConnection<'_, RedisConnectionManager>> {
        self.pool.get().await.map_err(Error::from)
    }

    /// Get a dedicated multiplexed connection (not from the pool).
    ///
    /// Useful for long-lived operations like pub/sub subscriptions.
    pub async fn get_multiplexed(&self) -> Result<redis::aio::MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::connection(format!("Failed to get multiplexed connection: {e}")))
    }

    /// Get a dedicated Client reference (for pub/sub).
    pub const fn client(&self) -> &Client {
        &self.client
    }

    /// Get the underlying config.
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Check pool health by running a PING command.
    pub async fn health_check(&self) -> Result<bool> {
        let mut conn = self.get().await?;
        let pong: String = redis::cmd("PING")
            .query_async(&mut *conn)
            .await
            .map_err(|e| Error::connection(format!("Health check failed: {e}")))?;
        Ok(pong == "PONG")
    }

    /// Get current pool status.
    pub fn status(&self) -> PoolStatus {
        let state = self.pool.state();
        PoolStatus {
            size: state.connections,
            idle: state.idle_connections,
            max_size: self.config.pool_size,
        }
    }
}


