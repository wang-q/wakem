use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

mod client;
mod platform;
mod window;

use client::DaemonClient;
use window::{AppCommand, MessageWindow};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

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
    let config_path = wakem_common::resolve_config_file_path(None)
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
