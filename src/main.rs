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
use config::Config;
use window::{AppCommand, MessageWindow};

/// wakem - Window Adjust, Keyboard Enhance, and Mouse
#[derive(Parser)]
#[command(name = "wakem")]
#[command(about = "wakem - Window/Keyboard/Mouse Enhancer")]
#[command(version)]
struct Cli {
    /// 实例ID（用于多实例）
    #[arg(short, long, default_value = "0")]
    instance: u32,

    /// 子命令
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动守护进程
    Daemon {
        /// 实例ID
        #[arg(short, long, default_value = "0")]
        instance: u32,
    },
    /// 获取服务端状态
    Status,
    /// 重载配置
    Reload,
    /// 保存当前配置到文件
    Save,
    /// 启用映射
    Enable,
    /// 禁用映射
    Disable,
    /// 打开配置文件夹
    Config,
    /// 列出运行中的实例
    Instances,
    /// 运行系统托盘（默认）
    Tray,
    /// 录制宏
    Record {
        /// 宏名称
        name: String,
    },
    /// 停止录制宏
    StopRecord,
    /// 播放宏
    Play {
        /// 宏名称
        name: String,
    },
    /// 列出所有宏
    Macros,
    /// 绑定宏到触发键
    BindMacro {
        /// 宏名称
        macro_name: String,
        /// 触发键
        trigger: String,
    },
    /// 删除宏
    DeleteMacro {
        /// 宏名称
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt().with_env_filter("info").init();

    let cli = Cli::parse();

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

/// 启动守护进程
async fn run_daemon(instance_id: u32) -> Result<()> {
    info!("Starting wakemd (instance {})...", instance_id);

    daemon::run_server(instance_id).await
}

/// 获取服务端状态
async fn cmd_status(instance_id: u32) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.get_status().await {
            Ok((active, loaded)) => {
                println!("wakemd instance {}:", instance_id);
                println!("  Active: {}", if active { "yes" } else { "no" });
                println!("  Config loaded: {}", if loaded { "yes" } else { "no" });
            }
            Err(e) => {
                eprintln!("Failed to get status: {}", e);
            }
        },
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
        }
    }
    Ok(())
}

/// 重载配置
async fn cmd_reload(instance_id: u32) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.reload_config().await {
            Ok(_) => {
                println!("Configuration reloaded successfully");
            }
            Err(e) => {
                eprintln!("Failed to reload config: {}", e);
            }
        },
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
        }
    }
    Ok(())
}

/// 保存配置到文件
async fn cmd_save(instance_id: u32) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.save_config().await {
            Ok(_) => {
                println!("Configuration saved successfully");
            }
            Err(e) => {
                eprintln!("Failed to save config: {}", e);
            }
        },
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
        }
    }
    Ok(())
}

/// 启用映射
async fn cmd_enable(instance_id: u32) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.set_active(true).await {
            Ok(_) => {
                println!("wakem enabled");
            }
            Err(e) => {
                eprintln!("Failed to enable: {}", e);
            }
        },
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
        }
    }
    Ok(())
}

/// 禁用映射
async fn cmd_disable(instance_id: u32) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.set_active(false).await {
            Ok(_) => {
                println!("wakem disabled");
            }
            Err(e) => {
                eprintln!("Failed to disable: {}", e);
            }
        },
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

/// 列出运行中的实例
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

/// 运行系统托盘
async fn run_tray(instance_id: u32) -> Result<()> {
    info!("wakem client starting (instance {})...", instance_id);

    // 加载配置获取图标路径
    let icon_path =
        config::resolve_config_file_path(None, instance_id).and_then(|path| {
            Config::from_file(&path)
                .ok()
                .and_then(|cfg| cfg.icon_path)
                .or_else(|| {
                    // 尝试加载程序目录下的 assets/icon.ico
                    path.parent().map(|p| {
                        p.join("assets")
                            .join("icon.ico")
                            .to_string_lossy()
                            .to_string()
                    })
                })
        });

    // 创建命令通道
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<AppCommand>(100);

    // 创建消息窗口（带自定义图标）
    let mut window = MessageWindow::with_icon_path(icon_path)?;

    // 设置命令回调
    let cmd_tx_for_callback = cmd_tx.clone();
    window.set_command_callback(move |cmd| {
        let _ = cmd_tx_for_callback.try_send(cmd);
    });

    // 初始化托盘
    window.init_tray()?;

    // 连接到服务端
    let mut client = DaemonClient::new();
    let connected = match client.connect_to_instance(instance_id).await {
        Ok(_) => {
            info!("Connected to wakemd instance {}", instance_id);

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
            error!(
                "Please make sure wakemd --instance {} is running",
                instance_id
            );
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
    let config_path = config::resolve_config_file_path(None, 0)
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

/// 录制宏
async fn cmd_record(instance_id: u32, name: &str) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.start_macro_recording(name).await {
            Ok(_) => {
                println!("Recording macro '{}'...", name);
                println!("Press Ctrl+Shift+Esc to stop recording");
            }
            Err(e) => eprintln!("Failed to start recording: {}", e),
        },
        Err(e) => eprintln!("Failed to connect: {}", e),
    }
    Ok(())
}

/// 停止录制宏
async fn cmd_stop_record(instance_id: u32) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.stop_macro_recording().await {
            Ok((name, count)) => {
                println!("Macro '{}' saved with {} actions", name, count);
            }
            Err(e) => eprintln!("Failed to stop recording: {}", e),
        },
        Err(e) => eprintln!("Failed to connect: {}", e),
    }
    Ok(())
}

/// 播放宏
async fn cmd_play(instance_id: u32, name: &str) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.play_macro(name).await {
            Ok(_) => println!("Playing macro '{}'", name),
            Err(e) => eprintln!("Failed to play macro: {}", e),
        },
        Err(e) => eprintln!("Failed to connect: {}", e),
    }
    Ok(())
}

/// 列出所有宏
async fn cmd_macros(instance_id: u32) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.get_macros().await {
            Ok(macros) => {
                if macros.is_empty() {
                    println!("No macros recorded");
                } else {
                    println!("Available macros:");
                    for name in macros {
                        println!("  - {}", name);
                    }
                }
            }
            Err(e) => eprintln!("Failed to get macros: {}", e),
        },
        Err(e) => eprintln!("Failed to connect: {}", e),
    }
    Ok(())
}

/// 绑定宏到触发键
async fn cmd_bind_macro(
    instance_id: u32,
    macro_name: &str,
    trigger: &str,
) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.bind_macro(macro_name, trigger).await {
            Ok(_) => println!("Macro '{}' bound to '{}'", macro_name, trigger),
            Err(e) => eprintln!("Failed to bind macro: {}", e),
        },
        Err(e) => eprintln!("Failed to connect: {}", e),
    }
    Ok(())
}

/// 删除宏
async fn cmd_delete_macro(instance_id: u32, name: &str) -> Result<()> {
    let mut client = DaemonClient::new();
    match client.connect_to_instance(instance_id).await {
        Ok(_) => match client.delete_macro(name).await {
            Ok(_) => println!("Macro '{}' deleted", name),
            Err(e) => eprintln!("Failed to delete macro: {}", e),
        },
        Err(e) => eprintln!("Failed to connect: {}", e),
    }
    Ok(())
}
