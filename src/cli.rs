//! 命令行参数解析

use clap::{Parser, Subcommand};

/// wakem - Window Adjust, Keyboard Enhance, and Mouse
#[derive(Parser)]
#[command(name = "wakem")]
#[command(about = "wakem - Window/Keyboard/Mouse Enhancer")]
#[command(version)]
pub struct Cli {
    /// 子命令
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
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
