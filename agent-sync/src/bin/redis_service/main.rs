//! Redis Windows Service binary entry point.
//!
//! Usage:
//!   redis-service.exe -c <config>           # Run as Windows Service (SCM dispatches here)
//!   redis-service.exe -c <config> --console  # Run in console mode (for development)

mod error;
mod event_log;
mod path_convert;
mod process_manager;
mod service_handler;

use std::path::PathBuf;

use clap::Parser;
use tokio::sync::mpsc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Redis Windows Service wrapper
#[derive(Parser, Debug)]
#[command(name = "redis-service", about = "Redis for Windows - Service Wrapper")]
struct Args {
    /// Path to the Redis configuration file
    #[arg(short = 'c', long = "config", default_value = "redis.conf")]
    config: String,

    /// Run in console mode instead of as a Windows Service
    #[arg(long)]
    console: bool,
}

/// Resolve a config path that works even when CWD is `C:\WINDOWS\system32`
/// (which is the default for Windows services).
///
/// Falls back to resolving relative to the service binary's own directory.
fn resolve_config_path(raw: &str) -> PathBuf {
    // Try direct canonicalization first (works if path is absolute and file exists)
    if let Ok(p) = dunce::canonicalize(raw) {
        return p;
    }

    // If the path is relative or canonicalize failed, resolve relative to
    // the service binary's directory (not CWD, which is system32 for services)
    if let Ok(exe) = std::env::current_exe() {
        if let Ok(p) = dunce::canonicalize(exe) {
            if let Some(exe_dir) = p.parent() {
                let candidate = exe_dir.join(raw);
                if let Ok(resolved) = dunce::canonicalize(&candidate) {
                    return resolved;
                }
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }

    // Final fallback: use as-is
    PathBuf::from(raw)
}

fn main() {
    let args = Args::parse();

    let config_path = resolve_config_path(&args.config);

    // base_dir = directory containing the config file (where redis-server.exe should be)
    let base_dir = config_path
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::env::current_dir().expect("Cannot determine current directory"));

    if args.console {
        // Console mode: initialize console logging, run with Ctrl+C handler
        init_console_logging();
        run_console(base_dir, config_path);
    } else {
        // Service mode: initialize event log + file logging, dispatch to SCM
        init_service_logging();
        run_service_mode(base_dir, config_path);
    }
}

/// Initialize logging for console mode (human-readable to stderr).
fn init_console_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,redis=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize logging for service mode (event log layer + optional file layer).
fn init_service_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,redis=info".into()),
        )
        .with(event_log::EventLogLayer::try_new().expect("Failed to create event log layer"))
        .init();
}

/// Run in console mode with Ctrl+C handling.
fn run_console(base_dir: PathBuf, config_path: PathBuf) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        // Ctrl+C handler
        let tx = shutdown_tx.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for Ctrl+C");
            tracing::info!("Ctrl+C received, initiating shutdown");
            let _ = tx.send(()).await;
        });

        match service_handler::run_service(base_dir, config_path, shutdown_rx).await {
            Ok(()) => tracing::info!("Service exited cleanly"),
            Err(e) => {
                tracing::error!("Service error: {}", e);
                std::process::exit(1);
            }
        }
    });
}

/// Run as a Windows Service via SCM dispatcher.
#[cfg(windows)]
fn run_service_mode(base_dir: PathBuf, config_path: PathBuf) {
    use windows_service::{define_windows_service, service_dispatcher};

    // The define_windows_service! macro generates a `extern "system"` function
    // that the SCM can call.
    define_windows_service!(ffi_service_main, service_main_wrapper);

    fn service_main_wrapper(args: Vec<std::ffi::OsString>) {
        service_handler::windows_scm::service_main(args);
    }

    service_handler::windows_scm::set_config(base_dir, config_path);

    if let Err(e) = service_dispatcher::start("Redis", ffi_service_main) {
        // If dispatch fails, the binary was likely run directly (not via SCM).
        eprintln!("Failed to start as Windows Service: {}", e);
        eprintln!("Use --console flag to run in console mode for development.");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn run_service_mode(base_dir: PathBuf, config_path: PathBuf) {
    eprintln!("Windows Service mode is only available on Windows.");
    eprintln!("Running in console mode instead.");
    run_console(base_dir, config_path);
}






