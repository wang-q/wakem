use anyhow::Result;
use clap::Parser;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

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
use config::Config;
use constants::IPC_CHANNEL_CAPACITY;

// Platform-specific imports
#[cfg(target_os = "windows")]
use platform::windows::{run_tray_message_loop, stop_tray, AppCommand};

#[cfg(target_os = "macos")]
use platform::macos::{run_tray_event_loop, AppCommand};

/// Simple daemon command executor macro to reduce boilerplate for parameterless methods
macro_rules! simple_daemon_command {
    ($name:ident, $method:ident, $success_msg:expr) => {
        async fn $name(instance_id: u32) -> Result<()> {
            execute_daemon_command(instance_id, |client| {
                Box::pin(async move {
                    client.$method().await?;
                    println!($success_msg);
                    Ok(())
                })
            })
            .await
        }
    };
}

// Use macro to generate parameterless command handlers
simple_daemon_command!(
    cmd_reload,
    reload_config,
    "Configuration reloaded successfully"
);
simple_daemon_command!(cmd_save, save_config, "Configuration saved successfully");

/// Enable mapping
async fn cmd_enable(instance_id: u32) -> Result<()> {
    execute_daemon_command(instance_id, |client| {
        Box::pin(async move {
            client.set_active(true).await?;
            println!("wakem enabled");
            Ok(())
        })
    })
    .await
}

/// Disable mapping
async fn cmd_disable(instance_id: u32) -> Result<()> {
    execute_daemon_command(instance_id, |client| {
        Box::pin(async move {
            client.set_active(false).await?;
            println!("wakem disabled");
            Ok(())
        })
    })
    .await
}

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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging (using log level from config or default info)
    init_logging(&cli);

    match cli.command {
        Some(Commands::Daemon { instance }) => run_daemon(instance).await,
        Some(Commands::Status) => cmd_status(cli.instance).await,
        Some(Commands::Reload) => cmd_reload(cli.instance).await,
        Some(Commands::Save) => cmd_save(cli.instance).await,
        Some(Commands::Enable) => cmd_enable(cli.instance).await,
        Some(Commands::Disable) => cmd_disable(cli.instance).await,
        Some(Commands::Config) => cmd_config().await,
        Some(Commands::Instances) => cmd_instances().await,
        Some(Commands::Record { name }) => cmd_record(cli.instance, &name).await,
        Some(Commands::StopRecord) => cmd_stop_record(cli.instance).await,
        Some(Commands::Play { name }) => cmd_play(cli.instance, &name).await,
        Some(Commands::Macros) => cmd_macros(cli.instance).await,
        Some(Commands::BindMacro {
            macro_name,
            trigger,
        }) => cmd_bind_macro(cli.instance, &macro_name, &trigger).await,
        Some(Commands::DeleteMacro { name }) => {
            cmd_delete_macro(cli.instance, &name).await
        }
        Some(Commands::Tray) | None => run_tray(cli.instance).await,
    }
}

/// Start the daemon
async fn run_daemon(instance_id: u32) -> Result<()> {
    info!("Starting wakemd (instance {})...", instance_id);

    daemon::run_server(instance_id).await
}

/// Generic daemon command executor to reduce boilerplate
/// Improvement: now propagates connection errors, operation errors also return Err
async fn execute_daemon_command<F>(instance_id: u32, operation: F) -> Result<()>
where
    F: FnOnce(
        &mut DaemonClient,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>,
    >,
{
    let mut client = DaemonClient::new();
    client.connect_to_instance(instance_id).await?;

    // Execute operation and propagate errors
    operation(&mut client).await
}

/// Get server status
async fn cmd_status(instance_id: u32) -> Result<()> {
    execute_daemon_command(instance_id, |client| {
        Box::pin(async move {
            let (active, loaded) = client.get_status().await?;
            println!("wakemd instance {}:", instance_id);
            println!("  Active: {}", if active { "yes" } else { "no" });
            println!("  Config loaded: {}", if loaded { "yes" } else { "no" });
            Ok(())
        })
    })
    .await
}

/// Open config folder
async fn cmd_config() -> Result<()> {
    open_config_folder().await?;
    println!("Config folder opened");
    Ok(())
}

/// List running instances
async fn cmd_instances() -> Result<()> {
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
}

/// Run system tray (Windows only)
#[cfg(target_os = "windows")]
async fn run_tray(instance_id: u32) -> Result<()> {
    info!("wakem client starting (instance {})...", instance_id);

    // Create command channel for communication between tray and async code
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<AppCommand>(IPC_CHANNEL_CAPACITY);

    // Clone for use in callback
    let cmd_tx_for_callback = cmd_tx.clone();

    // Spawn the tray message loop in a dedicated thread
    // (using std::thread instead of spawn_blocking for better control)
    let tray_handle = std::thread::spawn(move || {
        run_tray_message_loop(move |cmd| {
            let _ = cmd_tx_for_callback.try_send(cmd);
        })
    });

    // Connect to daemon
    let mut client_option: Option<DaemonClient> = None;
    let mut client = DaemonClient::new();

    match client.connect_to_instance(instance_id).await {
        Ok(_) => {
            info!("Connected to wakemd instance {}", instance_id);

            // Get initial status
            match client.get_status().await {
                Ok((active, loaded)) => {
                    info!("Daemon status: active={}, config_loaded={}", active, loaded);
                }
                Err(e) => {
                    error!("Failed to get status: {}", e);
                }
            }

            client_option = Some(client);
        }
        Err(e) => {
            error!("Failed to connect to daemon: {}", e);
            error!(
                "Please make sure wakemd --instance {} is running",
                instance_id
            );
        }
    }

    // Handle commands from tray
    let mut should_exit = false;
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
                if let Err(e) = open_config_folder().await {
                    error!("Failed to open config folder: {}", e);
                }
            }
            AppCommand::Exit => {
                info!("Exit command received");
                should_exit = true;
                break;
            }
        }
    }

    // Stop the tray message loop (if not already stopped)
    if should_exit {
        stop_tray();
    }

    // Wait for tray thread to complete (with timeout to avoid hanging)
    let timeout = tokio::time::Duration::from_secs(5);
    match tokio::time::timeout(timeout, async {
        tray_handle.join().unwrap_or_else(|e| {
            error!("Tray thread panicked: {:?}", e);
            Ok(())
        })
    })
    .await
    {
        Ok(result) => {
            if let Err(e) = result {
                error!("Tray thread error: {}", e);
            }
        }
        Err(_) => {
            error!("Tray thread did not complete within timeout");
        }
    }

    info!("wakem client shutdown complete");
    Ok(())
}

/// Run system tray (macOS)
/// On macOS, NSApplication must run on the main thread, so we spawn tokio in a background thread
#[cfg(target_os = "macos")]
async fn run_tray(instance_id: u32) -> Result<()> {
    use platform::macos::run_tray_event_loop;
    use std::thread;
    use std::time::Duration;

    info!("wakem client starting (instance {})...", instance_id);

    // Create channels for communication between tray and tokio
    let (cmd_tx, cmd_rx): (Sender<AppCommand>, Receiver<AppCommand>) = channel();
    let cmd_tx_for_tray = cmd_tx.clone();

    // Spawn tokio runtime in a background thread
    let tokio_handle = thread::spawn(move || {
        run_tokio_for_tray(cmd_rx, instance_id);
    });

    // Give tokio a moment to start
    thread::sleep(Duration::from_millis(500));

    // Run tray on the main thread (required by macOS)
    info!("Starting tray on main thread...");
    if let Err(e) = run_tray_event_loop(move |cmd| {
        let _ = cmd_tx_for_tray.send(cmd);
    }) {
        error!("Tray error: {}", e);
    }

    // Wait for tokio thread to finish
    let _ = tokio_handle.join();

    info!("wakem client shutdown complete");
    Ok(())
}

/// Run tokio runtime in background thread for macOS tray
#[cfg(target_os = "macos")]
fn run_tokio_for_tray(cmd_rx: Receiver<AppCommand>, instance_id: u32) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async {
        info!("Tokio runtime started in background thread");

        // Connect to daemon
        let mut client_option: Option<DaemonClient> = None;
        let mut client = DaemonClient::new();

        match client.connect_to_instance(instance_id).await {
            Ok(_) => {
                info!("Connected to wakemd instance {}", instance_id);

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
            }
            Err(e) => {
                error!("Failed to connect to daemon: {}", e);
                error!(
                    "Please make sure wakemd --instance {} is running",
                    instance_id
                );
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
                    if let Err(e) = open_config_folder_macos().await {
                        error!("Failed to open config folder: {}", e);
                    }
                }
                AppCommand::Exit => {
                    info!("Exit command received");
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

/// Open config folder (macOS)
#[cfg(target_os = "macos")]
async fn open_config_folder_macos() -> Result<()> {
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

/// Open config folder
async fn open_config_folder() -> Result<()> {
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

/// Record macro
async fn cmd_record(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    execute_daemon_command(instance_id, |client| {
        Box::pin(async move {
            client.start_macro_recording(&name_owned).await?;
            println!("Recording macro '{}'...", name_owned);
            println!("Press Ctrl+Shift+Esc to stop recording");
            Ok(())
        })
    })
    .await
}

/// Stop recording macro
async fn cmd_stop_record(instance_id: u32) -> Result<()> {
    execute_daemon_command(instance_id, |client| {
        Box::pin(async move {
            let (name, count) = client.stop_macro_recording().await?;
            println!("Macro '{}' saved with {} actions", name, count);
            Ok(())
        })
    })
    .await
}

/// Play macro
async fn cmd_play(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    execute_daemon_command(instance_id, |client| {
        Box::pin(async move {
            client.play_macro(&name_owned).await?;
            println!("Playing macro '{}'", name_owned);
            Ok(())
        })
    })
    .await
}

/// List all macros
async fn cmd_macros(instance_id: u32) -> Result<()> {
    execute_daemon_command(instance_id, |client| {
        Box::pin(async move {
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
    })
    .await
}

/// Bind macro to trigger key
async fn cmd_bind_macro(
    instance_id: u32,
    macro_name: &str,
    trigger: &str,
) -> Result<()> {
    let macro_name_owned = macro_name.to_string();
    let trigger_owned = trigger.to_string();
    execute_daemon_command(instance_id, |client| {
        Box::pin(async move {
            client.bind_macro(&macro_name_owned, &trigger_owned).await?;
            println!("Macro '{}' bound to '{}'", macro_name_owned, trigger_owned);
            Ok(())
        })
    })
    .await
}

/// Delete macro
async fn cmd_delete_macro(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    execute_daemon_command(instance_id, |client| {
        Box::pin(async move {
            client.delete_macro(&name_owned).await?;
            println!("Macro '{}' deleted", name_owned);
            Ok(())
        })
    })
    .await
}
