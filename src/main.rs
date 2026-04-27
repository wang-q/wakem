use anyhow::Result;
use clap::Parser;
use std::thread;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

mod cli;
mod client;
mod config;
mod constants;
mod daemon;
mod ipc;
mod platform;
mod runtime;
mod shutdown;
mod types;

use cli::{Cli, Commands};
use client::DaemonClient;
use platform::traits::{AppCommand, ApplicationControl, TrayLifecycle};

/// Error message for when the client is not connected to daemon
const ERR_NOT_CONNECTED: &str = "Not connected to daemon, attempting reconnection...";

/// Initialize logging system with support for reading log level from config file
/// Returns the parsed Config if successfully loaded, so it can be reused by the daemon
fn init_logging(cli: &Cli) -> (Option<config::Config>, Option<std::path::PathBuf>) {
    // Use explicit config path if provided, otherwise use default location
    let config_path = cli
        .config
        .clone()
        .or_else(|| config::resolve_config_file_path(None, cli.instance));

    let (log_level, config_result) = if let Some(ref path) = config_path {
        match config::Config::from_file(path) {
            Ok(cfg) => (cfg.log_level.clone(), Some(Ok(cfg))),
            Err(e) => {
                // Logging not yet initialized, eprintln is acceptable here
                eprintln!("Failed to load config for log level: {}", e);
                ("info".to_string(), Some(Err(e)))
            }
        }
    } else {
        ("info".to_string(), None)
    };

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    if let Some(result) = config_result {
        match result {
            Ok(cfg) => {
                info!("Logging initialized with level: {}", log_level);
                return (Some(cfg), config_path);
            }
            Err(err) => {
                debug!("Failed to load config for log level: {}", err);
            }
        }
    }

    info!("Logging initialized with level: {}", log_level);
    (None, config_path)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let (preloaded_config, config_path) = init_logging(&cli);

    match cli.command {
        Some(Commands::Daemon) => {
            run_daemon(cli.instance, preloaded_config, config_path)
        }
        Some(Commands::Status) => cmd_status_sync(cli.instance),
        Some(Commands::Reload) => cmd_reload_sync(cli.instance),
        Some(Commands::Save) => cmd_save_sync(cli.instance),
        Some(Commands::Enable) => cmd_enable_sync(cli.instance),
        Some(Commands::Disable) => cmd_disable_sync(cli.instance),
        Some(Commands::Config) => cmd_config_sync(cli.instance),
        Some(Commands::Instances) => cmd_instances_sync(),
        Some(Commands::Record { name }) => cmd_record_sync(cli.instance, &name),
        Some(Commands::StopRecord) => cmd_stop_record_sync(cli.instance),
        Some(Commands::Play { name }) => cmd_play_sync(cli.instance, &name),
        Some(Commands::Macros) => cmd_macros_sync(cli.instance),
        Some(Commands::BindMacro {
            macro_name,
            trigger,
        }) => cmd_bind_macro_sync(cli.instance, &macro_name, &trigger),
        Some(Commands::DeleteMacro { name }) => {
            cmd_delete_macro_sync(cli.instance, &name)
        }
        Some(Commands::Tray) => run_tray_sync(cli.instance, false, false), // Tray only, don't auto-start daemon, keep console
        None => run_tray_sync(cli.instance, true, true), // Default: auto-start daemon, detach console
    }
}

/// Start the daemon with multi-thread tokio runtime
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
        daemon::run_server_with_config(instance_id, preloaded_config, config_path).await
    })
}

/// Check if daemon is running
fn is_daemon_running(instance_id: u32) -> bool {
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
fn run_tray_sync(
    instance_id: u32,
    auto_start_daemon: bool,
    detach_console: bool,
) -> Result<()> {
    use platform::CurrentPlatform;

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

/// Wait for daemon thread to shutdown with timeout
fn wait_for_daemon_shutdown(
    handle: std::thread::JoinHandle<()>,
    timeout: std::time::Duration,
) -> Result<(), ()> {
    use std::sync::mpsc::channel;
    use std::thread;

    let (tx, rx) = channel();

    // Spawn a thread to wait for the daemon handle
    thread::spawn(move || {
        let _ = handle.join();
        let _ = tx.send(());
    });

    // Wait with timeout
    match rx.recv_timeout(timeout) {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}

/// Try to reconnect to daemon after connection loss
async fn try_reconnect(client_option: &mut Option<DaemonClient>, instance_id: u32) {
    use crate::constants::{DEFAULT_RETRY_DELAY_MS, MAX_RECONNECT_RETRIES};

    let mut client = DaemonClient::new();
    let max_retries = MAX_RECONNECT_RETRIES;
    let base_delay = tokio::time::Duration::from_millis(DEFAULT_RETRY_DELAY_MS);
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
async fn connect_and_handle_tray_commands(
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
fn run_tokio_for_tray(cmd_rx: mpsc::Receiver<AppCommand>, instance_id: u32) {
    use platform::CurrentPlatform;

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

/// Open config folder - sync version
fn open_config_folder_sync(instance_id: u32) -> Result<()> {
    use platform::CurrentPlatform;

    let config_path = config::resolve_config_file_path(None, instance_id)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_default());

    <CurrentPlatform as ApplicationControl>::open_folder(&config_path)?;
    Ok(())
}

/// Execute a closure with a cached single-threaded tokio runtime.
///
/// Using a thread-local runtime avoids the overhead of creating and destroying
/// a runtime for every CLI command. This is especially beneficial when multiple
/// commands are issued in quick succession.
///
/// The runtime reference is only valid within the closure, avoiding the need
/// for unsafe lifetime extension.
fn with_runtime<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&tokio::runtime::Runtime) -> Result<R>,
{
    thread_local! {
        static RUNTIME: tokio::runtime::Runtime = {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime")
        };
    }

    RUNTIME.with(f)
}

/// Execute an async operation with a daemon client connection
/// Reuses a cached runtime to avoid repeated creation/destruction overhead.
/// The closure receives ownership of the client to avoid async lifetime issues.
fn run_with_client<F, Fut>(instance_id: u32, op: F) -> Result<()>
where
    F: FnOnce(DaemonClient) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    with_runtime(|rt| {
        rt.block_on(async {
            let mut client = DaemonClient::new();
            client.connect_to_instance(instance_id).await?;
            op(client).await
        })
    })
}

/// Get server status - sync version
fn cmd_status_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        let (active, loaded) = client.get_status().await?;
        println!("wakemd instance {}:", instance_id);
        println!("  Active: {}", if active { "yes" } else { "no" });
        println!("  Config loaded: {}", if loaded { "yes" } else { "no" });
        Ok(())
    })
}

/// Reload configuration - sync version
fn cmd_reload_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.reload_config().await?;
        println!("Configuration reloaded successfully");
        Ok(())
    })
}

/// Save configuration - sync version
fn cmd_save_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.save_config().await?;
        println!("Configuration saved successfully");
        Ok(())
    })
}

/// Enable mapping - sync version
fn cmd_enable_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.set_active(true).await?;
        println!("wakem enabled");
        Ok(())
    })
}

/// Disable mapping - sync version
fn cmd_disable_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.set_active(false).await?;
        println!("wakem disabled");
        Ok(())
    })
}

/// Open config folder - sync version
fn cmd_config_sync(instance_id: u32) -> Result<()> {
    open_config_folder_sync(instance_id)?;
    println!("Config folder opened");
    Ok(())
}

/// Execute an async operation with a cached tokio runtime
/// Similar to `run_with_client` but without connecting to a daemon instance.
fn run_async<F, Fut>(op: F) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    with_runtime(|rt| rt.block_on(op()))
}

/// List running instances - sync version
fn cmd_instances_sync() -> Result<()> {
    run_async(|| async {
        let instances = ipc::discover_instances(None).await;

        println!("Running instances:");
        let mut found = false;
        for info in instances {
            if info.active {
                found = true;
                println!("  Instance {}: {} (active)", info.id, info.address);
            }
        }

        if !found {
            println!("  No running instances found");
        }

        Ok(())
    })
}

/// Record macro - sync version
fn cmd_record_sync(instance_id: u32, name: &str) -> Result<()> {
    // Validate macro name
    config::Config::validate_macro_name(name)?;

    let name_owned = name.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.start_macro_recording(&name_owned).await?;
        println!("Recording macro '{}'...", name_owned);
        println!("Press Ctrl+Shift+Esc to stop recording");
        Ok(())
    })
}

/// Stop recording macro - sync version
fn cmd_stop_record_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        let (name, count) = client.stop_macro_recording().await?;
        println!("Macro '{}' saved with {} actions", name, count);
        Ok(())
    })
}

/// Play macro - sync version
fn cmd_play_sync(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.play_macro(&name_owned).await?;
        println!("Playing macro '{}'", name_owned);
        Ok(())
    })
}

/// List all macros - sync version
fn cmd_macros_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        let macros = client.get_macros().await?;
        if macros.is_empty() {
            println!("No macros recorded");
        } else {
            println!("Available macros:");
            for name in macros {
                println!("  - {}", name);
            }
        }
        Ok(())
    })
}

/// Bind macro to trigger key - sync version
fn cmd_bind_macro_sync(instance_id: u32, macro_name: &str, trigger: &str) -> Result<()> {
    let macro_name_owned = macro_name.to_string();
    let trigger_owned = trigger.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.bind_macro(&macro_name_owned, &trigger_owned).await?;
        println!("Macro '{}' bound to '{}'", macro_name_owned, trigger_owned);
        Ok(())
    })
}

/// Delete macro - sync version
fn cmd_delete_macro_sync(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.delete_macro(&name_owned).await?;
        println!("Macro '{}' deleted", name_owned);
        Ok(())
    })
}
