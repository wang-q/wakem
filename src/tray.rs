use anyhow::Result;
use std::thread;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::client::DaemonClient;
use crate::config;
use crate::platform::traits::{AppCommand, ApplicationControl, TrayLifecycle};
use crate::platform::CurrentPlatform;

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

pub async fn connect_and_handle_tray_commands(
    mut cmd_rx: mpsc::Receiver<AppCommand>,
    instance_id: u32,
    on_exit: impl FnOnce(),
    open_config_folder: impl Fn(u32) -> Result<()>,
) {
    use crate::constants::{DEFAULT_RETRY_DELAY_MS, MAX_CONNECTION_RETRIES};

    info!("Tokio runtime started in background thread");

    let mut client_option: Option<DaemonClient>;
    let mut client = DaemonClient::new();
    let max_retries = MAX_CONNECTION_RETRIES;
    let retry_delay = tokio::time::Duration::from_millis(DEFAULT_RETRY_DELAY_MS);

    let mut attempt = 0u32;
    let mut retry_delay_pin = tokio::time::Instant::now();

    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(AppCommand::Exit) => {
                        info!("Exit command received during connection phase");
                        on_exit();
                        return;
                    }
                    Some(cmd) => {
                        warn!("Ignoring {:?} - not connected to daemon yet", cmd);
                    }
                    None => {
                        info!("Command channel closed");
                        return;
                    }
                }
            }
            result = async {
                if attempt > 0 && attempt < max_retries {
                    tokio::time::sleep_until(retry_delay_pin).await;
                }
                if attempt >= max_retries {
                    std::future::pending().await
                } else {
                    attempt += 1;
                    client.connect_to_instance(instance_id).await
                }
            }, if attempt < max_retries => {
                match result {
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
                            retry_delay_pin = tokio::time::Instant::now() + retry_delay;
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

fn open_config_folder_sync(instance_id: u32) -> Result<()> {
    let config_path = config::resolve_config_file_path(None, instance_id)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .ok_or_else(|| anyhow::anyhow!("Could not resolve config directory"))?;

    CurrentPlatform::open_folder(&config_path)
}

fn run_tokio_for_tray(cmd_rx: mpsc::Receiver<AppCommand>, instance_id: u32) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(connect_and_handle_tray_commands(
        cmd_rx,
        instance_id,
        CurrentPlatform::terminate_application,
        open_config_folder_sync,
    ));
}

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

pub fn run_tray_sync(
    instance_id: u32,
    auto_start_daemon: bool,
    detach_console: bool,
) -> Result<()> {
    info!("wakem starting (instance {})...", instance_id);

    if detach_console {
        CurrentPlatform::detach_console();
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

    let tray_result = CurrentPlatform::run_tray_message_loop(Box::new(move |cmd| {
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
                let _ = CurrentPlatform::force_kill_instance(instance_id);
            }
        }
    }

    info!("wakem shutdown complete");
    tray_result
}
