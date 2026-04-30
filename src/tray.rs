//! System tray entry point
//!
//! Extracted from `main.rs` to keep the CLI entry focused on argument parsing
//! and command dispatch. This module handles:
//!
//! - Platform-specific tray initialization (Windows / macOS)
//! - Async command handling between tray UI and daemon
//! - Daemon lifecycle management (auto-start, reconnect, shutdown)

use anyhow::Result;
use std::thread;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::client::DaemonClient;
use crate::config;
use crate::platform::traits::{AppCommand, ApplicationControl, TrayLifecycle};

/// Try to reconnect to daemon after connection loss
async fn try_reconnect(client_option: &mut Option<DaemonClient>, instance_id: u32) {
    use crate::constants::{DEFAULT_RETRY_DELAY_MS, MAX_RECONNECT_RETRIES};

    let mut client = DaemonClient::new();
    let max_retries = MAX_RECONNECT_RETRIES;
    let retry_delay = tokio::time::Duration::from_millis(DEFAULT_RETRY_DELAY_MS);

    for attempt in 1..=max_retries {
        if attempt > 1 {
            tokio::time::sleep(retry_delay).await;
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
///
/// This is the shared implementation for both Windows and macOS tray.
/// `on_exit` is called when the Exit command is received (platform-specific cleanup).
/// `open_config_folder` is the platform-specific function to open the config folder.
pub async fn connect_and_handle_tray_commands(
    mut cmd_rx: mpsc::Receiver<AppCommand>,
    instance_id: u32,
    on_exit: impl FnOnce(),
    open_config_folder: impl Fn(u32) -> Result<()>,
) {
    use crate::constants::{DEFAULT_RETRY_DELAY_MS, MAX_CONNECTION_RETRIES};

    info!("Tokio runtime started in background thread");

    let mut client_option: Option<DaemonClient> = None;
    let mut client = DaemonClient::new();
    let max_retries = MAX_CONNECTION_RETRIES;
    let retry_delay = tokio::time::Duration::from_millis(DEFAULT_RETRY_DELAY_MS);

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
                if let Some(ref mut client) = client_option {
                    match client.get_status().await {
                        Ok((current_active, _)) => {
                            let new_active = !current_active;
                            match client.set_active(new_active).await {
                                Ok(_) => {
                                    info!(
                                        "Daemon active state changed to: {}",
                                        new_active
                                    );
                                }
                                Err(e) => {
                                    error!("Failed to set active state: {}", e);
                                    try_reconnect(&mut client_option, instance_id).await;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to get status: {}", e);
                            try_reconnect(&mut client_option, instance_id).await;
                        }
                    }
                } else {
                    error!("Not connected to daemon, attempting reconnection...");
                    try_reconnect(&mut client_option, instance_id).await;
                }
            }
            AppCommand::ReloadConfig => {
                info!("Reload config command received");
                if let Some(ref mut client) = client_option {
                    match client.reload_config().await {
                        Ok(_) => {
                            info!("Configuration reloaded successfully");
                        }
                        Err(e) => {
                            error!("Failed to reload config: {}", e);
                            try_reconnect(&mut client_option, instance_id).await;
                        }
                    }
                } else {
                    error!("Not connected to daemon, attempting reconnection...");
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

/// Open config folder - Windows sync version
#[cfg(target_os = "windows")]
pub fn open_config_folder_sync(instance_id: u32) -> Result<()> {
    use std::process::Command;

    let config_path = config::resolve_config_file_path(None, instance_id)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| {
            std::env::var("USERPROFILE")
                .map(std::path::PathBuf::from)
                .unwrap_or_default()
        });

    Command::new("explorer").arg(config_path).spawn()?;

    Ok(())
}

/// Open config folder - macOS sync version
#[cfg(target_os = "macos")]
pub fn open_config_folder_sync(instance_id: u32) -> Result<()> {
    use std::process::Command;

    let config_path = config::resolve_config_file_path(None, instance_id)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_default()
        });

    Command::new("open").arg(config_path).spawn()?;
    Ok(())
}

/// Run tokio runtime in background thread for Windows tray
#[cfg(target_os = "windows")]
fn run_tokio_for_tray(cmd_rx: mpsc::Receiver<AppCommand>, instance_id: u32) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(connect_and_handle_tray_commands(
        cmd_rx,
        instance_id,
        || {
            crate::platform::windows::WindowsPlatform::stop_tray();
        },
        open_config_folder_sync,
    ));
}

/// Run tokio runtime in background thread for macOS tray
#[cfg(target_os = "macos")]
fn run_tokio_for_tray(cmd_rx: mpsc::Receiver<AppCommand>, instance_id: u32) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(connect_and_handle_tray_commands(
        cmd_rx,
        instance_id,
        || unsafe {
            use cocoa::appkit::NSApplication;
            use cocoa::base::nil;
            use objc::runtime::Class;
            use objc::{msg_send, sel, sel_impl};

            let app_class = Class::get("NSApplication").unwrap();
            let app: *mut objc::runtime::Object =
                msg_send![app_class, sharedApplication];
            if app != nil {
                let _: () = msg_send![app, terminate: nil];
            }
        },
        open_config_folder_sync,
    ));
}

/// Check if daemon is running
fn is_daemon_running(instance_id: u32) -> bool {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return false,
    };

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await.is_ok()
    })
}

/// Start the daemon (used by tray auto-start)
fn run_daemon(
    instance_id: u32,
    preloaded_config: Option<config::Config>,
    config_path: Option<std::path::PathBuf>,
) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        info!("Starting wakemd (instance {})...", instance_id);
        crate::daemon::run_server_with_config(instance_id, preloaded_config, config_path)
            .await
    })
}

/// Run system tray (Windows)
///
/// Tray message loop runs on main thread, tokio runs in background thread
#[cfg(target_os = "windows")]
pub fn run_tray_sync(
    instance_id: u32,
    auto_start_daemon: bool,
    detach_console: bool,
) -> Result<()> {
    use crate::platform::windows::WindowsPlatform;

    info!("wakem starting (instance {})...", instance_id);

    if detach_console {
        WindowsPlatform::detach_console();
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
                if let Err(e) = run_daemon(instance_id, None, None) {
                    error!("Daemon exited with error: {}", e);
                }
            } else {
                info!("Daemon already running");
            }
        }))
    } else {
        None
    };

    let tray_result = WindowsPlatform::run_tray_message_loop(Box::new(move |cmd| {
        let _ = cmd_tx_for_tray.blocking_send(cmd);
    }));

    let _ = cmd_tx.blocking_send(AppCommand::Exit);
    info!("Waiting for tokio thread to exit...");
    let _ = tokio_handle.join();
    info!("Tokio thread exited");

    if let Some(handle) = daemon_handle {
        info!("Waiting for daemon to shutdown...");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = handle.join();
            let _ = tx.send(());
        });
        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(_) => info!("Daemon shutdown successfully"),
            Err(_) => {
                error!("Daemon shutdown timed out after 10 seconds, forcing exit...");
                let _ = WindowsPlatform::force_kill_instance(instance_id);
            }
        }
    }

    info!("wakem shutdown complete");
    tray_result
}

/// Run system tray (macOS)
///
/// On macOS, NSApplication must run on the main thread, so we spawn tokio in a background thread
#[cfg(target_os = "macos")]
pub fn run_tray_sync(
    instance_id: u32,
    auto_start_daemon: bool,
    _detach_console: bool,
) -> Result<()> {
    use crate::platform::macos::run_tray_event_loop;

    info!("wakem starting (instance {})...", instance_id);

    let (cmd_tx, cmd_rx) = mpsc::channel::<AppCommand>(16);
    let cmd_tx_for_tray = cmd_tx.clone();

    let tokio_handle = thread::spawn(move || {
        run_tokio_for_tray(cmd_rx, instance_id);
    });

    let daemon_handle = if auto_start_daemon {
        Some(thread::spawn(move || {
            if !is_daemon_running(instance_id) {
                info!("Daemon not running, auto-starting...");
                if let Err(e) = run_daemon(instance_id, None, None) {
                    error!("Daemon exited with error: {}", e);
                }
            } else {
                info!("Daemon already running");
            }
        }))
    } else {
        None
    };

    info!("Starting tray on main thread...");
    if let Err(e) = run_tray_event_loop(move |cmd| {
        let _ = cmd_tx_for_tray.blocking_send(cmd);
    }) {
        error!("Tray error: {}", e);
    }

    let _ = cmd_tx.blocking_send(AppCommand::Exit);
    let _ = tokio_handle.join();

    if let Some(handle) = daemon_handle {
        info!("Waiting for daemon to shutdown...");
        let _ = handle.join();
    }

    info!("wakem shutdown complete");
    Ok(())
}
