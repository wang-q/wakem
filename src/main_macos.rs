//! macOS-specific main function
//!
//! On macOS, NSApplication must run on the main thread.
//! So we run the tray on the main thread and spawn tokio runtime in a background thread.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use tracing::{error, info};

use wakem::client::DaemonClient;
use wakem::config;
use wakem::platform::macos::{run_tray_event_loop, AppCommand};

const IPC_CHANNEL_CAPACITY: usize = 100;
const DEFAULT_INSTANCE_ID: u32 = 0;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    info!("wakem client starting (macOS main thread)...");

    // Create channels for communication between tray and tokio
    let (cmd_tx, cmd_rx): (Sender<AppCommand>, Receiver<AppCommand>) = channel();
    let cmd_tx_for_tray = cmd_tx.clone();

    // Spawn tokio runtime in a background thread
    let tokio_handle = thread::spawn(move || {
        run_tokio_runtime(cmd_rx, cmd_tx);
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
}

fn run_tokio_runtime(cmd_rx: Receiver<AppCommand>, cmd_tx: Sender<AppCommand>) {
    // Create tokio runtime
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async {
        info!("Tokio runtime started in background thread");

        // Connect to daemon
        let mut client_option: Option<DaemonClient> = None;
        let mut client = DaemonClient::new();

        match client.connect_to_instance(DEFAULT_INSTANCE_ID).await {
            Ok(_) => {
                info!("Connected to wakemd instance {}", DEFAULT_INSTANCE_ID);

                // Get initial status
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
                    DEFAULT_INSTANCE_ID
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
                    open_config_folder();
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

fn open_config_folder() {
    use std::process::Command;

    // Get config folder path
    let config_path = config::resolve_config_file_path(None, 0)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_default()
        });

    // Open folder using open command
    let _ = Command::new("open").arg(config_path).spawn();
}
