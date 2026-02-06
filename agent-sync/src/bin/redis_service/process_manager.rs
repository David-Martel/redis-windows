//! Redis process lifecycle manager.
//!
//! Handles spawning redis-server.exe, monitoring its health, and performing
//! graceful shutdown via `redis-cli SHUTDOWN`.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use super::error::ServiceError;
use super::path_convert;

/// Manages the redis-server child process.
pub struct ProcessManager {
    child: Child,
    port: u16,
    redis_cli_path: PathBuf,
}

impl ProcessManager {
    /// Start redis-server.exe with the given config file.
    ///
    /// Resolves the path to redis-server.exe relative to `base_dir`, converts
    /// paths for MSYS2 compatibility, and spawns the process.
    pub async fn start(base_dir: &Path, config_path: &Path) -> Result<Self, ServiceError> {
        let redis_server = base_dir.join("redis-server.exe");
        let redis_cli = base_dir.join("redis-cli.exe");

        if !redis_server.exists() {
            return Err(ServiceError::FileNotFound(
                redis_server.display().to_string(),
            ));
        }
        if !config_path.exists() {
            return Err(ServiceError::FileNotFound(
                config_path.display().to_string(),
            ));
        }

        // Parse port from config file
        let port = parse_port_from_config(config_path).unwrap_or(6380);

        // Convert config path for MSYS2/Cygwin redis-server
        let config_cygdrive = path_convert::to_cygdrive(&config_path.to_string_lossy());

        tracing::info!(
            "Starting redis-server: {} {}",
            redis_server.display(),
            config_cygdrive
        );

        let mut child = Command::new(&redis_server)
            .arg(&config_cygdrive)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| ServiceError::ProcessStart(e.to_string()))?;

        // Spawn log forwarding tasks for stdout and stderr
        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::info!(target: "redis", "{}", line);
                }
            });
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::warn!(target: "redis", "{}", line);
                }
            });
        }

        Ok(Self {
            child,
            port,
            redis_cli_path: redis_cli,
        })
    }

    /// Check if the child process has exited.
    pub fn has_exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    /// Get the exit code if the process has exited.
    pub fn exit_code(&mut self) -> Option<i32> {
        self.child.try_wait().ok().flatten().and_then(|s| s.code())
    }

    /// Attempt graceful shutdown via `redis-cli -p <port> SHUTDOWN`, then kill.
    pub async fn stop(&mut self, timeout: std::time::Duration) -> Result<(), ServiceError> {
        // Try graceful shutdown with redis-cli
        if self.redis_cli_path.exists() {
            tracing::info!("Sending SHUTDOWN via redis-cli -p {}", self.port);
            let _ = Command::new(&self.redis_cli_path)
                .args(["-p", &self.port.to_string(), "SHUTDOWN"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        }

        // Wait for graceful exit
        match tokio::time::timeout(timeout, self.child.wait()).await {
            Ok(Ok(status)) => {
                tracing::info!("Redis exited gracefully: {}", status);
                return Ok(());
            }
            Ok(Err(e)) => {
                tracing::warn!("Error waiting for Redis: {}", e);
            }
            Err(_) => {
                tracing::warn!("Timeout waiting for graceful shutdown, killing process");
            }
        }

        // Force kill
        if let Err(e) = self.child.kill().await {
            tracing::error!("Failed to kill redis-server: {}", e);
        }

        Ok(())
    }

    /// Get the configured Redis port.
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Parse the `port` directive from a Redis config file.
fn parse_port_from_config(path: &Path) -> Option<u16> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("port ") {
            let value = trimmed.strip_prefix("port ")?.trim();
            return value.parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_port_from_config() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_redis_port.conf");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "# Redis config").unwrap();
        writeln!(f, "port 6380").unwrap();
        writeln!(f, "bind 127.0.0.1").unwrap();
        drop(f);

        assert_eq!(parse_port_from_config(&path), Some(6380));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_port_missing() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_redis_noport.conf");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "bind 127.0.0.1").unwrap();
        drop(f);

        assert_eq!(parse_port_from_config(&path), None);
        std::fs::remove_file(&path).ok();
    }
}


