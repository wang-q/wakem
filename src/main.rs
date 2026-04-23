use anyhow::Result;
use clap::Parser;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use tracing::{debug, error, info};

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
use platform::windows::{run_tray_message_loop, AppCommand};

#[cfg(target_os = "macos")]
use platform::macos::{run_tray_event_loop, AppCommand};

/// Initialize logging system with support for reading log level from config file
fn init_logging(cli: &Cli) {
    let log_level = if let Some(config_path) =
        config::resolve_config_file_path(None, cli.instance)
    {
        // Try to read log level from config file
        config::Config::from_file(&config_path)
            .map(|cfg| cfg.log_level)
            .unwrap_or_else(|_| "info".to_string())
    } else {
        "info".to_string()
    };

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    info!("Logging initialized with level: {}", log_level);
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging (using log level from config or default info)
    init_logging(&cli);

    match cli.command {
        Some(Commands::Daemon { instance }) => run_daemon(instance),
        Some(Commands::Status) => cmd_status_sync(cli.instance),
        Some(Commands::Reload) => cmd_reload_sync(cli.instance),
        Some(Commands::Save) => cmd_save_sync(cli.instance),
        Some(Commands::Enable) => cmd_enable_sync(cli.instance),
        Some(Commands::Disable) => cmd_disable_sync(cli.instance),
        Some(Commands::Config) => cmd_config_sync(),
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
        Some(Commands::Tray) => run_tray_sync(cli.instance, false), // Tray only, don't auto-start daemon
        None => run_tray_sync(cli.instance, true), // Default: auto-start daemon if not running
    }
}

/// Start the daemon with multi-thread tokio runtime
fn run_daemon(instance_id: u32) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        info!("Starting wakemd (instance {})...", instance_id);
        daemon::run_server(instance_id).await
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
fn run_tray_sync(instance_id: u32, auto_start_daemon: bool) -> Result<()> {
    info!("wakem starting (instance {})...", instance_id);

    // Create sync channel for communication between tray and tokio
    let (cmd_tx, cmd_rx): (Sender<AppCommand>, Receiver<AppCommand>) = channel();
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
                if let Err(e) = run_daemon(instance_id) {
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
        let _ = cmd_tx_for_tray.send(cmd);
    });

    // Cleanup: signal tokio to stop, wait for threads
    let _ = cmd_tx.send(AppCommand::Exit); // Signal tokio to exit
    let _ = tokio_handle.join();

    if let Some(handle) = daemon_handle {
        info!("Waiting for daemon to shutdown...");
        let _ = handle.join();
    }

    info!("wakem shutdown complete");
    tray_result
}

/// Run tokio runtime in background thread for Windows tray
#[cfg(target_os = "windows")]
fn run_tokio_for_tray(cmd_rx: Receiver<AppCommand>, instance_id: u32) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async {
        info!("Tokio runtime started in background thread");

        // Connect to daemon with retry logic
        let mut client_option: Option<DaemonClient> = None;
        let mut client = DaemonClient::new();
        let max_retries = 10; // Retry up to 10 times (5 seconds total)
        let retry_delay = tokio::time::Duration::from_millis(500);

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

        // Handle commands from tray
        while let Ok(cmd) = cmd_rx.recv() {
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
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to get status: {}", e);
                            }
                        }
                    } else {
                        error!("Not connected to daemon, command ignored");
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
                            }
                        }
                    } else {
                        error!("Not connected to daemon, command ignored");
                    }
                }
                AppCommand::OpenConfigFolder => {
                    info!("Open config folder command received");
                    if let Err(e) = open_config_folder_sync() {
                        error!("Failed to open config folder: {}", e);
                    }
                }
                AppCommand::Exit => {
                    info!("Exit command received");

                    // Shutdown daemon before exiting
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

                    // Stop the tray message loop
                    // This sends WM_DESTROY to the tray window, causing run_tray_message_loop to return
                    platform::windows::stop_tray();

                    break;
                }
            }
        }

        info!("Tokio runtime shutting down");
    });
}

/// Run system tray (macOS)
/// On macOS, NSApplication must run on the main thread, so we spawn tokio in a background thread
#[cfg(target_os = "macos")]
fn run_tray_sync(instance_id: u32, auto_start_daemon: bool) -> Result<()> {
    use platform::macos::run_tray_event_loop;

    info!("wakem starting (instance {})...", instance_id);

    // Create channels for communication between tray and tokio
    let (cmd_tx, cmd_rx): (Sender<AppCommand>, Receiver<AppCommand>) = channel();
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
                if let Err(e) = run_daemon(instance_id) {
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
        let _ = cmd_tx_for_tray.send(cmd);
    }) {
        error!("Tray error: {}", e);
    }

    // Cleanup: signal tokio to stop, wait for threads
    let _ = cmd_tx.send(AppCommand::Exit); // Signal tokio to exit
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
fn run_tokio_for_tray(cmd_rx: Receiver<AppCommand>, instance_id: u32) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async {
        info!("Tokio runtime started in background thread");

        // Connect to daemon with retry logic
        let mut client_option: Option<DaemonClient> = None;
        let mut client = DaemonClient::new();
        let max_retries = 10; // Retry up to 10 times (5 seconds total)
        let retry_delay = tokio::time::Duration::from_millis(500);

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
                        error!("Failed to connect to daemon after {} attempts: {}", max_retries, e);
                        error!(
                            "Please make sure wakemd --instance {} is running",
                            instance_id
                        );
                    }
                }
            }
        }

        // Handle commands from tray
        while let Ok(cmd) = cmd_rx.recv() {
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
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to get status: {}", e);
                            }
                        }
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
                            }
                        }
                    }
                }
                AppCommand::OpenConfigFolder => {
                    info!("Open config folder command received");
                    if let Err(e) = open_config_folder_macos_sync() {
                        error!("Failed to open config folder: {}", e);
                    }
                }
                AppCommand::Exit => {
                    info!("Exit command received");

                    // Shutdown daemon before exiting
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

                    // Terminate the app
                    unsafe {
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
                    }
                    break;
                }
            }
        }

        info!("Tokio runtime shutting down");
    });
}

/// Open config folder (macOS) - sync version
#[cfg(target_os = "macos")]
fn open_config_folder_macos_sync() -> Result<()> {
    use std::process::Command;

    let config_path = config::resolve_config_file_path(None, 0)
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
fn open_config_folder_sync() -> Result<()> {
    use std::process::Command;

    // Get config folder path
    let config_path = config::resolve_config_file_path(None, 0)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| {
            std::env::var("USERPROFILE")
                .map(std::path::PathBuf::from)
                .unwrap_or_default()
        });

    // Open folder using explorer
    Command::new("explorer").arg(config_path).spawn()?;

    Ok(())
}

/// Get server status - sync version
fn cmd_status_sync(instance_id: u32) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        let (active, loaded) = client.get_status().await?;
        println!("wakemd instance {}:", instance_id);
        println!("  Active: {}", if active { "yes" } else { "no" });
        println!("  Config loaded: {}", if loaded { "yes" } else { "no" });
        Ok(())
    })
}

/// Reload configuration - sync version
fn cmd_reload_sync(instance_id: u32) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        client.reload_config().await?;
        println!("Configuration reloaded successfully");
        Ok(())
    })
}

/// Save configuration - sync version
fn cmd_save_sync(instance_id: u32) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        client.save_config().await?;
        println!("Configuration saved successfully");
        Ok(())
    })
}

/// Enable mapping - sync version
fn cmd_enable_sync(instance_id: u32) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        client.set_active(true).await?;
        println!("wakem enabled");
        Ok(())
    })
}

/// Disable mapping - sync version
fn cmd_disable_sync(instance_id: u32) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        client.set_active(false).await?;
        println!("wakem disabled");
        Ok(())
    })
}

/// Open config folder - sync version
fn cmd_config_sync() -> Result<()> {
    open_config_folder_sync()?;
    println!("Config folder opened");
    Ok(())
}

/// List running instances - sync version
fn cmd_instances_sync() -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let instances = ipc::discovery::discover_instances().await;

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
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        client.start_macro_recording(&name_owned).await?;
        println!("Recording macro '{}'...", name_owned);
        println!("Press Ctrl+Shift+Esc to stop recording");
        Ok(())
    })
}

/// Stop recording macro - sync version
fn cmd_stop_record_sync(instance_id: u32) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        let (name, count) = client.stop_macro_recording().await?;
        println!("Macro '{}' saved with {} actions", name, count);
        Ok(())
    })
}

/// Play macro - sync version
fn cmd_play_sync(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        client.play_macro(&name_owned).await?;
        println!("Playing macro '{}'", name_owned);
        Ok(())
    })
}

/// List all macros - sync version
fn cmd_macros_sync(instance_id: u32) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
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
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        client.bind_macro(&macro_name_owned, &trigger_owned).await?;
        println!("Macro '{}' bound to '{}'", macro_name_owned, trigger_owned);
        Ok(())
    })
}

/// Delete macro - sync version
fn cmd_delete_macro_sync(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        client.delete_macro(&name_owned).await?;
        println!("Macro '{}' deleted", name_owned);
        Ok(())
    })
}
