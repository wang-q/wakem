//! Command line argument parsing
//!
//! Unified CLI definition, serving as the single source of command line interface for the entire project.

use clap::{Parser, Subcommand};

/// wakem - Window Adjust, Keyboard Enhance, and Mouse
#[derive(Parser)]
#[command(name = "wakem")]
#[command(about = "wakem - Window/Keyboard/Mouse Enhancer")]
#[command(version)]
pub struct Cli {
    /// Instance ID (for multi-instance support)
    #[arg(short, long, default_value = "0")]
    pub instance: u32,

    /// Subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start daemon
    Daemon {
        /// Instance ID
        #[arg(short, long, default_value = "0")]
        instance: u32,
    },
    /// Get server status
    Status,
    /// Reload configuration
    Reload,
    /// Save current configuration to file
    Save,
    /// Enable mapping
    Enable,
    /// Disable mapping
    Disable,
    /// Open config folder
    Config,
    /// List running instances
    Instances,
    /// Run system tray (default)
    Tray,
    /// Record macro
    Record {
        /// Macro name
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
