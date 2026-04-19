use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

mod cli;
mod client;
mod config;
mod daemon;
mod ipc;
mod platform;
mod runtime;
mod types;
mod window;

use client::DaemonClient;
use window::{AppCommand, MessageWindow};

/// wakem - Window Adjust, Keyboard Enhance, and Mouse
#[derive(Parser)]
#[command(name = "wakem")]
#[command(about = "wakem - Window/Keyboard/Mouse Enhancer")]
#[command(version)]
struct Cli {
    /// 子命令
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动守护进程
    Daemon,
    /// 获取服务端状态
    Status,
    /// 重载配置
    Reload,
    /// 启用映射
    Enable,
    /// 禁用映射
    Disable,
    /// 打开配置文件夹
    Config,
    /// 运行系统托盘（默认）
    Tray,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Daemon) => run_daemon().await,
        Some(Commands::Status) => cmd_status().await,
        Some(Commands::Reload) => cmd_reload().await,
        Some(Commands::Enable) => cmd_enable().await,
        Some(Commands::Disable) => cmd_disable().await,
        Some(Commands::Config) => cmd_config().await,
        Some(Commands::Tray) | None => run_tray().await,
    }
}

/// 运行守护进程
async fn run_daemon() -> Result<()> {
    info!("wakemd starting...");
    daemon::run_server().await?;
    info!("wakemd shutting down...");
    Ok(())
}

/// 获取服务端状态
async fn cmd_status() -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect().await {
        Ok(_) => {
            match client.get_status().await {
                Ok((active, loaded)) => {
                    println!("wakemd status:");
                    println!("  Active: {}", if active { "enabled" } else { "disabled" });
                    println!("  Config loaded: {}", if loaded { "yes" } else { "no" });
                }
                Err(e) => {
                    eprintln!("Failed to get status: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
            eprintln!("Please make sure wakemd is running");
        }
    }
    Ok(())
}

/// 重载配置
async fn cmd_reload() -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect().await {
        Ok(_) => {
            match client.reload_config().await {
                Ok(_) => {
                    println!("Configuration reloaded successfully");
                }
                Err(e) => {
                    eprintln!("Failed to reload config: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
        }
    }
    Ok(())
}

/// 启用映射
async fn cmd_enable() -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect().await {
        Ok(_) => {
            match client.set_active(true).await {
                Ok(_) => {
                    println!("wakem enabled");
                }
                Err(e) => {
                    eprintln!("Failed to enable: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
        }
    }
    Ok(())
}

/// 禁用映射
async fn cmd_disable() -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect().await {
        Ok(_) => {
            match client.set_active(false).await {
                Ok(_) => {
                    println!("wakem disabled");
                }
                Err(e) => {
                    eprintln!("Failed to disable: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
        }
    }
    Ok(())
}

/// 打开配置文件夹
async fn cmd_config() -> Result<()> {
    open_config_folder().await?;
    println!("Config folder opened");
    Ok(())
}

/// 运行系统托盘
async fn run_tray() -> Result<()> {
    info!("wakem client starting...");

    // 创建命令通道
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<AppCommand>(100);

    // 创建消息窗口
    let mut window = MessageWindow::new()?;

    // 设置命令回调
    let cmd_tx_for_callback = cmd_tx.clone();
    window.set_command_callback(move |cmd| {
        let _ = cmd_tx_for_callback.try_send(cmd);
    });

    // 初始化托盘
    window.init_tray()?;

    // 连接到服务端
    let mut client = DaemonClient::new();
    let connected = match client.connect().await {
        Ok(_) => {
            info!("Connected to wakemd");

            // 获取初始状态
            match client.get_status().await {
                Ok((active, loaded)) => {
                    info!("Daemon status: active={}, config_loaded={}", active, loaded);
                }
                Err(e) => {
                    error!("Failed to get status: {}", e);
                }
            }
            true
        }
        Err(e) => {
            error!("Failed to connect to daemon: {}", e);
            error!("Please make sure wakemd is running");
            false
        }
    };

    // 使用 Arc<AtomicBool> 来共享退出状态
    let should_exit = Arc::new(AtomicBool::new(false));
    let should_exit_clone = should_exit.clone();

    // 启动命令处理任务
    let cmd_tx_clone = cmd_tx.clone();
    let mut client_option = if connected { Some(client) } else { None };

    let command_handler = tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            if should_exit_clone.load(Ordering::SeqCst) {
                break;
            }

            match cmd {
                AppCommand::ToggleActive => {
                    info!("Toggle active command received");
                    if let Some(ref mut client) = client_option {
                        // 先获取当前状态
                        match client.get_status().await {
                            Ok((current_active, _)) => {
                                let new_active = !current_active;
                                match client.set_active(new_active).await {
                                    Ok(_) => {
                                        info!("Daemon active state changed to: {}", new_active);
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
                    should_exit_clone.store(true, Ordering::SeqCst);
                    let _ = cmd_tx_clone.send(AppCommand::Exit).await;
                    break;
                }
            }
        }
    });

    // 运行消息循环（在单独线程中，因为 GetMessageW 是阻塞的）
    let window_handle = Arc::new(std::sync::Mutex::new(window));
    let window_clone = window_handle.clone();

    let msg_thread = std::thread::spawn(move || {
        let window = window_clone.lock().unwrap();
        if let Err(e) = window.run() {
            error!("Message loop error: {}", e);
        }
    });

    // 等待命令处理任务完成（即收到退出命令）
    let _ = command_handler.await;

    info!("Shutdown signal received");

    // 停止消息循环
    {
        let window = window_handle.lock().unwrap();
        window.stop();
    }

    // 等待消息线程结束
    let _ = msg_thread.join();

    info!("wakem client shutdown complete");
    Ok(())
}

/// 打开配置文件夹
async fn open_config_folder() -> Result<()> {
    use std::process::Command;

    // 获取配置文件夹路径
    let config_path = config::resolve_config_file_path(None)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| {
            std::env::var("USERPROFILE")
                .map(|p| std::path::PathBuf::from(p))
                .unwrap_or_default()
        });

    // 使用 explorer 打开文件夹
    Command::new("explorer").arg(config_path).spawn()?;

    Ok(())
}
