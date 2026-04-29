use anyhow::Result;
use std::thread;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::client::DaemonClient;
use crate::config;
use crate::constants;
use crate::platform::traits::{AppCommand, ApplicationControl, TrayLifecycle};
use crate::platform::CurrentPlatform;
use crate::runtime_util::with_runtime;

const ERR_NOT_CONNECTED: &str = "Not connected to daemon, attempting reconnection...";

/// Check if daemon is running
pub fn is_daemon_running(instance_id: u32) -> bool {
    with_runtime(|rt| {
        Ok(rt.block_on(async {
            let mut client = DaemonClient::new();
            client.connect_to_instance(instance_id).await.is_ok()
        }))
    })
    .unwrap_or(false)
}

/// Run system tray
/// Tray message loop runs on main thread, tokio runs in background thread
pub fn run_tray_sync(
    instance_id: u32,
    auto_start_daemon: bool,
    detach_console: bool,
) -> Result<()> {
    info!("wakem starting (instance {})...", instance_id);

    if detach_console {
        <CurrentPlatform as ApplicationControl>::detach_console();
    }

    let (cmd_tx, cmd_rx) = mpsc::channel::<AppCommand>(16);
    let cmd_tx_for_tray = cmd_tx.clone();

    let tokio_handle = thread::spawn(move || {
        run_tokio_for_tray(cmd_rx, instance_id);
    });

    let daemon_handle = if auto_start_daemon {
        Some(thread::spawn(move || {
            if !is_daemon_running(instance_id) {
                info!("Daemon not running, auto-starting...");
                if let Err(e) = run_daemon_inline(instance_id) {
                    error!("Daemon exited with error: {}", e);
                }
            } else {
                info!("Daemon already running");
            }
        }))
    } else {
        None
    };

    let tray_result = <CurrentPlatform as TrayLifecycle>::run_tray_message_loop(
        Box::new(move |cmd| {
            if let Err(e) = cmd_tx_for_tray.blocking_send(cmd) {
                debug!("Failed to send tray command: {}", e);
            }
        }),
    );

    let _ = cmd_tx.blocking_send(AppCommand::Exit);
    info!("Waiting for tokio thread to exit...");
    let _ = tokio_handle.join();
    info!("Tokio thread exited");

    if let Some(handle) = daemon_handle {
        info!("Waiting for daemon to shutdown...");
        match wait_for_daemon_shutdown(
            handle,
            std::time::Duration::from_secs(constants::DAEMON_SHUTDOWN_TIMEOUT_SECS),
        ) {
            Ok(_) => info!("Daemon shutdown successfully"),
            Err(_) => {
                error!(
                    "Daemon shutdown timed out after {} seconds, forcing exit...",
                    constants::DAEMON_SHUTDOWN_TIMEOUT_SECS
                );
                let _ = <CurrentPlatform as ApplicationControl>::force_kill_instance(
                    instance_id,
                );
            }
        }
    }

    info!("wakem shutdown complete");
    tray_result
}

/// Start daemon inline (for thread context where we need a new tokio runtime)
fn run_daemon_inline(instance_id: u32) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        info!("Starting wakemd (instance {})...", instance_id);
        crate::daemon::run_server_with_config(instance_id, None, None).await
    })
}

/// Wait for daemon thread to shutdown with timeout
pub fn wait_for_daemon_shutdown(
    handle: std::thread::JoinHandle<()>,
    timeout: std::time::Duration,
) -> anyhow::Result<()> {
    use std::sync::mpsc::channel;

    let (tx, rx) = channel();

    thread::spawn(move || {
        let _ = handle.join();
        let _ = tx.send(());
    });

    rx.recv_timeout(timeout)
        .map_err(|_| anyhow::anyhow!("Daemon shutdown timed out after {:?}", timeout))?;
    Ok(())
}

/// Try to reconnect to daemon after connection loss
async fn try_reconnect(client_option: &mut Option<DaemonClient>, instance_id: u32) {
    let mut client = DaemonClient::new();
    let max_retries = constants::MAX_RECONNECT_RETRIES;
    let base_delay =
        tokio::time::Duration::from_millis(constants::DEFAULT_RETRY_DELAY_MS);
    let max_delay = tokio::time::Duration::from_secs(8);

    for attempt in 1..=max_retries {
        if attempt > 1 {
            let delay = base_delay * 2u32.pow(attempt - 2);
            let delay = delay.min(max_delay);
            tokio::time::sleep(delay).await;
        }
        match client.connect_to_instance(instance_id).await {
            Ok(_) => {
                info!(
                    "Reconnected to daemon instance {} (attempt {})",
                    instance_id, attempt
                );
                *client_option = Some(client);
                return;
            }
            Err(e) => {
                debug!(
                    "Reconnection attempt {}/{} failed: {}",
                    attempt, max_retries, e
                );
            }
        }
    }

    warn!(
        "Failed to reconnect to daemon after {} attempts",
        max_retries
    );
    *client_option = None;
}

/// Connect to daemon with retry logic and handle tray commands
/// This is the shared implementation for both Windows and macOS tray.
/// `on_exit` is called when the Exit command is received (platform-specific cleanup).
/// `open_config_folder` is the platform-specific function to open the config folder.
pub async fn connect_and_handle_tray_commands(
    mut cmd_rx: mpsc::Receiver<AppCommand>,
    instance_id: u32,
    on_exit: impl FnOnce(),
    open_config_folder: impl Fn(u32) -> Result<()>,
) {
    info!("Tokio runtime started in background thread");

    let mut client_option: Option<DaemonClient> = None;
    let mut client = DaemonClient::new();
    let max_retries = constants::MAX_CONNECTION_RETRIES;
    let retry_delay =
        tokio::time::Duration::from_millis(constants::DEFAULT_RETRY_DELAY_MS);

    for attempt in 1..=max_retries {
        match client.connect_to_instance(instance_id).await {
            Ok(_) => {
                info!(
                    "Connected to wakemd instance {} (attempt {}/{})",
                    instance_id, attempt, max_retries
                );

                match client.get_status().await {
                    Ok((active, loaded)) => {
                        info!(
                            "Daemon status: active={}, config_loaded={}",
                            active, loaded
                        );
                    }
                    Err(e) => {
                        error!("Failed to get status: {}", e);
                    }
                }

                client_option = Some(client);
                break;
            }
            Err(e) => {
                if attempt < max_retries {
                    debug!(
                        "Connection attempt {}/{} failed, retrying in {:?}...",
                        attempt, max_retries, retry_delay
                    );
                    tokio::time::sleep(retry_delay).await;
                } else {
                    error!(
                        "Failed to connect to daemon after {} attempts: {}",
                        max_retries, e
                    );
                    error!(
                        "Please make sure wakemd --instance {} is running",
                        instance_id
                    );
                }
            }
        }
    }

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            AppCommand::ToggleActive => {
                info!("Toggle active command received");
                if let Some(ref mut c) = client_option {
                    match c.get_status().await {
                        Ok((current_active, _)) => {
                            let new_active = !current_active;
                            if let Err(e) = c.set_active(new_active).await {
                                error!("Failed to set active state: {}", e);
                                try_reconnect(&mut client_option, instance_id).await;
                            } else {
                                info!("Daemon active state changed to: {}", new_active);
                            }
                        }
                        Err(e) => {
                            error!("Failed to get status: {}", e);
                            try_reconnect(&mut client_option, instance_id).await;
                        }
                    }
                } else {
                    error!("{}", ERR_NOT_CONNECTED);
                    try_reconnect(&mut client_option, instance_id).await;
                }
            }
            AppCommand::ReloadConfig => {
                info!("Reload config command received");
                if let Some(ref mut c) = client_option {
                    if let Err(e) = c.reload_config().await {
                        error!("Failed to reload config: {}", e);
                        try_reconnect(&mut client_option, instance_id).await;
                    } else {
                        info!("Configuration reloaded successfully");
                    }
                } else {
                    error!("{}", ERR_NOT_CONNECTED);
                    try_reconnect(&mut client_option, instance_id).await;
                }
            }
            AppCommand::OpenConfigFolder => {
                info!("Open config folder command received");
                if let Err(e) = open_config_folder(instance_id) {
                    error!("Failed to open config folder: {}", e);
                }
            }
            AppCommand::Exit => {
                info!("Exit command received");

                if let Some(ref mut client) = client_option {
                    match client.shutdown().await {
                        Ok(_) => {
                            info!("Daemon shutdown successfully");
                        }
                        Err(e) => {
                            error!("Failed to shutdown daemon: {}", e);
                        }
                    }
                }

                on_exit();
                break;
            }
        }
    }

    info!("Tokio runtime shutting down");
}

/// Run tokio runtime in background thread for tray
pub fn run_tokio_for_tray(cmd_rx: mpsc::Receiver<AppCommand>, instance_id: u32) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(connect_and_handle_tray_commands(
        cmd_rx,
        instance_id,
        || {
            <CurrentPlatform as TrayLifecycle>::stop_tray();
            <CurrentPlatform as ApplicationControl>::terminate_application();
        },
        open_config_folder_sync,
    ));
}

/// Open config folder
pub fn open_config_folder_sync(instance_id: u32) -> Result<()> {
    let config_path = config::resolve_config_file_path(None, instance_id)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_default());

    <CurrentPlatform as ApplicationControl>::open_folder(&config_path)?;
    Ok(())
}
