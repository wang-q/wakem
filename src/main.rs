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

// Platform-specific imports
#[cfg(target_os = "windows")]
use platform::windows::run_tray_message_loop;

#[cfg(target_os = "macos")]
use platform::macos::run_tray_event_loop;

use platform::traits::AppCommand;

/// Initialize logging system with support for reading log level from config file
/// Returns the parsed Config if successfully loaded, so it can be reused by the daemon
fn init_logging(cli: &Cli) -> Option<config::Config> {
    let (log_level, config_result) = if let Some(config_path) =
        config::resolve_config_file_path(None, cli.instance)
    {
        match config::Config::from_file(&config_path) {
            Ok(cfg) => (cfg.log_level.clone(), Some(Ok(cfg))),
            Err(e) => {
                eprintln!("Debug: Failed to load config for log level: {}", e);
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
                return Some(cfg);
            }
            Err(err) => {
                debug!("Failed to load config for log level: {}", err);
            }
        }
    }

    info!("Logging initialized with level: {}", log_level);
    None
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let preloaded_config = init_logging(&cli);

    match cli.command {
        Some(Commands::Daemon) => run_daemon(cli.instance, preloaded_config),
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
fn run_daemon(instance_id: u32, preloaded_config: Option<config::Config>) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        info!("Starting wakemd (instance {})...", instance_id);
        daemon::run_server(instance_id, preloaded_config).await
    })
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

/// Run system tray (Windows)
/// Tray message loop runs on main thread, tokio runs in background thread
#[cfg(target_os = "windows")]
fn run_tray_sync(
    instance_id: u32,
    auto_start_daemon: bool,
    detach_console: bool,
) -> Result<()> {
    use windows::Win32::System::Console::FreeConsole;

    info!("wakem starting (instance {})...", instance_id);

    // Detach from console for default tray mode (no command). This prevents
    // any console window from showing when wakem is launched from GUI.
    // Explicit `wakem tray` keeps console for logging.
    if detach_console {
        unsafe {
            let _ = FreeConsole();
        }
    }

    // Create async channel for communication between tray and tokio
    let (cmd_tx, cmd_rx) = mpsc::channel::<AppCommand>(16);
    let cmd_tx_for_tray = cmd_tx.clone();

    // Start tokio runtime in background thread immediately - don't wait!
    let tokio_handle = thread::spawn(move || {
        run_tokio_for_tray(cmd_rx, instance_id);
    });

    // Auto-start daemon in background if needed (completely independent)
    let daemon_handle = if auto_start_daemon {
        Some(thread::spawn(move || {
            // Check and start daemon if needed
            if !is_daemon_running(instance_id) {
                info!("Daemon not running, auto-starting...");
                if let Err(e) = run_daemon(instance_id, None) {
                    error!("Daemon exited with error: {}", e);
                }
            } else {
                info!("Daemon already running");
            }
        }))
    } else {
        None
    };

    // Run tray message loop on main thread immediately - no blocking before this!
    let tray_result = run_tray_message_loop(move |cmd| {
        let _ = cmd_tx_for_tray.blocking_send(cmd);
    });

    // Cleanup: signal tokio to stop, wait for threads
    let _ = cmd_tx.blocking_send(AppCommand::Exit); // Signal tokio to exit
    info!("Waiting for tokio thread to exit...");
    let _ = tokio_handle.join();
    info!("Tokio thread exited");

    if let Some(handle) = daemon_handle {
        info!("Waiting for daemon to shutdown...");
        // Use a timeout to avoid blocking forever
        match wait_for_daemon_shutdown(handle, std::time::Duration::from_secs(10)) {
            Ok(_) => info!("Daemon shutdown successfully"),
            Err(_) => {
                error!("Daemon shutdown timed out after 10 seconds, forcing exit...");
                // Force kill the daemon process if it's still running
                force_kill_daemon(instance_id);
            }
        }
    }

    info!("wakem shutdown complete");
    tray_result
}

/// Wait for daemon thread to shutdown with timeout
#[cfg(target_os = "windows")]
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

/// Force kill the daemon process for a specific instance
#[cfg(target_os = "windows")]
fn force_kill_daemon(instance_id: u32) {
    use std::process::Command;
    use std::process::Stdio;

    // Try to find and kill only the specific instance by its window title
    // Instance 0 uses "wakemd", others use "wakemd-instance{N}"
    let window_title = if instance_id == 0 {
        "wakemd".to_string()
    } else {
        format!("wakemd-instance{}", instance_id)
    };

    // First try to kill by window title (more specific)
    let output = Command::new("taskkill")
        .args(["/F", "/FI", &format!("WINDOWTITLE eq {}", window_title)])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(result) if result.status.success() => {
            info!("Successfully killed daemon instance {}", instance_id);
            return;
        }
        _ => {
            // Fallback: try to find process by checking command line arguments
            // This is less precise but better than killing all wakem.exe
            debug!("Could not kill by window title, trying alternative method");
        }
    }

    // Last resort: try to find process by command line arguments using PowerShell
    // This is more precise than killing all wakem.exe processes
    warn!(
        "Falling back to killing wakem.exe by instance ID {}. This may affect other instances if PowerShell is unavailable.",
        instance_id
    );
    let ps_script = if instance_id == 0 {
        r#"Get-Process wakem -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -notmatch '--instance' } | Stop-Process -Force"#.to_string()
    } else {
        format!(
            r#"Get-Process wakem -ErrorAction SilentlyContinue | Where-Object {{ $_.CommandLine -match '--instance {}' }} | Stop-Process -Force"#,
            instance_id
        )
    };

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(result) if result.status.success() => {
            info!(
                "Successfully killed daemon instance {} via PowerShell",
                instance_id
            );
        }
        _ => {
            error!(
                "Failed to kill daemon instance {} via PowerShell",
                instance_id
            );
            error!("You may need to manually stop the process");
        }
    }
}

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

/// Run tokio runtime in background thread for Windows tray
#[cfg(target_os = "windows")]
fn run_tokio_for_tray(cmd_rx: mpsc::Receiver<AppCommand>, instance_id: u32) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(connect_and_handle_tray_commands(
        cmd_rx,
        instance_id,
        || {
            platform::windows::stop_tray();
        },
        open_config_folder_sync,
    ));
}

/// Run system tray (macOS)
/// On macOS, NSApplication must run on the main thread, so we spawn tokio in a background thread
#[cfg(target_os = "macos")]
fn run_tray_sync(
    instance_id: u32,
    auto_start_daemon: bool,
    _detach_console: bool,
) -> Result<()> {
    use platform::macos::run_tray_event_loop;

    info!("wakem starting (instance {})...", instance_id);

    // Create async channels for communication between tray and tokio
    let (cmd_tx, cmd_rx) = mpsc::channel::<AppCommand>(16);
    let cmd_tx_for_tray = cmd_tx.clone();

    // Start tokio runtime in background thread immediately - don't wait!
    let tokio_handle = thread::spawn(move || {
        run_tokio_for_tray(cmd_rx, instance_id);
    });

    // Auto-start daemon in background if needed (completely independent)
    let daemon_handle = if auto_start_daemon {
        Some(thread::spawn(move || {
            // Check and start daemon if needed
            if !is_daemon_running(instance_id) {
                info!("Daemon not running, auto-starting...");
                if let Err(e) = run_daemon(instance_id, None) {
                    error!("Daemon exited with error: {}", e);
                }
            } else {
                info!("Daemon already running");
            }
        }))
    } else {
        None
    };

    // Run tray on the main thread immediately - no blocking before this!
    info!("Starting tray on main thread...");
    if let Err(e) = run_tray_event_loop(move |cmd| {
        let _ = cmd_tx_for_tray.blocking_send(cmd);
    }) {
        error!("Tray error: {}", e);
    }

    // Cleanup: signal tokio to stop, wait for threads
    let _ = cmd_tx.blocking_send(AppCommand::Exit); // Signal tokio to exit
    let _ = tokio_handle.join();

    // Wait for daemon thread if we started it
    if let Some(handle) = daemon_handle {
        info!("Waiting for daemon to shutdown...");
        let _ = handle.join();
    }

    info!("wakem shutdown complete");
    Ok(())
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
        open_config_folder_macos_sync,
    ));
}

/// Open config folder (macOS) - sync version
#[cfg(target_os = "macos")]
fn open_config_folder_macos_sync(instance_id: u32) -> Result<()> {
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

/// Open config folder - sync version
fn open_config_folder_sync(instance_id: u32) -> Result<()> {
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

/// Execute an async operation with a daemon client connection
/// Creates a single-threaded runtime, connects to the specified instance, and runs the operation.
/// The closure receives ownership of the client to avoid async lifetime issues.
fn run_with_client<F, Fut>(instance_id: u32, op: F) -> Result<()>
where
    F: FnOnce(DaemonClient) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        op(client).await
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

/// List running instances - sync version
fn cmd_instances_sync() -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let instances = ipc::discover_instances().await;

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
