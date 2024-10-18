#[macro_use]
extern crate tracing;

use anyhow::Result;
use std::{
    env,
    error,
    fs::read_to_string,
    process,
};
use tracing_subscriber::EnvFilter;

pub fn setup_logs(pkg_name: &'static str, extra_default_directives: Vec<&'static str>) {
    let env = if get_env("RUST_LOG").is_none() {
        let pkg_name = pkg_name.replace('-', "_");
        let extra_directives = extra_default_directives.join(",");
        EnvFilter::new(format!("{pkg_name}=debug,rs_utils=debug{}{extra_directives}", if extra_directives.is_empty() { "" } else { "," }))
    } else {
        EnvFilter::from_default_env()
    };

    let log_str = env.to_string();

    tracing_subscriber::fmt::fmt()
        .with_env_filter(env)
        .with_target(false)
        .init();

    debug!("Initialized logger with directives: {log_str}");
}

#[cfg(windows)]
pub async fn wait_for_signal() -> Result<()> {
    tokio::signal::ctrl_c().await?;
    info!("Received Ctrl+C, shutting down...");
    Ok(())
}

#[cfg(unix)]
pub async fn wait_for_signal() -> Result<()> {
    use tokio::select;
    use tokio::signal::unix::{signal, SignalKind};

    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;

    select! {
        // Wait for SIGINT (which is sent on the first Ctrl+C)
        _ = interrupt.recv() => {
            info!("Received interrupt signal, shutting down...");
        }
        // Wait for SIGTERM
        _ = terminate.recv() => {
            info!("Received terminate signal, shutting down...");
        }
    }

    Ok(())
}

pub fn exit_on_error<V, E>(res: Result<V, E>, msg: &'static str) -> V
where
    E: error::Error + Send + Sync + 'static,
{
    exit_on_anyhow_error(res.map_err(|e| anyhow::Error::new(e)), msg)
}

pub fn exit_on_anyhow_error<V>(res: Result<V, anyhow::Error>, msg: &'static str) -> V {
    res.unwrap_or_else(|e| {
        error!("{msg}: {e}");
        process::exit(1)
    })
}

pub fn get_env(key: &str) -> Option<String> {
    // Check if the env var is set
    env::var(key)
        // This returns an error if the env var is not set, turn it into an option
        .ok()
        .and_then(|value| value_from_env_var(key, value))
}

pub fn get_env_exit(key: &str) -> String {
    get_env(key).unwrap_or_else(|| {
        error!("Env var {key} not set.");
        process::exit(2)
    })
}

fn value_from_env_var(key: &str, value: String) -> Option<String> {
    // Trim the value to remove any leading/trailing whitespace
    let value = value.trim();

    if value.is_empty() {
        // Check if the set var is empty or not
        error!("Env var {key} set but empty.");
        None
    } else {
        // Check if the value is a file path
        match value.strip_prefix("file:") {
            Some(file_path) => match read_to_string(file_path) {
                // If we can read the file, return the content trimmed
                Ok(file_content) => Some(file_content.trim().to_string()),
                Err(e) => {
                    error!("Failed to read file {file_path}: {e}");
                    None
                }
            }
            // Not a file, return the value as is
            None => Some(value.to_string()),
        }
    }
}