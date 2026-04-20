//! 命令行参数解析
//!
//! 统一的 CLI 定义，作为整个项目的命令行接口唯一来源。

use clap::{Parser, Subcommand};

/// wakem - Window Adjust, Keyboard Enhance, and Mouse
#[derive(Parser)]
#[command(name = "wakem")]
#[command(about = "wakem - Window/Keyboard/Mouse Enhancer")]
#[command(version)]
pub struct Cli {
    /// 实例ID（用于多实例）
    #[arg(short, long, default_value = "0")]
    pub instance: u32,

    /// 子命令
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
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
