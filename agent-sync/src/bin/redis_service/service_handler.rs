//! Windows Service Control Manager (SCM) integration.
//!
//! Handles service lifecycle: registration with the SCM, responding to
//! control events (Stop, Interrogate), and monitoring the Redis process.

use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::mpsc;

use super::error::ServiceError;
use super::process_manager::ProcessManager;

/// Run the Redis service logic (used in both console and SCM modes).
///
/// `shutdown_rx` receives a signal when the service should stop (Ctrl+C in
/// console mode, or SCM Stop in service mode).
pub async fn run_service(
    base_dir: PathBuf,
    config_path: PathBuf,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> Result<(), ServiceError> {
    tracing::info!("Starting Redis process");

    let mut pm = ProcessManager::start(&base_dir, &config_path).await?;
    tracing::info!("Redis started on port {}", pm.port());

    // Report running state (service mode will override this)
    tracing::info!("Service is now running");

    // Monitor loop: check process health + shutdown signal
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                tracing::info!("Shutdown signal received");
                break;
            }
            _ = interval.tick() => {
                if pm.has_exited() {
                    let code = pm.exit_code().unwrap_or(-1);
                    tracing::error!("Redis process exited unexpectedly with code {}", code);
                    return Err(ServiceError::ProcessStart(
                        format!("Redis exited with code {}", code),
                    ));
                }
            }
        }
    }

    // Graceful shutdown
    tracing::info!("Stopping Redis process");
    pm.stop(Duration::from_secs(10)).await?;
    tracing::info!("Redis process stopped");

    Ok(())
}

/// Windows SCM service entry point and control handler.
///
/// This module is only compiled on Windows with the "service" feature.
#[cfg(windows)]
pub mod windows_scm {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use std::time::Duration;

    use tokio::sync::mpsc;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};

    use super::ServiceError;

    /// Static storage for the shutdown sender and service config, needed because
    /// the SCM callback cannot capture state.
    static SHUTDOWN_TX: OnceLock<mpsc::Sender<()>> = OnceLock::new();
    static SERVICE_CONFIG: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();

    /// Initialize the static state before dispatching to SCM.
    pub fn set_config(base_dir: PathBuf, config_path: PathBuf) {
        let _ = SERVICE_CONFIG.set((base_dir, config_path));
    }

    /// The service main function called by the SCM dispatcher.
    pub fn service_main(_args: Vec<OsString>) {
        if let Err(e) = run_as_service() {
            tracing::error!("Service failed: {}", e);
        }
    }

    fn run_as_service() -> Result<(), ServiceError> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let _ = SHUTDOWN_TX.set(shutdown_tx);

        // Register control handler
        let status_handle =
            service_control_handler::register("Redis", move |control| match control {
                ServiceControl::Stop => {
                    if let Some(tx) = SHUTDOWN_TX.get() {
                        let _ = tx.try_send(());
                    }
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            })
            .map_err(|e| {
                ServiceError::WindowsService(format!("Failed to register control handler: {}", e))
            })?;

        // Report StartPending
        status_handle
            .set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: ServiceState::StartPending,
                controls_accepted: ServiceControlAccept::empty(),
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::from_secs(10),
                process_id: None,
            })
            .map_err(|e| {
                ServiceError::WindowsService(format!("Failed to set StartPending: {}", e))
            })?;

        let (base_dir, config_path) = SERVICE_CONFIG
            .get()
            .cloned()
            .ok_or_else(|| ServiceError::Config("Service config not initialized".into()))?;

        // Build a tokio runtime for the service
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ServiceError::ProcessStart(format!("Failed to create runtime: {}", e)))?;

        let result = rt.block_on(async {
            // Report Running
            status_handle
                .set_service_status(ServiceStatus {
                    service_type: ServiceType::OWN_PROCESS,
                    current_state: ServiceState::Running,
                    controls_accepted: ServiceControlAccept::STOP,
                    exit_code: ServiceExitCode::Win32(0),
                    checkpoint: 0,
                    wait_hint: Duration::default(),
                    process_id: None,
                })
                .map_err(|e| {
                    ServiceError::WindowsService(format!("Failed to set Running: {}", e))
                })?;

            // Run the service
            super::run_service(base_dir, config_path, shutdown_rx).await
        });

        // Report StopPending
        let _ = status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::StopPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(5),
            process_id: None,
        });

        // Report Stopped
        let exit_code = if result.is_ok() {
            ServiceExitCode::Win32(0)
        } else {
            ServiceExitCode::Win32(1)
        };

        let _ = status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code,
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        });

        result
    }
}


